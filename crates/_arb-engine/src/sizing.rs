use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
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
#[derive(Serialize, Deserialize, Clone)]
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
        let mut s = Self {
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
        };
        if let Some(path) = ADAPT_STATE_PATH.as_ref() {
            if let Some(loaded) = Self::load_from(path) {
                s = loaded;
            }
        }
        s
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

// Persistence helpers
static ADAPT_STATE_PATH: Lazy<Option<String>> =
    Lazy::new(|| std::env::var("ARB_ADAPT_STATE_PATH").ok());
static ADAPT_PERSIST_INTERVAL: Lazy<usize> = Lazy::new(|| {
    std::env::var("ARB_ADAPT_PERSIST_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .max(1)
});

impl AdaptiveSizerInner {
    fn persist_if_needed(&self) {
        if let Some(path) = ADAPT_STATE_PATH.as_ref() {
            if self.count == 0 {
                return;
            }
            if self.count % *ADAPT_PERSIST_INTERVAL != 0 {
                return;
            }
            if let Ok(json) = serde_json::to_string(self) {
                let tmp = format!("{}.tmp", path);
                if let Ok(mut f) = fs::File::create(&tmp) {
                    if f.write_all(json.as_bytes()).is_ok() {
                        let _ = fs::rename(&tmp, path);
                    }
                }
            }
        }
    }
    #[cfg(test)]
    fn force_persist(&self) {
        if let Some(path) = ADAPT_STATE_PATH.as_ref() {
            if self.count == 0 {
                return;
            }
            if let Ok(json) = serde_json::to_string(self) {
                let tmp = format!("{}.tmp", path);
                if let Ok(mut f) = fs::File::create(&tmp) {
                    if f.write_all(json.as_bytes()).is_ok() {
                        let _ = fs::rename(&tmp, path);
                    }
                }
            }
        }
    }
    fn load_from(path: &str) -> Option<Self> {
        let p = Path::new(path);
        if !p.exists() {
            return None;
        }
        let mut f = fs::File::open(p).ok()?;
        let mut buf = String::new();
        f.read_to_string(&mut buf).ok()?;
        let mut loaded: AdaptiveSizerInner = match serde_json::from_str(&buf) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("adaptive_sizer_persist_corrupt: path={} err={}", path, e);
                return None; // corruption guard: ignore bad file
            }
        };
        if loaded.cap == 0 || loaded.ring.len() != loaded.cap {
            return None;
        }
        if loaded.idx >= loaded.cap {
            loaded.idx = 0;
        }
        if loaded.count > loaded.cap {
            loaded.count = loaded.cap;
        }
        Some(loaded)
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
            g.persist_if_needed();
            // Update public multiplier gauge immediately (previously only updated during choose())
            let mult = g.multiplier();
            LAST_ADAPTIVE_MULTIPLIER.store((mult * 1_000_000.0) as u64, Ordering::Relaxed);
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

#[derive(Serialize, Deserialize)]
pub struct AdaptiveSnapshot {
    pub mode: &'static str,
    pub history_len: usize,
    pub count: usize,
    pub median_profit_bps: i32,
    pub multiplier: f64,
    pub min_mult: f64,
    pub max_mult: f64,
    pub threshold_bps: i32,
    pub state_path: Option<String>,
}

pub fn adaptive_state_snapshot() -> AdaptiveSnapshot {
    let mode = ADAPTIVE_SIZER.mode;
    let (history_len, count, median, mult, min_mult, max_mult, threshold) =
        if let Ok(g) = ADAPTIVE_SIZER.inner.lock() {
            (
                g.cap,
                g.count,
                g.median(),
                g.multiplier(),
                g.min_mult,
                g.max_mult,
                g.threshold_bps,
            )
        } else {
            (0, 0, 0, 1.0, 1.0, 1.0, 0)
        };
    AdaptiveSnapshot {
        mode: if mode == SizingMode::Adaptive {
            "adaptive"
        } else {
            "boundary"
        },
        history_len,
        count,
        median_profit_bps: median,
        multiplier: mult,
        min_mult,
        max_mult,
        threshold_bps: threshold,
        state_path: ADAPT_STATE_PATH.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_persistence_reload_and_corruption_guard() {
        // Skip if no path env configured (avoid interfering with real runs)
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("adapt_state.json");
        std::env::set_var("ARB_ADAPT_STATE_PATH", path.display().to_string());
        std::env::set_var("ARB_SIZING_MODE", "adaptive");
        std::env::set_var("ARB_ADAPT_HISTORY", "32");
        // Force new instance (ADAPT_PERSIST_INTERVAL already initialized before test; use force_persist)
        let sizer = AdaptiveSizerInner::new();
        assert_eq!(sizer.count, 0);
        // Simulate writes
        let mut s2 = sizer.clone();
        for i in 0..40 {
            s2.record(50 + (i as i32 % 10));
        }
        s2.force_persist();
        // Ensure file exists
        assert!(path.exists());
        // Reload
        let loaded = AdaptiveSizerInner::load_from(path.to_str().unwrap()).unwrap();
        assert!(loaded.count > 0);
        // Corrupt file
        fs::write(&path, b"{not_json").unwrap();
        let corrupt = AdaptiveSizerInner::load_from(path.to_str().unwrap());
        assert!(corrupt.is_none());
    }
}
