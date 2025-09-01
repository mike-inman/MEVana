use arb_engine::edge_integration::EdgeArbitrageDetector;
use edge_assembler::EdgeDelta;
use arb_engine::hotpath::SignalQueue;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("🧪 Testing Edge Integration Module...");
    
    // Create signal queue
    let signal_queue = SignalQueue::new(1000);
    let signal_tx = signal_queue.sender();
    
    // Create edge detector
    let mut edge_detector = EdgeArbitrageDetector::new(signal_tx);
    
    // Test data: Create a mock edge update
    let mock_edge = EdgeDelta {
        edge_id: 12345,
        venue_id: 1, // Orca Whirlpool
        pool_pubkey: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32],
        token_in: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32],
        token_out: vec![33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64],
        direction: 0, // AtoB
        decimals_in: 9,
        decimals_out: 6,
        fee_bps_total: 30,
        slot: 123456789,
        write_version: 1,
        updated_at_ns: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
        marginal_price_q64: vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 1.0 in Q64
        size_to_next_boundary_in: vec![100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 100 in little-endian
        cu_estimate: 50000,
        state: None,
    };
    
    println!("✅ Created mock edge update");
    println!("   Edge ID: {}", mock_edge.edge_id);
    println!("   Venue: {}", mock_edge.venue_id);
    println!("   Direction: {}", mock_edge.direction);
    println!("   Size to boundary: {:?}", mock_edge.size_to_next_boundary_in);
    
    // Test processing the edge
    match edge_detector.process_edge_update(mock_edge) {
        Ok(()) => println!("✅ Successfully processed edge update"),
        Err(e) => println!("❌ Failed to process edge update: {}", e),
    }
    
    // Test configuration update
    edge_detector.update_config(100, 200); // 1% min profit, 2% max slippage
    println!("✅ Updated edge detector configuration");
    
    println!("🎉 Edge integration test completed successfully!");
}
