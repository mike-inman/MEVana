use crate::ai_hooks::record_recent_signal;
use crate::hotpath::SignalTx;
use crate::models::{ArbSignal, Pair};
use crate::quoting::depth_adjustment_factor;
use crate::sizing::{choose_size, record_signal_profit_bps};
#[cfg(feature = "diagnostics")]
use crate::stats::{incr_cycles_examined, incr_cycles_profitable, incr_cycles_pruned};
use crate::stats::{
    record_depth_factor_bps, record_latency_ns, record_profit, record_size_utilization_bps,
};
use crate::stats::{record_edge_staleness_ns, LAST_EDGE_UPDATE_NS};
use crate::token_meta::prime_tokens;
// (imports above consolidated to avoid duplicate definitions)
use crate::edge_verifier::EdgeVerifier; // NEW
#[cfg(feature = "diagnostics")]
use crate::stats::{AVG_ADJACENCY, EDGE_PAIR_COUNT, MAX_EDGE_FACTOR, TOKEN_COUNT};
use anyhow::Result;
use dashmap::DashMap;
use edge_assembler::edge::EdgeDelta; // bring EdgeDelta into scope
use metrics::counter;
use once_cell::sync::Lazy;
use prost::Message;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing; // for decoding EdgeDelta from prost bytes

// Type alias to simplify complex tuple used for per-venue USDC price tracking
type VenuePriceEntry = (u64, f64, f64, String, String, u32, u128);

