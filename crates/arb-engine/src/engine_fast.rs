// OLD:
// use smallvec::SmallVec;
// use std::sync::Arc;
use dashmap::DashMap;

use crate::ai_hooks::record_recent_signal;
use crate::config::get_knobs;
use crate::edge_integration::EdgeArbitrageDetector;
use crate::fringe_risk_manager::FringeRiskManager;
use crate::hotpath::SignalTx;
use crate::models::{ArbSignal, Pair, Quote};
use crate::stats::record_profit;
use edge_assembler::edge::EdgeDelta;
use metrics::counter;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

static QUOTE_ARB_ID: AtomicU64 = AtomicU64::new(1);

pub struct FastEngine {
    // Also: use String instead of &'static str to avoid leaks from Box::leak
    quotes: DashMap<(Pair, String, bool, u128), Quote>,
    out: SignalTx,

    // NEW: Edge-based arbitrage detector for real-time signals
    edge_detector: Arc<EdgeArbitrageDetector>,
    
    // NEW: Fringe risk management for expanded token trading
    risk_manager: Arc<FringeRiskManager>,
}

impl FastEngine {
    pub fn new(out: SignalTx) -> Self {
        Self {
            quotes: DashMap::new(),
            out: out.clone(),
            // NEW: Initialize edge detector with its own SignalTx
            edge_detector: Arc::new(EdgeArbitrageDetector::new(out)),
            // NEW: Initialize fringe risk manager for expanded trading
            risk_manager: Arc::new(FringeRiskManager::new()),
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
            // NEW: Apply fringe risk management before sending signal
            let amount_sol = (amount_in as f64) / 1_000_000_000.0; // Convert lamports to SOL
            let risk_manager = Arc::clone(&self.risk_manager);
            let signal_clone = sig.clone();
            let out_clone = self.out.clone();
            
            // Async risk check (non-blocking for hot path)
            tokio::spawn(async move {
                if let Ok(approval) = risk_manager
                    .approve_trade(&signal_clone.pair.a, &signal_clone.pair.b, amount_sol, signal_clone.profit_bps)
                    .await
                {
                    match approval {
                        crate::fringe_risk_manager::TradeApproval::Approved { 
                            adjusted_amount_sol, 
                            risk_score, 
                            .. 
                        } => {
                            // Adjust signal amount if needed
                            let mut adjusted_signal = signal_clone;
                            if adjusted_amount_sol < amount_sol {
                                let scaling_factor = adjusted_amount_sol / amount_sol;
                                adjusted_signal.amount_in = ((adjusted_signal.amount_in as f64) * scaling_factor) as u128;
                                adjusted_signal.expected_profit_out = ((adjusted_signal.expected_profit_out as f64) * scaling_factor) as i128;
                            }
                            
                            if out_clone.try_send(adjusted_signal.clone()) {
                                record_recent_signal(adjusted_signal.clone());
                                counter!("arb_signals_total", "path" => "quotes_risk_approved");
                                record_profit(adjusted_signal.profit_bps);
                                
                                // Update position tracking
                                if let Err(e) = risk_manager.update_position(&adjusted_signal.pair.a, adjusted_amount_sol).await {
                                    tracing::warn!("Failed to update position tracking: {}", e);
                                }
                            }
                        }
                        crate::fringe_risk_manager::TradeApproval::Rejected { reasons } => {
                            counter!("arb_signals_rejected_total", "reason" => "fringe_risk");
                            tracing::debug!("🛡️ Trade rejected by risk manager: {:?}", reasons);
                        }
                    }
                } else {
                    // Fallback: send original signal if risk check fails
                    if out_clone.try_send(signal_clone.clone()) {
                        record_recent_signal(signal_clone.clone());
                        counter!("arb_signals_total", "path" => "quotes_fallback");
                        record_profit(signal_clone.profit_bps);
                    }
                }
            });
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
    
    /// Internal accessor for fringe risk manager
    pub fn risk_manager(&self) -> Arc<FringeRiskManager> {
        self.risk_manager.clone()
    }
}
