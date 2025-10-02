use crate::database::DatabaseService;
use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::{debug, error, info, warn};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

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

        info!(
            "✅ Kafka consumer initialized for arbitrage opportunities (broker: {})",
            kafka_broker
        );

        Ok(Self { consumer, database })
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
        let payload: Value = serde_json::from_str(message_str)?;
        let normalized = ParsedOpportunity::from_payload(&payload);

        debug!(
            "📨 Received arbitrage opportunity: {} (profit: {}bps, confidence: {:.2})",
            normalized.signature, normalized.profit_bps, normalized.confidence_score
        );

        // Insert into database using DatabaseService
        let inserted = self
            .database
            .insert_arbitrage_opportunity_from_kafka(
                &normalized.signature,
                &normalized.venue_a,
                &normalized.venue_b,
                &normalized.token_a,
                &normalized.token_b,
                normalized.profit_bps,
                normalized.slot,
                normalized.estimated_profit_sol,
                normalized.confidence_score,
                normalized.timestamp,
                &normalized.strategy,
            )
            .await?;

        if !inserted {
            return Ok(false); // Already exists, skip
        }

        info!(
            "💾 Persisted arbitrage opportunity: {} ({}bps profit, strategy: {})",
            normalized.signature, normalized.profit_bps, normalized.strategy
        );

        Ok(true)
    }
}

/// Normalised view over the arbitrage payload published by the hotpath engine.
#[derive(Debug, Clone)]
struct ParsedOpportunity {
    signature: String,
    venue_a: String,
    venue_b: String,
    token_a: String,
    token_b: String,
    profit_bps: i32,
    slot: i64,
    estimated_profit_sol: f64,
    confidence_score: f64,
    timestamp: DateTime<Utc>,
    strategy: String,
}

impl ParsedOpportunity {
    fn from_payload(payload: &Value) -> Self {
        let details = payload.get("details");

        let signature = details
            .and_then(|d| d.get("signature"))
            .and_then(Value::as_str)
            .or_else(|| payload.get("signature").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();

        let venues = payload
            .get("venues")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let venue_a = details
            .and_then(|d| d.get("venue_a"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                payload
                    .get("venue_a")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .or_else(|| venues.first().and_then(|v| v.as_str().map(str::to_owned)))
            .unwrap_or_default();

        let venue_b = details
            .and_then(|d| d.get("venue_b"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                payload
                    .get("venue_b")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .or_else(|| venues.get(1).and_then(|v| v.as_str().map(str::to_owned)))
            .unwrap_or_else(|| venue_a.clone());

        let token_a = details
            .and_then(|d| d.get("token_a"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                payload
                    .get("token_a")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .or_else(|| {
                payload
                    .get("token_pair")
                    .and_then(Value::as_str)
                    .and_then(|pair| pair.split_once('/').map(|(lhs, _)| lhs.to_string()))
            })
            .unwrap_or_default();

        let token_b = details
            .and_then(|d| d.get("token_b"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                payload
                    .get("token_b")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .or_else(|| {
                payload
                    .get("token_pair")
                    .and_then(Value::as_str)
                    .and_then(|pair| pair.split_once('/').map(|(_, rhs)| rhs.to_string()))
            })
            .unwrap_or_default();

        let profit_bps = details
            .and_then(|d| d.get("profit_bps"))
            .and_then(Value::as_i64)
            .or_else(|| payload.get("profit_bps").and_then(Value::as_i64))
            .unwrap_or(0) as i32;

        let slot = details
            .and_then(|d| d.get("slot"))
            .and_then(Value::as_u64)
            .or_else(|| payload.get("slot").and_then(Value::as_u64))
            .unwrap_or_default() as i64;

        let estimated_profit_sol = payload
            .get("estimated_profit_sol")
            .and_then(Value::as_f64)
            .or_else(|| {
                details
                    .and_then(|d| d.get("estimated_profit_sol"))
                    .and_then(Value::as_f64)
            })
            .or_else(|| {
                payload
                    .get("estimated_net_profit_sol")
                    .and_then(Value::as_f64)
            })
            .unwrap_or(0.0);

        let confidence_score = payload
            .get("confidence_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);

        let strategy = details
            .and_then(|d| d.get("strategy"))
            .and_then(Value::as_str)
            .or_else(|| payload.get("strategy").and_then(Value::as_str))
            .unwrap_or("unknown")
            .to_string();

        let timestamp = payload
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        Self {
            signature,
            venue_a,
            venue_b,
            token_a,
            token_b,
            profit_bps,
            slot,
            estimated_profit_sol,
            confidence_score,
            timestamp,
            strategy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParsedOpportunity;
    use serde_json::Value;

    #[test]
    fn parses_modern_hotpath_payload() {
        let raw = r#"{
            "confidence_score": 0.74,
            "details": {
                "profit_bps": 3,
                "signature": "test-sig",
                "slot": 370201870,
                "strategy": "multi-venue-atomic",
                "token_a": "USDT",
                "token_b": "SOL",
                "venue_a": "phoenix-clob",
                "venue_b": "single-venue"
            },
            "estimated_profit_sol": 0.000285,
            "profit_bps": 3,
            "signature": "test-sig",
            "token_pair": "USDT/SOL",
            "timestamp": "2025-09-30T06:05:22Z",
            "venues": ["phoenix-clob", "single-venue"]
        }"#;

        let value: Value = serde_json::from_str(raw).unwrap();
        let parsed = ParsedOpportunity::from_payload(&value);

        assert_eq!(parsed.signature, "test-sig");
        assert_eq!(parsed.venue_a, "phoenix-clob");
        assert_eq!(parsed.venue_b, "single-venue");
        assert_eq!(parsed.token_a, "USDT");
        assert_eq!(parsed.token_b, "SOL");
        assert_eq!(parsed.profit_bps, 3);
        assert_eq!(parsed.slot, 370201870);
        assert!((parsed.estimated_profit_sol - 0.000285).abs() < f64::EPSILON);
        assert!((parsed.confidence_score - 0.74).abs() < f64::EPSILON);
        assert_eq!(parsed.strategy, "multi-venue-atomic");
    }

    #[test]
    fn parses_legacy_payload() {
        let raw = r#"{
            "signature": "legacy-sig",
            "venue_a": "raydium",
            "venue_b": "orca",
            "token_a": "SOL",
            "token_b": "USDC",
            "profit_bps": 5,
            "slot": 123456,
            "estimated_net_profit_sol": 0.001,
            "confidence_score": 0.5,
            "timestamp": "2025-09-29T12:00:00Z",
            "strategy": "legacy"
        }"#;

        let value: Value = serde_json::from_str(raw).unwrap();
        let parsed = ParsedOpportunity::from_payload(&value);

        assert_eq!(parsed.signature, "legacy-sig");
        assert_eq!(parsed.venue_a, "raydium");
        assert_eq!(parsed.venue_b, "orca");
        assert_eq!(parsed.token_a, "SOL");
        assert_eq!(parsed.token_b, "USDC");
        assert_eq!(parsed.profit_bps, 5);
        assert_eq!(parsed.slot, 123456);
        assert!((parsed.estimated_profit_sol - 0.001).abs() < f64::EPSILON);
    }
}