// Global threshold atomics (live updated) + helpers
pub static MIN_PROFIT_BPS_ATOM: Lazy<AtomicU32> = Lazy::new(|| {
    AtomicU32::new(
        std::env::var("ARB_MIN_PROFIT_BPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50),
    )
});
pub static MAX_SLIPPAGE_BPS_ATOM: Lazy<AtomicU32> = Lazy::new(|| {
    AtomicU32::new(
        std::env::var("ARB_MAX_SLIPPAGE_BPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(150),
    )
});

pub fn set_edge_thresholds(min_profit_bps: Option<u32>, max_slippage_bps: Option<u32>) {
    if let Some(m) = min_profit_bps {
        MIN_PROFIT_BPS_ATOM.store(m, Ordering::Relaxed);
    }
    if let Some(s) = max_slippage_bps {
        MAX_SLIPPAGE_BPS_ATOM.store(s, Ordering::Relaxed);
    }
}

pub fn edge_threshold_snapshot() -> (u32, u32) {
    (
        MIN_PROFIT_BPS_ATOM.load(Ordering::Relaxed),
        MAX_SLIPPAGE_BPS_ATOM.load(Ordering::Relaxed),
    )
}

static EDGE_ARB_ID: AtomicU64 = AtomicU64::new(1_000_000);
static HOTPATH_LOG: Lazy<bool> = Lazy::new(|| {
    std::env::var("HOTPATH_LOG")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on"))
        .unwrap_or(false)
});

// Full edge capture (first N per venue) for debugging real data integrity
static EDGE_CAPTURE_FULL: Lazy<bool> = Lazy::new(|| {
    std::env::var("EDGE_CAPTURE_FULL")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
});
static EDGE_CAPTURE_LIMIT: Lazy<u64> = Lazy::new(|| {
    std::env::var("EDGE_CAPTURE_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(25)
});
static VENUE_CAPTURED_1: AtomicU64 = AtomicU64::new(0);
static VENUE_CAPTURED_2: AtomicU64 = AtomicU64::new(0);
static VENUE_CAPTURED_3: AtomicU64 = AtomicU64::new(0);
static VENUE_CAPTURED_4: AtomicU64 = AtomicU64::new(0);

fn capture_full_edge(edge: &EdgeDelta) {
    if !*EDGE_CAPTURE_FULL {
        return;
    }
    let counter = match edge.venue_id {
        1 => &VENUE_CAPTURED_1,
        2 => &VENUE_CAPTURED_2,
        3 => &VENUE_CAPTURED_3,
        4 => &VENUE_CAPTURED_4,
        _ => return,
    };
    let cur = counter.load(Ordering::Relaxed);
    if cur >= *EDGE_CAPTURE_LIMIT {
        return;
    }
    if counter
        .compare_exchange(cur, cur + 1, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    // Decode helper closures
    let to_b58 = |v: &Vec<u8>| bs58::encode(v).into_string();
    let parse_u128 = |bytes: &Vec<u8>| -> Option<u128> {
        if bytes.len() >= 16 {
            let mut b = [0u8; 16];
            b.copy_from_slice(&bytes[..16]);
            Some(u128::from_le_bytes(b))
        } else {
            None
        }
    };
    let price_u128 = parse_u128(&edge.marginal_price_q64).unwrap_or(0);
    let price = (price_u128 as f64) / 18446744073709551616.0; // 2^64
    let size_u128 = parse_u128(&edge.size_to_next_boundary_in).unwrap_or(0);
    let (state_kind, state_detail) = match &edge.state {
        Some(edge_assembler::edge::edge_delta::State::Clmm(s)) => {
            let liq = parse_u128(&s.liquidity).unwrap_or(0);
            (
                "clmm",
                format!(
                    "tick={} spacing={} liq={}",
                    s.tick_current, s.tick_spacing, liq
                ),
            )
        }
        Some(edge_assembler::edge::edge_delta::State::Cpmm(s)) => {
            let rin = parse_u128(&s.reserve_in).unwrap_or(0);
            let rout = parse_u128(&s.reserve_out).unwrap_or(0);
            ("cpmm", format!("rin={} rout={}", rin, rout))
        }
        Some(edge_assembler::edge::edge_delta::State::Dlmm(s)) => (
            "dlmm",
            format!(
                "active_bin={} depth_hint_len={}",
                s.active_bin,
                s.depth_hint_in.len()
            ),
        ),
        Some(edge_assembler::edge::edge_delta::State::Clob(s)) => (
            "clob",
            format!(
                "bid_px_len={} ask_px_len={}",
                s.best_bid_px_q64.len(),
                s.best_ask_px_q64.len()
            ),
        ),
        None => ("missing", "".to_string()),
    };
    println!(
        "EDGE_FULL venue={} id={} dir={} slot={} ts_ns={} fee_bps={} price_q64={} price={} size_boundary={} token_in={} token_out={} pool={} state={} {}", 
        edge.venue_id,
        edge.edge_id,
        edge.direction,
        edge.slot,
        edge.updated_at_ns,
        edge.fee_bps_total,
        price_u128,
        price,
        size_u128,
        to_b58(&edge.token_in),
        to_b58(&edge.token_out),
        to_b58(&edge.pool_pubkey),
        state_kind,
        state_detail
    );
}

// NEW: global verifier (enabled if EDGE_VERIFY=1)
static EDGE_VERIFIER: Lazy<EdgeVerifier> = Lazy::new(|| {
    let enabled = std::env::var("EDGE_VERIFY")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    EdgeVerifier::new(enabled)
});

pub fn edge_verifier_snapshot_json() -> serde_json::Value {
    EDGE_VERIFIER.snapshot_json()
}

pub fn edge_dashboard_json() -> serde_json::Value {
    EDGE_VERIFIER.dashboard_json()
}

// Re-export for other modules
pub use edge_assembler::edge::EdgeDelta as ProtoEdgeDelta; // keep public alias

/// Real-time edge-based arbitrage detector
/// Integrates with your existing edge assembler for <10ms latency
pub struct EdgeArbitrageDetector {
    // token_pair_key -> Vec<Arc<EdgeDelta>> (both directions included)
    edges: DashMap<String, Vec<Arc<EdgeDelta>>>,
    // token (base58) -> set of pair keys containing token (stored as Vec, dedup on insert)
    token_index: DashMap<String, Vec<String>>,
    // pool_id (base58) -> Vec<Arc<EdgeDelta>> (both directions)
    pool_index: DashMap<String, Vec<Arc<EdgeDelta>>>,
    // pair_key -> aggregated stats
    pair_stats: DashMap<String, PairEdgeStats>,
    signal_tx: SignalTx,
    // thresholds now always read from global atomics (mutable via HTTP API); legacy fields removed
    brute_force_edge_limit: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PairEdgeStats {
    pub signals_emitted: u64,
    pub max_profit_bps: i32,
    pub cumulative_profit_bps: i64,
    pub last_updated_ns: u64,
    // opportunity tracking (updated on every edge update)
    pub max_observed_potential_bps: i32,
    pub last_potential_bps: i32,
    pub opportunity_events: u64,
    pub last_opportunity_ns: u64,
}

impl EdgeArbitrageDetector {
    pub fn new(signal_tx: SignalTx) -> Self {
        Self {
            edges: DashMap::new(),
            token_index: DashMap::new(),
            pool_index: DashMap::new(),
            pair_stats: DashMap::new(),
            signal_tx,
            brute_force_edge_limit: std::env::var("ARB_3HOP_BRUTE_FORCE_EDGE_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4),
        }
    }

    /// Process incoming edge updates from edge assembler
    /// This is the HOT PATH - called for every pool state change
    #[inline]
    #[tracing::instrument(skip(self, edge), level = "trace")]
    pub fn process_edge_update(&self, edge: EdgeDelta) -> Result<()> {
        let pair_key = self.create_pair_key(&edge.token_in, &edge.token_out);
        // observe before mutation so we see raw input
        EDGE_VERIFIER.observe(&edge); // NEW
        capture_full_edge(&edge); // optional debug full capture
        self.edges
            .entry(pair_key.clone())
            .or_default()
            .retain(|e| e.edge_id != edge.edge_id);
        self.edges
            .entry(pair_key.clone())
            .or_default()
            .push(Arc::new(edge.clone()));

        // Update pool index
        let pool_b58 = bs58::encode(&edge.pool_pubkey).into_string();
        self.pool_index
            .entry(pool_b58.clone())
            .and_modify(|v| {
                v.retain(|e| e.edge_id != edge.edge_id || e.direction != edge.direction);
                v.push(Arc::new(edge.clone()));
            })
            .or_insert_with(|| vec![Arc::new(edge.clone())]);

        // Update token index (dedup simple linear scan; small vector expected per token)
        let a = bs58::encode(&edge.token_in).into_string();
        let b = bs58::encode(&edge.token_out).into_string();
        // Prime token metadata cache (fire & forget) for newly observed mints
        prime_tokens([a.as_str(), b.as_str()]);
        for t in [&a, &b] {
            self.token_index
                .entry(t.clone())
                .and_modify(|v| {
                    if !v.iter().any(|k| k == &pair_key) {
                        v.push(pair_key.clone())
                    }
                })
                .or_insert_with(|| vec![pair_key.clone()]);
        }
        // Always record last edge update timestamp (ns from edge message)
        let ts_ns = edge.updated_at_ns;
        LAST_EDGE_UPDATE_NS.store(ts_ns, Ordering::Relaxed);
        // Record wall-clock staleness (if clock drift positive)
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        if now_ns > ts_ns {
            record_edge_staleness_ns(now_ns - ts_ns);
        }
        self.scan_for_arbitrage(&pair_key)?;
        // After running arbitrage scan, update opportunity stats (cheap computation)
        self.update_opportunity_stats(&pair_key);

        Ok(())
    }

    /// Public: snapshot depth / liquidity info for a pool (latest per direction)
    pub fn pool_snapshot_json(&self, pool_id: &str) -> Option<serde_json::Value> {
        let guard = self.pool_index.get(pool_id)?; // DashMap guard
        if guard.is_empty() {
            return None;
        }
        // Choose latest edge (max updated_at_ns) per direction
        let mut latest_dir: [Option<Arc<EdgeDelta>>; 2] = [None, None];
        for e in guard.iter() {
            let slot_ts = e.updated_at_ns;
            let idx = if e.direction == 0 { 0 } else { 1 };
            let replace = latest_dir[idx]
                .as_ref()
                .map(|cur| slot_ts > cur.updated_at_ns)
                .unwrap_or(true);
            if replace {
                latest_dir[idx] = Some(e.clone());
            }
        }
        // Helper to parse u128
        let parse_u128 = |bytes: &Vec<u8>| -> Option<u128> {
            if bytes.len() < 16 {
                return None;
            }
            let mut b = [0u8; 16];
            b.copy_from_slice(&bytes[..16]);
            Some(u128::from_le_bytes(b))
        };
        let mut base = serde_json::json!({
            "pool_id": pool_id,
        });
        let add_edge =
            |name: &str,
             e: &Arc<EdgeDelta>,
             obj: &mut serde_json::Map<String, serde_json::Value>| {
                use serde_json::json;
                let price_q64 = parse_u128(&e.marginal_price_q64).unwrap_or(0);
                let size_boundary = parse_u128(&e.size_to_next_boundary_in).unwrap_or(0);
                let token_in = bs58::encode(&e.token_in).into_string();
                let token_out = bs58::encode(&e.token_out).into_string();
                let mut edge_obj = json!({
                    "direction": e.direction,
                    "venue_id": e.venue_id,
                    "token_in": token_in,
                    "token_out": token_out,
                    "updated_at_ns": e.updated_at_ns,
                    "slot": e.slot,
                    "fee_bps_total": e.fee_bps_total,
                    "price_q64": price_q64.to_string(),
                    "size_to_next_boundary_in": size_boundary.to_string(),
                });
                // State-specific extras
                if let Some(st) = &e.state {
                    match st {
                        edge_assembler::edge::edge_delta::State::Clmm(s) => {
                            let liq = parse_u128(&s.liquidity).unwrap_or(0);
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("kind".into(), json!("clmm"));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("tick_current".into(), json!(s.tick_current));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("tick_spacing".into(), json!(s.tick_spacing));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("liquidity".into(), json!(liq.to_string()));
                        }
                        edge_assembler::edge::edge_delta::State::Cpmm(s) => {
                            let rin = parse_u128(&s.reserve_in).unwrap_or(0);
                            let rout = parse_u128(&s.reserve_out).unwrap_or(0);
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("kind".into(), json!("cpmm"));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("reserve_in".into(), json!(rin.to_string()));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("reserve_out".into(), json!(rout.to_string()));
                        }
                        edge_assembler::edge::edge_delta::State::Dlmm(s) => {
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("kind".into(), json!("dlmm"));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("active_bin".into(), json!(s.active_bin));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("depth_hint_bins".into(), json!(s.depth_hint_in.len()));
                        }
                        edge_assembler::edge::edge_delta::State::Clob(s) => {
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("kind".into(), json!("clob"));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("best_bid_levels".into(), json!(s.best_bid_px_q64.len()));
                            edge_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("best_ask_levels".into(), json!(s.best_ask_px_q64.len()));
                        }
                    }
                }
                obj.insert(name.into(), edge_obj);
            };
        if let Some(e0) = &latest_dir[0] {
            if let Some(obj) = base.as_object_mut() {
                add_edge("dir0", e0, obj);
            }
        }
        if let Some(e1) = &latest_dir[1] {
            if let Some(obj) = base.as_object_mut() {
                add_edge("dir1", e1, obj);
            }
        }
        Some(base)
    }

    /// Ultra-fast arbitrage detection using stored edges
    /// Called for every edge update - must be <1ms
    #[inline]
    #[tracing::instrument(skip(self), level = "trace")]
    fn scan_for_arbitrage(&self, pair_key: &str) -> Result<()> {
        let guard = match self.edges.get(pair_key) {
            Some(g) => g,
            None => return Ok(()),
        };
        let edges_ref: &Vec<Arc<EdgeDelta>> = &guard;
        // Partition by direction for potential 2-leg cycle; 3-leg scan should proceed regardless
        let mut dir0: Vec<&Arc<EdgeDelta>> = Vec::new(); // A->B
        let mut dir1: Vec<&Arc<EdgeDelta>> = Vec::new(); // B->A
        for e in edges_ref.iter() {
            if e.direction == 0 {
                dir0.push(e);
            } else {
                dir1.push(e);
            }
        }
        if !dir0.is_empty() && !dir1.is_empty() {
            let mut best: Option<(ArbSignal, u64, i128)> = None;
            let mut best_profit: i128 = 0;
            for sell in &dir0 {
                for buy in &dir1 {
                    if sell.venue_id == buy.venue_id {
                        continue;
                    }
                    if let Some((profit_lamports, chosen_size, _base_size, ts, profit_bps)) =
                        self.evaluate_2leg(sell, buy)
                    {
                        if profit_lamports > best_profit {
                            best_profit = profit_lamports;
                            let id = EDGE_ARB_ID.fetch_add(1, Ordering::Relaxed);
                            let pair = Pair {
                                a: bs58::encode(&sell.token_in).into_string(),
                                b: bs58::encode(&sell.token_out).into_string(),
                            };
                            let sig = ArbSignal {
                                pair,
                                dex_sell: self.venue_id_to_dex_name(sell.venue_id),
                                dex_mid: None,
                                dex_buy: self.venue_id_to_dex_name(buy.venue_id),
                                amount_in: chosen_size,
                                expected_profit_out: profit_lamports,
                                slot: sell.slot.max(buy.slot),
                                sell_pool_id: bs58::encode(&sell.pool_pubkey).into_string(),
                                mid_pool_id: None,
                                buy_pool_id: bs58::encode(&buy.pool_pubkey).into_string(),
                                id,
                                profit_bps,
                                emitted_at_ns: SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos()
                                    as u64,
                            };
                            best = Some((sig, ts, profit_lamports));
                        }
                    }
                }
            }
            if let Some((sig, edge_ts, _)) = best {
                // update pair stats before emit
                self.update_pair_stats_from_signal(&sig, pair_key);
                if self.signal_tx.try_send(sig.clone()) {
                    use crate::stats::SIGNALS_EMITTED;
                    SIGNALS_EMITTED.fetch_add(1, Ordering::Relaxed);
                    record_recent_signal(sig.clone());
                    counter!("arb_signals_total", "path"=>"edges");
                    record_profit(sig.profit_bps);
                    record_signal_profit_bps(sig.profit_bps);
                    let now_ns = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;
                    if now_ns > edge_ts {
                        record_latency_ns(now_ns - edge_ts);
                    }
                    if *HOTPATH_LOG {
                        println!(
                            "HOT_ARB kind=2leg profit_bps={} profit_lamports={} amount_in={} dex_sell={} dex_buy={} sell_pool={} buy_pool={} slot={} id={} pair={}/{}",
                            sig.profit_bps,
                            sig.expected_profit_out,
                            sig.amount_in,
                            sig.dex_sell,
                            sig.dex_buy,
                            sig.sell_pool_id,
                            sig.buy_pool_id,
                            sig.slot,
                            sig.id,
                            sig.pair.a,
                            sig.pair.b,
                        );
                    }
                }
            }
        }

        // Attempt localized 3-edge cycles too (seeded by the two tokens of pair_key)
        let parts: Vec<_> = pair_key.split('_').collect();
        if parts.len() == 2 {
            self.scan_for_arbitrage_3hop_tokens(&[parts[0], parts[1]]);
        }
        // TEST MODE fallback: if few edges overall, brute force
        if self.edges.len() <= self.brute_force_edge_limit {
            self.scan_for_arbitrage_3hop_full();
        }

        Ok(())
    }
    fn scan_for_arbitrage_3hop_full(&self) {
        use std::collections::HashMap;
        let mut adj: HashMap<Vec<u8>, Vec<Arc<EdgeDelta>>> = HashMap::new();
        for entry in self.edges.iter() {
            for e in entry.value().iter() {
                let (from, to) = self.directed_tokens_slices(e);
                adj.entry(from.to_vec()).or_default().push(e.clone());
                adj.entry(to.to_vec()).or_default();
            }
        }
        if adj.is_empty() {
            return;
        }
        // Precompute global max factor for pruning
        let mut global_max_factor = 0f64;
        for list in adj.values() {
            for e in list {
                if let Some(p) = self.marginal_price(e) {
                    let f = p * self.fee_factor(e);
                    if f.is_finite() && f > global_max_factor {
                        global_max_factor = f;
                    }
                }
            }
        }
        #[cfg(feature = "diagnostics")]
        if global_max_factor.is_finite() && global_max_factor > 0.0 {
            // store scaled by 1e6
            let scaled = (global_max_factor * 1_000_000.0) as u64;
            let mut cur = MAX_EDGE_FACTOR.load(Ordering::Relaxed);
            while scaled > cur {
                if MAX_EDGE_FACTOR
                    .compare_exchange(cur, scaled, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
                cur = MAX_EDGE_FACTOR.load(Ordering::Relaxed);
            }
        }
        if global_max_factor <= 0.0 {
            return;
        }
        let threshold_factor =
            1.0 + (MIN_PROFIT_BPS_ATOM.load(Ordering::Relaxed) as f64) / 10_000.0;
        let mut best: Option<(ArbSignal, u64, i128)> = None; // signal, ts, scaled_profit
        for (start, list1) in adj.iter() {
            for e1 in list1.iter() {
                let (_f1, t1) = self.directed_tokens_slices(e1);
                // Prune after first edge if impossible to beat threshold
                if let Some(p1) = self.marginal_price(e1) {
                    let p1f = p1 * self.fee_factor(e1);
                    if p1f * global_max_factor * global_max_factor < threshold_factor {
                        #[cfg(feature = "diagnostics")]
                        incr_cycles_pruned();
                        continue;
                    }
                }
                if let Some(list2) = adj.get(t1) {
                    for e2 in list2.iter() {
                        if e2.venue_id == e1.venue_id {
                            continue;
                        }
                        let (_f2, t2) = self.directed_tokens_slices(e2);
                        if let (Some(p1), Some(p2)) =
                            (self.marginal_price(e1), self.marginal_price(e2))
                        {
                            let p12 = p1 * self.fee_factor(e1) * p2 * self.fee_factor(e2);
                            if p12 * global_max_factor < threshold_factor {
                                #[cfg(feature = "diagnostics")]
                                incr_cycles_pruned();
                                continue;
                            }
                        }
                        if let Some(list3) = adj.get(t2) {
                            for e3 in list3.iter() {
                                if e3.venue_id == e1.venue_id || e3.venue_id == e2.venue_id {
                                    continue;
                                }
                                let (_f3, t3) = self.directed_tokens_slices(e3);
                                if t3 == start.as_slice() {
                                    if let Some((
                                        profit_lamports,
                                        chosen_size,
                                        _base_size,
                                        profit_bps,
                                        ts,
                                    )) = self.evaluate_3leg(e1, e2, e3)
                                    {
                                        #[cfg(feature = "diagnostics")]
                                        incr_cycles_examined();
                                        let better = best
                                            .as_ref()
                                            .map(|(_, _, bp)| profit_lamports > *bp)
                                            .unwrap_or(true);
                                        if better {
                                            let id = EDGE_ARB_ID.fetch_add(1, Ordering::Relaxed);
                                            let pair = Pair {
                                                a: bs58::encode(&e1.token_in).into_string(),
                                                b: bs58::encode(&e1.token_out).into_string(),
                                            };
                                            let sig = ArbSignal {
                                                pair,
                                                dex_sell: self.venue_id_to_dex_name(e1.venue_id),
                                                dex_mid: Some(
                                                    self.venue_id_to_dex_name(e2.venue_id),
                                                ),
                                                dex_buy: self.venue_id_to_dex_name(e3.venue_id),
                                                amount_in: chosen_size,
                                                expected_profit_out: profit_lamports,
                                                slot: e1.slot.max(e2.slot).max(e3.slot),
                                                sell_pool_id: bs58::encode(&e1.pool_pubkey)
                                                    .into_string(),
                                                mid_pool_id: Some(
                                                    bs58::encode(&e2.pool_pubkey).into_string(),
                                                ),
                                                buy_pool_id: bs58::encode(&e3.pool_pubkey)
                                                    .into_string(),
                                                id,
                                                profit_bps,
                                                emitted_at_ns: SystemTime::now()
                                                    .duration_since(UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_nanos()
                                                    as u64,
                                            };
                                            best = Some((sig, ts, profit_lamports));
                                            #[cfg(feature = "diagnostics")]
                                            incr_cycles_profitable();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some((sig, ts, _p)) = best {
            // Derive pair_key (sorted) from signal pair (already sorted when created via token ordering logic for first edge)
            let pair_key = if sig.pair.a < sig.pair.b {
                format!("{}_{}", sig.pair.a, sig.pair.b)
            } else {
                format!("{}_{}", sig.pair.b, sig.pair.a)
            };
            self.update_pair_stats_from_signal(&sig, &pair_key);
            if self.signal_tx.try_send(sig.clone()) {
                use crate::stats::SIGNALS_EMITTED;
                SIGNALS_EMITTED.fetch_add(1, Ordering::Relaxed);
                record_recent_signal(sig.clone());
                counter!("arb_signals_total", "path"=>"edges3");
                record_profit(sig.profit_bps);
                record_signal_profit_bps(sig.profit_bps);
                let now_ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                if now_ns > ts {
                    record_latency_ns(now_ns - ts);
                }
                if *HOTPATH_LOG {
                    println!(
                        "HOT_ARB kind=3leg-full profit_bps={} profit_lamports={} amount_in={} dex_sell={} dex_mid={} dex_buy={} sell_pool={} mid_pool={} buy_pool={} slot={} id={} pair={}/{}",
                        sig.profit_bps,
                        sig.expected_profit_out,
                        sig.amount_in,
                        sig.dex_sell,
                        sig.dex_mid.clone().unwrap_or_default(),
                        sig.dex_buy,
                        sig.sell_pool_id,
                        sig.mid_pool_id.clone().unwrap_or_default(),
                        sig.buy_pool_id,
                        sig.slot,
                        sig.id,
                        sig.pair.a,
                        sig.pair.b,
                    );
                }
            }
        }
    }

    /// Snapshot of currently watched token pairs (pair key, tokens, edge count, venue set, last updated)
    pub fn list_pairs_json(&self, limit: Option<usize>) -> serde_json::Value {
        use crate::token_meta::{prime_tokens, symbol_snapshot};
        use serde_json::json;
        let mut rows: Vec<(u64, serde_json::Value)> = Vec::new();
        for entry in self.edges.iter() {
            let pair_key = entry.key().clone();
            let parts: Vec<&str> = pair_key.split('_').collect();
            let (token_a, token_b) = if parts.len() == 2 {
                (parts[0], parts[1])
            } else {
                ("?", "?")
            };
            // Prime token metadata (non-blocking) so future requests likely have symbols
            if token_a != "?" && token_b != "?" {
                prime_tokens([token_a, token_b]);
            }
            let sym_a = symbol_snapshot(token_a);
            let sym_b = symbol_snapshot(token_b);
            let pair_symbol = match (&sym_a, &sym_b) {
                (Some(a), Some(b)) => Some(format!("{}/{}", a, b)),
                _ => None,
            };
            let mut venues = std::collections::BTreeSet::new();
            let mut last_updated = 0u64;
            for e in entry.value().iter() {
                venues.insert(e.venue_id);
                if e.updated_at_ns > last_updated {
                    last_updated = e.updated_at_ns;
                }
            }
            let js = json!({
                "pair_key": pair_key,
                "token_a": token_a,
                "token_b": token_b,
                "symbol_a": sym_a,
                "symbol_b": sym_b,
                "pair_symbol": pair_symbol,
                "edge_count": entry.value().len(),
                "venues": venues.into_iter().collect::<Vec<_>>(),
                "last_updated_ns": last_updated
            });
            rows.push((last_updated, js));
        }
        // Sort descending by last_updated
        rows.sort_by(|a, b| b.0.cmp(&a.0));
        let lim = limit.unwrap_or(500);
        let arr: Vec<serde_json::Value> = rows.into_iter().take(lim).map(|(_, v)| v).collect();
        serde_json::Value::Array(arr)
    }

    #[cfg(any(test, feature = "stubs"))]
    pub fn force_scan_all(&self) {
        self.scan_for_arbitrage_3hop_full();
    }

    /// Calculate profit between two edges
    /// Compute potential spread opportunity every edge update: best A->B * best B->A (net fee factors)
    fn update_opportunity_stats(&self, pair_key: &str) {
        let guard = if let Some(g) = self.edges.get(pair_key) {
            g
        } else {
            return;
        };
        let edges_ref: &Vec<Arc<EdgeDelta>> = &guard;
        let mut best_dir0: Option<(&Arc<EdgeDelta>, f64)> = None; // A->B price * fee
        let mut best_dir1: Option<(&Arc<EdgeDelta>, f64)> = None; // B->A price * fee
        for e in edges_ref.iter() {
            if let Some(p) = self.marginal_price(e) {
                let eff = p * self.fee_factor(e);
                if e.direction == 0 {
                    if best_dir0.map(|(_, v)| eff > v).unwrap_or(true) {
                        best_dir0 = Some((e, eff));
                    }
                } else if best_dir1.map(|(_, v)| eff > v).unwrap_or(true) {
                    best_dir1 = Some((e, eff));
                }
            }
        }
        if let (Some((_a, pa)), Some((_b, pb))) = (best_dir0, best_dir1) {
            let potential = pa * pb - 1.0; // gross excess factor -1
            if potential.is_finite() {
                let potential_bps = (potential * 10_000.0) as i32;
                use dashmap::mapref::entry::Entry;
                let now_ns = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0);
                match self.pair_stats.entry(pair_key.to_string()) {
                    Entry::Occupied(mut occ) => {
                        let s = occ.get_mut();
                        s.last_potential_bps = potential_bps;
                        if potential_bps > s.max_observed_potential_bps {
                            s.max_observed_potential_bps = potential_bps;
                        }
                        s.opportunity_events += 1;
                        s.last_opportunity_ns = now_ns;
                    }
                    Entry::Vacant(vac) => {
                        vac.insert(PairEdgeStats {
                            signals_emitted: 0,
                            max_profit_bps: 0,
                            cumulative_profit_bps: 0,
                            last_updated_ns: 0,
                            max_observed_potential_bps: potential_bps,
                            last_potential_bps: potential_bps,
                            opportunity_events: 1,
                            last_opportunity_ns: now_ns,
                        });
                    }
                }
            }
        }
    }
    #[inline]
    fn evaluate_2leg(
        &self,
        edge1: &Arc<EdgeDelta>,
        edge2: &Arc<EdgeDelta>,
    ) -> Option<(i128, u128, u128, u64, i32)> {
        let base_size = self
            .parse_size_to_boundary(edge1)
            .min(self.parse_size_to_boundary(edge2));
        if base_size == 0 {
            return None;
        }
        let p1 = self.marginal_price(edge1)?;
        let p2 = self.marginal_price(edge2)?;
        let raw_gross = p1 * p2 * self.fee_factor(edge1) * self.fee_factor(edge2);
        if !raw_gross.is_finite() {
            return None;
        }
        let chosen = choose_size(base_size);
        let adj = depth_adjustment_factor(chosen, base_size, 2);
        if base_size > 0 {
            let util_bps = ((chosen as f64 / base_size as f64) * 10_000.0) as i32;
            record_size_utilization_bps(util_bps);
        }
        let depth_bps = (adj * 10_000.0) as i32;
        record_depth_factor_bps(depth_bps);
        let gross = raw_gross * adj;
        if !gross.is_finite() {
            return None;
        }
        let profit_bps = ((gross - 1.0) * 10_000.0) as i32;
        if profit_bps < MIN_PROFIT_BPS_ATOM.load(Ordering::Relaxed) as i32 {
            return None;
        }
        let profit_lamports = (chosen as f64 * (gross - 1.0)) as i128;
        let ts = edge1.updated_at_ns.min(edge2.updated_at_ns);
        Some((profit_lamports, chosen, base_size, ts, profit_bps))
    }

    /// Parse size to next boundary from edge data
    #[inline]
    fn parse_size_to_boundary(&self, edge: &EdgeDelta) -> u128 {
        let v = &edge.size_to_next_boundary_in;
        if v.len() < 16 {
            return 0;
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&v[..16]);
        u128::from_le_bytes(bytes)
    }

    /// Create pair key for fast lookup
    #[inline]
    fn create_pair_key(&self, token_in: &[u8], token_out: &[u8]) -> String {
        let a = bs58::encode(token_in).into_string();
        let b = bs58::encode(token_out).into_string();
        if a < b {
            format!("{}_{}", a, b)
        } else {
            format!("{}_{}", b, a)
        }
    }

    /// Convert venue ID to DEX name
    fn venue_id_to_dex_name(&self, venue_id: u32) -> String {
        match venue_id {
            1 => "orca_whirlpool".to_string(),
            2 => "raydium_cpmm".to_string(),
            3 => "meteora_dlmm".to_string(),
            4 => "phoenix".to_string(),
            _ => format!("unknown_{}", venue_id),
        }
    }

    #[inline]
    fn marginal_price(&self, edge: &EdgeDelta) -> Option<f64> {
        // Q64.64 format
        let v = &edge.marginal_price_q64;
        if v.len() < 16 {
            return None;
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&v[..16]);
        let raw = u128::from_le_bytes(bytes);
        let price = (raw as f64) / 18446744073709551616.0; // 2^64
        Some(price)
    }

    #[inline]
    fn fee_factor(&self, edge: &EdgeDelta) -> f64 {
        1.0 - (edge.fee_bps_total as f64 / 10_000.0)
    }

    #[allow(dead_code)]
    fn directed_tokens(&self, edge: &Arc<EdgeDelta>) -> (Vec<u8>, Vec<u8>) {
        if edge.direction == 0 {
            (edge.token_in.clone(), edge.token_out.clone())
        } else {
            (edge.token_out.clone(), edge.token_in.clone())
        }
    }
    fn directed_tokens_slices<'a>(&'a self, edge: &'a Arc<EdgeDelta>) -> (&'a [u8], &'a [u8]) {
        if edge.direction == 0 {
            (&edge.token_in, &edge.token_out)
        } else {
            (&edge.token_out, &edge.token_in)
        }
    }

    fn scan_for_arbitrage_3hop_tokens(&self, seed_tokens: &[&str]) {
        use std::collections::{HashMap, HashSet, VecDeque};
        const MAX_TOKENS: usize = 32;
        let mut visited: HashSet<Vec<u8>> = HashSet::new();
        let mut queue: VecDeque<Vec<u8>> = seed_tokens
            .iter()
            .map(|s| bs58::decode(s).into_vec().unwrap_or_default())
            .collect();
        let mut adj: HashMap<Vec<u8>, Vec<Arc<EdgeDelta>>> = HashMap::new();

        while let Some(tok_bytes) = queue.pop_front() {
            if !visited.insert(tok_bytes.clone()) {
                continue;
            }
            if visited.len() > MAX_TOKENS {
                break;
            }
            let tok_b58 = bs58::encode(&tok_bytes).into_string();
            if let Some(pair_keys) = self.token_index.get(&tok_b58) {
                for pk in pair_keys.iter() {
                    if let Some(edges) = self.edges.get(pk) {
                        for e in edges.iter() {
                            let (from, to) = self.directed_tokens_slices(e);
                            if from == tok_bytes.as_slice() {
                                adj.entry(from.to_vec()).or_default().push(e.clone());
                                adj.entry(to.to_vec()).or_default();
                                if !visited.iter().any(|v| v.as_slice() == to) {
                                    queue.push_back(to.to_vec());
                                }
                            }
                        }
                    }
                }
            }
        }

        if visited.len() < 3 {
            return;
        }
        // Precompute global max factor among discovered adjacency graph for pruning
        let mut global_max_factor = 0f64;
        for list in adj.values() {
            for e in list {
                if let Some(p) = self.marginal_price(e) {
                    let f = p * self.fee_factor(e);
                    if f.is_finite() && f > global_max_factor {
                        global_max_factor = f;
                    }
                }
            }
        }
        #[cfg(feature = "diagnostics")]
        if global_max_factor.is_finite() && global_max_factor > 0.0 {
            let scaled = (global_max_factor * 1_000_000.0) as u64;
            let mut cur = MAX_EDGE_FACTOR.load(Ordering::Relaxed);
            while scaled > cur {
                if MAX_EDGE_FACTOR
                    .compare_exchange(cur, scaled, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
                cur = MAX_EDGE_FACTOR.load(Ordering::Relaxed);
            }
        }
        if global_max_factor <= 0.0 {
            return;
        }
        let threshold_factor =
            1.0 + (MIN_PROFIT_BPS_ATOM.load(Ordering::Relaxed) as f64) / 10_000.0;
        let mut best: Option<(ArbSignal, u64, i128)> = None; // signal, ts, scaled_profit
        for (start_tok, list1) in &adj {
            for e1 in list1.iter() {
                let (_f1, t1) = self.directed_tokens_slices(e1);
                if let Some(p1) = self.marginal_price(e1) {
                    let p1f = p1 * self.fee_factor(e1);
                    if p1f * global_max_factor * global_max_factor < threshold_factor {
                        #[cfg(feature = "diagnostics")]
                        incr_cycles_pruned();
                        continue;
                    }
                }
                if let Some(list2) = adj.get(t1) {
                    for e2 in list2.iter() {
                        if e2.venue_id == e1.venue_id {
                            continue;
                        }
                        let (_f2, t2) = self.directed_tokens_slices(e2);
                        if let (Some(p1), Some(p2)) =
                            (self.marginal_price(e1), self.marginal_price(e2))
                        {
                            let p12 = p1 * self.fee_factor(e1) * p2 * self.fee_factor(e2);
                            if p12 * global_max_factor < threshold_factor {
                                #[cfg(feature = "diagnostics")]
                                incr_cycles_pruned();
                                continue;
                            }
                        }
                        if let Some(list3) = adj.get(t2) {
                            for e3 in list3.iter() {
                                if e3.venue_id == e1.venue_id || e3.venue_id == e2.venue_id {
                                    continue;
                                }
                                let (_f3, t3) = self.directed_tokens_slices(e3);
                                if t3 == start_tok.as_slice() {
                                    if let Some((
                                        profit_lamports,
                                        chosen_size,
                                        _base_size,
                                        profit_bps,
                                        ts,
                                    )) = self.evaluate_3leg(e1, e2, e3)
                                    {
                                        #[cfg(feature = "diagnostics")]
                                        incr_cycles_examined();
                                        let better = best
                                            .as_ref()
                                            .map(|(_, _, bp)| profit_lamports > *bp)
                                            .unwrap_or(true);
                                        if better {
                                            let id = EDGE_ARB_ID.fetch_add(1, Ordering::Relaxed);
                                            let pair = Pair {
                                                a: bs58::encode(&e1.token_in).into_string(),
                                                b: bs58::encode(&e3.token_out).into_string(),
                                            };
                                            let sig = ArbSignal {
                                                pair,
                                                dex_sell: self.venue_id_to_dex_name(e1.venue_id),
                                                dex_mid: Some(
                                                    self.venue_id_to_dex_name(e2.venue_id),
                                                ),
                                                dex_buy: self.venue_id_to_dex_name(e3.venue_id),
                                                amount_in: chosen_size,
                                                expected_profit_out: profit_lamports,
                                                slot: e1.slot.max(e2.slot).max(e3.slot),
                                                sell_pool_id: bs58::encode(&e1.pool_pubkey)
                                                    .into_string(),
                                                mid_pool_id: Some(
                                                    bs58::encode(&e2.pool_pubkey).into_string(),
                                                ),
                                                buy_pool_id: bs58::encode(&e3.pool_pubkey)
                                                    .into_string(),
                                                id,
                                                profit_bps,
                                                emitted_at_ns: SystemTime::now()
                                                    .duration_since(UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_nanos()
                                                    as u64,
                                            };
                                            best = Some((sig, ts, profit_lamports));
                                            #[cfg(feature = "diagnostics")]
                                            incr_cycles_profitable();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some((sig, ts, _p)) = best {
            let pair_key = if sig.pair.a < sig.pair.b {
                format!("{}_{}", sig.pair.a, sig.pair.b)
            } else {
                format!("{}_{}", sig.pair.b, sig.pair.a)
            };
            self.update_pair_stats_from_signal(&sig, &pair_key);
            if self.signal_tx.try_send(sig.clone()) {
                use crate::stats::SIGNALS_EMITTED;
                SIGNALS_EMITTED.fetch_add(1, Ordering::Relaxed);
                record_recent_signal(sig.clone());
                counter!("arb_signals_total", "path" => "edges3");
                record_profit(sig.profit_bps);
                record_signal_profit_bps(sig.profit_bps);
                let now_ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                if now_ns > ts {
                    record_latency_ns(now_ns - ts);
                }
                if *HOTPATH_LOG {
                    println!(
                        "HOT_ARB kind=3leg-local profit_bps={} profit_lamports={} amount_in={} dex_sell={} dex_mid={} dex_buy={} sell_pool={} mid_pool={} buy_pool={} slot={} id={} pair={}/{}",
                        sig.profit_bps,
                        sig.expected_profit_out,
                        sig.amount_in,
                        sig.dex_sell,
                        sig.dex_mid.clone().unwrap_or_default(),
                        sig.dex_buy,
                        sig.sell_pool_id,
                        sig.mid_pool_id.clone().unwrap_or_default(),
                        sig.buy_pool_id,
                        sig.slot,
                        sig.id,
                        sig.pair.a,
                        sig.pair.b,
                    );
                }
            }
        }
    }

    pub fn evaluate_3leg(
        &self,
        e1: &Arc<EdgeDelta>,
        e2: &Arc<EdgeDelta>,
        e3: &Arc<EdgeDelta>,
    ) -> Option<(i128, u128, u128, i32, u64)> {
        let base_size = self
            .parse_size_to_boundary(e1)
            .min(self.parse_size_to_boundary(e2))
            .min(self.parse_size_to_boundary(e3));
        if base_size == 0 {
            return None;
        }
        let p1 = self.marginal_price(e1)?;
        let p2 = self.marginal_price(e2)?;
        let p3 = self.marginal_price(e3)?;
        let raw_gross =
            p1 * p2 * p3 * self.fee_factor(e1) * self.fee_factor(e2) * self.fee_factor(e3);
        if !raw_gross.is_finite() {
            return None;
        }
        let chosen = choose_size(base_size);
        let adj = depth_adjustment_factor(chosen, base_size, 3);
        if base_size > 0 {
            let util_bps = ((chosen as f64 / base_size as f64) * 10_000.0) as i32;
            record_size_utilization_bps(util_bps);
        }
        let depth_bps = (adj * 10_000.0) as i32;
        record_depth_factor_bps(depth_bps);
        let gross = raw_gross * adj;
        if !gross.is_finite() {
            return None;
        }
        let profit_bps = ((gross - 1.0) * 10_000.0) as i32;
        if profit_bps < MIN_PROFIT_BPS_ATOM.load(Ordering::Relaxed) as i32 {
            return None;
        }
        let profit_lamports = (chosen as f64 * (gross - 1.0)) as i128;
        let ts = e1.updated_at_ns.min(e2.updated_at_ns).min(e3.updated_at_ns);
        Some((profit_lamports, chosen, base_size, profit_bps, ts))
    }

    /// Update configuration (for AI agent tuning)
    pub fn update_config(&mut self, min_profit_bps: u32, max_slippage_bps: u32) {
        // No struct fields to mutate; propagate to global atomics.
        set_edge_thresholds(Some(min_profit_bps), Some(max_slippage_bps));
    }

    #[cfg(feature = "diagnostics")]
    pub fn refresh_structural_gauges(&self) {
        EDGE_PAIR_COUNT.store(self.edges.len() as u64, Ordering::Relaxed);
        TOKEN_COUNT.store(self.token_index.len() as u64, Ordering::Relaxed);
        // Compute average adjacency: total outgoing edges / tokens
        let mut total_adj = 0u64;
        let mut token_nodes = 0u64;
        for entry in self.token_index.iter() {
            token_nodes += 1;
            // Count unique outgoing edges from this token (approx via pair_keys)
            total_adj += entry.value().len() as u64;
        }
        if token_nodes > 0 {
            let avg = (total_adj * 1_000) / token_nodes; // scale by 1e3
            AVG_ADJACENCY.store(avg as u64, Ordering::Relaxed);
        }
    }

    // --- Test helpers ---
    pub fn test_calculate_edge_profit_3(
        &self,
        e1: &EdgeDelta,
        e2: &EdgeDelta,
        e3: &EdgeDelta,
    ) -> Option<i128> {
        self.evaluate_3leg(
            &Arc::new(e1.clone()),
            &Arc::new(e2.clone()),
            &Arc::new(e3.clone()),
        )
        .map(|(p, _, _, _, _)| p)
    }
}

impl EdgeArbitrageDetector {
    #[inline]
    fn update_pair_stats_from_signal(&self, sig: &crate::models::ArbSignal, pair_key: &str) {
        let now_ns = sig.emitted_at_ns;
        let profit_bps = sig.profit_bps;
        use dashmap::mapref::entry::Entry;
        match self.pair_stats.entry(pair_key.to_string()) {
            Entry::Occupied(mut occ) => {
                let e = occ.get_mut();
                e.signals_emitted += 1;
                e.cumulative_profit_bps += profit_bps as i64;
                if profit_bps > e.max_profit_bps {
                    e.max_profit_bps = profit_bps;
                }
                e.last_updated_ns = now_ns;
            }
            Entry::Vacant(vac) => {
                vac.insert(PairEdgeStats {
                    signals_emitted: 1,
                    max_profit_bps: profit_bps,
                    cumulative_profit_bps: profit_bps as i64,
                    last_updated_ns: now_ns,
                    max_observed_potential_bps: 0,
                    last_potential_bps: 0,
                    opportunity_events: 0,
                    last_opportunity_ns: 0,
                });
            }
        }
    }

    /// Public stats snapshot for pairs ordered by max_profit_bps then signals_emitted
    pub fn list_pair_stats_json(&self, limit: Option<usize>) -> serde_json::Value {
        use crate::token_meta::{prime_tokens, symbol_snapshot};
        let mut rows: Vec<serde_json::Value> = Vec::new();
        for stat_entry in self.pair_stats.iter() {
            let pair_key = stat_entry.key();
            let parts: Vec<&str> = pair_key.split('_').collect();
            if parts.len() != 2 {
                continue;
            }
            let (token_a, token_b) = (parts[0], parts[1]);
            prime_tokens([token_a, token_b]);
            let sym_a = symbol_snapshot(token_a);
            let sym_b = symbol_snapshot(token_b);
            let pair_symbol = match (&sym_a, &sym_b) {
                (Some(a), Some(b)) => Some(format!("{}/{}", a, b)),
                _ => None,
            };
            let s = stat_entry.value();
            rows.push(serde_json::json!({
                "pair_key": pair_key,
                "token_a": token_a,
                "token_b": token_b,
                "symbol_a": sym_a,
                "symbol_b": sym_b,
                "pair_symbol": pair_symbol,
                "signals_emitted": s.signals_emitted,
                "max_profit_bps": s.max_profit_bps,
                "cumulative_profit_bps": s.cumulative_profit_bps,
                "last_updated_ns": s.last_updated_ns,
                "max_observed_potential_bps": s.max_observed_potential_bps,
                "last_potential_bps": s.last_potential_bps,
                "opportunity_events": s.opportunity_events,
                "last_opportunity_ns": s.last_opportunity_ns
            }));
        }
        rows.sort_by(|a, b| {
            // Primary: max realized profit
            let a_profit = a
                .get("max_profit_bps")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let b_profit = b
                .get("max_profit_bps")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if a_profit == b_profit {
                // Secondary: max observed potential
                let a_pot = a
                    .get("max_observed_potential_bps")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let b_pot = b
                    .get("max_observed_potential_bps")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if a_pot == b_pot {
                    // Tertiary: number of signals emitted
                    let a_sig = a
                        .get("signals_emitted")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let b_sig = b
                        .get("signals_emitted")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    b_sig.cmp(&a_sig)
                } else {
                    b_pot.cmp(&a_pot)
                }
            } else {
                b_profit.cmp(&a_profit)
            }
        });
        let lim = limit.unwrap_or(500);
        serde_json::Value::Array(rows.into_iter().take(lim).collect())
    }

    /// Latest cross-venue price observations for a given USDC mint with metadata enrichment.
    pub fn usdc_prices_json(&self, usdc_mint: &str) -> serde_json::Value {
        use crate::token_meta::{decimals_snapshot, prime_tokens, symbol_snapshot};
        use serde_json::json;
        // venue_id -> (ts, other_per_usdc, usdc_per_other, other_mint, pool, direction, raw_q64)
        let mut per_venue: BTreeMap<u32, VenuePriceEntry> = BTreeMap::new();
        let decode_q64 = |bytes: &Vec<u8>| -> Option<(f64, u128)> {
            if bytes.len() < 16 {
                return None;
            }
            let mut b = [0u8; 16];
            b.copy_from_slice(&bytes[..16]);
            let raw = u128::from_le_bytes(b);
            const TWO64: f64 = 18446744073709551616.0; // 2^64
            Some(((raw as f64) / TWO64, raw))
        };
        for entry in self.edges.iter() {
            for e in entry.value().iter() {
                let a = bs58::encode(&e.token_in).into_string();
                let b = bs58::encode(&e.token_out).into_string();
                if a != usdc_mint && b != usdc_mint {
                    continue;
                }
                if let Some((price, raw_q64)) = decode_q64(&e.marginal_price_q64) {
                    let should_replace = match per_venue.get(&e.venue_id) {
                        Some(existing) => e.updated_at_ns > existing.0,
                        None => true,
                    };
                    if should_replace {
                        let other = if a == usdc_mint { b.clone() } else { a.clone() };
                        let direction = if a == usdc_mint { 0 } else { 1 }; // orientation of edge
                        let (other_per_usdc, usdc_per_other) = if direction == 0 {
                            if price > 0.0 {
                                (price, 1.0 / price)
                            } else {
                                continue;
                            }
                        } else if price > 0.0 {
                            (1.0 / price, price)
                        } else {
                            continue;
                        };
                        per_venue.insert(
                            e.venue_id,
                            (
                                e.updated_at_ns,
                                other_per_usdc,
                                usdc_per_other,
                                other,
                                bs58::encode(&e.pool_pubkey).into_string(),
                                direction,
                                raw_q64,
                            ),
                        );
                    }
                }
            }
        }
        prime_tokens([usdc_mint]);
        let usdc_sym = symbol_snapshot(usdc_mint);
        let usdc_dec = decimals_snapshot(usdc_mint);
        let mut out = Vec::new();
        for (venue, (ts, other_per_usdc, usdc_per_other, other, pool, dir, raw_q64)) in
            per_venue.into_iter()
        {
            prime_tokens([other.as_str()]);
            let other_sym = symbol_snapshot(&other);
            let other_dec = decimals_snapshot(&other);
            let adjusted = if let (Some(d_u), Some(d_o)) = (usdc_dec, other_dec) {
                let exp = (d_u as i32) - (d_o as i32);
                other_per_usdc * 10f64.powi(exp)
            } else {
                other_per_usdc
            };
            out.push(json!({
                "venue_id": venue,
                "updated_at_ns": ts,
                "usdc_mint": usdc_mint,
                "usdc_symbol": usdc_sym,
                "usdc_decimals": usdc_dec,
                "other_mint": other,
                "other_symbol": other_sym,
                "other_decimals": other_dec,
                "direction": dir,
                "price_other_per_1_usdc": other_per_usdc,
                "price_usdc_per_1_other": usdc_per_other,
                "adjusted_price_other_per_1_usdc": adjusted,
                "raw_price_q64": raw_q64.to_string(),
                "pool_id": pool
            }));
        }
        serde_json::Value::Array(out)
    }

    /// Generic latest cross-venue price observations for a base/quote mint pair.
    /// Returns latest edge per venue involving exactly the two mints (any direction) with normalized prices.
    pub fn pair_prices_json(&self, base_mint: &str, quote_mint: &str) -> serde_json::Value {
        use crate::token_meta::{decimals_snapshot, prime_tokens, symbol_snapshot};
        use serde_json::json;
        if base_mint == quote_mint {
            return serde_json::json!({"error":"same_mint"});
        }
        let mut per_venue: BTreeMap<u32, VenuePriceEntry> = BTreeMap::new();
        let decode_q64 = |bytes: &Vec<u8>| -> Option<(f64, u128)> {
            if bytes.len() < 16 {
                return None;
            }
            let mut b = [0u8; 16];
            b.copy_from_slice(&bytes[..16]);
            let raw = u128::from_le_bytes(b);
            const TWO64: f64 = 18446744073709551616.0;
            Some(((raw as f64) / TWO64, raw))
        };
        for entry in self.edges.iter() {
            for e in entry.value().iter() {
                let a = bs58::encode(&e.token_in).into_string();
                let b = bs58::encode(&e.token_out).into_string();
                let involves = (a.as_str() == base_mint && b.as_str() == quote_mint)
                    || (a.as_str() == quote_mint && b.as_str() == base_mint);
                if !involves {
                    continue;
                }
                if let Some((price, raw_q64)) = decode_q64(&e.marginal_price_q64) {
                    let should_replace = match per_venue.get(&e.venue_id) {
                        Some(ex) => e.updated_at_ns > ex.0,
                        None => true,
                    };
                    if should_replace {
                        // Edge expresses price = out / in.
                        // We want price of base in terms of quote and vice versa.
                        let (base_per_quote, quote_per_base, direction) =
                            if a == base_mint && b == quote_mint {
                                if price > 0.0 {
                                    (1.0 / price, price, 0)
                                } else {
                                    continue;
                                }
                            } else {
                                // a == quote, b == base
                                if price > 0.0 {
                                    (price, 1.0 / price, 1)
                                } else {
                                    continue;
                                }
                            };
                        per_venue.insert(
                            e.venue_id,
                            (
                                e.updated_at_ns,
                                base_per_quote,
                                quote_per_base,
                                base_mint.to_string(),
                                quote_mint.to_string(),
                                direction,
                                raw_q64,
                            ),
                        );
                    }
                }
            }
        }
        // Metadata
        prime_tokens([base_mint, quote_mint]);
        let base_sym = symbol_snapshot(base_mint);
        let quote_sym = symbol_snapshot(quote_mint);
        let base_dec = decimals_snapshot(base_mint);
        let quote_dec = decimals_snapshot(quote_mint);
        let mut out = Vec::new();
        for (venue, (ts, base_per_quote, quote_per_base, _bm, _qm, direction, raw_q64)) in
            per_venue.into_iter()
        {
            let adjusted = if let (Some(db), Some(dq)) = (base_dec, quote_dec) {
                // Convert base_per_quote into human decimals: multiply by 10^{dq - db}
                let exp = (dq as i32) - (db as i32);
                base_per_quote * 10f64.powi(exp)
            } else {
                base_per_quote
            };
            out.push(json!({
                "venue_id": venue,
                "updated_at_ns": ts,
                "base_mint": base_mint,
                "base_symbol": base_sym,
                "base_decimals": base_dec,
                "quote_mint": quote_mint,
                "quote_symbol": quote_sym,
                "quote_decimals": quote_dec,
                "price_base_per_1_quote": base_per_quote,
                "price_quote_per_1_base": quote_per_base,
                "adjusted_price_base_per_1_quote": adjusted,
                "direction": direction,
                "raw_price_q64": raw_q64.to_string()
            }));
        }
        serde_json::Value::Array(out)
    }

    /// Lightweight internal counts (pairs tracked, pairs with realized/opportunity stats)
    pub fn stats_counts(&self) -> (usize, usize) {
        (self.edges.len(), self.pair_stats.len())
    }
}
// <-- end impl EdgeArbitrageDetector

/// Kafka consumer for edge assembler messages
/// This connects your existing edge assembler to the arb engine
pub struct EdgeConsumer {
    detector: Arc<EdgeArbitrageDetector>,
    consumer: rdkafka::consumer::StreamConsumer,
    topic: String,
}

impl EdgeConsumer {
    pub fn new(
        detector: Arc<EdgeArbitrageDetector>,
        kafka_broker: String,
        topic: String,
    ) -> Result<Self> {
        let consumer: rdkafka::consumer::StreamConsumer = rdkafka::config::ClientConfig::new()
            .set("group.id", "arb-engine-edge-consumer")
            .set("bootstrap.servers", &kafka_broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set("fetch.wait.max.ms", "1")
            .set("fetch.min.bytes", "1")
            .create()?;

        Ok(Self {
            detector,
            consumer,
            topic,
        })
    }

    /// Start consuming edge updates from Kafka
    pub async fn start(&self) -> Result<()> {
        use rdkafka::consumer::Consumer;
        use rdkafka::message::Message;

        println!("🚀 Starting Edge Consumer for topic: {}", self.topic);

        // Subscribe to edge delta topic
        self.consumer.subscribe(&[&self.topic])?;

        // Main consumption loop
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Some(payload) = message.payload() {
                        counter!("edge_messages_total");
                        match decode_and_validate_edge(payload) {
                            Ok(ed) => {
                                if let Err(e) = self.detector.process_edge_update(ed) {
                                    eprintln!("❌ Error processing edge: {}", e);
                                    counter!("edge_rejected_total", "reason" => "process_error");
                                }
                            }
                            Err(reason) => {
                                counter!("edge_decode_errors_total", "reason" => reason);
                            }
                        }
                    } else {
                        // empty payload
                        counter!("edge_rejected_total", "reason" => "empty_payload");
                    }
                    if let Err(e) = self
                        .consumer
                        .commit_message(&message, rdkafka::consumer::CommitMode::Async)
                    {
                        eprintln!("❌ Failed to commit offset: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Kafka error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}

/// Decode prost EdgeDelta bytes and apply basic validation.
pub fn decode_and_validate_edge(bytes: &[u8]) -> Result<EdgeDelta, &'static str> {
    let ed: EdgeDelta = EdgeDelta::decode(bytes).map_err(|_| "decode_fail")?;
    if ed.direction > 1 {
        return Err("bad_direction");
    }
    if ed.marginal_price_q64.len() != 16 {
        return Err("bad_price_len");
    }
    if ed.size_to_next_boundary_in.len() != 16 {
        return Err("bad_size_len");
    }
    if ed.fee_bps_total > 2000 {
        return Err("fee_high");
    }
    // Basic price sanity
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&ed.marginal_price_q64[..16]);
    let raw = u128::from_le_bytes(arr) as f64;
    let price = raw / 18446744073709551616.0; // 2^64
    if !price.is_finite() {
        return Err("price_nan");
    }
    if price <= 0.0 || price > 1e12 {
        return Err("price_range");
    }
    Ok(ed)
}

// Lightweight wrapper for tests (kept public for integration tests).
pub fn test_decode_validate_edge(bytes: &[u8]) -> Result<(), &'static str> {
    decode_and_validate_edge(bytes).map(|_| ())
}

// end of file
