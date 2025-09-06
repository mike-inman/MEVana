use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use edge_assembler::edge::{edge_delta, EdgeDelta};
use prost::Message;

#[derive(Default, Debug)]
pub struct VenueStats {
    pub total: AtomicU64,
    pub dir0: AtomicU64,
    pub dir1: AtomicU64,
    pub ok: AtomicU64,
    pub bad_price: AtomicU64,
    pub bad_size: AtomicU64,
    pub bad_state_variant: AtomicU64,
    pub bad_tokens: AtomicU64,
    pub distinct_pools: AtomicU64,
    pub distinct_pairs: AtomicU64,
    pub first_error: RwLock<Option<String>>,
    pub bad_dump_count: AtomicU64,
    pub last_update_ns: AtomicU64, // newest updated_at_ns seen for this venue
    // feature readiness flags (1 = observed at least one valid example)
    pub feat_price_len: AtomicU64,
    pub feat_size_len: AtomicU64,
    pub feat_tokens_len: AtomicU64,
    pub feat_state_variant: AtomicU64,
}

impl VenueStats {
    fn record_err(&self, kind: &str, detail: String, counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.first_error.write().unwrap();
        if guard.is_none() {
            *guard = Some(format!("{}:{}", kind, detail));
        }
    }
}

#[derive(Clone)]
pub struct EdgeVerifier {
    venues: Arc<HashMap<u32, Arc<VenueStats>>>,
    pools_seen: Arc<RwLock<HashMap<u32, HashSet<Vec<u8>>>>>,
    pairs_seen: Arc<RwLock<HashMap<u32, HashSet<(Vec<u8>, Vec<u8>)>>>>,
    last_emit: Arc<RwLock<Instant>>,
    emit_interval: Duration,
    enabled: bool,
    log_bad: bool,
    log_all: bool,
    dump_bad_limit: u64,
    dump_raw: bool,
}

