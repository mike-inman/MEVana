use anyhow::Result;
use log::{debug, error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{Message, TopicPartitionList};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;
use tokio::select;

// Copy the structs from hotpath_arb_engine for compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub signature: String,
    pub slot: u64,
    pub profit_bps: i32,
    pub venue_a: String,
    pub venue_b: String,
    pub token_a: String,
    pub token_b: String,
    pub strategy: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub opportunity_id: String,
    pub timestamp_ns: u64,
    pub executed: bool,
    pub success: Option<bool>,
    pub actual_profit_bps: Option<i32>,
    pub execution_time_us: Option<u64>,
    pub gas_cost_sol: Option<f64>,
    pub slippage_bps: Option<i32>,
    pub failure_reason: Option<String>,
    pub transaction_signature: Option<String>,
    pub bundle_id: Option<String>,
    pub raw_result: serde_json::Value,
}

pub struct KafkaDbWriter {
    consumer: StreamConsumer,
    db_pool: PgPool,
    batch_size: usize,
    flush_interval_ms: u64,
}

impl KafkaDbWriter {
    pub async fn new(
        kafka_broker: &str,
        db_url: &str,
        batch_size: Option<usize>,
        flush_interval_ms: Option<u64>,
    ) -> Result<Self> {
        // Initialize Kafka consumer
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "mevana-db-writer-v2")
            .set("bootstrap.servers", kafka_broker)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false") // Manual commit for reliability
            .set("auto.offset.reset", "earliest")
            .create()?;

        // Subscribe to Kafka topics
        let topics = vec!["alerts.arbitrage.opportunities", "alerts.arbitrage.executions"];
        let mut topic_partition_list = TopicPartitionList::new();
        for topic in &topics {
            topic_partition_list.add_partition(topic, 0);
        }
        consumer.subscribe(&topics)?;

        info!("✅ Kafka consumer initialized (broker: {}, topics: {:?})", kafka_broker, topics);

        // Initialize database connection
        let db_pool = PgPool::connect(db_url).await?;
        info!("✅ Database connected: {}", db_url);

