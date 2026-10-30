use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
    str::FromStr,
};

use solana_sdk::pubkey::Pubkey;
use whirlpool_core::{decode::decode_whirlpool, exact};

#[test]
fn real_public_swaps_match_both_vault_amounts_exactly() {
    let bytes =
        zstd::decode_all(Cursor::new(include_bytes!("golden/whirlpool-v1.jsonl.zst"))).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let program = Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap();
    let mut count = 0;
    let mut crossings = 0;
    let mut pools = BTreeSet::new();
    let mut spacings = BTreeSet::new();
    let mut directions = BTreeSet::new();
    for line in text.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(row["version"], 1);
        assert_eq!(row["result"], "exact_match");
        assert_eq!(
            row["source"],
            "Real public on-chain data; quote correctness only"
        );
        let pool_id = Pubkey::from_str(row["pool"].as_str().unwrap()).unwrap();
        let pre: Vec<u8> = serde_json::from_value(row["pre_pool"].clone()).unwrap();
        let pool = decode_whirlpool(&pre).unwrap();
        let mut arrays = BTreeMap::new();
        for (address, bytes) in row["arrays"].as_object().unwrap() {
            let key = Pubkey::from_str(address).unwrap();
            let bytes: Vec<u8> = serde_json::from_value(bytes.clone()).unwrap();
            arrays.insert(key, exact::decode_array(&bytes).unwrap());
        }
        let input = row["observed_input"].as_u64().unwrap();
        let output = row["observed_output"].as_u64().unwrap();
        let a_to_b = row["a_to_b"].as_bool().unwrap();
        let exact_in =
            exact::quote_exact_in(&program, &pool_id, &pool, &arrays, input, a_to_b).unwrap();
        let exact_out =
            exact::quote_exact_out(&program, &pool_id, &pool, &arrays, output, a_to_b).unwrap();
        assert!(
            (exact_in.input_amount == input && exact_in.output_amount == output)
                || (exact_out.input_amount == input && exact_out.output_amount == output),
            "{}@{}",
            row["signature"].as_str().unwrap(),
            row["slot"]
        );
        count += 1;
        if row["crossed_tick"].as_bool().unwrap() {
            crossings += 1;
        }
        pools.insert(pool_id);
        spacings.insert(pool.tick_spacing);
        directions.insert(a_to_b);
    }
    assert!(count >= 20);
    assert!(crossings >= 5);
    assert!(pools.len() >= 5);
    assert!(spacings.len() >= 5);
    assert_eq!(directions, [false, true].into());
    assert!(include_bytes!("golden/whirlpool-v1.jsonl.zst").len() <= 256 * 1024);
}
