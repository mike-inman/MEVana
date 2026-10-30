use std::{collections::BTreeMap, io::Cursor, sync::mpsc, time::Duration};

use orca_whirlpools_core::{TickArrayFacade, TickFacade};
use solana_sdk::pubkey::Pubkey;
use whirlpool_core::{decode::decode_whirlpool, exact};

fn pool() -> whirlpool_core::types::WhirlpoolMinimal {
    let bytes = zstd::decode_all(Cursor::new(include_bytes!(
        "fixtures/usdc_usdt_pool.bin.zst"
    )))
    .unwrap();
    decode_whirlpool(&bytes).unwrap()
}

#[test]
fn adaptive_and_missing_arrays_are_distinct_errors() {
    let mut pool = pool();
    let program = Pubkey::new_unique();
    let pool_id = Pubkey::new_unique();
    let arrays = BTreeMap::new();
    pool.tick_spacing_seed = pool.tick_spacing.wrapping_add(1).to_le_bytes();
    assert_eq!(
        exact::quote_exact_in(&program, &pool_id, &pool, &arrays, 100, true).unwrap_err(),
        exact::ExactError::AdaptiveFeeUnsupported
    );
    pool.tick_spacing_seed = pool.tick_spacing.to_le_bytes();
    assert!(matches!(
        exact::quote_exact_in(&program, &pool_id, &pool, &arrays, 100, true),
        Err(exact::ExactError::MissingTickArray { .. })
    ));
}

#[test]
fn integer_quotes_work_in_both_directions() {
    let mut pool = pool();
    pool.tick_spacing = 1;
    pool.tick_spacing_seed = 1_u16.to_le_bytes();
    pool.tick_current_index = 0;
    pool.sqrt_price = 1_u128 << 64;
    pool.liquidity = 1_000_000_000_000;
    pool.fee_rate = 3_000;
    let program = Pubkey::new_unique();
    let pool_id = Pubkey::new_unique();
    let (key, _) = exact::derive_tick_array_address(&program, &pool_id, 0);
    let mut arrays = BTreeMap::new();
    arrays.insert(
        key,
        exact::DecodedTickArray {
            whirlpool: pool_id,
            facade: TickArrayFacade {
                start_tick_index: 0,
                ticks: [TickFacade::default(); 88],
            },
        },
    );
    let (previous, _) = exact::derive_tick_array_address(&program, &pool_id, -88);
    arrays.insert(
        previous,
        exact::DecodedTickArray {
            whirlpool: pool_id,
            facade: TickArrayFacade {
                start_tick_index: -88,
                ticks: [TickFacade::default(); 88],
            },
        },
    );
    for a_to_b in [true, false] {
        let input = exact::quote_exact_in(&program, &pool_id, &pool, &arrays, 100, a_to_b).unwrap();
        assert_eq!(input.input_amount, 100);
        assert!(input.output_amount > 0);
        let output = exact::quote_exact_out(
            &program,
            &pool_id,
            &pool,
            &arrays,
            input.output_amount,
            a_to_b,
        )
        .unwrap();
        assert_eq!(output.output_amount, input.output_amount);
        assert!(output.input_amount <= input.input_amount);
    }
}

#[test]
fn dynamic_length_is_strict() {
    let mut data = vec![0; 148];
    data[..8].copy_from_slice(&whirlpool_core::decode::anchor_discriminator(
        "account:DynamicTickArray",
    ));
    assert_eq!(exact::decode_array(&data).unwrap().facade.ticks.len(), 88);
    data.push(0);
    assert!(matches!(
        exact::decode_array(&data),
        Err(exact::ExactError::InvalidTickArray(_))
    ));
}

#[test]
fn captured_dynamic_array_decodes() {
    let bytes = zstd::decode_all(Cursor::new(include_bytes!(
        "fixtures/dynamic_tick_array.bin.zst"
    )))
    .unwrap();
    assert!(bytes.len() >= 148);
    let decoded = exact::decode_array(&bytes).unwrap();
    assert_eq!(decoded.facade.ticks.len(), 88);
    assert_eq!(
        (bytes.len() - 148) / 112,
        decoded
            .facade
            .ticks
            .iter()
            .filter(|tick| tick.initialized)
            .count()
    );
}

#[test]
fn exhausting_supplied_arrays_terminates_as_missing_array() {
    let mut pool = pool();
    pool.tick_spacing = 1;
    pool.tick_spacing_seed = 1_u16.to_le_bytes();
    pool.tick_current_index = 0;
    pool.sqrt_price = 1_u128 << 64;
    pool.liquidity = 1_000;
    let program = Pubkey::new_unique();
    let pool_id = Pubkey::new_unique();
    let arrays: BTreeMap<_, _> = [-88, 0]
        .into_iter()
        .map(|start_tick_index| {
            let (key, _) = exact::derive_tick_array_address(&program, &pool_id, start_tick_index);
            (
                key,
                exact::DecodedTickArray {
                    whirlpool: pool_id,
                    facade: TickArrayFacade {
                        start_tick_index,
                        ticks: [TickFacade::default(); 88],
                    },
                },
            )
        })
        .collect();
    for a_to_b in [true, false] {
        let (send, receive) = mpsc::channel();
        let pool = pool.clone();
        let arrays = arrays.clone();
        std::thread::spawn(move || {
            let result =
                exact::quote_exact_in(&program, &pool_id, &pool, &arrays, 1_000_000, a_to_b);
            let _ = send.send(result);
        });
        let result = receive
            .recv_timeout(Duration::from_secs(10))
            .expect("exact quote exceeded 10 s watchdog");
        assert!(matches!(
            result,
            Err(exact::ExactError::MissingTickArray { .. })
        ));
    }
}
