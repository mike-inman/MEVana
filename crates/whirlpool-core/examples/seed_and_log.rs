use anyhow::{Context, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use whirlpool_core::{
    decode::{decode_tick_array, decode_whirlpool},
    math::q64_to_f64,
    pda::{actual_tick_index, derive_tick_array_address, three_tick_arrays_around},
    tick_window::{find_nearest_initialized_ticks, TickArrayView},
};

fn main() -> Result<()> {
    // Configure RPC URL and a Whirlpool pool pubkey to test
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let pool_str = std::env::args()
        .nth(1)
        .expect("usage: cargo run --example seed_and_log ");
    let program_id = Pubkey::from_str_const("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
    let pool_pk = Pubkey::from_str(&pool_str).expect("invalid pool pubkey");

    let rpc = RpcClient::new(rpc_url);

    // Fetch pool account
    let pool_acc = rpc.get_account(&pool_pk).context("get pool")?;
    let pool = decode_whirlpool(&pool_acc.data).context("decode whirlpool")?;
    println!(
        "[WHIRLPOOL] pool={} tick_current={} spacing={} fee_lp={} fee_proto={} sqrt_price_x64={}",
        pool_pk,
        pool.tick_current_index,
        pool.tick_spacing,
        pool.fee_rate,
        pool.protocol_fee_rate,
        pool.sqrt_price
    );

    // Derive tick-array PDAs
    let (lower_start, current_start, upper_start) =
        three_tick_arrays_around(pool.tick_current_index, pool.tick_spacing as i32);
    let (ta_lower, _) = derive_tick_array_address(&program_id, &pool_pk, lower_start);
    let (ta_current, _) = derive_tick_array_address(&program_id, &pool_pk, current_start);
    let (ta_upper, _) = derive_tick_array_address(&program_id, &pool_pk, upper_start);
    println!(
        "[TICK_PDAS] starts=({}, {}, {}) addrs=({}, {}, {})",
        lower_start, current_start, upper_start, ta_lower, ta_current, ta_upper
    );

    // Fetch and decode arrays (if present)
    let accs = rpc
        .get_multiple_accounts(&[ta_lower, ta_current, ta_upper])
        .context("get_multiple_accounts")?;
    let mut views = [None, None, None];

    for (idx, acc_opt) in accs.into_iter().enumerate() {
        let addr = match idx {
            0 => ta_lower,
            1 => ta_current,
            _ => ta_upper,
        };
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
                        println!(
                            "  start={} whirlpool_ok={} initialized_count={}",
                            ta.start_tick_index,
                            ta.whirlpool == pool_pk,
                            init_count
                        );
                        // print first few initialized ticks:
                        let mut printed = 0;
                        for (i, t) in ta.ticks.iter().enumerate() {
                            if t.liquidity_gross > 0 {
                                let idx_tick = actual_tick_index(
                                    ta.start_tick_index,
                                    i,
                                    pool.tick_spacing as i32,
                                );
                                println!("    init_tick={}", idx_tick);
                                printed += 1;
                                if printed >= 6 {
                                    break;
                                }
                            }
                        }
                        let view = TickArrayView {
                            start_tick_index: ta.start_tick_index,
                            ticks: &ta.ticks,
                        };
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
    let (lower_opt, upper_opt) =
        find_nearest_initialized_ticks(&window, pool.tick_current_index, pool.tick_spacing as i32);
    println!(
        "[NEAREST] next_lower={:?} next_upper={:?}",
        lower_opt, upper_opt
    );

    // Marginal price (for sanity)
    let p = {
        let sp = q64_to_f64(pool.sqrt_price);
        // price_B_per_A without decimals
        sp * sp
    };
    println!("[MARGINAL] price(B/A)~{}", p);

    Ok(())
}
