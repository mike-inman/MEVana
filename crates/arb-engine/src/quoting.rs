use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU32, Ordering};

// Configurable linear price impact (bps) at full boundary utilization per leg.
static DEPTH_IMPACT_BPS: Lazy<AtomicU32> = Lazy::new(|| {
    AtomicU32::new(
        std::env::var("ARB_DEPTH_PRICE_IMPACT_BPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
    )
});

pub fn update_depth_impact_bps(v: u32) { DEPTH_IMPACT_BPS.store(v, Ordering::Relaxed); }

/// Compute multiplicative adjustment factor for a cycle with N legs applying linear impact.
pub fn depth_adjustment_factor(chosen_size: u128, base_size: u128, legs: u32) -> f64 {
    if base_size == 0 || chosen_size == 0 { return 1.0; }
    let impact_bps = DEPTH_IMPACT_BPS.load(Ordering::Relaxed) as f64;
    if impact_bps <= 0.0 { return 1.0; }
    let util = (chosen_size as f64 / base_size as f64).clamp(0.0, 1.0);
    let per_leg = 1.0 - impact_bps * util / 10_000.0;
    if per_leg <= 0.0 { return 0.0; }
    per_leg.powi(legs as i32)
}
