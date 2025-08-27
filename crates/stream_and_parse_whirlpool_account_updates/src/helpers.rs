use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;

use crate::{decode_tick_array, on_tick_array_update};

// PDA derivation for Orca TickArray
pub fn derive_tick_array_address(
    program_id: &Pubkey,
    whirlpool: &Pubkey,
    start_tick_index: i32,
) -> (Pubkey, u8) {
    let le = start_tick_index.to_le_bytes();
    Pubkey::find_program_address(&[b"tick_array", whirlpool.as_ref(), &le], program_id)
}

// Compute the three TickArray starts around the current tick
pub fn three_tick_arrays_around(tick_current: i32, tick_spacing: i32) -> (i32, i32, i32) {
    let per_array = 88;
    let span = tick_spacing * per_array;
    // floor division toward -inf to handle negative ticks correctly
    let floor_div = |x: i32, y: i32| -> i32 {
        let mut q = x / y;
        if (x ^ y) < 0 && x % y != 0 {
            q -= 1;
        }
        q
    };
    let current_start = floor_div(tick_current, span) * span;
    let lower_start = current_start - span;
    let upper_start = current_start + span;
    (lower_start, current_start, upper_start)
}

// Convert sqrt_price_x64 (Q64.64) to f64 sqrt price
pub fn q64_to_f64(sp_x64: u128) -> f64 {
    let scale = 18446744073709551616.0_f64; // 2^64
    (sp_x64 as f64) / scale
}

// Marginal price tokenB per tokenA from sqrt_price_x64 and decimals
pub fn marginal_price_b_per_a(sqrt_price_x64: u128, decimals_a: i32, decimals_b: i32) -> f64 {
    let sp = q64_to_f64(sqrt_price_x64);
    let price_no_decimals = sp * sp; // price_B_per_A without decimals
    let scale = 10f64.powi(decimals_b - decimals_a);
    price_no_decimals * scale
}

// Approx sqrt_price_x64 at a tick boundary (using floats for step 1/2)
pub fn tick_to_sqrt_price_x64(tick: i32) -> u128 {
    // sqrtPrice = 1.0001^(tick/2)
    let sp = 1.0001_f64.powf(tick as f64 / 2.0);
    let scaled = sp * 18446744073709551616.0_f64; // 2^64
    if scaled <= 0.0 {
        0
    } else {
        scaled as u128
    }
}

// Rough “size to next tick” estimate in token_in units including fees (for A->B direction)
pub fn size_to_next_tick_in_estimate_ab(
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    next_tick_index: i32,
    total_fee_bps: u32,
) -> u128 {
    let sp = q64_to_f64(sqrt_price_x64);
    let sp_next = q64_to_f64(tick_to_sqrt_price_x64(next_tick_index));
    if sp_next <= 0.0 || sp <= sp_next {
        return 0;
    }
    // Uniswap v3 math (float approx):
    // dx_raw = L * (sp - sp_next) / (sp * sp_next)
    let l = liquidity as f64;
    let dx_raw = l * (sp - sp_next) / (sp * sp_next);
    if dx_raw <= 0.0 {
        return 0;
    }
    let fee_mult = 10000.0_f64 / (10000.0_f64 - total_fee_bps as f64);
    let dx_in = dx_raw * fee_mult;
    dx_in.max(0.0) as u128
}

// Same but for B->A (price increases)
pub fn size_to_next_tick_in_estimate_ba(
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    next_tick_index: i32,
    total_fee_bps: u32,
) -> u128 {
    let sp = q64_to_f64(sqrt_price_x64);
    let sp_next = q64_to_f64(tick_to_sqrt_price_x64(next_tick_index));
    if sp_next <= sp {
        return 0;
    }
    // Uniswap v3 math (float approx):
    // dx_raw (tokenB in) = L * (sp_next - sp)
    let l = liquidity as f64;
    let dx_raw = l * (sp_next - sp);
    if dx_raw <= 0.0 {
        return 0;
    }
    let fee_mult = 10000.0_f64 / (10000.0_f64 - total_fee_bps as f64);
    let dx_in = dx_raw * fee_mult;
    dx_in.max(0.0) as u128
}

// One-time seeding of three TickArrays via RPC, then reuse stream for deltas.
pub fn seed_tick_arrays_via_rpc(
    rpc: &RpcClient,
    program_id: &Pubkey,
    pool_pubkey: &Pubkey,
    tick_current: i32,
    tick_spacing: i32,
    lower_start: i32,
    current_start: i32,
    upper_start: i32,
) -> Result<()> {
    let (ta_lower, _) = derive_tick_array_address(program_id, pool_pubkey, lower_start);
    let (ta_current, _) = derive_tick_array_address(program_id, pool_pubkey, current_start);
    let (ta_upper, _) = derive_tick_array_address(program_id, pool_pubkey, upper_start);

    let addrs = vec![ta_lower, ta_current, ta_upper];
    let accounts = rpc
        .get_multiple_accounts(&addrs)
        .map_err(|e| anyhow!("get_multiple_accounts failed: {}", e))?;

    for (idx, acc_opt) in accounts.into_iter().enumerate() {
        let addr = addrs[idx];
        match acc_opt {
            None => {
                log::info!("tick_array addr={} missing (PDA not initialized)", addr);
            }
            Some(acc) => {
                if &acc.owner != program_id {
                    log::info!("tick_array addr={} wrong owner {}", addr, acc.owner);
                    continue;
                }
                let data = acc.data.as_slice();
                let disc_ok = data.len() >= 8; // quick check only
                match decode_tick_array(data) {
                    Ok(ta) => {
                        let init_count = ta.ticks.iter().filter(|t| t.liquidity_gross > 0).count();
                        log::info!(
                            "tick_array addr={} len={} disc_ok={} start_tick_index={} whirlpool_ok={} initialized_ticks={}",
                            addr,
                            data.len(),
                            disc_ok,
                            ta.start_tick_index,
                            ta.whirlpool == *pool_pubkey,
                            init_count
                        );
                        // Print first few initialized indices
                        let mut printed = 0;
                        for (i, t) in ta.ticks.iter().enumerate() {
                            if t.liquidity_gross > 0 {
                                let idx_tick = ta.start_tick_index + (i as i32) * tick_spacing;
                                log::info!("init_tick={}", idx_tick);
                                printed += 1;
                                if printed >= 6 {
                                    break;
                                }
                            }
                        }
                        // Feed the same update path for in-memory state
                        let _ = on_tick_array_update(
                            pool_pubkey,
                            ta.start_tick_index,
                            tick_spacing,
                            data,
                        );
                    }
                    Err(e) => {
                        log::info!("tick_array addr={} decode error: {}", addr, e);
                    }
                }
            }
        }
    }

    Ok(())
}
