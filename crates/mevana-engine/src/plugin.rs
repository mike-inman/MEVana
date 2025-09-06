use crate::models::{DexEvent, Pair, Quote};
use async_trait::async_trait;

/// Minimal, IDL-friendly adapter: your existing IDL-based decoders just return DexEvent(s).
#[async_trait]
pub trait DexPlugin: Send + Sync {
    /// Human-readable name (e.g., "whirlpool", "raydium-clmm")
    fn dex(&self) -> &'static str;

    /// Program IDs handled by this plugin. Used to route geyser messages.
    fn program_ids(&self) -> &'static [&'static str];

    /// Called on parsed transaction or account messages from your geyser/Kafka layer.
    /// Implementations convert raw messages → DexEvent(s) (PoolUpdate or Swap).
    async fn handle_galpha(&self, raw: &serde_json::Value) -> anyhow::Result<Vec<DexEvent>>;

    /// Return quotes for this pair and discrete amount candidates (small, med, large).
    /// Fast path: spot price or shallow simulation. Deeper sim can be added later.
    async fn quote_exact_in(&self, pair: &Pair, amounts_in: &[u128]) -> anyhow::Result<Vec<Quote>>;
}
