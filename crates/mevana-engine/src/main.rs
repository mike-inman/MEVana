use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

use mevana_engine::{engine::{run_engine, Engine, StreamEvent}, whirlpool::WhirlpoolPlugin};
use mevana_engine::plugin::DexPlugin;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("mevana_engine=info".parse()?))
        .init();

    // Register plugins. Add Raydium/Meteora/Jupiter implementations next to WhirlpoolPlugin.
    let plugins: Vec<Arc<dyn DexPlugin>> = vec![Arc::new(WhirlpoolPlugin::new())];
    let engine = Arc::new(Engine::new(plugins));

    // In your real pipeline, feed Kafka → StreamEvent::RawUpdate(json).
    let (tx, rx) = mpsc::channel(1024);

    // Example: push a pool update JSON shaped like your existing PoolSnapshot
    let example_pool_json = serde_json::json!({
        "dex":"whirlpool",
        "pool_id":"pool_abc",
        "slot": 300_000_000u64,
        "token_a_mint":"So11111111111111111111111111111111111111112",
        "token_b_mint":"USDc111111111111111111111111111111111111111",
        "decimals_a":9,
        "decimals_b":6,
        "sqrt_price_x64":"7922816251426433759", // dummy
        "liquidity":"12345678901234567890",
        "tick_current": 123456,
        "tick_spacing": 64,
        "fee_bps_total": 30
    });
    tx.send(StreamEvent::RawUpdate(example_pool_json)).await.ok();

    tokio::spawn(run_engine(rx, engine.clone()));

    // Keep running
    tokio::signal::ctrl_c().await?;
    Ok(())
}
