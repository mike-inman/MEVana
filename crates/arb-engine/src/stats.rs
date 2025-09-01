use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct DynHistogramConfig {
    pub target_buckets: usize,
    pub max_value_bps: i64,
}
impl Default for DynHistogramConfig {
    fn default() -> Self {
        Self {
            target_buckets: 20,
            max_value_bps: 10_000,
        }
    }
}

pub struct DynHistogram {
    bounds: Vec<i64>, // inclusive upper bounds
    counts: Vec<AtomicU64>,
    overflow: AtomicU64, // values > last bound
    sum: AtomicI64,
    max: AtomicI64,
    count: AtomicU64,
}

impl DynHistogram {
    fn new(cfg: &DynHistogramConfig) -> Self {
        let mut bounds = Vec::with_capacity(cfg.target_buckets);
        // exponential-ish spacing
        for i in 0..cfg.target_buckets {
            bounds.push(
                ((cfg.max_value_bps as f64)
                    * ((i as f64 + 1.0) / (cfg.target_buckets as f64)).powf(1.6))
                    as i64,
            );
        }
        bounds.sort();
        let counts = bounds.iter().map(|_| AtomicU64::new(0)).collect();
        Self {
            bounds,
            counts,
            overflow: AtomicU64::new(0),
            sum: AtomicI64::new(0),
            max: AtomicI64::new(0),
            count: AtomicU64::new(0),
        }
    }
    fn record(&self, v: i64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(v, Ordering::Relaxed);
        let mut cur = self.max.load(Ordering::Relaxed);
        while v > cur {
            if self
                .max
                .compare_exchange(cur, v, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
            cur = self.max.load(Ordering::Relaxed);
        }
        let mut placed = false;
        for (i, b) in self.bounds.iter().enumerate() {
            if v <= *b {
                self.counts[i].fetch_add(1, Ordering::Relaxed);
                placed = true;
                break;
            }
        }
        if !placed {
            self.overflow.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn maybe_resize(&mut self, cfg: &DynHistogramConfig) {
        let total = self.count.load(Ordering::Relaxed);
        if total < 100 {
            return;
        }
        let overflow = self.overflow.load(Ordering::Relaxed);
        let last = *self.bounds.last().unwrap_or(&0);
        let overflow_ratio_high = overflow * 20 > total && last < cfg.max_value_bps; // >5%
        let bucket_mismatch = self.bounds.len() != cfg.target_buckets || last != cfg.max_value_bps;
        if !overflow_ratio_high && !bucket_mismatch {
            return;
        }
        // cumulative distribution (approx)
        let mut cumulative: Vec<(i64, u64)> = Vec::with_capacity(self.bounds.len());
        let mut run = 0u64;
        for (i, b) in self.bounds.iter().enumerate() {
            run += self.counts[i].load(Ordering::Relaxed);
            cumulative.push((*b, run));
        }
        let total_with_overflow = run + overflow;
        // new bounds
        let mut new_bounds: Vec<i64> = (0..cfg.target_buckets)
            .map(|i| {
                let frac = (i as f64 + 1.0) / (cfg.target_buckets as f64);
                (((cfg.max_value_bps as f64) * frac.powf(1.6)) as i64).min(cfg.max_value_bps)
            })
            .collect();
        new_bounds.sort();
        let mut new_counts_raw = vec![0u64; new_bounds.len()];
        if total_with_overflow > 0 {
            for (i, nb) in new_bounds.iter().enumerate() {
                let mut old_cum = total_with_overflow;
                for (ob, cum) in &cumulative {
                    if ob >= nb {
                        old_cum = *cum;
                        break;
                    }
                }
                let prev: u64 = new_counts_raw[..i].iter().sum();
                new_counts_raw[i] = old_cum.saturating_sub(prev);
            }
            let assigned: u64 = new_counts_raw.iter().sum();
            if assigned < total_with_overflow {
                if let Some(last_ct) = new_counts_raw.last_mut() {
                    *last_ct += total_with_overflow - assigned;
                }
            }
        }
        self.bounds = new_bounds;
        self.counts = new_counts_raw
            .into_iter()
            .map(|c| {
                let a = AtomicU64::new(0);
                a.store(c, Ordering::Relaxed);
                a
            })
            .collect();
        self.overflow.store(0, Ordering::Relaxed);
    }
    fn render(&self, name: &str, help: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# HELP {} {}\n# TYPE {} histogram\n",
            name, help, name
        ));
        let mut cumulative = 0u64;
        for (i, b) in self.bounds.iter().enumerate() {
            let c = self.counts[i].load(Ordering::Relaxed);
            cumulative += c;
            out.push_str(&format!("{}_bucket{{le=\"{}\"}} {}\n", name, b, cumulative));
        }
        let total = self.count.load(Ordering::Relaxed);
        // overflow counts live in +Inf bucket
        out.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", name, total));
        out.push_str(&format!(
            "{}_sum {}\n{}_count {}\n{}_max {}\n",
            name,
            self.sum.load(Ordering::Relaxed),
            name,
            total,
            name,
            self.max.load(Ordering::Relaxed)
        ));
        // Percentiles (approx by linear scan)
        if total > 0 {
            let targets = [0.5_f64, 0.9, 0.99];
            for t in targets.iter() {
                let need = (total as f64 * t) as u64;
                if need == 0 {
                    continue;
                }
                let mut run = 0u64;
                let mut val = *self.bounds.last().unwrap_or(&0);
                for (i, b) in self.bounds.iter().enumerate() {
                    run += self.counts[i].load(Ordering::Relaxed);
                    if run >= need {
                        val = *b;
                        break;
                    }
                }
                out.push_str(&format!("{}_quantile{{q=\"{:.2}\"}} {}\n", name, t, val));
            }
        }
        out
    }
}

pub struct StatsRegistry {
    profit_hist: Mutex<DynHistogram>,
    latency_hist: Mutex<DynHistogram>, // edge -> signal latency
    ingest_latency_hist: Mutex<DynHistogram>, // kafka receive -> processed
    staleness_hist: Mutex<DynHistogram>, // arrival delay vs embedded updated_at_ns
    profit_cfg: Mutex<DynHistogramConfig>,
    latency_cfg: Mutex<DynHistogramConfig>,
    last_resize_check: AtomicU64, // monotonic counter of records when last resize attempted
}
impl StatsRegistry {
    fn load_cfg() -> (DynHistogramConfig, DynHistogramConfig) {
        let profit_max = std::env::var("ARBS_PROFIT_MAX_BPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000);
        let profit_buckets = std::env::var("ARBS_PROFIT_BUCKETS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);
        let latency_max_ms = std::env::var("ARBS_LATENCY_MAX_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5_000_000); // big default
        let latency_buckets = std::env::var("ARBS_LATENCY_BUCKETS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);
        (
            DynHistogramConfig {
                target_buckets: profit_buckets,
                max_value_bps: profit_max,
            },
            DynHistogramConfig {
                target_buckets: latency_buckets,
                max_value_bps: latency_max_ms,
            },
        )
    }
    fn new() -> Self {
        let (pc, lc) = Self::load_cfg();
        // Staleness uses latency cfg by default (reuse shape)
        Self {
            profit_hist: Mutex::new(DynHistogram::new(&pc)),
            latency_hist: Mutex::new(DynHistogram::new(&lc)),
            ingest_latency_hist: Mutex::new(DynHistogram::new(&lc)),
            staleness_hist: Mutex::new(DynHistogram::new(&lc)),
            profit_cfg: Mutex::new(pc),
            latency_cfg: Mutex::new(lc),
            last_resize_check: AtomicU64::new(0),
        }
    }
    fn maybe_resize(&self) {
        // throttle resize checks every 10k samples
        let c = {
            let ph = self.profit_hist.lock().unwrap();
            let lh = self.latency_hist.lock().unwrap();
            ph.count.load(Ordering::Relaxed)
                + lh.count.load(Ordering::Relaxed)
                + self
                    .ingest_latency_hist
                    .lock()
                    .unwrap()
                    .count
                    .load(Ordering::Relaxed)
                + self
                    .staleness_hist
                    .lock()
                    .unwrap()
                    .count
                    .load(Ordering::Relaxed)
        };
        let last = self.last_resize_check.load(Ordering::Relaxed);
        if c < last + 10_000 {
            return;
        }
        if self
            .last_resize_check
            .compare_exchange(last, c, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            if let Ok(mut ph) = self.profit_hist.lock() {
                if let Ok(pc) = self.profit_cfg.lock() {
                    ph.maybe_resize(&pc);
                }
            }
            if let Ok(mut lh) = self.latency_hist.lock() {
                if let Ok(lc) = self.latency_cfg.lock() {
                    lh.maybe_resize(&lc);
                }
            }
            if let Ok(mut ih) = self.ingest_latency_hist.lock() {
                if let Ok(lc) = self.latency_cfg.lock() {
                    ih.maybe_resize(&lc);
                }
            }
            if let Ok(mut sh) = self.staleness_hist.lock() {
                if let Ok(lc) = self.latency_cfg.lock() {
                    sh.maybe_resize(&lc);
                }
            }
        }
    }
    pub fn record_profit_bps(&self, bps: i32) {
        if let Ok(ph) = self.profit_hist.lock() {
            ph.record(bps as i64);
        }
        self.maybe_resize();
    }
    pub fn record_latency_ns(&self, ns: u64) {
        if let Ok(lh) = self.latency_hist.lock() {
            lh.record((ns / 1_000_000) as i64);
        }
        self.maybe_resize();
    }
    pub fn record_ingest_latency_ns(&self, ns: u64) {
        if let Ok(ih) = self.ingest_latency_hist.lock() {
            ih.record((ns / 1_000_000) as i64);
        }
        self.maybe_resize();
    }
    pub fn record_edge_staleness_ns(&self, ns: u64) {
        if let Ok(sh) = self.staleness_hist.lock() {
            sh.record((ns / 1_000_000) as i64);
        }
        self.maybe_resize();
    }
    pub fn render(&self) -> String {
        let p = self.profit_hist.lock().unwrap();
        let l = self.latency_hist.lock().unwrap();
        let il = self.ingest_latency_hist.lock().unwrap();
        let sh = self.staleness_hist.lock().unwrap();
        #[allow(unused_mut)]
        let mut out = format!(
            "{}{}{}{}",
            p.render("arb_signal_profit_bps", "Arbitrage signal profit in bps"),
            l.render(
                "arb_signal_latency_ms",
                "Latency from edge update to signal emit in ms"
            ),
            il.render(
                "arb_edge_ingest_latency_ms",
                "Kafka receive to edge processed latency in ms"
            ),
            sh.render(
                "arb_edge_staleness_ms_hist",
                "Wall-clock arrival delay vs embedded updated_at_ns in ms"
            ),
        );
        #[cfg(feature = "diagnostics")]
        {
            use std::fmt::Write as _;
            let examined = CYCLES_EXAMINED.load(Ordering::Relaxed);
            let profitable = CYCLES_PROFITABLE.load(Ordering::Relaxed);
            let pruned = CYCLES_PRUNED.load(Ordering::Relaxed);
            let efficiency = if examined + pruned > 0 {
                // fraction of eliminated paths among considered/pruned set
                (pruned as f64) / ((pruned + examined) as f64)
            } else {
                0.0
            };
            let _ = write!(
                out,
                "# TYPE arb_cycles_examined_total counter\narb_cycles_examined_total {}\n# TYPE arb_cycles_profitable_total counter\narb_cycles_profitable_total {}\n# TYPE arb_cycles_pruned_total counter\narb_cycles_pruned_total {}\n# TYPE arb_pruning_efficiency gauge\narb_pruning_efficiency {}\n",
                examined, profitable, pruned, efficiency
            );
        }
        out
    }
    #[cfg(test)]
    pub fn test_snapshot_profit_bounds(&self) -> Vec<i64> {
        self.profit_hist.lock().unwrap().bounds.clone()
    }
    pub fn reload_config(&self) {
        let (pc, lc) = Self::load_cfg();
        if let Ok(mut c) = self.profit_cfg.lock() {
            *c = pc;
        }
        if let Ok(mut c) = self.latency_cfg.lock() {
            *c = lc;
        }
        if let Ok(mut ph) = self.profit_hist.lock() {
            ph.maybe_resize(&self.profit_cfg.lock().unwrap());
        }
        if let Ok(mut lh) = self.latency_hist.lock() {
            lh.maybe_resize(&self.latency_cfg.lock().unwrap());
        }
    }
}
pub static STATS: Lazy<StatsRegistry> = Lazy::new(StatsRegistry::new);
pub fn record_profit(bps: i32) {
    STATS.record_profit_bps(bps);
}
pub fn record_latency_ns(ns: u64) {
    STATS.record_latency_ns(ns);
}
pub fn record_ingest_latency_ns(ns: u64) {
    STATS.record_ingest_latency_ns(ns);
}
pub fn record_edge_staleness_ns(ns: u64) {
    STATS.record_edge_staleness_ns(ns);
}
pub fn render_all() -> String {
    let mut out = STATS.render();
    // Always-on ingestion gauges (not behind diagnostics)
    {
        use std::fmt::Write as _;
        let (
            ready,
            err_ratio,
            _stale_ms,
            total,
            errs,
            circuit,
            half_open,
            _max_stale,
            _err_thresh,
            _last,
        ) = compute_readiness();
        let _ = write!(out,
            "# TYPE arb_edge_circuit_open gauge\narb_edge_circuit_open {}\n# TYPE arb_edge_circuit_half_open gauge\narb_edge_circuit_half_open {}\n# TYPE arb_edge_error_ratio gauge\narb_edge_error_ratio {}\n# TYPE arb_edge_readiness gauge\narb_edge_readiness {}\n# TYPE arb_edge_messages_total counter\narb_edge_messages_total {}\n# TYPE arb_edge_errors_total counter\narb_edge_errors_total {}\n# TYPE arb_edge_circuit_half_open_total counter\narb_edge_circuit_half_open_total {}\n# TYPE arb_edge_circuit_half_open_errors counter\narb_edge_circuit_half_open_errors {}\n",
            if circuit {1} else {0}, if half_open {1} else {0}, err_ratio, if ready {1} else {0}, total, errs,
            CIRCUIT_HALF_OPEN_TOTAL.load(Ordering::Relaxed), CIRCUIT_HALF_OPEN_ERRORS.load(Ordering::Relaxed));
        // build info gauge (labels)
        let version = std::env::var("BUILD_VERSION")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
        let git_sha = std::env::var("GIT_SHA")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "unknown".into());
        let service = std::env::var("SERVICE_NAME")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "arb-engine".into());
        let _ = write!(out, "# TYPE arb_build_info gauge\narb_build_info{{version=\"{}\",git_sha=\"{}\",service=\"{}\"}} 1\n", version, git_sha, service);
    }
    #[cfg(feature = "diagnostics")]
    {
        use std::fmt::Write as _;
        // Snapshot structural gauges emitted by arb-engine (set externally).
        let max_factor = MAX_EDGE_FACTOR.load(Ordering::Relaxed) as f64 / 1_000_000.0; // stored scaled
        let edge_pairs = EDGE_PAIR_COUNT.load(Ordering::Relaxed);
        let token_count = TOKEN_COUNT.load(Ordering::Relaxed);
        let avg_adj = AVG_ADJACENCY.load(Ordering::Relaxed) as f64 / 1_000.0;
        // Edge staleness age (ms)
        let last_edge = LAST_EDGE_UPDATE_NS.load(Ordering::Relaxed);
        if last_edge > 0 {
            let now_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            if now_ns > last_edge {
                let age_ms = (now_ns - last_edge) / 1_000_000;
                let _ = write!(
                    out,
                    "# TYPE arb_edge_staleness_ms gauge\narb_edge_staleness_ms {}\n",
                    age_ms
                );
            }
        }
        // Memory (resident) bytes if available
        if let Some(rss) = process_resident_bytes() {
            let _ = write!(
                out,
                "# TYPE arb_process_resident_bytes gauge\narb_process_resident_bytes {}\n",
                rss
            );
        }
        let _ = write!(out,
            "# TYPE arb_max_edge_factor gauge\narb_max_edge_factor {}\n# TYPE arb_edge_pairs_total gauge\narb_edge_pairs_total {}\n# TYPE arb_tokens_indexed_total gauge\narb_tokens_indexed_total {}\n# TYPE arb_avg_adjacency gauge\narb_avg_adjacency {}\n",
            max_factor, edge_pairs, token_count, avg_adj);
    }
    out
}
pub fn reload_stats_config() {
    STATS.reload_config();
}

// --- Diagnostics counters (feature-gated) ---
#[cfg(feature = "diagnostics")]
pub static CYCLES_EXAMINED: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "diagnostics")]
pub static CYCLES_PROFITABLE: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "diagnostics")]
pub static CYCLES_PRUNED: AtomicU64 = AtomicU64::new(0);

