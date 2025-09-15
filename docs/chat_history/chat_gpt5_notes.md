Last updated: 2025-09-15

Great timing. Here’s exactly what you asked for: a drop‑in “nearest initialized tick” utility plus where to put it and how to call it. I’ll also include a conservative capacity-to-next-tick estimator so your edge assembler can emit a useful size hint right away.

Where to put it
- In your Orca Whirlpool “edge-assembler” service (the process that turns Geyser updates into normalized edges).
- Create a module file, e.g. src/whirlpool/tick_window.rs.
- Use it from the same place you already decode Whirlpool pool and TickArray accounts.

Code: tick window types, nearest-tick finders, and size-to-next estimates

// src/whirlpool/tick_window.rs

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

// Reuse your existing Tick and TickArray shapes (fixed [Tick; 88], whirlpool before ticks).
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct Tick {
    pub liquidity_net: i128,            // 16
    pub liquidity_gross: u128,          // 16
    pub fee_growth_outside_a: u128,     // 16
    pub fee_growth_outside_b: u128,     // 16
    pub reward_growths_outside: [u128; 3], // 48
}

#[derive(Clone, Debug)]
pub struct TickArrayView<'a> {
    pub start_tick_index: i32,
    pub ticks: &'a [Tick; 88],
}

#[derive(Clone, Debug, Default)]
pub struct TickWindow<'a> {
    pub lower: Option<TickArrayView<'a>>,
    pub current: Option<TickArrayView<'a>>,
    pub upper: Option<TickArrayView<'a>>,
}

// Helpers for integer floor/ceil division that work with negatives
#[inline]
fn floor_div_i32(x: i32, y: i32) -> i32 {
    let mut q = x / y;
    if (x ^ y) < 0 && x % y != 0 { q -= 1; }
    q
}
#[inline]
fn ceil_div_i32(x: i32, y: i32) -> i32 {
    let mut q = x / y;
    if (x ^ y) >= 0 && x % y != 0 { q += 1; }
    q
}

#[inline]
fn is_initialized(t: &Tick) -> bool { t.liquidity_gross > 0 }

#[inline]
fn actual_tick_index(start: i32, idx_in_array: usize, tick_spacing: i32) -> i32 {
    start + (idx_in_array as i32) * tick_spacing
}

// Scan downwards for the nearest initialized tick strictly <= tick_current (A->B direction boundary)
pub fn find_next_lower_initialized_tick(
    window: &TickWindow,
    tick_current: i32,
    tick_spacing: i32,
) -> Option<i32> {
    let cur = window.current.as_ref()?;
    // index in current array of the tick boundary at or below tick_current
    let off = floor_div_i32(tick_current - cur.start_tick_index, tick_spacing)
        .clamp(0, 87);
    // scan current array downward
    for i in (0..=off as usize).rev() {
        let t = &cur.ticks[i];
        if is_initialized(t) {
            return Some(actual_tick_index(cur.start_tick_index, i, tick_spacing));
        }
    }
    // scan lower array from the end
    if let Some(low) = window.lower.as_ref() {
        for i in (0..88).rev() {
            let t = &low.ticks[i];
            if is_initialized(t) {
                return Some(actual_tick_index(low.start_tick_index, i, tick_spacing));
            }
        }
    }
    None
}

// Scan upwards for the nearest initialized tick strictly > tick_current (B->A direction boundary)
pub fn find_next_upper_initialized_tick(
    window: &TickWindow,
    tick_current: i32,
    tick_spacing: i32,
) -> Option<i32> {
    let cur = window.current.as_ref()?;
    // first index strictly above tick_current
    let off = ceil_div_i32(tick_current + 1 - cur.start_tick_index, tick_spacing)
        .clamp(0, 88);
    // scan current array upward
    for i in off as usize..88 {
        let t = &cur.ticks[i];
        if is_initialized(t) {
            return Some(actual_tick_index(cur.start_tick_index, i, tick_spacing));
        }
    }
    // scan upper array from the start
    if let Some(up) = window.upper.as_ref() {
        for i in 0..88 {
            let t = &up.ticks[i];
            if is_initialized(t) {
                return Some(actual_tick_index(up.start_tick_index, i, tick_spacing));
            }
        }
    }
    None
}

// Convenience to get both sides in one call
pub fn find_nearest_initialized_ticks(
    window: &TickWindow,
    tick_current: i32,
    tick_spacing: i32,
) -> (Option<i32>, Option<i32>) {
    (
        find_next_lower_initialized_tick(window, tick_current, tick_spacing),
        find_next_upper_initialized_tick(window, tick_current, tick_spacing),
    )
}

// -------------------- Optional: conservative size-to-next estimates --------------------
// These use f64 for simplicity to get you moving. We'll replace with integer Q64.64 later.

#[inline]
fn q64_to_f64(sp_x64: u128) -> f64 {
    (sp_x64 as f64) / 18446744073709551616.0 // 2^64
}

