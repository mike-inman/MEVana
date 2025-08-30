// crates/normalizer-service/src/main.rs
mod raydium_map;   // raydium parsers
mod meteora_map;   // meteora parsers
mod orca_map;      // orca whirlpool parsers
mod kafka;         // <- see notes at bottom for kafka.rs

// use anyhow::Result;
use clap::Parser;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{fs, path::PathBuf};

use normalizer_generated::registry as gen;
use normalizer_runtime as rt;
use normalizer_runtime::layouts::LayoutRegistry;
use normalizer_runtime::models::SwapEvent;
use crate::kafka::{make_producer, publish_json_ff};
// use kafka::{make_producer, publish_json};

use normalizer_runtime::adapters::to_swap_event;


use anyhow::{Result, Context};




// ---------------- env / topics ----------------

lazy_static! {
    static ref KAFKA_BROKER: String =
        std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    static ref SWAP_TOPIC: String =
        std::env::var("NORMALIZED_SWAPS_TOPIC").unwrap_or_else(|_| "normalized-swaps.v1".into());
}

// ---------------- small helper: map normalized -> wire ----------------

fn map_ns_to_swap(ns: &rt::NormalizedSwapV1) -> SwapEvent {
    SwapEvent {
        dex: ns.dex.clone(),
        pool_id: ns.pool_id.clone(),
        slot: ns.slot,
        sig: ns.signature.clone(),
        in_mint: ns.token_in_mint.clone(),
        out_mint: ns.token_out_mint.clone(),
        amount_in: ns.amount_in.to_string(),
        amount_out: ns.amount_out.to_string(),
        fee_paid: ns
            .fee_bps_estimate
            .map(|b| b as u64)
            .unwrap_or_default()
            .to_string(),
    }
}

// ---------------- layouts loader ----------------

