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
#[cfg(feature = "diagnostics")]
use crate::stats::{AVG_ADJACENCY, EDGE_PAIR_COUNT, MAX_EDGE_FACTOR, TOKEN_COUNT};
use anyhow::Result;
use dashmap::DashMap;
use edge_assembler::edge::EdgeDelta;
use metrics::counter;
use once_cell::sync::Lazy;
use prost::Message;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing; // for decoding EdgeDelta from prost bytes

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
            .unwrap_or(100),
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

// Re-export for other modules
pub use edge_assembler::edge::EdgeDelta as ProtoEdgeDelta;

/// Real-time edge-based arbitrage detector
/// Integrates with your existing edge assembler for <10ms latency
pub struct EdgeArbitrageDetector {
    // token_pair_key -> Vec<Arc<EdgeDelta>> (both directions included)
    edges: DashMap<String, Vec<Arc<EdgeDelta>>>,
    // token (base58) -> set of pair keys containing token (stored as Vec, dedup on insert)
    token_index: DashMap<String, Vec<String>>,
    signal_tx: SignalTx,
    // thresholds now always read from global atomics (mutable via HTTP API); legacy fields removed
    brute_force_edge_limit: usize,
}

impl EdgeArbitrageDetector {
    pub fn new(signal_tx: SignalTx) -> Self {
        Self {
            edges: DashMap::new(),
            token_index: DashMap::new(),
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
        self.edges
            .entry(pair_key.clone())
            .or_insert_with(Vec::new)
            .retain(|e| e.edge_id != edge.edge_id);
        self.edges
            .entry(pair_key.clone())
            .or_insert_with(Vec::new)
            .push(Arc::new(edge.clone()));

        // Update token index (dedup simple linear scan; small vector expected per token)
        let a = bs58::encode(&edge.token_in).into_string();
        let b = bs58::encode(&edge.token_out).into_string();
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

        Ok(())
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
                    if let Some((profit_lamports, chosen_size, base_size, ts, profit_bps)) =
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
                if self.signal_tx.try_send(sig.clone()) {
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
                                        base_size,
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
            if self.signal_tx.try_send(sig.clone()) {
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
            }
        }
    }

    #[cfg(any(test, feature = "stubs"))]
    pub fn force_scan_all(&self) {
        self.scan_for_arbitrage_3hop_full();
    }

    /// Calculate arbitrage between two venues
    #[inline]
    // build_signal_* helpers removed (signals built inline by evaluate functions)

    /// Calculate profit between two edges
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

    fn directed_tokens(&self, edge: &Arc<EdgeDelta>) -> (Vec<u8>, Vec<u8>) {
        // Retained for compatibility (allocating) – consider removing usages.
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
                                        base_size,
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
            if self.signal_tx.try_send(sig.clone()) {
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
