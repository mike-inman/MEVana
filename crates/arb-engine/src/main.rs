use std::sync::Arc;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use tracing_subscriber::EnvFilter;

use arb_engine::engine::Engine;
use arb_engine::whirlpool_adapter::WhirlpoolAdapter;
use arb_engine::models::Pair;
use normalizer_runtime::models::{PoolSnapshot, SwapEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("arb_engine=info".parse()?))
        .init();

    // Print feature information for debugging
    let features = arb_engine::get_feature_info();
    eprintln!("🔧 Arb-engine features enabled: {:?}", features);

    #[cfg(feature = "stubs")]
    eprintln!("🟡 DEVELOPMENT MODE: Using stub implementations - not for production use");

    #[cfg(not(feature = "stubs"))]
    eprintln!("✅ PRODUCTION MODE: Using real implementations");

    // 1) Register adapters (add Raydium/Meteora adapters next)
    let engine = Arc::new(Engine::new(vec![Arc::new(WhirlpoolAdapter::new())]));

    // 2) Kafka consumers (use your real topic names)
    let broker = std::env::var("BROKER").unwrap_or("localhost:29092".into());
    let topic_pool = std::env::var("TOPIC_POOLS").unwrap_or("pools.v1".into());
    let topic_swaps = std::env::var("TOPIC_SWAPS").unwrap_or("normalized-swaps.v1".into());

    let c_pool: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", "arb-engine.pools")
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "latest")
        .create()?;
    c_pool.subscribe(&[&topic_pool])?;

    let c_swaps: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", "arb-engine.swaps")
        .set("enable.partition.eof", "false")
        .set("auto.offset.reset", "latest")
        .create()?;
    c_swaps.subscribe(&[&topic_swaps])?;

    // 3) Main loop: react to pool updates; periodically refresh quotes + scan
    let test_sizes = vec![1_000u128, 10_000u128, 100_000u128]; // tune per pair/decimals

    loop {
        tokio::select! {
            m = c_pool.recv() => {
                if let Ok(m) = m {
                    if let Some(Ok(payload)) = m.payload_view::<str>() {
                        if let Ok(snap) = serde_json::from_str::<PoolSnapshot>(payload) {
                            // feed pool update
                            engine.on_pool_update(&snap).await;

                            // refresh quotes for this pair
                            let pair = Pair::canonical(&snap.token_a_mint, &snap.token_b_mint);
                            engine.refresh_pair(&pair, &test_sizes).await;

                            if let Some(sig) = engine.scan_cycle(&pair, test_sizes[0], snap.slot) {
                                // TODO: produce to "arb.signal" topic or call your executor directly
                                tracing::info!(?sig, "arb-signal");
                            }
                        }
                    }
                }
            }
            m = c_swaps.recv() => {
                if let Ok(m) = m {
                    if let Some(Ok(payload)) = m.payload_view::<str>() {
                        if let Ok(se) = serde_json::from_str::<SwapEvent>(payload) {
                            engine.on_swap_event(&se).await;
                        }
                    }
                }
            }
        }
    }
}