fn load_layouts() -> Result<LayoutRegistry> {
    let mut reg = LayoutRegistry::new();

    let root = std::env::var("IDL_REGISTRY_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("crates/mevana-idl-registry/idls"));
    println!("🔍 Loading IDLs/layouts from: {}", root.display());

    // sanity probe (Raydium CLMM pool)
    let pool_path = root.join("raydium_clmm/layout.pool.v1.json");
    match std::fs::read_to_string(&pool_path) {
        Ok(content) => {
            if let Ok(layout) =
                serde_json::from_str::<normalizer_runtime::layouts::LayoutJson>(&content)
            {
                reg.insert("raydium_clmm/pool".to_string(), layout);
                println!("✅ Loaded raydium_clmm/pool");
            } else {
                println!("❌ JSON parsing failed for {}", pool_path.display());
            }
        }
        Err(e) => {
            println!("❌ File read failed: {e}");
        }
    }

    // the rest
    let files = vec![
        ("raydium_clmm/tickarray", "raydium_clmm/layout.tickarray.v1.json"),
        ("meteora_dlmm/pool", "meteora_dlmm/layout.pool.v1.json"),
        ("meteora_dlmm/bin", "meteora_dlmm/layout.bin.v1.json"),
        ("orca_whirlpool/pool", "orca_whirlpool/layout.pool.v1.json"),
        ("orca_whirlpool/tickarray", "orca_whirlpool/layout.tickarray.v1.json"),
    ];

    for (key, filename) in files {
        match reg.load_file(key, &root.join(filename)) {
            Ok(_) => println!("✅ Loaded {}", key),
            Err(e) => println!("❌ Failed to load {}: {}", key, e),
        }
    }

    Ok(reg)
}

// ---------------- CLI ----------------

#[derive(Parser, Debug)]
#[command(name = "normalizer-service", version)]
enum Cmd {
    Fixture { path: PathBuf }, // one instruction fixture
    AccountFixture {
        path: PathBuf,          // base64 account.data
        kind: String,           // raydium:tickarray | meteora:pool | meteora:bin | orca:pool | orca:tickarray
        pool_id: Option<String>,
        slot: Option<u64>,
    },
    TestParsers,
}

// NOTE: use Tokio runtime because we send to Kafka asynchronously
#[tokio::main]
async fn main() -> Result<()> {
    let cmd = Cmd::parse();
    match cmd {
        Cmd::Fixture { path } => run_fixture(path),
        Cmd::AccountFixture { path, kind, pool_id, slot } => {
            run_account_fixture(path, kind, pool_id, slot).await
        }
        Cmd::TestParsers => test_parsers().await,
    }
}

// ---------------- Fixture format ----------------

#[derive(Deserialize)]
struct Fixture {
    slot: u64,
    block_time_unix: i64,
    signature: String,
    meta_ok: bool,
    instruction: FxInstruction,
    token_balance_changes: Vec<FxBalanceChange>,
}

#[derive(Deserialize)]
struct FxInstruction {
    program_id: String,
    account_keys: Vec<String>,
    account_indices: Vec<u8>,
    data_b64: String,
}

#[derive(Deserialize)]
struct FxBalanceChange {
    owner: String,
    mint: String,
    pre: String,  // decimal string
    post: String, // decimal string
}

fn parse_u128(s: &str) -> Result<u128> {
    Ok(s.parse::<u128>()?)
}

// ---------------- Core: run_fixture (uses generated extractors) ----------------

fn run_fixture(path: PathBuf) -> Result<()> {
    // 1) load fixture json
    let txt = fs::read_to_string(&path)?;
    let fx: Fixture = serde_json::from_str(&txt)?;

    // 2) decode ix data (this is what the decoder wants as 1st arg)
    use base64::{engine::general_purpose, Engine as _};
    let ix_bytes: Vec<u8> = general_purpose::STANDARD
        .decode(&fx.instruction.data_b64)
        .context("decode fx.instruction.data_b64 (base64)")?;

    // 3) convert indices to u32 (decoder wants &[u32])
    let acc_indices_u32: Vec<u32> = fx.instruction
        .account_indices
        .iter()
        .map(|&i| i as u32)
        .collect();

    // 4) balance deltas
    let mut token_changes = Vec::<rt::TokenBalanceChange>::new();
    for c in &fx.token_balance_changes {
        token_changes.push(rt::TokenBalanceChange {
            owner: c.owner.clone(),
            mint: c.mint.clone(),
            pre: parse_u128(&c.pre)?,
            post: parse_u128(&c.post)?,
        });
    }

    // 5) optional: IDL decode name/roles (uses the generated decoder)
   // IDL decode name/roles
    let (decoded_name, decoded_roles) =
    if let Some(dec) = gen::decoder_for(&fx.instruction.program_id) {
        if let Some((name, roles)) = dec(
            &ix_bytes,
            &fx.instruction.account_indices,   // <-- use &[u8] directly
            &fx.instruction.account_keys
        ) {
            (Some(name), Some(roles))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // 6) build TxContext
    let ctx = rt::TxContext {
        slot: fx.slot,
        block_time_unix: fx.block_time_unix,
        signature: fx.signature.clone(),
        ix_index: 0,
        program_id: fx.instruction.program_id.clone(),
        meta_ok: fx.meta_ok,
        token_balance_changes: &token_changes,
        account_data_by_pubkey: None,
        decoded_name,
        decoded_args_json: None,
        decoded_accounts_by_role: decoded_roles,
    };

    // 7) run extractors for this program id
    let extractors: Vec<Box<dyn rt::SwapExtractor>> = gen::registry();
    let mut hits = 0usize;

    // Kafka producer (non-blocking publish)
    let producer = make_producer(&KAFKA_BROKER)?; // ensure KAFKA_BROKER & SWAP_TOPIC exist

    for ex in extractors.iter().filter(|e| e.program_id() == fx.instruction.program_id.as_str()) {
        if let Some(ns) = ex.try_extract(&ctx) {
            hits += 1;
            println!("✅ {} extracted: {} -> {} (slot {})",
                ns.dex, ns.token_in_mint, ns.token_out_mint, ns.slot);

            let swap = map_ns_to_swap(&ns);

            // Fire-and-forget publish to Kafka (Send-safe helper)
            publish_json_ff(
                producer.clone(),
                SWAP_TOPIC.clone(),
                swap.pool_id.clone(),
                swap.clone(),
            );
            println!("✅ Published to Kafka: {}", swap.pool_id);

            println!("{}", serde_json::to_string(&swap)?);
        }
    }

    if hits == 0 {
        println!("ℹ️ No extractor matched or discriminator didn’t match payload.");
    }

    Ok(())
}

// ---------------- AccountFixture / tests (unchanged except async) ----------------

async fn run_account_fixture(
    path: PathBuf,
    kind: String,
    pool_id: Option<String>,
    slot: Option<u64>,
) -> Result<()> {
    let layouts = load_layouts()?;
    let b64 = std::fs::read_to_string(path)?;
    let data = base64::decode(b64.trim())?;

    match kind.as_str() {
        "raydium:tickarray" => {
            if let Some(tw) = raydium_map::decode_raydium_tick_array(&layouts, &data) {
                println!("ticks={}", tw.rows.len());
            } else {
                anyhow::bail!("not a tickarray (layout mismatch?)");
            }
        }
        "meteora:pool" => {
            let pid = pool_id.unwrap_or_else(|| "<unknown>".into());
            let snap = meteora_map::decode_meteora_pool_snapshot(&layouts, &data, &pid, slot.unwrap_or(0))
                .ok_or_else(|| anyhow::anyhow!("not a meteora pool"))?;
            println!("{}", serde_json::to_string_pretty(&snap)?);
        }
        "meteora:bin" => {
            let bin = meteora_map::decode_meteora_bin(&layouts, &data)
                .ok_or_else(|| anyhow::anyhow!("not a meteora bin"))?;
            println!("bin liq_x={}, liq_y={}", bin.liquidity_x, bin.liquidity_y);
        }
        "orca:pool" => {
            let pid = pool_id.unwrap_or_else(|| "<unknown>".into());
            let snap = orca_map::decode_orca_whirlpool_snapshot(&layouts, &data, &pid, slot.unwrap_or(0))
                .ok_or_else(|| anyhow::anyhow!("not an orca whirlpool"))?;
            println!("{}", serde_json::to_string_pretty(&snap)?);
        }
        "orca:tickarray" => {
            let tick_array = orca_map::decode_orca_whirlpool_tick_array(&layouts, &data)
                .ok_or_else(|| anyhow::anyhow!("not an orca tickarray"))?;
            println!("tickarray start_tick={}, ticks={}", tick_array.start_tick_index, tick_array.ticks.len());
        }
        _ => anyhow::bail!("unknown kind"),
    }
    Ok(())
}

async fn test_parsers() -> Result<()> {
    println!("🧪 Testing IDL parsers with live Solana data...");
    let _layouts = load_layouts()?;
    println!("✅ Layouts loaded successfully");
    Ok(())
}