// Structural / pruning tuning gauges (updated by edge detector loops)
#[cfg(feature = "diagnostics")]
pub static MAX_EDGE_FACTOR: AtomicU64 = AtomicU64::new(0); // scaled by 1e6
#[cfg(feature = "diagnostics")]
pub static EDGE_PAIR_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "diagnostics")]
pub static TOKEN_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "diagnostics")]
pub static AVG_ADJACENCY: AtomicU64 = AtomicU64::new(0); // scaled by 1e3
                                                         // Last edge update timestamp (ns). Always available for health endpoint.
pub static LAST_EDGE_UPDATE_NS: AtomicU64 = AtomicU64::new(0);
// Ingestion health counters (always on) + circuit breaker flag
pub static EDGE_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static EDGE_DECODE_ERRORS: AtomicU64 = AtomicU64::new(0);
pub static EDGE_PROCESS_ERRORS: AtomicU64 = AtomicU64::new(0);
pub static CIRCUIT_OPEN: AtomicBool = AtomicBool::new(false);
pub static CIRCUIT_HALF_OPEN: AtomicBool = AtomicBool::new(false);
pub static CIRCUIT_LAST_OPEN_NS: AtomicU64 = AtomicU64::new(0);
pub static CIRCUIT_HALF_OPEN_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static CIRCUIT_HALF_OPEN_ERRORS: AtomicU64 = AtomicU64::new(0);

