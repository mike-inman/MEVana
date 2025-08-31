use arb_engine::whirlpool_meta::{self, WhirlpoolVaultsMsg};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

//use crate::models::{ArbSignal, Pair, Quote};
//use crate::plugin::DexPlugin;

use arb_engine::{
    config::get_knobs, engine_fast::FastEngine, hotpath::SignalQueue, DexPlugin, WhirlpoolAdapter,
};

#[cfg(feature = "stubs")]
use arb_engine::executor::Executor;

//use mevana_engine::{plugin::DexPlugin, whirlpool::WhirlpoolPlugin}; // your existing plugin

use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig, Message,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // 0) Hot signal ring
    let ring = SignalQueue::new(1024);
    let tx = ring.sender();
    let _rx = ring.receiver();

    // 1) Fast engine
    let engine = Arc::new(FastEngine::new(tx));

    // 2) Register DEX plugins (add Raydium/Meteora/Pumpfun here)
    let whirl = Arc::new(WhirlpoolAdapter::new());
    let plugins: Vec<Arc<dyn DexPlugin>> = vec![whirl];

    // 3) Kafka consumers
    let broker = std::env::var("BROKER").unwrap_or("localhost:29092".into());
    let topic_swaps = std::env::var("TOPIC_SWAPS").unwrap_or("normalized.swaps".into());
    let c_swaps: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", "arb-hot.swaps")
        .set("auto.offset.reset", "latest")
        .create()?;
    c_swaps.subscribe(&[&topic_swaps])?;

    // 4) Quote refresher loop (tick on pool updates ideally; here as periodic demo)
    let engine2 = engine.clone();
    let plugins2 = plugins.clone();
    let mut js: tokio::task::JoinSet<anyhow::Result<()>> = tokio::task::JoinSet::new();
    js.spawn(async move {
        let watch_pairs = vec![]; // fill from Redis/Kafka or your curated list
        loop {
            for p in &watch_pairs {
                for pl in &plugins2 {
                    // Ask each plugin for fast quotes (spot) for a fixed probe size
                    let kb = get_knobs();
                    let qs = pl
                        .quote_exact_in(p, &[kb.test_amount_in_base])
                        .await
                        .unwrap_or_default();
                    engine2.upsert_quotes(&qs);
                    engine2.scan_pair(p, kb.test_amount_in_base, 0);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    });

    // 5) Executor task — drain signals and fire bundles
    js.spawn(async move {
        // Print feature information for debugging
        let features = arb_engine::get_feature_info();
        eprintln!("🔧 Arb-engine features enabled: {:?}", features);

        // Plug your bundler + flash loan providers here
        let _fee_payer = solana_sdk::signature::Keypair::new();

        #[cfg(feature = "stubs")]
        {
            eprintln!("🟡 STUB: Using NoopBundler and NoFlash for development testing");

            struct NoopBundler;
            #[async_trait::async_trait]
            impl arb_engine::BundleSubmitter for NoopBundler {
                async fn submit_bundle(
                    &self,
                    _txs: &[solana_sdk::transaction::VersionedTransaction],
                ) -> anyhow::Result<()> {
                    eprintln!("🟡 STUB: NoopBundler - would submit {} transactions to bundle", _txs.len());
                    Ok(())
                }
            }

            struct NoFlash;
            #[async_trait::async_trait]
            impl arb_engine::FlashLoanProvider for NoFlash {
                async fn build_flash_loan_bundle(
                    &self,
                    amount: u64,
                    route_instructions: &[solana_sdk::instruction::Instruction],
                ) -> anyhow::Result<smallvec::SmallVec<[solana_sdk::instruction::Instruction; 16]>>
                {
                    eprintln!("🟡 STUB: NoFlash - would wrap {} instructions with flash loan for {} lamports",
                             route_instructions.len(), amount);
                    // Return original instructions unchanged
                    Ok(route_instructions.iter().cloned().collect())
                }
            }

            let ex = arb_engine::FastExecutor::new(_fee_payer, NoopBundler, Some(NoFlash));

            loop {
                if let Some(sig) = _rx.try_recv() {
                    let _ = ex.execute(&sig).await; // fire and (optionally) forget for <10ms budget
                } else {
                    tokio::task::yield_now().await;
                }
            }
        }

        #[cfg(not(feature = "stubs"))]
        {
            eprintln!("✅ PRODUCTION: Using real bundler and flash loan providers");
            // TODO: Implement real bundler and flash loan provider integration
            // This would connect to Jito, other block engines, and flash loan protocols
            return Err(anyhow::anyhow!("Production bundler and flash loan providers not implemented yet"));
        }
    });

    // 6) Swaps consumer (enrich off hot path if you want)
    js.spawn(async move {
        loop {
            let m = c_swaps.recv().await.unwrap();
            if let Some(Ok(_payload)) = m.payload_view::<str>() {
                // your enrich/log/metrics path here – not on the hot path
            }
        }
    });

    while let Some(res) = js.join_next().await {
        let _ = res?;
    }
    Ok(())
}

/*

Wire meta updates from Kafka (normalizer → arb-engine)
4a) arb-engine consumer

/ ...
let topic_wmeta = std::env::var("TOPIC_WHIRL_META").unwrap_or("pools.whirlpool.meta.v1".into());
let c_wmeta: StreamConsumer = ClientConfig::new()
    .set("bootstrap.servers", &broker)
    .set("group.id", "arb-hot.whirl.meta")
    .set("auto.offset.reset", "latest")
    .create()?;
c_wmeta.subscribe(&[&topic_wmeta])?;

// Spawn a task:
js.spawn(async move {
    loop {
        let m = c_wmeta.recv().await.unwrap();
        if let Some(Ok(payload)) = m.payload_view::<str>() {
            if let Ok(msg) = serde_json::from_str::<WhirlpoolVaultsMsg>(payload) {
                whirlpool_meta::upsert_vaults(&msg);
            }
        }
    }
});


*/
