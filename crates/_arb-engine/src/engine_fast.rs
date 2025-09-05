// OLD:
// use smallvec::SmallVec;
// use std::sync::Arc;
use dashmap::DashMap;

use crate::ai_hooks::record_recent_signal;
use crate::config::get_knobs;
use crate::edge_integration::EdgeArbitrageDetector;
use crate::hotpath::SignalTx;
use crate::models::{ArbSignal, Pair, Quote};
use crate::stats::record_profit;
use edge_assembler::edge::EdgeDelta;
use metrics::counter;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;

static QUOTE_ARB_ID: AtomicU64 = AtomicU64::new(1);

pub struct FastEngine {
    // Also: use String instead of &'static str to avoid leaks from Box::leak
    quotes: DashMap<(Pair, String, bool, u128), Quote>,
    out: SignalTx,

    // NEW: Edge-based arbitrage detector for real-time signals
    edge_detector: Arc<EdgeArbitrageDetector>,
}

impl FastEngine {
    pub fn new(out: SignalTx) -> Self {
        Self {
            quotes: DashMap::new(),
            out: out.clone(),
            // NEW: Initialize edge detector with its own SignalTx
            edge_detector: Arc::new(EdgeArbitrageDetector::new(out)),
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

                    let id = QUOTE_ARB_ID.fetch_add(1, Ordering::Relaxed);
                    let profit_bps = if amount_in > 0 {
                        ((profit as f64 / amount_in as f64) * 10_000.0) as i32
                    } else {
                        0
                    };
                    let emitted_at_ns = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;
                    let cand = ArbSignal {
                        pair: pair.clone(),
                        dex_sell: q1.dex.clone(),
                        dex_mid: None,
                        dex_buy: q2.dex.clone(),
                        amount_in,
                        expected_profit_out: profit,
                        slot,
                        sell_pool_id: q1.pool_id.clone(),
                        mid_pool_id: None,
                        buy_pool_id: q2.pool_id.clone(),
                        id,
                        profit_bps,
                        emitted_at_ns,
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
            if self.out.try_send(sig.clone()) {
                record_recent_signal(sig.clone());
                counter!("arb_signals_total", "path" => "quotes");
                record_profit(sig.profit_bps);
            }
        }
    }

    /// NEW: Process real-time edge updates from edge assembler
    /// This is the HOT PATH - called for every pool state change
    #[inline]
    pub fn process_edge_update(&self, edge: EdgeDelta) -> anyhow::Result<()> {
    self.edge_detector.process_edge_update(edge)
    }

    #[cfg(feature = "diagnostics")]
    pub fn refresh_structural_gauges(&self) {
    self.edge_detector.refresh_structural_gauges();
    }

    // (update_edge_config removed; thresholds are global atomics now)

    /// Test helper to lower thresholds
    #[cfg(test)]
    pub fn _edge_detector_mut(&mut self) -> &mut EdgeArbitrageDetector {
    Arc::get_mut(&mut self.edge_detector).expect("only borrowed mut in tests")
    }

    /// Internal accessor (read-only) to edge detector for registration (non-mutable)
    pub fn _edge_detector_ref(&self) -> Option<Arc<EdgeArbitrageDetector>> {
    Some(self.edge_detector.clone())
    }
}
