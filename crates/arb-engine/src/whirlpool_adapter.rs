use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

use crate::models::{Pair, Quote};
use crate::plugin::DexPlugin;
use normalizer_runtime::models::PoolSnapshot;

#[derive(Default)]
struct WState {
    pools: DashMap<String, PoolSnapshot>, // pool_id -> snapshot
    by_pair: DashMap<Pair, String>,       // pair -> pool_id
}

pub struct WhirlpoolAdapter {
    state: Arc<WState>,
}

impl WhirlpoolAdapter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(WState::default()),
        }
    }

    fn pick_pool(&self, pair: &Pair) -> Option<PoolSnapshot> {
        self.state
            .by_pair
            .get(pair)
            .and_then(|pid| self.state.pools.get(pid.value()).map(|v| v.clone()))
    }

    // sqrt_price_x64 (Q64.64) -> spot A-per-B, adjusted by decimals
    fn spot_price_ab(s: &PoolSnapshot) -> Option<f64> {
        let sp_u128: u128 = s.sqrt_price_x64.parse().ok()?;
        // divide by 2^64 (as f64) — fast path
        let sp = (sp_u128 as f64) / (u64::MAX as f64 + 1.0);
        let p = sp * sp;

        let dec_adj = (s.decimals_a as i32) - (s.decimals_b as i32);
        Some(p * 10f64.powi(dec_adj))
    }
}

#[async_trait]
impl DexPlugin for WhirlpoolAdapter {
    fn dex(&self) -> &'static str {
        "whirlpool"
    }
    fn program_ids(&self) -> &'static [&'static str] {
        &["whirLbM2vNtMeJ6z18WZb98sPNT1L4GzWk2vuyZgq2U"]
    }

    async fn on_pool_update(&self, snap: &PoolSnapshot) -> anyhow::Result<()> {
        // Align ticks to spacing to avoid the classic “tick jitter” bugs
        let mut fixed = snap.clone();
        let spacing = fixed.tick_spacing.max(1);
        fixed.tick_current = (fixed.tick_current / spacing) * spacing;

        self.state
            .pools
            .insert(fixed.pool_id.clone(), fixed.clone());
        let pair = Pair::canonical(&fixed.token_a_mint, &fixed.token_b_mint);
        self.state.by_pair.insert(pair, fixed.pool_id.clone());
        Ok(())
    }

    async fn quote_exact_in(&self, pair: &Pair, sizes: &[u128]) -> anyhow::Result<Vec<Quote>> {
        let Some(snap) = self.pick_pool(pair) else {
            return Ok(vec![]);
        };
        let fee = (snap.fee_bps_total as f64) / 10_000.0;
        let p_ab = Self::spot_price_ab(&snap).unwrap_or(0.0);
        if p_ab <= 0.0 {
            return Ok(vec![]);
        }

        let mut out = Vec::with_capacity(sizes.len() * 2);

        // A->B
        for &ain in sizes {
            let aout = ((ain as f64) * p_ab) * (1.0 - fee);
            out.push(Quote {
                dex: self.dex().into(),
                pool_id: snap.pool_id.clone(),
                pair: pair.clone(),
                side_ab: true,
                amount_in: ain,
                amount_out: if aout.is_finite() && aout > 0.0 {
                    aout as u128
                } else {
                    0
                },
                slot: snap.slot,
            });
        }
        // B->A
        for &bin in sizes {
            let bout = ((bin as f64) / p_ab) * (1.0 - fee);
            out.push(Quote {
                dex: self.dex().into(),
                pool_id: snap.pool_id.clone(),
                pair: pair.clone(),
                side_ab: false,
                amount_in: bin,
                amount_out: if bout.is_finite() && bout > 0.0 {
                    bout as u128
                } else {
                    0
                },
                slot: snap.slot,
            });
        }
        Ok(out)
    }
}