impl EdgeVerifier {
    pub fn new(enabled: bool) -> Self {
        let mut map = HashMap::new();
        // EDGE_VENUES env var allows dynamic venue list e.g. "1,2,3,4,5" (defaults to 1..=4)
        let venue_ids: Vec<u32> = std::env::var("EDGE_VENUES")
            .ok()
            .map(|s| {
                s.split([',', ' '])
                    .filter_map(|p| {
                        let t = p.trim();
                        if t.is_empty() {
                            None
                        } else {
                            t.parse::<u32>().ok()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|v: &Vec<u32>| !v.is_empty())
            .unwrap_or_else(|| vec![1, 2, 3, 4]);
        for vid in venue_ids {
            map.insert(vid, Arc::new(VenueStats::default()));
        }
        let log_bad = std::env::var("EDGE_LOG_BAD")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true); // default true to surface issues fast
        let log_all = std::env::var("EDGE_LOG_ALL")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let emit_interval = std::env::var("EDGE_STATS_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(10));
        let dump_raw = std::env::var("EDGE_DUMP_RAW")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let dump_bad_limit = std::env::var("EDGE_DUMP_BAD_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5u64);
        Self {
            venues: Arc::new(map),
            pools_seen: Arc::new(RwLock::new(HashMap::new())),
            pairs_seen: Arc::new(RwLock::new(HashMap::new())),
            last_emit: Arc::new(RwLock::new(Instant::now())),
            emit_interval,
            enabled,
            log_bad,
            log_all,
            dump_bad_limit,
            dump_raw,
        }
    }

    pub fn observe(&self, ed: &EdgeDelta) {
        if !self.enabled {
            return;
        }
        let v = match self.venues.get(&ed.venue_id) {
            Some(v) => v,
            None => return,
        };
        v.total.fetch_add(1, Ordering::Relaxed);
        if ed.direction == 0 {
            v.dir0.fetch_add(1, Ordering::Relaxed);
        } else if ed.direction == 1 {
            v.dir1.fetch_add(1, Ordering::Relaxed);
        }
        let mut had_error = false;
        let mut reasons: Vec<&'static str> = Vec::new();
        // Validate tokens length (expect 32 bytes)
        if ed.token_in.len() != 32 || ed.token_out.len() != 32 {
            v.record_err(
                "bad_tokens",
                format!("{}:{}", ed.token_in.len(), ed.token_out.len()),
                &v.bad_tokens,
            );
            had_error = true;
            reasons.push("bad_tokens");
        }
        // Validate marginal_price
        if ed.marginal_price_q64.len() != 16 {
            v.record_err(
                "bad_price_len",
                ed.marginal_price_q64.len().to_string(),
                &v.bad_price,
            );
            had_error = true;
            reasons.push("bad_price_len");
        }
        // size boundary
        if ed.size_to_next_boundary_in.len() != 16 {
            v.record_err(
                "bad_size_len",
                ed.size_to_next_boundary_in.len().to_string(),
                &v.bad_size,
            );
            had_error = true;
            reasons.push("bad_size_len");
        }
        // State variant correctness
        let state_kind = if let Some(state) = &ed.state {
            let ok = match (ed.venue_id, state) {
                (1, edge_delta::State::Clmm(_)) => true,
                (2, edge_delta::State::Cpmm(_)) => true,
                (3, edge_delta::State::Dlmm(_)) => true,
                (4, edge_delta::State::Clob(_)) => true,
                _ => false,
            };
            if !ok {
                v.record_err(
                    "state_variant",
                    format!("vid={} mismatched", ed.venue_id),
                    &v.bad_state_variant,
                );
                had_error = true;
                reasons.push("state_variant");
            }
            match state {
                edge_delta::State::Clmm(_) => "clmm",
                edge_delta::State::Cpmm(_) => "cpmm",
                edge_delta::State::Dlmm(_) => "dlmm",
                edge_delta::State::Clob(_) => "clob",
            }
        } else {
            v.record_err(
                "state_missing",
                format!("vid={}", ed.venue_id),
                &v.bad_state_variant,
            );
            had_error = true;
            reasons.push("state_missing");
            "missing"
        };
        if !had_error {
            v.ok.fetch_add(1, Ordering::Relaxed);
            // mark feature flags on first success occurrences
            if ed.marginal_price_q64.len() == 16 {
                v.feat_price_len.store(1, Ordering::Relaxed);
            }
            if ed.size_to_next_boundary_in.len() == 16 {
                v.feat_size_len.store(1, Ordering::Relaxed);
            }
            if ed.token_in.len() == 32 && ed.token_out.len() == 32 {
                v.feat_tokens_len.store(1, Ordering::Relaxed);
            }
            v.feat_state_variant.store(1, Ordering::Relaxed); // success implies correct state variant
        }
        // update last_update_ns if newer
        if ed.updated_at_ns > v.last_update_ns.load(Ordering::Relaxed) {
            v.last_update_ns.store(ed.updated_at_ns, Ordering::Relaxed);
        }
        // Optional per-edge logging
        if (self.log_all && !had_error) || (self.log_bad && had_error) {
            // Convert fields for readability
            let pool_b58 = bs58::encode(&ed.pool_pubkey).into_string();
            let tin = bs58::encode(&ed.token_in).into_string();
            let tout = bs58::encode(&ed.token_out).into_string();
            if had_error {
                println!("EDGE_BAD venue={} reasons={} dir={} pool={} tok_in={} tok_out={} price_len={} size_len={} state={}",
                    ed.venue_id,
                    reasons.join("|"),
                    ed.direction,
                    pool_b58,
                    tin,
                    tout,
                    ed.marginal_price_q64.len(),
                    ed.size_to_next_boundary_in.len(),
                    state_kind,
                );
                if self.dump_raw {
                    // Only dump first N per venue to avoid log flood
                    let dumped = v.bad_dump_count.fetch_add(1, Ordering::Relaxed);
                    if dumped < self.dump_bad_limit {
                        // Encode full prost message
                        let mut buf = Vec::with_capacity(256);
                        if ed.encode(&mut buf).is_ok() {
                            use base64::Engine as _;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
                            let hex = hex::encode(&buf);
                            println!(
                                "EDGE_BAD_DUMP venue={} idx={} bytes={} b64={} hex={} ",
                                ed.venue_id,
                                dumped,
                                buf.len(),
                                b64,
                                hex
                            );
                        }
                    }
                }
            } else {
                println!("EDGE_OK venue={} dir={} pool={} tok_in={} tok_out={} price_len={} size_len={} state={}",
                    ed.venue_id,
                    ed.direction,
                    pool_b58,
                    tin,
                    tout,
                    ed.marginal_price_q64.len(),
                    ed.size_to_next_boundary_in.len(),
                    state_kind,
                );
            }
        }
        // Track distinct pools / pairs
        {
            let mut pools = self.pools_seen.write().unwrap();
            let set = pools.entry(ed.venue_id).or_insert_with(HashSet::new);
            if set.insert(ed.pool_pubkey.clone()) {
                let distinct = set.len() as u64;
                v.distinct_pools.store(distinct, Ordering::Relaxed);
            }
        }
        {
            let mut pairs = self.pairs_seen.write().unwrap();
            let set = pairs.entry(ed.venue_id).or_insert_with(HashSet::new);
            let mut a = ed.token_in.clone();
            let mut b = ed.token_out.clone();
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            if set.insert((a, b)) {
                let distinct = set.len() as u64;
                v.distinct_pairs.store(distinct, Ordering::Relaxed);
            }
        }
        // Periodic emit
        let mut guard = self.last_emit.write().unwrap();
        if guard.elapsed() >= self.emit_interval {
            *guard = Instant::now();
            self.emit_summary_log();
        }
    }

    fn emit_summary_log(&self) {
        for (vid, stats) in self.venues.iter() {
            let first_err = stats
                .first_error
                .read()
                .unwrap()
                .clone()
                .unwrap_or_else(|| "none".into());
            let total = stats.total.load(Ordering::Relaxed).max(1); // avoid div0
            let ok = stats.ok.load(Ordering::Relaxed);
            let success_pct = (ok * 100) / total;
            let last_ns = stats.last_update_ns.load(Ordering::Relaxed);
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            let staleness_ms = if last_ns > 0 && now_ns > last_ns {
                (now_ns - last_ns) / 1_000_000
            } else {
                0
            };
            println!(
                "EDGE_STATS venue={} total={} ok={} success_pct={} dir0={} dir1={} pools={} pairs={} bad_price={} bad_size={} bad_state={} bad_tokens={} staleness_ms={} first_error={}",
                vid, total, ok, success_pct,
                stats.dir0.load(Ordering::Relaxed),
                stats.dir1.load(Ordering::Relaxed),
                stats.distinct_pools.load(Ordering::Relaxed),
                stats.distinct_pairs.load(Ordering::Relaxed),
                stats.bad_price.load(Ordering::Relaxed),
                stats.bad_size.load(Ordering::Relaxed),
                stats.bad_state_variant.load(Ordering::Relaxed),
                stats.bad_tokens.load(Ordering::Relaxed),
                staleness_ms,
                first_err);
            // Export gauges (without labels for portability). If label separation needed, create distinct metric names per venue.
            // (Prometheus exporter already scrapes /metrics; for now we only log gauges.)
            // Feature readiness aggregated (1 if all present else 0)
            // feature readiness aggregated (all four observed => implied by zero missing features in dashboard JSON)
        }
    }

    pub fn snapshot_json(&self) -> serde_json::Value {
        use serde_json::json;
        let mut arr = Vec::new();
        for (vid, stats) in self.venues.iter() {
            let last_ns = stats.last_update_ns.load(Ordering::Relaxed);
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            let staleness_ms = if last_ns > 0 && now_ns > last_ns {
                (now_ns - last_ns) / 1_000_000
            } else {
                0
            };
            arr.push(json!({
                "venue_id": vid,
                "total": stats.total.load(Ordering::Relaxed),
                "ok": stats.ok.load(Ordering::Relaxed),
                "success_pct": if stats.total.load(Ordering::Relaxed)>0 { stats.ok.load(Ordering::Relaxed)*100 / stats.total.load(Ordering::Relaxed) } else {0},
                "dir0": stats.dir0.load(Ordering::Relaxed),
                "dir1": stats.dir1.load(Ordering::Relaxed),
                "distinct_pools": stats.distinct_pools.load(Ordering::Relaxed),
                "distinct_pairs": stats.distinct_pairs.load(Ordering::Relaxed),
                "bad_price": stats.bad_price.load(Ordering::Relaxed),
                "bad_size": stats.bad_size.load(Ordering::Relaxed),
                "bad_state": stats.bad_state_variant.load(Ordering::Relaxed),
                "bad_tokens": stats.bad_tokens.load(Ordering::Relaxed),
                "first_error": stats.first_error.read().unwrap().clone(),
                "staleness_ms": staleness_ms,
                "features": {
                    "price_len": stats.feat_price_len.load(Ordering::Relaxed),
                    "size_len": stats.feat_size_len.load(Ordering::Relaxed),
                    "tokens_len": stats.feat_tokens_len.load(Ordering::Relaxed),
                    "state_variant": stats.feat_state_variant.load(Ordering::Relaxed),
                }
            }));
        }
        json!({"venues": arr})
    }

    /// More opinionated dashboard summary including missing feature names.
    pub fn dashboard_json(&self) -> serde_json::Value {
        use serde_json::json;
        let mut venues_out = Vec::new();
        for (vid, stats) in self.venues.iter() {
            let mut missing = Vec::new();
            if stats.feat_price_len.load(Ordering::Relaxed) == 0 {
                missing.push("price_len");
            }
            if stats.feat_size_len.load(Ordering::Relaxed) == 0 {
                missing.push("size_len");
            }
            if stats.feat_tokens_len.load(Ordering::Relaxed) == 0 {
                missing.push("tokens_len");
            }
            if stats.feat_state_variant.load(Ordering::Relaxed) == 0 {
                missing.push("state_variant");
            }
            let last_ns = stats.last_update_ns.load(Ordering::Relaxed);
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            let staleness_ms = if last_ns > 0 && now_ns > last_ns {
                (now_ns - last_ns) / 1_000_000
            } else {
                0
            };
            venues_out.push(json!({
                "venue_id": vid,
                "total": stats.total.load(Ordering::Relaxed),
                "ok": stats.ok.load(Ordering::Relaxed),
                "success_pct": if stats.total.load(Ordering::Relaxed)>0 { stats.ok.load(Ordering::Relaxed)*100 / stats.total.load(Ordering::Relaxed) } else {0},
                "staleness_ms": staleness_ms,
                "missing_features": missing,
            }));
        }
        json!({"venues": venues_out})
    }
}

// (Dashboard JSON helper via EdgeVerifier instance lives where static is defined)
