use arb_engine::edge_integration::EdgeArbitrageDetector;
use arb_engine::hotpath::SignalQueue;
use edge_assembler::edge::{edge_delta, ClmmState, CpmmState, DlmmState, EdgeDelta};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn q64(price: f64) -> Vec<u8> {
    ((price * 18446744073709551616.0) as u128)
        .to_le_bytes()
        .to_vec()
}
fn size(v: u128) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

// Construct a 3-leg cycle A->B->C->A with net profit
// Legs:
// A->B price 1.02, B->C 1.02, C->A 1.01; fees are 0.1% per leg.
// Fees each 0.1% (10 bps)
#[tokio::test]
async fn three_leg_cycle_generates_signal() {
    if std::env::var("SKIP_SLOW_TESTS").is_ok() {
        return;
    }
    let ring = SignalQueue::new(8);
    let tx = ring.sender();
    let rx = ring.receiver();
    let det = Arc::new(EdgeArbitrageDetector::new(tx));

    let a = vec![1u8; 32];
    let b = vec![2u8; 32];
    let c = vec![3u8; 32];
    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    // e1: A->B direction 0
    let e1 = EdgeDelta {
        edge_id: 1,
        venue_id: 1,
        pool_pubkey: vec![11; 32],
        token_in: a.clone(),
        token_out: b.clone(),
        direction: 0,
        decimals_in: 9,
        decimals_out: 9,
        fee_bps_total: 10,
        slot: 1,
        write_version: 1,
        updated_at_ns: now_ns,
        marginal_price_q64: q64(1.02),
        size_to_next_boundary_in: size(10_000),
        cu_estimate: 50_000,
        state: Some(edge_delta::State::Clmm(ClmmState {
            sqrt_price_x64: q64(1.0),
            liquidity: size(1_000_000),
            tick_current: 0,
            tick_spacing: 64,
            size_to_next_in: size(10_000),
        })),
    };
    // e2: B->C direction 0
    let e2 = EdgeDelta {
        edge_id: 2,
        venue_id: 2,
        pool_pubkey: vec![22; 32],
        token_in: b.clone(),
        token_out: c.clone(),
        direction: 0,
        decimals_in: 9,
        decimals_out: 9,
        fee_bps_total: 10,
        slot: 1,
        write_version: 1,
        updated_at_ns: now_ns,
        marginal_price_q64: q64(1.02),
        size_to_next_boundary_in: size(10_000),
        cu_estimate: 50_000,
        state: Some(edge_delta::State::Cpmm(CpmmState {
            reserve_in: size(1_000_000),
            reserve_out: size(1_020_000),
        })),
    };
    // e3: C->A direction 0 price 1.01
    let e3 = EdgeDelta {
        edge_id: 3,
        venue_id: 3,
        pool_pubkey: vec![33; 32],
        token_in: c.clone(),
        token_out: a.clone(),
        direction: 0,
        decimals_in: 9,
        decimals_out: 9,
        fee_bps_total: 10,
        slot: 1,
        write_version: 1,
        updated_at_ns: now_ns,
        marginal_price_q64: q64(1.01),
        size_to_next_boundary_in: size(10_000),
        cu_estimate: 50_000,
        state: Some(edge_delta::State::Dlmm(DlmmState {
            active_bin: 0,
            bin_price_q64: q64(1.01),
            depth_hint_in: size(10_000),
        })),
    };

    det.process_edge_update(e1).unwrap();
    det.process_edge_update(e2).unwrap();
    det.process_edge_update(e3.clone()).unwrap();
    // Trigger additional scans to ensure 3-hop detection runs
    det.process_edge_update(e3.clone()).unwrap();

    // Reduced timeout; cycle detection happens inline with process_edge_update calls.
    let sig = tokio::time::timeout(std::time::Duration::from_millis(120), async move {
        rx.recv_async().await
    })
    .await
    .expect("no 3-leg signal");
    assert!(
        sig.profit_bps > 0,
        "expected positive profit bps for 3-leg cycle"
    );
    assert!(sig.dex_mid.is_some(), "mid leg should be populated");
}
