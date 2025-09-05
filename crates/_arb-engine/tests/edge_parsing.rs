use arb_engine::edge_integration::decode_and_validate_edge;
use arb_engine::edge_verifier::EdgeVerifier;
use edge_assembler::edge::{edge_delta, ClmmState, ClobState, CpmmState, DlmmState, EdgeDelta};
use prost::Message;
use rand::RngCore;

fn make_u128_bytes(v: u128) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
fn make_u64_bytes(v: u64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

fn build_edge(venue: u32, id: u64) -> EdgeDelta {
    let mut token_in = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_in);
    let mut token_out = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_out);
    let mut pool = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut pool);
    let price: u128 = (1.0_f64 * 18446744073709551616.0) as u128; // 1.0 in Q64.64
    let size: u128 = 5_000_000;
    let fee_bps = 30u32;
    let state = match venue {
        1 => edge_delta::State::Clmm(ClmmState {
            sqrt_price_x64: make_u128_bytes(price),
            liquidity: make_u128_bytes(1_000_000),
            tick_current: 0,
            tick_spacing: 64,
            size_to_next_in: make_u128_bytes(size),
        }),
        2 => edge_delta::State::Cpmm(CpmmState {
            reserve_in: make_u128_bytes(10_000_000),
            reserve_out: make_u128_bytes(10_100_000),
        }),
        3 => edge_delta::State::Dlmm(DlmmState {
            active_bin: 0,
            bin_price_q64: make_u128_bytes(price),
            depth_hint_in: make_u128_bytes(size),
        }),
        4 => edge_delta::State::Clob(ClobState {
            best_bid_px_q64: make_u128_bytes(price),
            best_bid_size: make_u64_bytes(1000),
            best_ask_px_q64: make_u128_bytes(price),
            best_ask_size: make_u64_bytes(1200),
        }),
        _ => panic!("bad venue"),
    };
    EdgeDelta {
        edge_id: id,
        venue_id: venue,
        pool_pubkey: pool.to_vec(),
        token_in: token_in.to_vec(),
        token_out: token_out.to_vec(),
        direction: 0,
        decimals_in: 6,
        decimals_out: 6,
        fee_bps_total: fee_bps,
        state: Some(state),
        slot: 100,
        write_version: 0,
        updated_at_ns: 1_000_000,
        marginal_price_q64: make_u128_bytes(price),
        size_to_next_boundary_in: make_u128_bytes(size),
        cu_estimate: 0,
    }
}

#[test]
fn test_edge_decoding_all_venues() {
    let verifier = EdgeVerifier::new(true);
    for venue in 1..=4u32 {
        let edge = build_edge(venue, venue as u64);
        let bytes = EdgeDelta::encode_to_vec(&edge);
        // decode path under test
        let decoded = decode_and_validate_edge(&bytes).expect("decode ok");
        // feed verifier
        verifier.observe(&decoded);
        // print a concise line (visible with --nocapture)
        println!(
            "TEST_EDGE_FULL venue={} id={} price_q64_bytes={} size_boundary_bytes={}",
            decoded.venue_id,
            decoded.edge_id,
            decoded.marginal_price_q64.len(),
            decoded.size_to_next_boundary_in.len()
        );
    }
    let snap = verifier.snapshot_json();
    println!("TEST_EDGE_SNAPSHOT {}", snap);
    // basic assertions: ensure one ok edge per venue and zero errors
    let venues = snap["venues"].as_array().expect("venues arr");
    assert_eq!(venues.len(), 4);
    for v in venues {
        assert_eq!(v["ok"].as_u64().unwrap(), 1, "each venue ok=1");
        assert_eq!(v["bad_price"].as_u64().unwrap(), 0);
        assert_eq!(v["bad_size"].as_u64().unwrap(), 0);
        assert_eq!(v["bad_state"].as_u64().unwrap(), 0);
        assert_eq!(v["bad_tokens"].as_u64().unwrap(), 0);
    }
}
