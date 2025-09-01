//! Minimal placeholder Whirlpool plugin implementation to keep fast iteration.
//! Replace with real decoding / quoting logic later.

use async_trait::async_trait;
use crate::plugin::DexPlugin;
use crate::models::{DexEvent, Pair, Quote};

pub struct WhirlpoolPlugin;

impl WhirlpoolPlugin { pub fn new() -> Self { Self } }

#[async_trait]
impl DexPlugin for WhirlpoolPlugin {
    fn dex(&self) -> &'static str { "whirlpool" }
    fn program_ids(&self) -> &'static [&'static str] { &[] }
    async fn handle_galpha(&self, _raw: &serde_json::Value) -> anyhow::Result<Vec<DexEvent>> { Ok(vec![]) }
    async fn quote_exact_in(&self, _pair: &Pair, _amounts_in: &[u128]) -> anyhow::Result<Vec<Quote>> { Ok(vec![]) }
}
