use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use futures::StreamExt;
use log::{debug, error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message as _;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
struct SwapEvent {
    dex: String,
    pool_id: Option<String>,
    slot: Option<u64>,
    sig: Option<String>,
    in_mint: Option<String>,
    out_mint: Option<String>,
    amount_in: Option<String>,
    amount_out: Option<String>,
    #[serde(default)]
    _fee_paid: Option<String>,
}

#[derive(Debug, Serialize)]
struct PriceSnapshot {
    ts: i64,
    dex: String,
    pair: String,
    price: f64,
    src_sig: String,
    window_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pool_id: Option<String>,
}

struct AggregatorConfig {
    broker: String,
    input_topic: String,
    output_topic: String,
    group_id: String,
    window_secs: u64,
    min_amount_threshold: f64,
}

impl AggregatorConfig {
    fn from_env() -> Self {
        let broker =
            std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());
        let input_topic =
            std::env::var("INPUT_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed.v2".to_string());
        let output_topic =
            std::env::var("OUTPUT_TOPIC").unwrap_or_else(|_| "prices.latest".to_string());
        let group_id = std::env::var("GROUP_ID").unwrap_or_else(|_| "price-aggregator".to_string());
        let window_secs = std::env::var("WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);
        let min_amount_threshold = std::env::var("MIN_AMOUNT_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(1.0);

        Self {
            broker,
            input_topic,
            output_topic,
            group_id,
            window_secs,
            min_amount_threshold,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = AggregatorConfig::from_env();

    info!(
        "🚀 Price aggregator starting (input={}, output={}, broker={})",
        config.input_topic, config.output_topic, config.broker
    );

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.broker)
        .set("group.id", &config.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("auto.offset.reset", "latest")
        .create()
        .context("failed to create kafka consumer")?;

    consumer
        .subscribe(&[&config.input_topic])
        .context("failed to subscribe to swap topic")?;

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.broker)
        .create()
        .context("failed to create kafka producer")?;

    let mut stream = consumer.stream();
    let mut last_prices: HashMap<String, f64> = HashMap::new();

    while let Some(message) = stream.next().await {
        match message {
            Ok(m) => {
                if let Some(payload) = m.payload() {
                    if let Err(err) =
                        handle_message(payload, &producer, &config, &mut last_prices).await
                    {
                        warn!("⚠️ Failed to process swap: {err:#}");
                    }
                }
            }
            Err(err) => {
                error!("❌ Kafka consumer error: {err}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}

async fn handle_message(
    payload: &[u8],
    producer: &FutureProducer,
    config: &AggregatorConfig,
    last_prices: &mut HashMap<String, f64>,
) -> Result<()> {
    let swap: SwapEvent = serde_json::from_slice(payload).context("deserializing swap payload")?;

    let amount_in = swap
        .amount_in
        .as_deref()
        .unwrap_or("0")
        .parse::<f64>()
        .unwrap_or(0.0);
    let amount_out = swap
        .amount_out
        .as_deref()
        .unwrap_or("0")
        .parse::<f64>()
        .unwrap_or(0.0);

    if amount_in <= config.min_amount_threshold || amount_out <= 0.0 {
        debug!("Skipping tiny or invalid swap amount_in={amount_in} amount_out={amount_out}");
        return Ok(());
    }

    let price = amount_out / amount_in;
    if !price.is_finite() || price <= 0.0 {
        debug!("Skipping non-finite price value: {price}");
        return Ok(());
    }

    let in_mint = swap
        .in_mint
        .clone()
        .unwrap_or_else(|| "UNKNOWN_IN".to_string());
    let out_mint = swap
        .out_mint
        .clone()
        .unwrap_or_else(|| "UNKNOWN_OUT".to_string());
    let pair = format!("{in_mint}/{out_mint}");

    let last = last_prices.get(&pair).copied().unwrap_or_default();
    let price_changed = (price - last).abs() / (last.max(1e-9)) > 0.0001;
    if price_changed {
        last_prices.insert(pair.clone(), price);

        let snapshot = PriceSnapshot {
            ts: Utc::now().timestamp(),
            dex: swap.dex.clone(),
            pair,
            price,
            src_sig: swap.sig.clone().unwrap_or_default(),
            window_secs: config.window_secs,
            slot: swap.slot,
            pool_id: swap.pool_id.clone(),
        };

        let payload = serde_json::to_string(&snapshot)?;
        producer
            .send(
                FutureRecord::to(&config.output_topic)
                    .key(snapshot.src_sig.as_str())
                    .payload(&payload),
                Duration::from_secs(0),
            )
            .await
            .map_err(|(err, _)| err)
            .context("publishing price snapshot")?;

        debug!(
            "📈 Published price snapshot dex={} pair={} price={:.6}",
            snapshot.dex, snapshot.pair, snapshot.price
        );
    }

    Ok(())
}
