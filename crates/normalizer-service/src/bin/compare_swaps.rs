use anyhow::{Context, Result};
use dotenv::dotenv;
use env_logger;
use log::{error, info, warn};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Deserialize, Clone)]
struct V1Swap {
    sig: String,
    dex: String,
    token_in: Option<String>,
    token_out: Option<String>,
    amount_in: Option<f64>,
    amount_out: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct V2Swap {
    signature: String,
    dex: String,
    token_in_mint: String,
    token_out_mint: String,
    amount_in: u128,
    amount_out: u128,
    price: f64,
}

#[derive(Default)]
struct State {
    v1_by_sig: HashMap<String, V1Swap>,
    v2_by_sig: HashMap<String, V2Swap>,
}

// NEW: Configuration structure with validation
#[derive(Debug, Clone)]
struct ServiceConfig {
    broker: String,
    topic_v1: String,
    topic_v2: String,
}

impl ServiceConfig {
    fn from_env() -> Result<Self> {
        // Load environment variables with validation
        let broker = std::env::var("KAFKA_BROKER")
            .map_err(|_| anyhow::anyhow!("KAFKA_BROKER environment variable is required"))?;

        let topic_v1 =
            std::env::var("TOPIC_V1").unwrap_or_else(|_| "orca-swaps.parsed".to_string());

        let topic_v2 =
            std::env::var("TOPIC_V2").unwrap_or_else(|_| "orca-swaps.parsed.v2".to_string());

        Ok(Self {
            broker,
            topic_v1,
            topic_v2,
        })
    }

    fn log_config(&self) {
        info!("=== COMPARE SWAPS SERVICE CONFIGURATION ===");
        info!("Kafka Broker: {}", self.broker);
        info!("Topic V1: {}", self.topic_v1);
        info!("Topic V2: {}", self.topic_v2);
        info!("==========================================");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    // Load and validate configuration
    let config = ServiceConfig::from_env().context("Failed to load service configuration")?;

    config.log_config();

    info!("compare-swaps: Starting comparison service");

    let state = Arc::new(Mutex::new(State::default()));

    // V1 consumer
    {
        let broker = config.broker.clone();
        let topic = config.topic_v1.clone();
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let c: StreamConsumer = match ClientConfig::new()
                .set("bootstrap.servers", &broker)
                .set("group.id", "compare-swaps-v1")
                .set("enable.auto.commit", "true")
                .set("auto.offset.reset", "latest")
                .create()
            {
                Ok(consumer) => consumer,
                Err(e) => {
                    error!("Failed to create V1 consumer: {}", e);
                    return;
                }
            };

            if let Err(e) = c.subscribe(&[&topic]) {
                error!("Failed to subscribe to V1 topic: {}", e);
                return;
            }

            info!("V1 consumer started for topic: {}", topic);

            loop {
                match c.recv().await {
                    Ok(m) => {
                        if let Some(p) = m.payload() {
                            if let Ok(v) = serde_json::from_slice::<V1Swap>(p) {
                                let mut s = state.lock().await;
                                s.v1_by_sig.insert(v.sig.clone(), v);
                            }
                        }
                    }
                    Err(e) => warn!("v1 recv error: {e:?}"),
                }
            }
        });
    }

    // V2 consumer
    {
        let broker = config.broker.clone();
        let topic = config.topic_v2.clone();
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let c: StreamConsumer = match ClientConfig::new()
                .set("bootstrap.servers", &broker)
                .set("group.id", "compare-swaps-v2")
                .set("enable.auto.commit", "true")
                .set("auto.offset.reset", "latest")
                .create()
            {
                Ok(consumer) => consumer,
                Err(e) => {
                    error!("Failed to create V2 consumer: {}", e);
                    return;
                }
            };

            if let Err(e) = c.subscribe(&[&topic]) {
                error!("Failed to subscribe to V2 topic: {}", e);
                return;
            }

            info!("V2 consumer started for topic: {}", topic);

            loop {
                match c.recv().await {
                    Ok(m) => {
                        if let Some(p) = m.payload() {
                            if let Ok(v) = serde_json::from_slice::<V2Swap>(p) {
                                let mut s = state.lock().await;
                                s.v2_by_sig.insert(v.signature.clone(), v);
                            }
                        }
                    }
                    Err(e) => warn!("v2 recv error: {e:?}"),
                }
            }
        });
    }

    info!("Both consumers started, beginning comparison loop");

    // Periodically compare overlaps
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tick.tick().await;
        let mut comparisons = 0usize;
        let mut mint_mismatch = 0usize;
        let mut missing_fields = 0usize;

        {
            let s = state.lock().await;
            for (sig, v1) in s.v1_by_sig.iter() {
                if let Some(v2) = s.v2_by_sig.get(sig) {
                    comparisons += 1;

                    let mints_match = v1
                        .token_in
                        .as_ref()
                        .map(|m| m == &v2.token_in_mint)
                        .unwrap_or(false)
                        && v1
                            .token_out
                            .as_ref()
                            .map(|m| m == &v2.token_out_mint)
                            .unwrap_or(false);
                    if !mints_match {
                        mint_mismatch += 1;
                    }

                    let p1 = match (v1.amount_in, v1.amount_out) {
                        (Some(ai), Some(ao)) if ai > 0.0 && ao > 0.0 => Some(ao / ai),
                        _ => None,
                    };
                    if p1.is_none() {
                        missing_fields += 1;
                    }

                    if let Some(p1v) = p1 {
                        let p2v = v2.price;
                        let diff = (p1v - p2v).abs();
                        info!(
                            "cmp sig={} mints={} v1_price={:.6} v2_price={:.6} diff={:.6}",
                            sig, mints_match, p1v, p2v, diff
                        );
                    } else {
                        info!(
                            "cmp sig={} mints={} v1_price=NA v2_price={:.6}",
                            sig, mints_match, v2.price
                        );
                    }
                }
            }
        }

        info!(
            "compare summary: pairs={} mint_mismatch={} v1_missing_fields={}",
            comparisons, mint_mismatch, missing_fields
        );
    }
}
