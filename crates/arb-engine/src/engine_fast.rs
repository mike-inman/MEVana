// OLD:
// use smallvec::SmallVec;
// use std::sync::Arc;
use dashmap::DashMap;

use crate::config::get_knobs;
use crate::edge_integration::{EdgeArbitrageDetector, EdgeDelta};
use crate::hotpath::SignalTx;
use crate::models::{ArbSignal, Pair, Quote};

pub struct FastEngine {
    // Also: use String instead of &'static str to avoid leaks from Box::leak
    quotes: DashMap<(Pair, String, bool, u128), Quote>,
    out: SignalTx,

    // NEW: Edge-based arbitrage detector for real-time signals
    edge_detector: EdgeArbitrageDetector,
}

impl FastEngine {
    pub fn new(out: SignalTx) -> Self {
        Self {
            quotes: DashMap::new(),
            out: out.clone(),

            // NEW: Initialize edge detector with its own SignalTx
            edge_detector: EdgeArbitrageDetector::new(out),
        }
    }

    #[inline]
    pub fn upsert_quotes(&self, qs: &[Quote]) {
        for q in qs {
            self.quotes.insert(
                (q.pair.clone(), q.dex.clone(), q.side_ab, q.amount_in),
                q.clone(),
            );
        }
    }

    #[inline]
    pub fn scan_pair(&self, pair: &Pair, amount_in: u128, slot: u64) {
        let kb = get_knobs();
        let mut ab: Vec<Quote> = Vec::new();
        let mut ba: Vec<Quote> = Vec::new();

        for r in self.quotes.iter() {
            let (kpair, _kdex, side, amt) = r.key();
            if *kpair != *pair || *amt != amount_in {
                continue;
            }

            let q = r.value().clone();
            if *side {
                ab.push(q);
            } else {
                ba.push(q);
            }
        }

        let mut best: Option<ArbSignal> = None;
        for q1 in &ab {
            for q2 in &ba {
                if q1.dex == q2.dex {
                    continue;
                }
                let profit = (q2.amount_out as i128) - (q1.amount_in as i128);
                if profit >= (kb.min_profit_lamports as i128) {
                    // inside scan_pair, when you build `cand`:

                    let cand = ArbSignal {
                        pair: pair.clone(),
                        dex_sell: q1.dex.clone(),
                        dex_buy: q2.dex.clone(),
                        amount_in,
                        expected_profit_out: profit,
                        slot,
                        sell_pool_id: q1.pool_id.clone(), // NEW
                        buy_pool_id: q2.pool_id.clone(),  // NEW
                    };

                    if best
                        .as_ref()
                        .map(|b| b.expected_profit_out)
                        .unwrap_or_default()
                        < profit
                    {
                        best = Some(cand);
                    }
                }
            }
        }
        if let Some(sig) = best {
            let _ = self.out.try_send(sig);
        }
    }

    /// NEW: Process real-time edge updates from edge assembler
    /// This is the HOT PATH - called for every pool state change
    #[inline]
    pub fn process_edge_update(&self, edge: EdgeDelta) -> anyhow::Result<()> {
        self.edge_detector.process_edge_update(edge)
    }

    /// NEW: Update edge detector configuration (for AI agent tuning)
    pub fn update_edge_config(&mut self, min_profit_bps: u32, max_slippage_bps: u32) {
        self.edge_detector
            .update_config(min_profit_bps, max_slippage_bps);
    }
}
