use arb_engine::edge_integration::test_decode_validate_edge;
use edge_assembler::edge::EdgeDelta;
use prost::Message;

#[test]
fn decode_valid_edge() {
    let ed = EdgeDelta {
        edge_id: 42,
        venue_id: 1,
        pool_pubkey: vec![1; 32],
        token_in: vec![2; 32],
        token_out: vec![3; 32],
        direction: 0,
        decimals_in: 9,
        decimals_out: 6,
        fee_bps_total: 30,
        state: None,
        slot: 1,
        write_version: 1,
        updated_at_ns: 123,
        marginal_price_q64: 1u128.to_le_bytes().to_vec(),
        size_to_next_boundary_in: 100u128.to_le_bytes().to_vec(),
        cu_estimate: 10_000,
    };
    let bytes = EdgeDelta::encode_to_vec(&ed);
    assert!(test_decode_validate_edge(&bytes).is_ok());
}

#[test]
fn reject_bad_direction() {
    let mut ed = EdgeDelta {
        edge_id: 42,
        venue_id: 1,
        pool_pubkey: vec![1; 32],
        token_in: vec![2; 32],
        token_out: vec![3; 32],
        direction: 2,
        decimals_in: 9,
        decimals_out: 6,
        fee_bps_total: 30,
        state: None,
        slot: 1,
        write_version: 1,
        updated_at_ns: 123,
        marginal_price_q64: 1u128.to_le_bytes().to_vec(),
        size_to_next_boundary_in: 100u128.to_le_bytes().to_vec(),
        cu_estimate: 10_000,
    };
    let bytes = EdgeDelta::encode_to_vec(&ed);
    assert!(test_decode_validate_edge(&bytes).is_err());
    ed.direction = 1; // good
    let bytes = EdgeDelta::encode_to_vec(&ed);
    assert!(test_decode_validate_edge(&bytes).is_ok());
}
