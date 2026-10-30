use std::{io::Cursor, str::FromStr};

use solana_sdk::pubkey::Pubkey;
use whirlpool_core::decode::{decode_tick_array, decode_whirlpool};

const POOL_KEY: &str = "4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4";

#[test]
fn captured_pool_and_fixed_tick_array_decode() {
    let pool_bytes = zstd::decode_all(Cursor::new(include_bytes!(
        "fixtures/usdc_usdt_pool.bin.zst"
    )))
    .unwrap();
    let tick_bytes = zstd::decode_all(Cursor::new(include_bytes!(
        "fixtures/usdc_usdt_tick_array.bin.zst"
    )))
    .unwrap();
    assert_eq!(pool_bytes.len(), 653);
    assert_eq!(tick_bytes.len(), 9_988);

    let pool = decode_whirlpool(&pool_bytes).unwrap();
    let ticks = decode_tick_array(&tick_bytes).unwrap();
    assert_eq!(
        pool.token_mint_a.to_string(),
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    assert_eq!(
        pool.token_mint_b.to_string(),
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
    );
    assert_eq!(ticks.whirlpool, Pubkey::from_str(POOL_KEY).unwrap());
    assert_eq!(
        ticks.start_tick_index % (i32::from(pool.tick_spacing) * 88),
        0
    );
    assert!(ticks.ticks.iter().any(|tick| tick.initialized));

    assert!(decode_tick_array(&tick_bytes[..tick_bytes.len() - 1]).is_err());
    let mut too_long = tick_bytes;
    too_long.push(0);
    assert!(decode_tick_array(&too_long).is_err());
}