// Uniswap v3 math (float approx) to the next boundary.
// A->B (price decreases): dx_raw = L * (sp - sp_next) / (sp * sp_next)
pub fn estimate_dx_to_next_tick_ab(
    sqrt_price_x64: u128,
    liquidity: u128,
    next_lower_tick: i32,
    total_fee_bps: u32,
) -> Option<u128> {
    let sp = q64_to_f64(sqrt_price_x64);
    let sp_next = tick_to_sqrt_price_f64(next_lower_tick)?;
    if sp_next >= sp { return None; }
    let l = liquidity as f64;
    let dx_raw = l * (sp - sp_next) / (sp * sp_next);
    if dx_raw <= 0.0 { return None; }
    let fee_mult = 10000.0_f64 / (10000.0_f64 - total_fee_bps as f64);
    Some((dx_raw * fee_mult) as u128)
}

// B->A (price increases): tokenB in within tick: dx_raw = L * (sp_next - sp)
pub fn estimate_dx_to_next_tick_ba(
    sqrt_price_x64: u128,
    liquidity: u128,
    next_upper_tick: i32,
    total_fee_bps: u32,
) -> Option<u128> {
    let sp = q64_to_f64(sqrt_price_x64);
    let sp_next = tick_to_sqrt_price_f64(next_upper_tick)?;
    if sp_next <= sp { return None; }
    let l = liquidity as f64;
    let dx_raw = l * (sp_next - sp);
    if dx_raw <= 0.0 { return None; }
    let fee_mult = 10000.0_f64 / (10000.0_f64 - total_fee_bps as f64);
    Some((dx_raw * fee_mult) as u128)
}

// sqrt price at a tick boundary (float, for now).
pub fn tick_to_sqrt_price_f64(tick: i32) -> Option<f64> {
    // sqrtPrice = 1.0001^(tick/2)
    let sp = 1.0001_f64.powf(tick as f64 / 2.0);
    if sp.is_finite() && sp > 0.0 {
        Some(sp)
    } else {
        None
    }
}

How to use it in your assembler

- Keep your per-pool state with the three arrays (lower/current/upper) cached.
- Build a TickWindow<'_> when you update or seed arrays.
- Call find_nearest_initialized_ticks to get the boundaries.
- Compute size-to-next estimates and include them in your EdgeDelta for prefiltering.

Example wiring (in your existing on_whirlpool_update handler or right after seeding arrays):

use crate::whirlpool::tick_window::{
    TickArrayView, TickWindow, find_nearest_initialized_ticks,
    estimate_dx_to_next_tick_ab, estimate_dx_to_next_tick_ba
};

// 1) Build a TickWindow from your cached arrays for this pool
let tick_window = TickWindow {
    lower: cached_lower.as_ref().map(|ta| TickArrayView { start_tick_index: ta.start_tick_index, ticks: &ta.ticks }),
    current: cached_current.as_ref().map(|ta| TickArrayView { start_tick_index: ta.start_tick_index, ticks: &ta.ticks }),
    upper: cached_upper.as_ref().map(|ta| TickArrayView { start_tick_index: ta.start_tick_index, ticks: &ta.ticks }),
};

// 2) Find nearest initialized ticks
let (next_lower_opt, next_upper_opt) =
    find_nearest_initialized_ticks(&tick_window, pool.tick_current, pool.tick_spacing);

// 3) Compute conservative input sizes to those ticks (optional but useful)
let total_fee_bps = pool.fee_bps as u32 + pool.protocol_fee_bps as u32;
let size_ab = next_lower_opt.and_then(|t|
    estimate_dx_to_next_tick_ab(pool.sqrt_price_x64, pool.liquidity, t, total_fee_bps)
);
let size_ba = next_upper_opt.and_then(|t|
    estimate_dx_to_next_tick_ba(pool.sqrt_price_x64, pool.liquidity, t, total_fee_bps)
);

// 4) Log or publish in your EdgeDelta (integers only)
println!(
    "[NEAREST_TICKS] cur={} next_lower={:?} next_upper={:?} size_to_next_in A->B={:?} B->A={:?}",
    pool.tick_current, next_lower_opt, next_upper_opt, size_ab, size_ba
);

// EdgeDelta payload example fields (CLMM variant):
// - sqrt_price_x64: u128
// - liquidity: u128
// - tick_current: i32
// - tick_spacing: i32
// - fee_bps_total: u16
// - size_to_next_in_ab: u128 (or 0 if None)
// - size_to_next_in_ba: u128 (or 0 if None)

Notes and edge cases
- If a neighbor array (lower/upper) PDA is not initialized, you can’t cross into it. In that case, find_nearest_* will only search within the current array and may return None; treat capacity beyond current array as zero and keep your quotes capped.
- Tick indices must be computed as start + i * tick_spacing. Don’t assume spacing=1.
- The size_to_next_* functions here are float-based rough estimates to get you moving. In the next step we’ll replace them with exact integer Q64.64 formulas and support crossing multiple ticks.
- Call these functions after you seed arrays via RPC the first time; then apply Geyser deltas as they arrive and recompute only when the relevant arrays change or when tick_current moves enough to change which arrays are “current.”

If you prefer, I can package this into a small “whirlpool-core” crate with feature flags and unit tests, and then the next step we’ll drop in the integer quoter that uses the same TickWindow to quote across multiple ticks.