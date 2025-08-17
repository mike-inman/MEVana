use anyhow::Result;
use chrono::Utc;
use log::info;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let broker = env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());
    let prices_topic = env::var("PRICES_TOPIC").unwrap_or_else(|_| "prices.latest".to_string());
    let swaps_topic = env::var("SWAPS_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed".to_string());

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("compression.type", "lz4")
        .set("linger.ms", "5")
        .set("acks", "all")
        .create()?;

    let now = Utc::now().timestamp();

    // One realistic NormalizedSwap
    let swap = serde_json::json!({
        "ts": now,
        "slot": 123456789u64,
        "sig": "DemoSigDeAdBeEf00112233445566778899AaBbCcDdEeFf",
        "dex": "orca",
        "token_in":  "So11111111111111111111111111111111111111112",
        "amount_in":  1.2345,
        "token_out": "Es9vMFrzaCERZ4YQhQ4Z1cdq9VHtV6fQ5bJ9bGbkQJ6q", // USDT as example
        "amount_out": 250.12
    });

    // One PriceSnapshot matching the same pair
    let price = serde_json::json!({
        "ts": now,
        "dex": "Orca",
        "pair": "SOL/USDC",
        "price": 200.54321,
        "src_sig": "DemoSigDeAdBeEf00112233445566778899AaBbCcDdEeFf",
        "window_secs": 5u64
    });

    info!(
        "Seeding to {} → [{} , {}]",
        broker, swaps_topic, prices_topic
    );

    // Produce swap
    let swap_key = "orca:SOL/USDC";
    let swap_payload = swap.to_string();
    if let Err((e, _)) = producer
        .send(
            FutureRecord::to(&swaps_topic)
                .key(swap_key)
                .payload(&swap_payload),
            Duration::from_secs(5),
        )
        .await
    {
        eprintln!("failed to send swap: {:?}", e);
    } else {
        info!("sent 1 swap to {}", swaps_topic);
    }

    // Produce price
    let price_key = "Orca:SOL/USDC";
    let price_payload = price.to_string();
    if let Err((e, _)) = producer
        .send(
            FutureRecord::to(&prices_topic)
                .key(price_key)
                .payload(&price_payload),
            Duration::from_secs(5),
        )
        .await
    {
        eprintln!("failed to send price: {:?}", e);
    } else {
        info!("sent 1 price to {}", prices_topic);
    }

    Ok(())
}
