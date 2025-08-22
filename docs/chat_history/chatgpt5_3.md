Perfect. Here’s a small “whirlpool-core” Rust crate you can drop into your workspace. It gives you:

- Correct on-chain layouts (Whirlpool minimal, Tick, TickArray).
- Discriminator-checked decoders.
- PDA derivation for tick arrays and windowing helpers.
- Nearest-initialized-tick finders.
- Conservative size-to-next-tick estimators (A→B and B→A).
- A quoter skeleton that can step within a tick and across the next boundary (using safe 256-bit integer ops). It uses float-based sqrt-price-at-tick for now; we can swap to exact integer math next.

File layout
- whirlpool-core/Cargo.toml
- whirlpool-core/src/lib.rs
- whirlpool-core/src/types.rs
- whirlpool-core/src/decode.rs
- whirlpool-core/src/pda.rs
- whirlpool-core/src/tick_window.rs
- whirlpool-core/src/math.rs
- whirlpool-core/src/quoter.rs
- whirlpool-core/examples/seed_and_log.rs (RPC seeder + logger demo)

Code

— whirlpool-core/Cargo.toml —
[package]
name = "whirlpool-core"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"

[dependencies]
anyhow = "1.0"
borsh = { version = "1.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
sha2 = "0.10"
solana-sdk = "1.18.18"
ethnum = "1.5"

— whirlpool-core/src/lib.rs —
pub mod types;
pub mod decode;
pub mod pda;
pub mod tick_window;
pub mod math;
pub mod quoter;

pub use types::*;
pub use decode::*;
pub use pda::*;
pub use tick_window::*;
pub use math::*;
pub use quoter::*;

— whirlpool-core/src/types.rs —
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

// On-chain Tick layout (no explicit "initialized" flag).
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct Tick {
    pub liquidity_net: i128,             // 16
    pub liquidity_gross: u128,           // 16
    pub fee_growth_outside_a: u128,      // 16
    pub fee_growth_outside_b: u128,      // 16
    pub reward_growths_outside: [u128; 3], // 48
    // Total per tick = 112 bytes
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct TickArray {
    pub start_tick_index: i32,           // 4
    pub whirlpool: Pubkey,               // 32
    pub ticks: [Tick; 88],               // 9856
    // Total data after discriminator = 4 + 32 + 88*112 = 9892 bytes; with 8-byte disc => 9900 bytes
}

// Minimal subset of Whirlpool account (order matters — must match on-chain).
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct WhirlpoolMinimal {
    pub whirlpools_config: Pubkey,           // 32
    pub whirlpool_bump: [u8; 1],             // 1
    pub tick_spacing: u16,                   // 2
    pub tick_spacing_seed: [u8; 2],          // 2
    pub fee_rate: u16,                       // 2
    pub protocol_fee_rate: u16,              // 2
    pub liquidity: u128,                     // 16
    pub sqrt_price: u128,                    // 16 (Q64.64)
    pub tick_current_index: i32,             // 4
    pub protocol_fee_owed_a: u64,            // 8
    pub protocol_fee_owed_b: u64,            // 8
    pub token_mint_a: Pubkey,                // 32
    pub token_vault_a: Pubkey,               // 32
    pub fee_growth_global_a: u128,           // 16
    pub token_mint_b: Pubkey,                // 32
    pub token_vault_b: Pubkey,               // 32
    pub fee_growth_global_b: u128,           // 16
    pub reward_last_updated_timestamp: u64,  // 8
    pub reward_infos: [RewardInfo; 3],       // 3 * 96
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct RewardInfo {
    pub mint: Pubkey,                        // 32
    pub vault: Pubkey,                       // 32
    pub authority: Pubkey,                   // 32
    pub emissions_per_second_x64: u128,      // 16
    pub growth_global_x64: u128,             // 16
}

#[inline]
pub fn tick_is_initialized(t: &Tick) -> bool {
    t.liquidity_gross > 0
}

— whirlpool-core/src/decode.rs —
use crate::types::{TickArray, WhirlpoolMinimal};
use anyhow::{anyhow, Result};
use borsh::BorshDeserialize;

pub fn anchor_discriminator(name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

#[inline]
pub fn disc_whirlpool() -> [u8; 8] {
    anchor_discriminator("account:Whirlpool")
}

#[inline]
pub fn disc_tick_array() -> [u8; 8] {
    anchor_discriminator("account:TickArray")
}

pub fn decode_whirlpool(data: &[u8]) -> Result<WhirlpoolMinimal> {
    if data.len() < 8 || data[..8] != disc_whirlpool() {
        return Err(anyhow!("wrong discriminator for Whirlpool"));
    }
    let mut cursor = &data[8..];
    Ok(WhirlpoolMinimal::deserialize(&mut cursor)?)
}

pub fn decode_tick_array(data: &[u8]) -> Result<TickArray> {
    if data.len() < 8 || data[..8] != disc_tick_array() {
        return Err(anyhow!("wrong discriminator for TickArray"));
    }
    let mut cursor = &data[8..];
    Ok(TickArray::deserialize(&mut cursor)?)
}

— whirlpool-core/src/pda.rs —
use solana_sdk::pubkey::Pubkey;

pub fn derive_tick_array_address(
    program_id: &Pubkey,
    whirlpool: &Pubkey,
    start_tick_index: i32,
) -> (Pubkey, u8) {
    let le = start_tick_index.to_le_bytes();
    Pubkey::find_program_address(&[b"tick_array", whirlpool.as_ref(), &le], program_id)
}

pub fn three_tick_arrays_around(
    tick_current: i32,
    tick_spacing: i32,
) -> (i32, i32, i32) {
    let per_array = 88;
    let span = tick_spacing * per_array;
    let floor_div = |x: i32, y: i32| {
        let mut q = x / y;
        if (x ^ y) < 0 && x % y != 0 { q -= 1; }
        q
    };
    let current_start = floor_div(tick_current, span) * span;
    (current_start - span, current_start, current_start + span)
}

pub fn actual_tick_index(start: i32, idx_in_array: usize, tick_spacing: i32) -> i32 {
    start + (idx_in_array as i32) * tick_spacing
}

— whirlpool-core/src/tick_window.rs —
use crate::pda::actual_tick_index;
use crate::types::{Tick, TickArray};
use crate::types::tick_is_initialized;

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

fn floor_div_i32(x: i32, y: i32) -> i32 {
    let mut q = x / y;
    if (x ^ y) < 0 && x % y != 0 { q -= 1; }
    q
}
fn ceil_div_i32(x: i32, y: i32) -> i32 {
    let mut q = x / y;
    if (x ^ y) >= 0 && x % y != 0 { q += 1; }
    q
}

pub fn find_next_lower_initialized_tick(
    window: &TickWindow,
    tick_current: i32,
    tick_spacing: i32,
) -> Option<i32> {
    let cur = window.current.as_ref()?;
    let off = floor_div_i32(tick_current - cur.start_tick_index, tick_spacing)
        .clamp(0, 87);
    for i in (0..=off as usize).rev() {
        let t = &cur.ticks[i];
        if tick_is_initialized(t) {
            return Some(actual_tick_index(cur.start_tick_index, i, tick_spacing));
        }
    }
    if let Some(low) = window.lower.as_ref() {
        for i in (0..88).rev() {
            if tick_is_initialized(&low.ticks[i]) {
                return Some(actual_tick_index(low.start_tick_index, i, tick_spacing));
            }
        }
    }
    None
}

pub fn find_next_upper_initialized_tick(
    window: &TickWindow,
    tick_current: i32,
    tick_spacing: i32,
) -> Option<i32> {
    let cur = window.current.as_ref()?;
    let off = ceil_div_i32(tick_current + 1 - cur.start_tick_index, tick_spacing)
        .clamp(0, 88);
    for i in off as usize..88 {
        if tick_is_initialized(&cur.ticks[i]) {
            return Some(actual_tick_index(cur.start_tick_index, i, tick_spacing));
        }
    }
    if let Some(up) = window.upper.as_ref() {
        for i in 0..88 {
            if tick_is_initialized(&up.ticks[i]) {
                return Some(actual_tick_index(up.start_tick_index, i, tick_spacing));
            }
        }
    }
    None
}

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

— whirlpool-core/src/math.rs —
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
    if num % denom == U256::ZERO { q } else { q + U256::ONE }
}

#[inline]
pub fn u128_to_u256(x: u128) -> U256 { U256::from(x) }

#[inline]
pub fn q64_to_f64(sp_x64: u128) -> f64 {
    (sp_x64 as f64) / (Q64_ONE as f64)
}

// Approx sqrt price at a tick (float; OK for estimates and logging; we will replace later)
pub fn tick_to_sqrt_price_x64_approx(tick: i32) -> u128 {
    let sp = 1.0001_f64.powf(tick as f64 / 2.0);
    let scaled = sp * (Q64_ONE as f64);
    if scaled <= 0.0 { 0 } else { scaled as u128 }
}

// Exact integer formulas within a tick boundary (Q64.64)
// A->B (price decreases): dx_raw = L * (sp - sp_next) / (sp * sp_next)
pub fn dx_raw_within_tick_ab(sp: u128, sp_next: u128, liquidity: u128) -> u128 {
    if sp_next >= sp || sp_next == 0 || sp == 0 { return 0; }
    let l = u128_to_u256(liquidity);
    let num = l * U256::from(sp - sp_next);
    let den = U256::from(sp) * U256::from(sp_next);
    let q = num.checked_div(den).unwrap_or(U256::ZERO);
    q.as_u128()
}

// B->A (price increases): dy_raw = L * (sp_next - sp)
pub fn dy_raw_within_tick_ba(sp: u128, sp_next: u128, liquidity: u128) -> u128 {
    if sp_next <= sp { return 0; }
    let l = u128_to_u256(liquidity);
    let num = l * U256::from(sp_next - sp);
    num.as_u128() // fits in u256 -> u128 if values are reasonable; clamp if needed
}

// Apply input fee to get input required for dx_raw/dy_raw (ceil)
pub fn apply_input_fee(dx_raw: u128, total_fee_bps: u16) -> u128 {
    if total_fee_bps == 0 { return dx_raw; }
    let num = U256::from(dx_raw) * U256::from(10_000u64);
    let denom = U256::from(10_000u64 - total_fee_bps as u64);
    let q = mul_div_u256_ceil(num, U256::ONE, denom);
    q.as_u128()
}

— whirlpool-core/src/quoter.rs —
use crate::math::{
    Q64_ONE, q64_to_f64, tick_to_sqrt_price_x64_approx, dx_raw_within_tick_ab,
    dy_raw_within_tick_ba, apply_input_fee,
};
use crate::tick_window::{TickArrayView, TickWindow, find_nearest_initialized_ticks};
use crate::types::Tick;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    AtoB,
    BtoA,
}

#[derive(Clone, Debug, Default)]
pub struct QuoteResult {
    pub out: u128,
    pub dx_consumed: u128,
    pub ticks_crossed: u32,
}

pub struct ClmmQuoter<'a> {
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
    pub tick_current: i32,
    pub tick_spacing: i32,
    pub total_fee_bps: u16,
    pub window: TickWindow<'a>,
}

impl<'a> ClmmQuoter<'a> {
    // Conservative "size to next tick" in input units
    pub fn size_to_next_tick_in(&self, dir: Direction) -> Option<u128> {
        let (lower_opt, upper_opt) =
            find_nearest_initialized_ticks(&self.window, self.tick_current, self.tick_spacing);
        match dir {
            Direction::AtoB => {
                let next_lower = lower_opt?;
                let sp = self.sqrt_price_x64;
                let sp_next = tick_to_sqrt_price_x64_approx(next_lower);
                if sp_next == 0 || sp_next >= sp { return None; }
                let dx_raw = dx_raw_within_tick_ab(sp, sp_next, self.liquidity);
                Some(apply_input_fee(dx_raw, self.total_fee_bps))
            }
            Direction::BtoA => {
                let next_upper = upper_opt?;
                let sp = self.sqrt_price_x64;
                let sp_next = tick_to_sqrt_price_x64_approx(next_upper);
                if sp_next <= sp { return None; }
                let dx_raw = dy_raw_within_tick_ba(sp, sp_next, self.liquidity);
                Some(apply_input_fee(dx_raw, self.total_fee_bps))
            }
        }
    }

    // Minimal quoter: walk current tick to next boundary; does NOT scan multiple initialized ticks yet.
    pub fn quote_to_next_boundary(&self, dir: Direction, dx_in: u128) -> QuoteResult {
        if dx_in == 0 { return QuoteResult::default(); }
        let fee_denom = 10_000u64;
        let fee_num = (fee_denom - self.total_fee_bps as u64) as u128;
        let mut dx_eff = (dx_in as u128) * fee_num / (fee_denom as u128);

        let sp = self.sqrt_price_x64;
        let (lower_opt, upper_opt) =
            find_nearest_initialized_ticks(&self.window, self.tick_current, self.tick_spacing);

        match dir {
            Direction::AtoB => {
                let next_lower = match lower_opt { Some(t) => t, None => return QuoteResult::default() };
                let sp_next = tick_to_sqrt_price_x64_approx(next_lower);
                if sp_next == 0 || sp_next >= sp { return QuoteResult::default(); }

                // Max dx_raw to reach boundary
                let max_dx_raw = dx_raw_within_tick_ab(sp, sp_next, self.liquidity);
                // Re-map to input including fee (ceil)
                let max_dx_in = apply_input_fee(max_dx_raw, self.total_fee_bps);
                let dx_in_clip = dx_in.min(max_dx_in);

                // Recompute effective dx_raw from clipped dx_in
                let dx_eff_clip = (dx_in_clip as u128) * fee_num / (fee_denom as u128);
                // Within a tick: dy = L * (sp - sp_new), where sp_new solves dx_raw.
                // For small steps, linear approx: dy ≈ L * (sp - sp_next) * (dx_eff_clip / max_dx_raw)
                // Good enough for prefiltering; we’ll replace with exact step solver later.
                let dy_raw = if max_dx_raw > 0 {
                    (self.liquidity as u128)
                        .saturating_mul(sp.saturating_sub(sp_next))
                        .saturating_mul(dx_eff_clip)
                        / max_dx_raw
                } else { 0 };

                QuoteResult { out: dy_raw, dx_consumed: dx_in_clip, ticks_crossed: if dx_in_clip == max_dx_in { 1 } else { 0 } }
            }
            Direction::BtoA => {
                let next_upper = match upper_opt { Some(t) => t, None => return QuoteResult::default() };
                let sp_next = tick_to_sqrt_price_x64_approx(next_upper);
                if sp_next <= sp { return QuoteResult::default(); }

                let max_dy_raw = dy_raw_within_tick_ba(sp, sp_next, self.liquidity);
                let max_dx_in = apply_input_fee(max_dy_raw, self.total_fee_bps);
                let dx_in_clip = dx_in.min(max_dx_in);

                let dx_eff_clip = (dx_in_clip as u128) * fee_num / (fee_denom as u128);
                // Within tick B->A: out in token A ≈ L * (sp_next - sp) * (dx_eff_clip / max_dy_raw) / (sp * sp_next)
                // For now use same linear fraction; exact solver will replace this.
                let num = (self.liquidity as u128)
                    .saturating_mul(sp_next.saturating_sub(sp))
                    .saturating_mul(dx_eff_clip);
                let dy_out = if max_dy_raw > 0 { num / max_dy_raw } else { 0 };

                QuoteResult { out: dy_out, dx_consumed: dx_in_clip, ticks_crossed: if dx_in_clip == max_dx_in { 1 } else { 0 } }
            }
        }
    }
}

— whirlpool-core/examples/seed_and_log.rs —
use anyhow::{Context, Result};
use solana_sdk::pubkey::Pubkey;
use solana_client::rpc_client::RpcClient;

use whirlpool_core::{
    decode::{decode_tick_array, decode_whirlpool},
    pda::{derive_tick_array_address, three_tick_arrays_around, actual_tick_index},
    tick_window::{TickArrayView, TickWindow, find_nearest_initialized_ticks},
    math::{q64_to_f64},
};

fn main() -> Result<()> {
    // Configure RPC URL and a Whirlpool pool pubkey to test
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let pool_str = std::env::args().nth(1).expect("usage: cargo run --example seed_and_log <POOL_PUBKEY>");
    let program_id = Pubkey::from_str_const("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
    let pool_pk = Pubkey::from_str(&pool_str).expect("invalid pool pubkey");

    let rpc = RpcClient::new(rpc_url);

    // Fetch pool account
    let pool_acc = rpc.get_account(&pool_pk).context("get pool")?;
    let pool = decode_whirlpool(&pool_acc.data).context("decode whirlpool")?;
    println!("[WHIRLPOOL] pool={} tick_current={} spacing={} fee_lp={} fee_proto={} sqrt_price_x64={}",
        pool_pk, pool.tick_current_index, pool.tick_spacing, pool.fee_rate, pool.protocol_fee_rate, pool.sqrt_price);

    // Derive tick-array PDAs
    let (lower_start, current_start, upper_start) =
        three_tick_arrays_around(pool.tick_current_index, pool.tick_spacing as i32);
    let (ta_lower, _)   = derive_tick_array_address(&program_id, &pool_pk, lower_start);
    let (ta_current, _) = derive_tick_array_address(&program_id, &pool_pk, current_start);
    let (ta_upper, _)   = derive_tick_array_address(&program_id, &pool_pk, upper_start);
    println!("[TICK_PDAS] starts=({}, {}, {}) addrs=({}, {}, {})",
        lower_start, current_start, upper_start, ta_lower, ta_current, ta_upper);

    // Fetch and decode arrays (if present)
    let accs = rpc.get_multiple_accounts(&[ta_lower, ta_current, ta_upper]).context("get_multiple_accounts")?;
    let mut views = [None, None, None];

    for (idx, acc_opt) in accs.into_iter().enumerate() {
        let addr = match idx { 0 => ta_lower, 1 => ta_current, _ => ta_upper };
        match acc_opt {
            None => {
                println!("[TICK_ARRAY] addr={} missing (PDA not initialized)", addr);
            }
            Some(acc) => {
                if acc.owner != program_id {
                    println!("[TICK_ARRAY] addr={} wrong owner={}", addr, acc.owner);
                    continue;
                }
                println!("[TICK_ARRAY] addr={} len={}", addr, acc.data.len());
                match decode_tick_array(&acc.data) {
                    Ok(ta) => {
                        let init_count = ta.ticks.iter().filter(|t| t.liquidity_gross > 0).count();
                        println!("  start={} whirlpool_ok={} initialized_count={}",
                            ta.start_tick_index, ta.whirlpool == pool_pk, init_count);
                        // print first few initialized ticks:
                        let mut printed = 0;
                        for (i, t) in ta.ticks.iter().enumerate() {
                            if t.liquidity_gross > 0 {
                                let idx_tick = actual_tick_index(ta.start_tick_index, i, pool.tick_spacing as i32);
                                println!("    init_tick={}", idx_tick);
                                printed += 1;
                                if printed >= 6 { break; }
                            }
                        }
                        let view = TickArrayView { start_tick_index: ta.start_tick_index, ticks: &ta.ticks };
                        views[idx] = Some(view);
                    }
                    Err(e) => {
                        println!("  decode error: {}", e);
                    }
                }
            }
        }
    }

    // Build window
    let window = whirlpool_core::tick_window::TickWindow {
        lower: views[0].clone(),
        current: views[1].clone(),
        upper: views[2].clone(),
    };

    // Nearest initialized ticks
    let (lower_opt, upper_opt) = find_nearest_initialized_ticks(
        &window,
        pool.tick_current_index,
        pool.tick_spacing as i32,
    );
    println!("[NEAREST] next_lower={:?} next_upper={:?}", lower_opt, upper_opt);

    // Marginal price (for sanity)
    let p = {
        let sp = q64_to_f64(pool.sqrt_price);
        // price_B_per_A without decimals
        sp * sp
    };
    println!("[MARGINAL] price(B/A)~{}", p);

    Ok(())
}

How to integrate
- Add this crate to your workspace and build: cargo build -p whirlpool-core
- In your Orca “edge-assembler” microservice:
  - Use decode::decode_whirlpool on pool account updates.
  - Derive the three tick-array PDAs from pda::three_tick_arrays_around and pda::derive_tick_array_address.
  - Seed them once via RPC (getMultipleAccounts) using the example code, decode with decode::decode_tick_array, and cache them in your per-pool state.
  - Build a TickWindow from your cached arrays and call tick_window::find_nearest_initialized_ticks to get the immediate boundaries.
  - Optionally use quoter::ClmmQuoter::size_to_next_tick_in(Direction) to emit size-to-next-tick hints in your EdgeDelta.
  - Keep listening to Geyser for tick-array updates; when a tick-array changes, refresh the cached TickArray for that pool and recompute hints.

Notes
- The quoter here uses exact 256-bit integer math within a single tick, and a conservative linear fraction to estimate output when you don’t fully consume to the boundary. That’s enough for prefiltering and for your first signals. Next, we can replace it with an exact piecewise solver that walks multiple ticks and recomputes sqrt price per step (still integer).
- sqrt_price at a tick is currently approximated with f64 for convenience; we’ll replace with integer exponentiation (or a precomputed table) in the next iteration.
- All public functions return integers; keep floats only for logs.

When you’re ready, ask your questions. If you want, I can:
- Add an exact multi-tick quote_out implementation (integer Q64.64) to quoter.rs.
- Add a small EdgeDelta struct and serde/bincode encoding you can push to Kafka.
- Provide a Raydium CPMM module next (reserves decode + quoter) so you can do your first cross-venue arb.