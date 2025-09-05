use arb_engine::edge_integration::EdgeArbitrageDetector;
use arb_engine::hotpath::SignalQueue;
use edge_assembler::edge::EdgeDelta;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("🧪 Testing Edge Integration with Realistic Arbitrage Scenario...\n");
    
    // Create signal queue
    let signal_queue = SignalQueue::new(1000);
    let signal_tx = signal_queue.sender();
    let signal_rx = signal_queue.receiver();
    
    // Create edge detector with lower thresholds for testing
    let edge_detector = Arc::new(EdgeArbitrageDetector::new(signal_tx));
    
    // Set very low thresholds for testing (0.1% profit, 5% slippage)
    arb_engine::edge_integration::set_edge_thresholds(Some(10), Some(500));
    
    // Common test data
    let current_slot = 123456789u64;
    let now_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    
    // Token mints (USDC and SOL for example)
    let usdc_mint = vec![1u8; 32]; // Mock USDC mint
    let sol_mint = vec![2u8; 32];  // Mock SOL mint
    
    // Pool pubkeys
    let orca_pool = vec![10u8; 32];
    let raydium_pool = vec![20u8; 32];
    
    println!("📊 Creating arbitrage scenario:");
    println!("   - Orca: SOL → USDC at 100 USDC/SOL");
    println!("   - Raydium: USDC → SOL at 0.0098 SOL/USDC");
    println!("   - Expected profit: ~2% round trip\n");
    
    // Edge 1: Orca pool - SOL to USDC (sell SOL at 100 USDC/SOL)
    let edge1 = EdgeDelta {
        edge_id: 1001,
        venue_id: 1, // Orca
        pool_pubkey: orca_pool.clone(),
        token_in: sol_mint.clone(),
        token_out: usdc_mint.clone(),
        direction: 0, // A to B
        decimals_in: 9,
        decimals_out: 6,
        fee_bps_total: 30, // 0.3% fee
        slot: current_slot,
        write_version: 1,
        updated_at_ns: now_ns,
        // Price: 100 USDC per SOL = 100 * 2^64 / 10^(9-6) = 100 * 2^64 / 1000
        marginal_price_q64: {
            let price = 100.0 * 18446744073709551616.0 / 1000.0;
            (price as u128).to_le_bytes().to_vec()
        },
        size_to_next_boundary_in: (1_000_000_000u128).to_le_bytes().to_vec(), // 1 SOL liquidity
        cu_estimate: 50000,
        state: None,
    };
    
    // Edge 2: Raydium pool - USDC to SOL (buy SOL at 0.0098 SOL/USDC)
    // This creates arbitrage: sell 1 SOL for 100 USDC, buy back 0.98 SOL with 100 USDC
    let edge2 = EdgeDelta {
        edge_id: 2001,
        venue_id: 2, // Raydium
        pool_pubkey: raydium_pool.clone(),
        token_in: usdc_mint.clone(),
        token_out: sol_mint.clone(),
        direction: 0, // A to B
        decimals_in: 6,
        decimals_out: 9,
        fee_bps_total: 25, // 0.25% fee
        slot: current_slot,
        write_version: 1,
        updated_at_ns: now_ns + 1000, // Slightly after first edge
        // Price: 0.0098 SOL per USDC = 0.0098 * 2^64 * 1000
        marginal_price_q64: {
            let price = 0.0098 * 18446744073709551616.0 * 1000.0;
            (price as u128).to_le_bytes().to_vec()
        },
        size_to_next_boundary_in: (100_000_000u128).to_le_bytes().to_vec(), // 100 USDC liquidity
        cu_estimate: 50000,
        state: None,
    };
    
    // For 2-hop arbitrage, we need the reverse direction edges
    // Edge 3: Orca pool - USDC to SOL (reverse of edge1)
    let edge3 = EdgeDelta {
        edge_id: 1002,
        venue_id: 1, // Orca
        pool_pubkey: orca_pool.clone(),
        token_in: usdc_mint.clone(),
        token_out: sol_mint.clone(),
        direction: 1, // B to A (reverse)
        decimals_in: 6,
        decimals_out: 9,
        fee_bps_total: 30,
        slot: current_slot,
        write_version: 1,
        updated_at_ns: now_ns,
        // Inverse price: 0.01 SOL per USDC (1/100)
        marginal_price_q64: {
            let price = 0.01 * 18446744073709551616.0 * 1000.0;
            (price as u128).to_le_bytes().to_vec()
        },
        size_to_next_boundary_in: (100_000_000u128).to_le_bytes().to_vec(),
        cu_estimate: 50000,
        state: None,
    };
    
    // Edge 4: Raydium pool - SOL to USDC (reverse of edge2)
    let edge4 = EdgeDelta {
        edge_id: 2002,
        venue_id: 2, // Raydium
        pool_pubkey: raydium_pool.clone(),
        token_in: sol_mint.clone(),
        token_out: usdc_mint.clone(),
        direction: 1, // B to A (reverse)
        decimals_in: 9,
        decimals_out: 6,
        fee_bps_total: 25,
        slot: current_slot,
        write_version: 1,
        updated_at_ns: now_ns + 1000,
        // Inverse price: 102 USDC per SOL (creates arbitrage opportunity)
        marginal_price_q64: {
            let price = 102.0 * 18446744073709551616.0 / 1000.0;
            (price as u128).to_le_bytes().to_vec()
        },
        size_to_next_boundary_in: (1_000_000_000u128).to_le_bytes().to_vec(),
        cu_estimate: 50000,
        state: None,
    };
    
    println!("🔄 Processing edges...");
    
    // Process all edges
    for (i, edge) in [edge1, edge2, edge3, edge4].iter().enumerate() {
        match edge_detector.process_edge_update(edge.clone()) {
            Ok(()) => println!("   ✅ Edge {} processed (venue: {}, direction: {})", 
                               i + 1, edge.venue_id, edge.direction),
            Err(e) => println!("   ❌ Edge {} failed: {}", i + 1, e),
        }
    }
    
    // Give time for async processing
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Check for signals
    println!("\n📡 Checking for arbitrage signals...");
    let mut signal_count = 0;
    while let Some(signal) = signal_rx.try_recv() {
        signal_count += 1;
        println!("\n🎯 ARBITRAGE SIGNAL #{}", signal_count);
        println!("   ID: {}", signal.id);
        println!("   Pair: {} <-> {}", signal.pair.a, signal.pair.b);
        println!("   Route: {} → {} → {}", 
                 signal.dex_sell, 
                 signal.dex_mid.as_ref().unwrap_or(&"direct".to_string()),
                 signal.dex_buy);
        println!("   Amount In: {}", signal.amount_in);
        println!("   Expected Profit: {} ({} bps)", 
                 signal.expected_profit_out, signal.profit_bps);
        println!("   Pools: {} → {} → {}", 
                 &signal.sell_pool_id[..8],
                 signal.mid_pool_id.as_ref().map(|s| &s[..8]).unwrap_or("N/A"),
                 &signal.buy_pool_id[..8]);
    }
    
    if signal_count == 0 {
        println!("❌ No arbitrage signals generated!");
        println!("\n🔍 Debugging tips:");
        println!("   1. Check profit thresholds (currently 10 bps)");
        println!("   2. Verify price calculations");
        println!("   3. Ensure edges have opposite directions");
        println!("   4. Check liquidity boundaries");
    } else {
        println!("\n✅ Successfully detected {} arbitrage opportunity(s)!", signal_count);
    }
    
    // Additional debugging info
    println!("\n📈 Edge Statistics:");
    println!("   - Minimum profit threshold: {} bps", 
             arb_engine::edge_integration::edge_threshold_snapshot().0);
    println!("   - Maximum slippage: {} bps", 
             arb_engine::edge_integration::edge_threshold_snapshot().1);
}