use async_trait::async_trait;
use crate::models::{Pair, Quote};
use normalizer_runtime::models::{PoolSnapshot, SwapEvent};

// What the engine expects from each DEX adapter.
#[async_trait]
pub trait DexPlugin: Send + Sync {
    fn dex(&self) -> &'static str;
    fn program_ids(&self) -> &'static [&'static str];

    // Feed *already* normalized messages (your normalizer-service outputs) into the plugin.
    // Return how many state mutations you applied (optional use).
    async fn on_pool_update(&self, snap: &PoolSnapshot) -> anyhow::Result<()>;
    async fn on_swap_event(&self, _swap: &SwapEvent) -> anyhow::Result<()> { Ok(()) }

    // Quick quotes (spot-level, no deep sim). Multiple size buckets welcome.
    async fn quote_exact_in(
        &self,
        pair: &Pair,
        amounts_in: &[u128],
    ) -> anyhow::Result<Vec<Quote>>;
}
