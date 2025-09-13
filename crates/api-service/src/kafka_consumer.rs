use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::{debug, error, info, warn};
use rdkafka::{
    consumer::{StreamConsumer, Consumer},
    config::ClientConfig,
    message::Message,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;
use crate::database::DatabaseService;

/// Arbitrage opportunity from Kafka - matches the format published by hotpath engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaArbitrageOpportunity {
    pub signature: String,
    pub venue_a: String,
    pub venue_b: String,
    pub token_a: String,
    pub token_b: String,
    pub profit_bps: i32,
    pub slot: u64,
    pub estimated_net_profit_sol: f64,
    pub confidence_score: f64,
    pub timestamp: String, // RFC3339 format
    pub strategy: String,
}

/// Database model for persisting arbitrage opportunities
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DbArbitrageOpportunity {
    pub id: Uuid,
    pub signature: String,
    pub venue_a: String,
    pub venue_b: String,
    pub token_a: String,
    pub token_b: String,
    pub profit_bps: i32,
    pub slot: i64,
    pub estimated_net_profit_sol: f64,
    pub confidence_score: f64,
    pub timestamp: DateTime<Utc>,
    pub strategy: String,
    pub created_at: DateTime<Utc>,
}

pub struct ArbitrageKafkaConsumer {
    consumer: StreamConsumer,
    database: DatabaseService,
}

impl ArbitrageKafkaConsumer {
    pub fn new(kafka_broker: String, database: DatabaseService) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_broker)
            .set("group.id", "api-service-arbitrage-consumer")
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("auto.offset.reset", "latest") // Start from latest to avoid processing old opportunities
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "1000")
            .create()?;

        consumer.subscribe(&["alerts.arbitrage.opportunities"])?;

        info!("✅ Kafka consumer initialized for arbitrage opportunities (broker: {})", kafka_broker);

        Ok(Self {
            consumer,
            database,
        })
    }

    pub async fn start_consuming(&self) -> Result<()> {
        info!("🚀 Starting Kafka consumer for arbitrage opportunities...");
        
        let mut stream = self.consumer.stream();
        
        while let Some(message) = stream.next().await {
            match message {
                Ok(borrowed_message) => {
                    if let Some(payload) = borrowed_message.payload() {
                        match self.process_opportunity_message(payload).await {
                            Ok(inserted) => {
                                if inserted {
                                    debug!("✅ Processed and inserted arbitrage opportunity");
                                } else {
                                    debug!("⚠️  Arbitrage opportunity already exists, skipped");
                                }
                            }
                            Err(e) => {
                                error!("❌ Failed to process arbitrage opportunity: {}", e);
                                // Continue processing other messages even if one fails
                            }
                        }
                    } else {
                        warn!("⚠️  Received empty message payload");
                    }
                }
                Err(e) => {
                    error!("❌ Kafka consumer error: {}", e);
                    // Brief delay before retrying to avoid overwhelming the broker
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        warn!("🔄 Kafka consumer stream ended unexpectedly, this should not happen");
        Ok(())
    }

    async fn process_opportunity_message(&self, payload: &[u8]) -> Result<bool> {
        let message_str = std::str::from_utf8(payload)?;
        let kafka_opportunity: KafkaArbitrageOpportunity = serde_json::from_str(message_str)?;
        
        debug!("📨 Received arbitrage opportunity: {} (profit: {}bps, confidence: {:.2})", 
               kafka_opportunity.signature, kafka_opportunity.profit_bps, kafka_opportunity.confidence_score);

        // Parse timestamp
        let timestamp = DateTime::parse_from_rfc3339(&kafka_opportunity.timestamp)?
            .with_timezone(&Utc);

        // Insert into database using DatabaseService
        let inserted = self.database.insert_arbitrage_opportunity_from_kafka(
            &kafka_opportunity.signature,
            &kafka_opportunity.venue_a,
            &kafka_opportunity.venue_b,
            &kafka_opportunity.token_a,
            &kafka_opportunity.token_b,
            kafka_opportunity.profit_bps,
            kafka_opportunity.slot as i64,
            kafka_opportunity.estimated_net_profit_sol,
            kafka_opportunity.confidence_score,
            timestamp,
            &kafka_opportunity.strategy,
        ).await?;

        if !inserted {
            return Ok(false); // Already exists, skip
        }

        info!("💾 Persisted arbitrage opportunity: {} ({}bps profit, strategy: {})", 
              kafka_opportunity.signature, kafka_opportunity.profit_bps, kafka_opportunity.strategy);

        Ok(true)
    }
}