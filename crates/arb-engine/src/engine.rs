use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{ArbSignal, Pair, Quote};
use crate::plugin::DexPlugin;
use dashmap::DashMap;
use normalizer_runtime::models::{PoolSnapshot, SwapEvent};

pub struct Engine {
    plugins: Vec<Arc<dyn DexPlugin>>,
    // last quotes by (pair, dex, side, amount_in)
    last_quotes: DashMap<(Pair, String, bool, u128), Quote>,
}

impl Engine {
    pub fn new(plugins: Vec<Arc<dyn DexPlugin>>) -> Self {
        Self {
            plugins,
            last_quotes: DashMap::new(),
        }
    }

    pub async fn on_pool_update(&self, snap: &PoolSnapshot) {
        for p in &self.plugins {
            // naive broadcast — you can route by program id if you include it in PoolSnapshot
            let _ = p.on_pool_update(snap).await;
        }
    }

    pub async fn on_swap_event(&self, se: &SwapEvent) {
        for p in &self.plugins {
            let _ = p.on_swap_event(se).await;
        }
    }

    pub async fn refresh_pair(&self, pair: &Pair, sizes: &[u128]) {
        for p in &self.plugins {
            if let Ok(v) = p.quote_exact_in(pair, sizes).await {
                for q in v {
                    self.last_quotes
                        .insert((pair.clone(), q.dex.clone(), q.side_ab, q.amount_in), q);
                }
            }
        }
    }

    // ultrafast 2-leg A->B then B->A across different DEXs
    pub fn scan_cycle(&self, pair: &Pair, test_size: u128, slot: u64) -> Option<ArbSignal> {
        let mut ab: HashMap<String, Quote> = HashMap::new();
        let mut ba: HashMap<String, Quote> = HashMap::new();
        for r in self.last_quotes.iter() {
            let (kpair, kdex, side, amt) = r.key();
            if *kpair != *pair || *amt != test_size {
                continue;
            }
            if *side {
                ab.insert(kdex.clone(), (*r).clone());
            } else {
                ba.insert(kdex.clone(), (*r).clone());
            }
        }

        let mut best: Option<ArbSignal> = None;
        for (sell, q1) in &ab {
            for (buy, q2) in &ba {
                if sell == buy {
                    continue;
                }
                // NOTE: we keep amount_in fixed for the fast filter (super low latency).
                let profit = (q2.amount_out as i128) - (q1.amount_in as i128);
                if profit > 0 {
                    let sig = ArbSignal {
                        pair: pair.clone(),
                        dex_sell: sell.clone(),
                        dex_buy: buy.clone(),
                        amount_in: q1.amount_in,
                        expected_profit_out: profit,
                        slot,
                    };
                    if best
                        .as_ref()
                        .map(|b| b.expected_profit_out)
                        .unwrap_or_default()
                        < profit
                    {
                        best = Some(sig);
                    }
                }
            }
        }
        best
    }
}
