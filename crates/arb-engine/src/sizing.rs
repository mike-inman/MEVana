use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Mode selector (env: ARB_SIZING_MODE = boundary | adaptive)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SizingMode {
    Boundary,
    Adaptive,
}

impl SizingMode {
    fn from_env() -> Self {
        match std::env::var("ARB_SIZING_MODE").ok().as_deref() {
            Some("adaptive") => SizingMode::Adaptive,
            _ => SizingMode::Boundary,
        }
    }
}

/// Simple adaptive sizer that scales chosen size by a multiplier derived from
/// the recent profit_bps distribution of emitted signals.
/// This is an initial scaffold; future versions can plug in depth/slippage curves.
struct AdaptiveSizerInner {
    ring: Vec<i32>,
    cap: usize,
    idx: usize,
    count: usize,
    min_mult: f64,
    max_mult: f64,
    threshold_bps: i32, // current min profit threshold snapshot (for interpolation)
}

impl AdaptiveSizerInner {
    fn new() -> Self {
        let cap = std::env::var("ARB_ADAPT_HISTORY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(256)
            .clamp(16, 4096);
        let mut min_mult = std::env::var("ARB_ADAPT_MIN_MULTIPLIER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.4);
        if min_mult < 0.01 {
            min_mult = 0.01;
        }
        if min_mult > 1.0 {
            min_mult = 1.0;
        }
        let mut max_mult = std::env::var("ARB_ADAPT_MAX_MULTIPLIER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);
        if max_mult < min_mult {
            max_mult = min_mult;
        }
        if max_mult > 1.0 {
            max_mult = 1.0;
        }
        Self {
            ring: vec![0; cap],
            cap,
            idx: 0,
            count: 0,
            min_mult,
            max_mult,
            threshold_bps: std::env::var("ARB_MIN_PROFIT_BPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50) as i32,
        }
    }
    fn record(&mut self, profit_bps: i32) {
        self.ring[self.idx] = profit_bps;
        self.idx = (self.idx + 1) % self.cap;
        if self.count < self.cap {
            self.count += 1;
        }
    }
    fn median(&self) -> i32 {
        if self.count == 0 {
            return 0;
        }
        let mut vals: Vec<i32> = if self.count == self.cap {
            self.ring.clone()
        } else {
            self.ring[..self.count].to_vec()
        };
        vals.sort_unstable();
        vals[vals.len() / 2]
    }
    fn multiplier(&self) -> f64 {
        if self.count < 8 {
            return self.min_mult; // warmup conservative
        }
        let med = self.median();
        let low = self.threshold_bps;
        let high = low * 4; // heuristic scaling window
        if med <= low {
            self.min_mult
        } else if med >= high {
            self.max_mult
        } else {
            let t = (med - low) as f64 / (high - low) as f64; // 0..1
            self.min_mult + t * (self.max_mult - self.min_mult)
        }
    }
}

struct AdaptiveSizer {
    inner: Mutex<AdaptiveSizerInner>,
    mode: SizingMode,
}

impl AdaptiveSizer {
    fn new() -> Self {
        Self {
            inner: Mutex::new(AdaptiveSizerInner::new()),
            mode: SizingMode::from_env(),
        }
    }
    fn record_profit_bps(&self, bps: i32) {
        if self.mode != SizingMode::Adaptive {
            return;
        }
        if let Ok(mut g) = self.inner.lock() {
            g.record(bps);
        }
    }
    fn choose(&self, base_size: u128) -> u128 {
        match self.mode {
            SizingMode::Boundary => base_size,
            SizingMode::Adaptive => {
                let mult = if let Ok(g) = self.inner.lock() {
                    g.multiplier()
                } else {
                    1.0
                };
                let scaled = (base_size as f64 * mult).floor() as u128;
                let min_abs = std::env::var("ARB_ADAPT_MIN_SIZE_LAMPORTS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                let final_size = scaled.clamp(min_abs, base_size.max(min_abs));
                LAST_ADAPTIVE_MULTIPLIER.store((mult * 1_000_000.0) as u64, Ordering::Relaxed);
                final_size
            }
        }
    }
}

static ADAPTIVE_SIZER: Lazy<AdaptiveSizer> = Lazy::new(AdaptiveSizer::new);
pub static LAST_ADAPTIVE_MULTIPLIER: AtomicU64 = AtomicU64::new(0); // scaled by 1e6

/// Public API: record a profit bps from an emitted signal (expected profit basis).
pub fn record_signal_profit_bps(bps: i32) {
    ADAPTIVE_SIZER.record_profit_bps(bps);
}

/// Public API: choose a final size given the boundary-derived base size.
pub fn choose_size(base_size: u128) -> u128 {
    ADAPTIVE_SIZER.choose(base_size)
}
