//! Synthetic EdgeDelta generator.
//! Produces JSONL with base64 prost EdgeDelta frames suitable for arb_replay.

use base64::Engine; // bring trait into scope
use edge_assembler::edge::EdgeDelta;
use prost::Message;
use rand::Rng;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: gen_synthetic_edges <out.jsonl> [count=10000]");
        std::process::exit(1);
    }
    let path = &args[1];
    let count: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let mut w = BufWriter::new(File::create(path)?);
    let start_ns = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    let mut rng = rand::thread_rng();
    for i in 0..count {
        let mut ed = EdgeDelta::default();
        // Random but consistent-ish tokens (32 bytes)
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        rng.fill(&mut a);
        rng.fill(&mut b);
        ed.token_in = a.to_vec();
        ed.token_out = b.to_vec();
        ed.direction = if i % 2 == 0 { 0 } else { 1 };
        ed.venue_id = (rng.gen_range(1..=4)) as u32;
        ed.slot = 1000 + i as u64;
        ed.updated_at_ns = start_ns + (i as u64) * 1_000_000; // 1ms apart
                                                              // Marginal price q64 ~ around 1.0 with slight noise
        let price = 1.0 + rng.gen_range(-0.001..0.001);
        let raw = (price * 18446744073709551616.0) as u128; // *2^64
        ed.marginal_price_q64 = raw.to_le_bytes().to_vec();
        // Fee bps
        ed.fee_bps_total = rng.gen_range(5..30);
        // Size boundary  (u128 bytes)
        let size: u128 = rng.gen_range(1_000_000u128..10_000_000u128);
        ed.size_to_next_boundary_in = size.to_le_bytes().to_vec();
        // required pools
        let mut pool = [0u8; 32];
        rng.fill(&mut pool);
        ed.pool_pubkey = pool.to_vec();
        let bytes = ed.encode_to_vec();
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        writeln!(w, "{{\"edge_b64\":\"{}\"}}", b64)?;
    }
    w.flush()?;
    eprintln!("wrote {} synthetic edges to {}", count, path);
    Ok(())
}
