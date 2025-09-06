//! Minimal placeholder Whirlpool plugin implementation to keep fast iteration.
//! Replace with real decoding / quoting logic later.

use crate::models::{DexEvent, Pair, Quote};
use crate::plugin::DexPlugin;
use async_trait::async_trait;

pub struct WhirlpoolPlugin;

impl Default for WhirlpoolPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl WhirlpoolPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DexPlugin for WhirlpoolPlugin {
    fn dex(&self) -> &'static str {
        "whirlpool"
    }
    fn program_ids(&self) -> &'static [&'static str] {
        &[]
    }
    async fn handle_galpha(&self, _raw: &serde_json::Value) -> anyhow::Result<Vec<DexEvent>> {
        Ok(vec![])
    }
    async fn quote_exact_in(
        &self,
        _pair: &Pair,
        _amounts_in: &[u128],
    ) -> anyhow::Result<Vec<Quote>> {
        Ok(vec![])
    }
}
