use anyhow::{Context, Result};
use chrono::Utc;
use dotenv::dotenv;
use env_logger;
use log::{info, warn};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use rdkafka::Message; // for msg.payload()
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
struct ServiceConfig {
    kafka_broker: String,
    input_topic: String,
    output_topic: String,
    group_id: String,
}

impl ServiceConfig {
    fn from_env() -> Self {
        Self {
            kafka_broker: std::env::var("KAFKA_BROKER")
                .unwrap_or_else(|_| "localhost:29092".to_string()),
            input_topic: std::env::var("INPUT_TOPIC")
                .unwrap_or_else(|_| "shyft-full-transactions-raw".to_string()),
            output_topic: std::env::var("OUTPUT_TOPIC")
                .unwrap_or_else(|_| "arb.candidates.dev".to_string()),
            group_id: std::env::var("GROUP_ID")
                .unwrap_or_else(|_| "arb-candidates-dev".to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();
    let cfg = ServiceConfig::from_env();
    info!(
        "Starting arb-candidates: input={}, output={}",
        cfg.input_topic, cfg.output_topic
    );

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.kafka_broker)
        .set("group.id", &cfg.group_id)
        .set("auto.offset.reset", "latest")
        .create()
        .context("create consumer")?;
    consumer.subscribe(&[&cfg.input_topic])?;

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.kafka_broker)
        .create()
        .context("create producer")?;

    loop {
        match consumer.recv().await {
            Ok(msg) => {
                if let Some(payload) = msg.payload() {
                    if let Err(e) = process_and_publish(payload, &producer, &cfg).await {
                        warn!("failed to process message: {}", e);
                    }
                }
            }
            Err(e) => warn!("kafka consumer error: {}", e),
        }
    }
}

async fn process_and_publish(
    payload: &[u8],
    producer: &FutureProducer,
    cfg: &ServiceConfig,
) -> Result<()> {
    let v: Value = serde_json::from_slice(payload).context("parse raw json")?;

    // Best-effort signature extraction from known fields
    let signature = v
        .get("transaction_signature")
        .and_then(|s| s.as_str())
        .or_else(|| v.get("signature").and_then(|s| s.as_str()))
        .unwrap_or("");

    let block_time_opt = v
        .get("full_shyft_response")
        .and_then(|x| x.get("result"))
        .and_then(|r| r.get("blockTime"))
        .and_then(|b| b.as_i64());

    let now = Utc::now().to_rfc3339();
    let candidate = serde_json::json!({
        "label": "arb_candidate_dev_v1",
        "ingest_ts": now,
        "signature": signature,
        "block_time": block_time_opt,
    });

    producer
        .send(
            FutureRecord::to(&cfg.output_topic)
                .key(signature)
                .payload(&serde_json::to_vec(&candidate)?),
            Duration::from_secs(5),
        )
        .await
        .map_err(|(e, _)| anyhow::anyhow!("producer error: {}", e))?;

    Ok(())
}
