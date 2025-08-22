use ethnum::U256;

// Q64.64 helpers
pub const Q64: u32 = 64;
pub const Q64_ONE: u128 = 1u128 << Q64;

#[inline]
pub fn mul_div_u256(n: U256, d: U256, denom: U256) -> U256 {
    // Assumes denom > 0
    (n * d) / denom
}

#[inline]
pub fn mul_div_u256_ceil(n: U256, d: U256, denom: U256) -> U256 {
    let num = n * d;
    let q = num / denom;
    if num % denom == U256::ZERO {
        q
    } else {
        q + U256::ONE
    }
}

#[inline]
pub fn u128_to_u256(x: u128) -> U256 {
    U256::from(x)
}

#[inline]
pub fn q64_to_f64(sp_x64: u128) -> f64 {
    (sp_x64 as f64) / (Q64_ONE as f64)
}

// Approx sqrt price at a tick (float; OK for estimates and logging; we will replace later)
pub fn tick_to_sqrt_price_x64_approx(tick: i32) -> u128 {
    let sp = 1.0001_f64.powf(tick as f64 / 2.0);
    let scaled = sp * (Q64_ONE as f64);
    if scaled <= 0.0 {
        0
    } else {
        scaled as u128
    }
}

// Exact integer formulas within a tick boundary (Q64.64)
// A->B (price decreases): dx_raw = L * (sp - sp_next) / (sp * sp_next)
pub fn dx_raw_within_tick_ab(sp: u128, sp_next: u128, liquidity: u128) -> u128 {
    if sp_next >= sp || sp_next == 0 || sp == 0 {
        return 0;
    }
    let l = u128_to_u256(liquidity);
    let num = l * U256::from(sp - sp_next);
    let den = U256::from(sp) * U256::from(sp_next);
    let q = num.checked_div(den).unwrap_or(U256::ZERO);
    q.as_u128()
}

// B->A (price increases): dy_raw = L * (sp_next - sp)
pub fn dy_raw_within_tick_ba(sp: u128, sp_next: u128, liquidity: u128) -> u128 {
    if sp_next <= sp {
        return 0;
    }
    let l = u128_to_u256(liquidity);
    let num = l * U256::from(sp_next - sp);
    num.as_u128() // fits in u256 -> u128 if values are reasonable; clamp if needed
}

// Apply input fee to get input required for dx_raw/dy_raw (ceil)
pub fn apply_input_fee(dx_raw: u128, total_fee_bps: u16) -> u128 {
    if total_fee_bps == 0 {
        return dx_raw;
    }
    let num = U256::from(dx_raw) * U256::from(10_000u64);
    let denom = U256::from(10_000u64 - total_fee_bps as u64);
    let q = mul_div_u256_ceil(num, U256::ONE, denom);
    q.as_u128()
}
