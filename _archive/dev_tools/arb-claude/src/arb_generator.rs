use base64::Engine;
use edge_assembler::edge::EdgeDelta;
use prost::Message;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generates synthetic EdgeDelta messages with controlled arbitrage opportunities
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let output_file = args.get(1).unwrap_or(&"test_edges.jsonl".to_string()).clone();
    let scenario = args.get(2).unwrap_or(&"simple".to_string()).clone();
    
    println!("🔧 Generating arbitrage test data...");
    println!("   Output: {}", output_file);
    println!("   Scenario: {}", scenario);
    
    let mut writer = BufWriter::new(File::create(&output_file)?);
    let base_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    
    // Common token mints (use deterministic values for testing)
    let usdc_mint = generate_mint_bytes(1); // USDC
    let sol_mint = generate_mint_bytes(2);  // SOL
    let usdt_mint = generate_mint_bytes(3); // USDT
    
    match scenario.as_str() {
        "simple" => generate_simple_arb(&mut writer, base_time, &usdc_mint, &sol_mint)?,
        "triangle" => generate_triangle_arb(&mut writer, base_time, &usdc_mint, &sol_mint, &usdt_mint)?,
        "highfreq" => generate_high_frequency(&mut writer, base_time, &usdc_mint, &sol_mint)?,
        "stress" => generate_stress_test(&mut writer, base_time)?,
        _ => {
            println!("Unknown scenario. Available: simple, triangle, highfreq, stress");
            return Ok(());
        }
    }
    
    writer.flush()?;
    println!("✅ Generated test data in {}", output_file);
    println!("\n📝 To test:");
    println!("   cargo run --bin arb_replay -- --input {} --burst", output_file);
    
    Ok(())
}

fn generate_mint_bytes(seed: u8) -> Vec<u8> {
    let mut bytes = vec![seed; 32];
    bytes[31] = seed; // Make last byte unique
    bytes
}

fn generate_simple_arb(
    writer: &mut BufWriter<File>,
    base_time: u64,
    usdc: &[u8],
    sol: &[u8],
) -> anyhow::Result<()> {
    println!("\n📊 Simple 2-hop arbitrage scenario:");
    println!("   Venue 1: SOL→USDC @ 100 USDC/SOL");
    println!("   Venue 2: USDC→SOL @ 0.0095 SOL/USDC");
    println!("   Expected: ~5% profit opportunity\n");
    
    let edges = vec![
        // Orca: SOL -> USDC at favorable price
        create_edge(1, 1, sol, usdc, 0, 100.0, 30, 1_000_000_000, base_time),
        // Orca: USDC -> SOL (reverse direction)
        create_edge(2, 1, usdc, sol, 1, 0.01, 30, 100_000_000, base_time),
        // Raydium: USDC -> SOL at better price (arbitrage opportunity)
        create_edge(3, 2, usdc, sol, 0, 0.0095, 25, 100_000_000, base_time + 1000),
        // Raydium: SOL -> USDC (reverse)
        create_edge(4, 2, sol, usdc, 1, 105.0, 25, 1_000_000_000, base_time + 1000),
    ];
    
    write_edges(writer, &edges)?;
    Ok(())
}

fn generate_triangle_arb(
    writer: &mut BufWriter<File>,
    base_time: u64,
    usdc: &[u8],
    sol: &[u8],
    usdt: &[u8],
) -> anyhow::Result<()> {
    println!("\n📊 Triangle arbitrage scenario:");
    println!("   Path: SOL → USDC → USDT → SOL");
    println!("   Expected: ~2% profit through 3 hops\n");
    
    let edges = vec![
        // SOL -> USDC
        create_edge(101, 1, sol, usdc, 0, 100.0, 30, 1_000_000_000, base_time),
        create_edge(102, 1, usdc, sol, 1, 0.01, 30, 100_000_000, base_time),
        
        // USDC -> USDT (slight premium)
        create_edge(201, 2, usdc, usdt, 0, 1.005, 10, 1_000_000_000, base_time),
        create_edge(202, 2, usdt, usdc, 1, 0.995, 10, 1_000_000_000, base_time),
        
        // USDT -> SOL (completing the triangle with profit)
        create_edge(301, 3, usdt, sol, 0, 0.0097, 20, 100_000_000, base_time),
        create_edge(302, 3, sol, usdt, 1, 103.0, 20, 1_000_000_000, base_time),
    ];
    
    write_edges(writer, &edges)?;
    Ok(())
}

