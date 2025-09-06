use arb_engine::edge_integration::EdgeArbitrageDetector;
use arb_engine::hotpath::SignalQueue;
use edge_assembler::edge::EdgeDelta;
use std::sync::Arc;

fn q64(price: f64) -> Vec<u8> {
    ((price * 18446744073709551616.0) as u128)
        .to_le_bytes()
        .to_vec()
}
fn size(v: u128) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

fn mk_edge(
    id: u64,
    venue: u32,
    tin: u8,
    tout: u8,
    price: f64,
    fee_bps: u32,
    sz: u128,
) -> EdgeDelta {
    EdgeDelta {
        edge_id: id,
        venue_id: venue,
        pool_pubkey: vec![id as u8; 32],
        token_in: vec![tin; 32],
        token_out: vec![tout; 32],
        direction: 0,
        decimals_in: 9,
        decimals_out: 9,
        fee_bps_total: fee_bps,
        slot: 1,
        write_version: 1,
        updated_at_ns: id,
        marginal_price_q64: q64(price),
        size_to_next_boundary_in: size(sz),
        cu_estimate: 50_000,
        state: None,
    }
}

#[test]
fn three_leg_profit_zero_size() {
    let ring = SignalQueue::new(4);
    let det = EdgeArbitrageDetector::new(ring.sender());
    let e1 = Arc::new(mk_edge(1, 1, 1, 2, 1.01, 10, 0)); // zero size
    let e2 = Arc::new(mk_edge(2, 2, 2, 3, 1.01, 10, 10_000));
    let e3 = Arc::new(mk_edge(3, 3, 3, 1, 1.01, 10, 10_000));
    // evaluate_3leg returns Option<(profit_lamports, chosen_size, base_size, profit_bps, ts)>
    assert!(
        det.evaluate_3leg(&e1, &e2, &e3).is_none(),
        "expected None for zero size"
    );
}

#[test]
fn three_leg_profit_below_min() {
    let ring = SignalQueue::new(4);
    let det = EdgeArbitrageDetector::new(ring.sender());
    // Prices produce near break-even after fees: each leg 1.0001 with 10 bps fee ~ (1.0001*0.999)^3 < 1 + 50bps threshold
    let e1 = Arc::new(mk_edge(1, 1, 1, 2, 1.0001, 10, 10_000));
    let e2 = Arc::new(mk_edge(2, 2, 2, 3, 1.0001, 10, 10_000));
    let e3 = Arc::new(mk_edge(3, 3, 3, 1, 1.0001, 10, 10_000));
    let res = det.evaluate_3leg(&e1, &e2, &e3);
    assert!(
        res.is_none(),
        "profit should be below min threshold: {:?}",
        res
    );
}
