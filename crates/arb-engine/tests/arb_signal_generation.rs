use arb_engine::{hotpath::SignalQueue, FastEngine};
use edge_assembler::{edge::edge_delta, ClmmState, CpmmState, EdgeDelta};
use std::sync::Arc;

fn encode_q64_64(price: f64) -> Vec<u8> {
    let raw = (price * 18446744073709551616.0) as u128; // price * 2^64
    raw.to_le_bytes().to_vec()
}

#[tokio::test]
async fn generates_signal_from_two_edges() {
    if std::env::var("SKIP_SLOW_TESTS").is_ok() {
        return;
    }
    let q = Arc::new(SignalQueue::new(16));
    let tx = q.sender();
    let engine = FastEngine::new(tx);
    // Token mints (dummy 32 bytes)
    let token_a = vec![1u8; 32];
    let token_b = vec![2u8; 32];

    // Synthetic 2% mispricing per leg exceeds the edge detector's 50 bps default
    // after both 30 bps fees, without changing a global threshold.
    let price_ab = 1.02;
    let price_ba = 1.02;
    let fee_bps = 30u32;

    let base_edge = |direction: u32, mp: f64| EdgeDelta {
        edge_id: if direction == 0 { 10 } else { 11 },
        venue_id: if direction == 0 { 1 } else { 2 },
        pool_pubkey: vec![9u8; 32],
        token_in: if direction == 0 {
            token_a.clone()
        } else {
            token_b.clone()
        },
        token_out: if direction == 0 {
            token_b.clone()
        } else {
            token_a.clone()
        },
        direction,
        decimals_in: 9,
        decimals_out: 9,
        fee_bps_total: fee_bps,
        slot: 1,
        write_version: 1,
        updated_at_ns: 0,
        marginal_price_q64: encode_q64_64(mp),
        size_to_next_boundary_in: (1000u128).to_le_bytes().to_vec(),
        cu_estimate: 50000,
        state: Some(if direction == 0 {
            edge_delta::State::Clmm(ClmmState {
                sqrt_price_x64: encode_q64_64(1.0),
                liquidity: (1_000_000u128).to_le_bytes().to_vec(),
                tick_current: 0,
                tick_spacing: 64,
                size_to_next_in: (1000u128).to_le_bytes().to_vec(),
            })
        } else {
            edge_delta::State::Cpmm(CpmmState {
                reserve_in: (1_000_000u128).to_le_bytes().to_vec(),
                reserve_out: (1_005_000u128).to_le_bytes().to_vec(),
            })
        }),
    };

    let e1 = base_edge(0, price_ab);
    let e2 = base_edge(1, price_ba);
    engine.process_edge_update(e1).unwrap();
    engine.process_edge_update(e2).unwrap();

    // Receive signal
    let rx = q.receiver();
    // Allow a brief delay
    // Short timeout to keep test fast; detection is synchronous so 80ms is plenty.
    let sig = tokio::time::timeout(std::time::Duration::from_millis(80), async move {
        rx.recv_async().await
    })
    .await
    .expect("no signal");
    assert!(sig.profit_bps > 0, "profit_bps should be positive");
}