// Compute readiness + supporting numbers for gauges & endpoints.
pub fn compute_readiness() -> (bool, f64, u64, u64, u64, bool, bool, u64, f64, u64) {
    use std::sync::atomic::Ordering;
    let last = LAST_EDGE_UPDATE_NS.load(Ordering::Relaxed);
    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let staleness_ms = if last > 0 && now_ns > last {
        (now_ns - last) / 1_000_000
    } else {
        u64::MAX
    };
    let max_stale: u64 = std::env::var("READINESS_MAX_STALENESS_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000);
    let total = EDGE_TOTAL.load(Ordering::Relaxed);
    let decode_err = EDGE_DECODE_ERRORS.load(Ordering::Relaxed);
    let proc_err = EDGE_PROCESS_ERRORS.load(Ordering::Relaxed);
    let errs = decode_err + proc_err;
    let err_ratio = if total > 0 {
        errs as f64 / total as f64
    } else {
        0.0
    };
    let err_thresh: f64 = std::env::var("READINESS_MAX_ERROR_RATIO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.50);
    let circuit = CIRCUIT_OPEN.load(Ordering::Relaxed);
    let half_open = CIRCUIT_HALF_OPEN.load(Ordering::Relaxed);
    let ready =
        last > 0 && staleness_ms <= max_stale && err_ratio <= err_thresh && !circuit && !half_open;
    (
        ready,
        err_ratio,
        staleness_ms,
        total,
        errs,
        circuit,
        half_open,
        max_stale,
        err_thresh,
        last,
    )
}

#[cfg(feature = "diagnostics")]
pub fn incr_cycles_examined() {
    CYCLES_EXAMINED.fetch_add(1, Ordering::Relaxed);
}
#[cfg(feature = "diagnostics")]
pub fn incr_cycles_profitable() {
    CYCLES_PROFITABLE.fetch_add(1, Ordering::Relaxed);
}
#[cfg(feature = "diagnostics")]
pub fn incr_cycles_pruned() {
    CYCLES_PRUNED.fetch_add(1, Ordering::Relaxed);
}

// --- Memory helpers (best-effort) ---
#[cfg(feature = "diagnostics")]
fn process_resident_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        // Parse /proc/self/statm (2nd field is resident pages)
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            let mut parts = statm.split_whitespace();
            let _size = parts.next();
            if let Some(resident) = parts.next() {
                if let Ok(rp) = resident.parse::<u64>() {
                    let page = 4096u64; // assume 4k
                    return Some(rp * page);
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    unsafe {
        use libc::{getrusage, rusage, RUSAGE_SELF};
        let mut ru: rusage = std::mem::zeroed();
        if getrusage(RUSAGE_SELF, &mut ru) == 0 {
            // ru_maxrss: bytes on macOS (per man page it's in bytes since 10.9)
            if ru.ru_maxrss > 0 {
                return Some(ru.ru_maxrss as u64);
            }
        }
    }
    None
}
#[cfg(not(feature = "diagnostics"))]
fn process_resident_bytes() -> Option<u64> {
    None
}
