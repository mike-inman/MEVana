#![allow(unused_imports, clippy::single_component_path_imports, deprecated)]

// crates/normalizer-service/src/bin/tap_orca_debug.rs
// Debug binary for outputting full transaction JSON

use anyhow::{Context, Result};
use bs58;
use chrono::Utc;
use dotenv::dotenv;
use env_logger;
use log::{error, info, warn};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{ClientConfig, Message as _};
use serde_json;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude as ypp;
use yellowstone_grpc_proto::prost::Message as _;

// Configuration structure for debug mode
#[derive(Debug, Clone)]
struct DebugConfig {
    broker: String,
    in_topic: String,
    debug_topic: String,
    group_id: String,
}

impl DebugConfig {
    fn from_env() -> Result<Self> {
        let broker = std::env::var("KAFKA_BROKER")
            .map_err(|_| anyhow::anyhow!("KAFKA_BROKER environment variable is required"))?;

        let in_topic = std::env::var("IN_TOPIC").unwrap_or_else(|_| "dex-transactions".to_string());

        let debug_topic =
            std::env::var("DEBUG_TOPIC").unwrap_or_else(|_| "normalizer-debug-full-tx".to_string());

        let group_id = std::env::var("GROUP_ID").unwrap_or_else(|_| "tap-orca-debug".to_string());

        Ok(Self {
            broker,
            in_topic,
            debug_topic,
            group_id,
        })
    }

    fn log_config(&self) {
        info!("=== DEBUG SERVICE CONFIGURATION ===");
        info!("Kafka Broker: {}", self.broker);
        info!("Input Topic: {}", self.in_topic);
        info!("Debug Topic: {}", self.debug_topic);
        info!("Group ID: {}", self.group_id);
        info!("==================================");
    }
}

/// Builds a complete, prettified JSON representation of the entire transaction
fn build_full_transaction_json_debug(
    update: &ypp::SubscribeUpdateTransaction,
    keys_b58: &[String],
) -> Result<serde_json::Value> {
    let Some(info) = update.transaction.as_ref() else {
        return Err(anyhow::anyhow!("No transaction info available"));
    };

    let Some(msg) = info.transaction.as_ref().and_then(|t| t.message.as_ref()) else {
        return Err(anyhow::anyhow!("No transaction message available"));
    };

    // Build the COMPLETE transaction JSON with EVERYTHING
    let full_tx = serde_json::json!({
        "debug_label": "COMPLETE_TRANSACTION_EVERYTHING_DEBUG_OUTPUT",
        "timestamp": Utc::now().to_rfc3339(),
        "slot": update.slot,
        "signature": bs58::encode(&info.signature).into_string(),
        "raw_payload_size": update.encoded_len() as u64,

        // COMPLETE TRANSACTION DATA
        "transaction": {
            "message": {
                "account_keys": keys_b58,
                "recent_blockhash": bs58::encode(&msg.recent_blockhash).into_string(),
                "instructions": msg.instructions.iter().enumerate().map(|(idx, ci)| {
                    let pid_idx = ci.program_id_index as usize;
                    let program_id = if pid_idx < keys_b58.len() { &keys_b58[pid_idx] } else { "unknown" };

                    serde_json::json!({
                        "index": idx,
                        "program_id": program_id,
                        "accounts": ci.accounts.iter().map(|&i| {
                            if (i as usize) < keys_b58.len() { &keys_b58[i as usize] } else { "unknown" }
                        }).collect::<Vec<_>>(),
                        "data": base64::encode(&ci.data),
                        "data_length": ci.data.len(),
                        "data_hex": ci.data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(""),
                    })
                }).collect::<Vec<_>>(),
            },

            // COMPLETE METADATA
            "meta": {
                "fee": info.meta.as_ref().map(|m| m.fee),
                "log_messages": info.meta.as_ref().map(|m| m.log_messages.clone()),
                "compute_units_consumed": info.meta.as_ref().and_then(|m| m.compute_units_consumed),
            },
        },

        // RAW BYTES FOR COMPLETE ANALYSIS
        "raw_instruction_data": msg.instructions.iter().enumerate().map(|(idx, ci)| {
            serde_json::json!({
                "index": idx,
                "raw_bytes": ci.data.to_vec(),
                "base64": base64::encode(&ci.data),
                "hex": ci.data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(""),
                "length": ci.data.len(),
            })
        }).collect::<Vec<_>>(),

        // COMPLETE ACCOUNT INFORMATION
        "account_details": keys_b58.iter().enumerate().map(|(idx, key)| {
            serde_json::json!({
                "index": idx,
                "address": key,
                "is_writable": false, // Simplified for now
            })
        }).collect::<Vec<_>>(),
    });

    Ok(full_tx)
}

fn build_keys_and_balance_changes(info: &ypp::SubscribeUpdateTransactionInfo) -> Vec<String> {
    // Account keys (base58)
    let keys_b58: Vec<String> = info
        .transaction
        .as_ref()
        .and_then(|t| t.message.as_ref())
        .map(|msg| {
            msg.account_keys
                .iter()
                .map(|k| bs58::encode(k).into_string())
                .collect()
        })
        .unwrap_or_default();

    keys_b58
}

async fn handle_debug(
    payload: &[u8],
    producer: &FutureProducer,
    debug_topic: &str,
    _config: &DebugConfig,
) -> Result<()> {
    // Decode the Yellowstone SubscribeUpdateTransaction
    let update = ypp::SubscribeUpdateTransaction::decode(payload)
        .context("prost decode SubscribeUpdateTransaction")?;

    let Some(info) = update.transaction.as_ref() else {
        return Ok(());
    };

    let keys_b58 = build_keys_and_balance_changes(info);

    // Build full transaction JSON for debugging
    let full_tx_json = build_full_transaction_json_debug(&update, &keys_b58)?;

    // Send to debug topic with descriptive label
    let debug_key = format!("debug_{}", bs58::encode(&info.signature).into_string());
    let debug_payload = serde_json::to_vec(&full_tx_json)?;

    let _ = producer
        .send(
            FutureRecord::to(debug_topic)
                .key(&debug_key)
                .payload(&debug_payload),
            std::time::Duration::from_secs(5),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send debug message: {:?}", e))?;

    info!(
        "🔍 DEBUG: Full transaction sent to topic '{}' for signature: {}",
        debug_topic,
        bs58::encode(&info.signature).into_string()
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    // Load and validate configuration
    let config = DebugConfig::from_env().context("Failed to load debug configuration")?;

    config.log_config();

    info!("🚀 Starting Debug Service...");

    // Create Kafka consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.broker)
        .set("group.id", &config.group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()
        .context("Failed to create consumer")?;

    if let Err(e) = consumer.subscribe(&[&config.in_topic]) {
        error!("Failed to subscribe to topic: {}", e);
        return Err(anyhow::anyhow!("Failed to subscribe: {}", e));
    }

    info!("✅ Successfully subscribed to topic: {}", config.in_topic);

    // Create Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.broker)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create producer")?;

    info!("✅ Kafka producer created successfully");

    info!("🚀 Starting debug message processing loop...");

    loop {
        match consumer.recv().await {
            Err(e) => warn!("kafka recv error: {e:?}"),
            Ok(m) => {
                if let Some(bytes) = m.payload() {
                    if let Err(e) =
                        handle_debug(bytes, &producer, &config.debug_topic, &config).await
                    {
                        error!("handle_debug error: {e:#}");
                    }
                } else {
                    warn!("empty payload");
                }
                let _ = consumer.commit_message(&m, rdkafka::consumer::CommitMode::Async);
            }
        }
    }
}
