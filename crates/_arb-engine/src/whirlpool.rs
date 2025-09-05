use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;

use crate::models::{DexEvent, Pair, PoolSnapshot, Quote};
use crate::plugin::DexPlugin;

#[derive(Default)]
pub struct WhirlpoolState {
    // Keyed by pool_id
    pools: DashMap<String, PoolSnapshot>,
    // For quick lookup by pair → pool_id (choose best/last seen)
    pair_to_pool: DashMap<Pair, String>,
}

pub struct WhirlpoolPlugin {
    state: Arc<WhirlpoolState>,
    // You can store fee tiers supported, etc., if needed.
}

impl WhirlpoolPlugin {
    pub fn new() -> Self {
        Self { state: Arc::new(WhirlpoolState::default()) }
    }

    fn spot_price_ab(&self, snap: &PoolSnapshot) -> Option<f64> {
        // sqrt_price_x64 is Q64.64; convert to f64 for a fast path.
        let sp_str = &snap.sqrt_price_x64;
        let sp_u128: u128 = sp_str.parse().ok()?;
        let sp = (sp_u128 as f64) / (u64::MAX as f64 + 1.0); // divide by 2^64
        let price_a_per_b = sp * sp; // (sqrtPrice)^2

        // Adjust for decimals: price here is (A/B). We want A-per-B in "token units".
        let dec_adj = (snap.decimals_a as i32) - (snap.decimals_b as i32);
        let scale = 10f64.powi(dec_adj);
        Some(price_a_per_b * scale)
    }

    fn pick_pool_for_pair(&self, pair: &Pair) -> Option<PoolSnapshot> {
        let pool_id = self.state.pair_to_pool.get(pair)?.clone();
        self.state.pools.get(&pool_id).map(|r| r.clone())
    }
}

#[async_trait]
impl DexPlugin for WhirlpoolPlugin {
    fn dex(&self) -> &'static str { "whirlpool" }

    fn program_ids(&self) -> &'static [&'static str] {
        // Main Whirlpool program id; add testnet if you need
        &["whirLbM2vNtMeJ6z18WZb98sPNT1L4GzWk2vuyZgq2U"]
    }

    async fn handle_galpha(&self, raw: &serde_json::Value) -> anyhow::Result<Vec<DexEvent>> {
        // Expect your existing geyser/Kafka layer to give you a normalized JSON.
        // Here we accept either pool account updates or parsed swap tx.
        // For brevity, we detect by presence of fields.
        if raw.get("sqrt_price_x64").is_some() {
            // Treat as PoolSnapshot (account update)
            let snap: PoolSnapshot = serde_json::from_value(raw.clone())?;
            // FIX for "tick issues": ensure we store signed tick_current and clamp to spacing grid
            let tick_spacing = snap.tick_spacing.max(1); // guard
            let aligned_tick = (snap.tick_current / tick_spacing) * tick_spacing;
            let mut to_store = snap.clone();
            to_store.tick_current = aligned_tick;

            self.state.pools.insert(to_store.pool_id.clone(), to_store.clone());
            let pair = Pair::canonical(&to_store.token_a_mint, &to_store.token_b_mint);
            self.state.pair_to_pool.insert(pair, to_store.pool_id.clone());

            Ok(vec![DexEvent::PoolUpdate(to_store)])
        } else if raw.get("amount_in").is_some() && raw.get("amount_out").is_some() {
            // Treat as SwapEvent (transaction)
            let swap: crate::models::SwapEvent = serde_json::from_value(raw.clone())?;
            Ok(vec![DexEvent::Swap(swap)])
        } else {
            Ok(vec![])
        }
    }

    async fn quote_exact_in(
        &self,
        pair: &Pair,
        amounts_in: &[u128],
    ) -> anyhow::Result<Vec<Quote>> {
        let Some(snap) = self.pick_pool_for_pair(pair) else { return Ok(vec![]) };

        // Spot price fast path:
        // A->B: out = in * P * (1 - fee)
        // B->A: out = in / P * (1 - fee)
        // fee_bps_total like 30 => 0.3%
        let fee = (snap.fee_bps_total as f64) / 10_000.0;
        let p_ab = self.spot_price_ab(&snap).unwrap_or(0.0);
        if p_ab <= 0.0 { return Ok(vec![]); }

        let mut out = Vec::new();

        // A -> B quotes
        for &ain in amounts_in {
            let ain_f = ain as f64;
            let aout = (ain_f * p_ab) * (1.0 - fee);
            let aout_u = if aout.is_finite() && aout > 0.0 { aout as u128 } else { 0 };
            out.push(Quote {
                dex: self.dex().into(),
                pool_id: snap.pool_id.clone(),
                pair: pair.clone(),
                side_ab: true,
                amount_in: ain,
                amount_out: aout_u,
                slot: snap.slot,
            });
        }

        // B -> A quotes
        for &bin in amounts_in {
            let bin_f = bin as f64;
            let bout = (bin_f / p_ab) * (1.0 - fee);
            let bout_u = if bout.is_finite() && bout > 0.0 { bout as u128 } else { 0 };
            out.push(Quote {
                dex: self.dex().into(),
                pool_id: snap.pool_id.clone(),
                pair: pair.clone(),
                side_ab: false,
                amount_in: bin,
                amount_out: bout_u,
                slot: snap.slot,
            });
        }

        Ok(out)
    }
}