        Ok(Self {
            consumer,
            db_pool,
            batch_size: batch_size.unwrap_or(100),
            flush_interval_ms: flush_interval_ms.unwrap_or(1000),
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("🚀 Kafka-to-Database Writer started (batch_size={}, flush_interval={}ms)", 
               self.batch_size, self.flush_interval_ms);

        let mut opportunity_buffer = Vec::new();
        let mut execution_buffer = Vec::new();
        let mut last_flush = std::time::Instant::now();
        let flush_interval = Duration::from_millis(self.flush_interval_ms);

        loop {
            select! {
                // Receive messages from Kafka
                message = self.consumer.recv() => {
                    match message {
                        Ok(m) => {
                            if let Err(e) = self.process_message(
                                &m, 
                                &mut opportunity_buffer, 
                                &mut execution_buffer
                            ).await {
                                error!("❌ Failed to process Kafka message: {}", e);
                            }

                            // Check if we need to flush due to batch size
                            if opportunity_buffer.len() >= self.batch_size || 
                               execution_buffer.len() >= self.batch_size {
                                self.flush_buffers(&mut opportunity_buffer, &mut execution_buffer).await;
                                last_flush = std::time::Instant::now();
                            }
                        }
                        Err(e) => {
                            error!("❌ Kafka consumer error: {}", e);
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }

                // Periodic flush
                _ = tokio::time::sleep(flush_interval) => {
                    if last_flush.elapsed() >= flush_interval && 
                       (!opportunity_buffer.is_empty() || !execution_buffer.is_empty()) {
                        self.flush_buffers(&mut opportunity_buffer, &mut execution_buffer).await;
                        last_flush = std::time::Instant::now();
                    }
                }
            }
        }
    }

    async fn process_message(
        &self,
        message: &rdkafka::message::BorrowedMessage<'_>,
        opportunity_buffer: &mut Vec<ArbitrageOpportunity>,
        execution_buffer: &mut Vec<ExecutionResult>,
    ) -> Result<()> {
        let topic = message.topic();
        let payload = match message.payload_view::<str>() {
            Some(Ok(payload)) => payload,
            Some(Err(e)) => {
                warn!("⚠️ Failed to decode message payload: {}", e);
                return Ok(());
            }
            None => {
                warn!("⚠️ Empty message payload");
                return Ok(());
            }
        };

        match topic {
            "alerts.arbitrage.opportunities" => {
                match serde_json::from_str::<ArbitrageOpportunity>(payload) {
                    Ok(opportunity) => {
                        debug!("📥 Received opportunity: {} ({})", &opportunity.signature[0..8], opportunity.profit_bps);
                        opportunity_buffer.push(opportunity);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to parse opportunity JSON: {}", e);
                    }
                }
            }
            "alerts.arbitrage.executions" => {
                match serde_json::from_str::<ExecutionResult>(payload) {
                    Ok(execution) => {
                        debug!("📥 Received execution: {}", &execution.opportunity_id[0..8]);
                        execution_buffer.push(execution);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to parse execution JSON: {}", e);
                    }
                }
            }
            _ => {
                debug!("📭 Unknown topic: {}", topic);
            }
        }

        Ok(())
    }

    async fn flush_buffers(
        &self,
        opportunity_buffer: &mut Vec<ArbitrageOpportunity>,
        execution_buffer: &mut Vec<ExecutionResult>,
    ) {
        let start_time = std::time::Instant::now();
        let total_events = opportunity_buffer.len() + execution_buffer.len();

        if total_events == 0 {
            return;
        }

        // Start database transaction
        match self.db_pool.begin().await {
            Ok(mut tx) => {
                let mut success = true;

                // Write opportunities
                for opportunity in opportunity_buffer.drain(..) {
                    if let Err(e) = self.write_opportunity(&mut tx, &opportunity).await {
                        error!("❌ Failed to write opportunity {}: {}", &opportunity.signature[0..8], e);
                        success = false;
                        break;
                    }
                }

                // Write execution results
                if success {
                    for execution in execution_buffer.drain(..) {
                        if let Err(e) = self.write_execution(&mut tx, &execution).await {
                            error!("❌ Failed to write execution {}: {}", &execution.opportunity_id[0..8], e);
                            success = false;
                            break;
                        }
                    }
                }

                // Commit or rollback
                if success {
                    match tx.commit().await {
                        Ok(_) => {
                            let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
                            info!("✅ Flushed {} events to DB in {:.2}ms", total_events, elapsed_ms);
                        }
                        Err(e) => {
                            error!("❌ Failed to commit transaction: {}", e);
                        }
                    }
                } else {
                    if let Err(e) = tx.rollback().await {
                        error!("❌ Failed to rollback transaction: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("❌ Failed to start database transaction: {}", e);
            }
        }

        // Clear any remaining items from failed transaction
        opportunity_buffer.clear();
        execution_buffer.clear();
    }

    async fn write_opportunity(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO arbitrage_opportunities 
            (opportunity_id, timestamp, slot, venue_sell, venue_buy, token_pair,
             profit_bps, amount_in, expected_amount_out, price_impact_pct,
             detection_time_us, strategy_type, raw_opportunity)
            VALUES ($1, to_timestamp($2::double precision / 1000000000), $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (opportunity_id) DO NOTHING
            "#
        )
        .bind(&opportunity.signature)
        .bind(chrono::DateTime::parse_from_rfc3339(&opportunity.timestamp).unwrap().timestamp_nanos_opt().unwrap_or_default() as f64)
        .bind(opportunity.slot as i64)
        .bind(&opportunity.venue_a)
        .bind(&opportunity.venue_b)
        .bind(&format!("{}/{}", opportunity.token_a, opportunity.token_b))
        .bind(opportunity.profit_bps)
        .bind(0.0) // amount_in - not available in producer struct
        .bind(0.0) // expected_amount_out - not available in producer struct  
        .bind(None::<f64>) // price_impact_pct - not available in producer struct
        .bind(0i64) // detection_time_us - not available in producer struct
        .bind(&opportunity.strategy)
        .bind(serde_json::json!({}))
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn write_execution(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        execution: &ExecutionResult,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO execution_results 
            (opportunity_id, timestamp, executed, success, actual_profit_bps,
             execution_time_us, gas_cost_sol, slippage_bps, failure_reason,
             transaction_signature, bundle_id, raw_result)
            VALUES ($1, to_timestamp($2::double precision / 1000000000), $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(&execution.opportunity_id)
        .bind(execution.timestamp_ns as f64)
        .bind(execution.executed)
        .bind(execution.success)
        .bind(execution.actual_profit_bps)
        .bind(execution.execution_time_us.map(|x| x as i64))
        .bind(execution.gas_cost_sol)
        .bind(execution.slippage_bps)
        .bind(&execution.failure_reason)
        .bind(&execution.transaction_signature)
        .bind(&execution.bundle_id)
        .bind(&execution.raw_result)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://mev:mevpassword@localhost:5433/mevdb".to_string());
    let kafka_broker = std::env::var("KAFKA_BROKER")
        .unwrap_or_else(|_| "localhost:29092".to_string());

    info!("🚀 Starting MEVana Kafka-to-Database Writer Service");
    info!("📊 Database: {}", database_url);
    info!("📡 Kafka: {}", kafka_broker);

    let writer = KafkaDbWriter::new(&kafka_broker, &database_url, None, None).await?;
    writer.run().await?;

    Ok(())
}