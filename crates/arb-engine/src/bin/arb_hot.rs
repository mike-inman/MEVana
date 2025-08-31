use std::sync::Arc;
use tokio::task::JoinSet;
use tracing_subscriber::EnvFilter;

//use crate::models::{ArbSignal, Pair, Quote};
//use crate::plugin::DexPlugin;

use arb_engine::{
    config::get_knobs,
    engine_fast::FastEngine,
    executor::{BundleSubmitter, Executor, FastExecutor, FlashLoanProvider},
    hotpath::SignalQueue,
    DexPlugin, WhirlpoolAdapter,
};

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
    let rx = ring.receiver();

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
    let mut js = JoinSet::new();
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
        // Plug your bundler + flash loan providers here
        struct NoopBundler;
        #[async_trait::async_trait]
        impl BundleSubmitter for NoopBundler {
            async fn submit_bundle(
                &self,
                _txs: &[solana_sdk::transaction::VersionedTransaction],
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }
        struct NoFlash;
        #[async_trait::async_trait]
        impl FlashLoanProvider for NoFlash {
            async fn build_flash_loan_bundle(
                &self,
                _a: u64,
                _r: &[solana_sdk::instruction::Instruction],
            ) -> anyhow::Result<smallvec::SmallVec<[solana_sdk::instruction::Instruction; 16]>>
            {
                Ok(Default::default())
            }
        }
        let fee_payer = solana_sdk::signature::Keypair::new();
        let ex = arb_engine::executor::FastExecutor::new(fee_payer, NoopBundler, Some(NoFlash));

        loop {
            if let Some(sig) = rx.try_recv() {
                let _ = ex.execute(&sig).await; // fire and (optionally) forget for <10ms budget
            } else {
                tokio::task::yield_now().await;
            }
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
        res?;
    }
    Ok(())
}