fn generate_high_frequency(
    writer: &mut BufWriter<File>,
    base_time: u64,
    usdc: &[u8],
    sol: &[u8],
) -> anyhow::Result<()> {
    println!("\n📊 High-frequency scenario:");
    println!("   Simulating rapid price updates with fleeting opportunities\n");
    
    let mut edges = Vec::new();
    let mut time = base_time;
    
    for i in 0..100 {
        let price_offset = (i as f64 * 0.1).sin() * 2.0; // Oscillating price
        
        // Venue 1 prices
        edges.push(create_edge(
            1000 + i * 4,
            1,
            sol,
            usdc,
            0,
            100.0 + price_offset,
            30,
            1_000_000_000,
            time,
        ));
        edges.push(create_edge(
            1001 + i * 4,
            1,
            usdc,
            sol,
            1,
            0.01 - price_offset * 0.0001,
            30,
            100_000_000,
            time,
        ));
        
        // Venue 2 prices (lagging slightly)
        edges.push(create_edge(
            1002 + i * 4,
            2,
            sol,
            usdc,
            0,
            100.0 + price_offset * 0.8, // Lag creates arb
            25,
            1_000_000_000,
            time + 500,
        ));
        edges.push(create_edge(
            1003 + i * 4,
            2,
            usdc,
            sol,
            1,
            0.01 - price_offset * 0.00008,
            25,
            100_000_000,
            time + 500,
        ));
        
        time += 10_000_000; // 10ms between updates
    }
    
    write_edges(writer, &edges)?;
    println!("   Generated {} edge updates", edges.len());
    Ok(())
}

fn generate_stress_test(
    writer: &mut BufWriter<File>,
    base_time: u64,
) -> anyhow::Result<()> {
    println!("\n📊 Stress test scenario:");
    println!("   Many token pairs with varying arbitrage opportunities\n");
    
    let mut edges = Vec::new();
    let mut edge_id = 10000u64;
    
    for token_a_id in 1..=10 {
        for token_b_id in (token_a_id + 1)..=10 {
            let token_a = generate_mint_bytes(token_a_id);
            let token_b = generate_mint_bytes(token_b_id);
            
            // Create slight price discrepancies between venues
            let base_price = (token_a_id as f64) * 10.0 / (token_b_id as f64);
            
            for venue in 1..=3 {
                let price_mult = 1.0 + (venue as f64 - 2.0) * 0.01; // ±1% variation
                
                edges.push(create_edge(
                    edge_id,
                    venue,
                    &token_a,
                    &token_b,
                    0,
                    base_price * price_mult,
                    30,
                    1_000_000_000,
                    base_time + edge_id * 1000,
                ));
                edge_id += 1;
                
                edges.push(create_edge(
                    edge_id,
                    venue,
                    &token_b,
                    &token_a,
                    1,
                    1.0 / (base_price * price_mult),
                    30,
                    1_000_000_000,
                    base_time + edge_id * 1000,
                ));
                edge_id += 1;
            }
        }
    }
    
    write_edges(writer, &edges)?;
    println!("   Generated {} edges across {} token pairs", edges.len(), 45);
    Ok(())
}

fn create_edge(
    edge_id: u64,
    venue_id: u32,
    token_in: &[u8],
    token_out: &[u8],
    direction: u32,
    price: f64,
    fee_bps: u32,
    liquidity: u128,
    timestamp: u64,
) -> EdgeDelta {
    let pool_key = {
        let mut key = vec![venue_id as u8; 32];
        key[0] = (edge_id % 256) as u8;
        key
    };
    
    EdgeDelta {
        edge_id,
        venue_id,
        pool_pubkey: pool_key,
        token_in: token_in.to_vec(),
        token_out: token_out.to_vec(),
        direction,
        decimals_in: 9,
        decimals_out: 6,
        fee_bps_total: fee_bps,
        slot: 1000000 + edge_id,
        write_version: 1,
        updated_at_ns: timestamp,
        marginal_price_q64: {
            // Adjust for decimals difference (9-6=3, factor of 1000)
            let adjusted_price = if direction == 0 {
                price / 1000.0
            } else {
                price * 1000.0
            };
            let q64_price = (adjusted_price * 18446744073709551616.0) as u128;
            q64_price.to_le_bytes().to_vec()
        },
        size_to_next_boundary_in: liquidity.to_le_bytes().to_vec(),
        cu_estimate: 50000,
        state: None,
    }
}

fn write_edges(writer: &mut BufWriter<File>, edges: &[EdgeDelta]) -> anyhow::Result<()> {
    for edge in edges {
        let bytes = edge.encode_to_vec();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        writeln!(writer, r#"{{"edge_b64":"{}"}}"#, b64)?;
    }
    Ok(())
}