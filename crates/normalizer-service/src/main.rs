#![allow(warnings)]

// crates/normalizer-service/src/main.rs
mod kafka;
mod meteora_map; // meteora parsers
mod orca_map; // orca whirlpool parsers
mod raydium_map; // raydium parsers // <- see notes at bottom for kafka.rs

// use anyhow::Result;
use crate::kafka::publish_json_ff;
//use clap::Parser;
use lazy_static::lazy_static;
//use normalizer_generated::registry as gen;
//use normalizer_runtime as rt;
use normalizer_runtime::layouts::LayoutRegistry;
use normalizer_runtime::models::SwapEvent;
use serde::Deserialize;
use std::{fs, path::PathBuf};
// use kafka::{make_producer, publish_json};
use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use prometheus::{Counter, Encoder, TextEncoder};

use clap::{Parser, Subcommand};
use normalizer_generated::registry as gen;
use normalizer_runtime as rt;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use yellowstone_grpc_proto::prelude as ypp;
use yellowstone_grpc_proto::prost::Message as _;

// Removed unused imports
// type SlotTimeCache = Arc<Mutex<LruCache<u64, i64>>>;

// static WARN_MISSING_BLOCKTIME: Once = Once::new();

// ---- CLI ----
#[derive(Parser)]
#[command(name = "normalizer-service")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

// ==== args ====
#[derive(Parser, Debug, Clone)]
struct Args {
    /// If set, skip events that don't have a block_time yet
    #[arg(long, default_value_t = false)]
    require_block_time: bool,

    /// Kafka broker (env still supported)
    #[arg(long, env = "KAFKA_BROKER", default_value = "localhost:9092")]
    kafka_broker: String,

    /// Output topic for normalized swaps
    #[arg(long, env = "SWAP_TOPIC", default_value = "enriched.swaps.all")]
    swap_topic: String,
}

// Warn-once helper when block_time_unix==0
use std::sync::atomic::{AtomicBool, Ordering};
static WARNED_NO_BT: AtomicBool = AtomicBool::new(false);
fn warn_no_block_time_once() {
    if !WARNED_NO_BT.swap(true, Ordering::Relaxed) {
        log::warn!(
            "Yellowstone message has no block_time; using 0. Wire a block-stream/RPC later."
        );
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a local JSON fixture through the extractors
    Fixture { path: std::path::PathBuf },

    /// Read Yellowstone tx from Kafka and push normalized swaps to another topic
    Pipe {
        #[arg(long, env = "IN_TOPIC")]
        in_topic: String,
        #[arg(long, env = "OUT_TOPIC", default_value = "normalized.swaps")]
        out_topic: String,
        #[arg(long, env = "KAFKA_BROKER", default_value = "localhost:9092")]
        broker: String,
        #[arg(long, env = "GROUP_ID", default_value = "normalizer-pipe")]
        group: String,
    },

    /// Tail a topic and print messages
    Tail {
        #[arg(long, env = "TOPIC")]
        topic: String,
        #[arg(long, env = "KAFKA_BROKER", default_value = "localhost:9092")]
        broker: String,
        #[arg(long, env = "GROUP_ID", default_value = "normalizer-tail")]
        group: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Fixture { path } => run_fixture(path), // your existing function
        Cmd::Pipe {
            in_topic,
            out_topic,
            broker,
            group,
        } => run_pipe(&broker, &group, &in_topic, &out_topic).await,
        Cmd::Tail {
            topic,
            broker,
            group,
        } => run_tail(&broker, &group, &topic).await,
    }
}

// ---- PIPE: raw Yellowstone -> normalized.swaps ----
async fn run_pipe(
    broker: &str,
    group: &str,
    in_topic: &str,
    out_topic: &str,
) -> anyhow::Result<()> {
    env_logger::init();
    log::info!("pipe: {in_topic} -> {out_topic} @ {broker} (group={group})");

    // Register metrics
    let registry = prometheus::default_registry();
    registry
        .register(Box::new(TRANSACTIONS_PROCESSED.clone()))
        .unwrap();
    registry
        .register(Box::new(SWAPS_EXTRACTED.clone()))
        .unwrap();

    // Start metrics server
    tokio::spawn(async move {
        use warp::Filter;
        let metrics_route = warp::path("metrics").and(warp::get()).map(|| {
            let encoder = TextEncoder::new();
            let metric_families = prometheus::gather();
            let mut buffer = Vec::new();
            if encoder.encode(&metric_families, &mut buffer).is_ok() {
                warp::reply::with_status(
                    String::from_utf8_lossy(&buffer).to_string(),
                    warp::http::StatusCode::OK,
                )
            } else {
                warp::reply::with_status(
                    "encode_error".into(),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        });
        let health = warp::path("health").and(warp::get()).map(|| "OK");
        let routes = metrics_route.or(health);
        let port: u16 = std::env::var("METRICS_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(9901);
        let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        log::info!("normalizer-service metrics listening on {}", addr);
        warp::serve(routes).run(addr).await;
    });

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("group.id", group)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()?;
    consumer.subscribe(&[in_topic])?;

    // let producer = crate::kafka::make_producer(broker)?;
    let producer = crate::kafka::make_producer()?;
    let extractors: Vec<Box<dyn rt::SwapExtractor>> = gen::registry();

    loop {
        match consumer.recv().await {
            Err(e) => log::warn!("kafka recv error: {e:?}"),
            Ok(m) => {
                if let Some(bytes) = m.payload() {
                    // handle_raw(payload, &extractors, &producer, &SWAP_TOPIC, slot_time.as_ref()).await?;

                    if let Err(e) = handle_raw(bytes, &extractors, &producer, out_topic).await {
                        log::error!("handle_raw error: {e:#}");
                    }
                }
                let _ = consumer.commit_message(&m, rdkafka::consumer::CommitMode::Async);
            }
        }
    }
}

async fn handle_raw(
    payload: &[u8],
    extractors: &[Box<dyn rt::SwapExtractor>],
    producer: &rdkafka::producer::FutureProducer,
    out_topic: &str,
) -> anyhow::Result<()> {
    TRANSACTIONS_PROCESSED.inc();
    let update = ypp::SubscribeUpdateTransaction::decode(payload)?;
    let info = match update.transaction.as_ref() {
        Some(i) => i,
        None => return Ok(()),
    };

    let swaps = normalizer_runtime::transaction::extract_swaps_from_update(
        &update,
        extractors,
        &gen::decoder_for,
        None,
    )?;

    warn_no_block_time_once();

    for swap in swaps {
        SWAPS_EXTRACTED.inc();
        let target_topic = resolve_swap_topic(out_topic, &swap.dex);
        crate::kafka::publish_json_ff(
            producer.clone(),
            target_topic,
            swap.pool_id.clone(),
            swap.clone(),
        );
        log::info!(
            "→ normalized: {} {} -> {} (slot {})",
            swap.dex,
            swap.token_in_mint,
            swap.token_out_mint,
            swap.slot
        );
    }

    Ok(())
}

// ---- TAIL: print a topic ----
async fn run_tail(broker: &str, group: &str, topic: &str) -> anyhow::Result<()> {
    env_logger::init();
    let c: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("group.id", group)
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest")
        .create()?;
    c.subscribe(&[topic])?;
    log::info!("tailing topic {topic}");
    loop {
        let m = c.recv().await?;
        if let Some(p) = m.payload() {
            println!("{}", String::from_utf8_lossy(p));
        }
    }
}

// ---------------- env / topics ----------------

lazy_static! {
    static ref KAFKA_BROKER: String =
        std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    static ref SWAP_TOPIC: String =
        std::env::var("NORMALIZED_SWAPS_TOPIC").unwrap_or_else(|_| "normalized-swaps.v1".into());
    static ref TRANSACTIONS_PROCESSED: Counter = Counter::new(
        "normalizer_transactions_processed",
        "Total transactions processed by normalizer"
    )
    .unwrap();
    static ref SWAPS_EXTRACTED: Counter = Counter::new(
        "normalizer_swaps_extracted",
        "Total swaps extracted by normalizer"
    )
    .unwrap();
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

fn resolve_swap_topic(default_topic: &str, dex: &str) -> String {
    match dex {
        "whirlpool" | "orca_whirlpool" => "orca-swaps.parsed.v2".to_string(),
        "raydium_clmm" => "raydium-swaps.parsed".to_string(),
        "raydium_amm" => "raydium-swaps.parsed".to_string(),
        "meteora_dlmm" => "meteora-swaps.parsed".to_string(),
        "meteora_damm" => "meteora-swaps.parsed".to_string(),
        "phoenix" => "phoenix-swaps.parsed".to_string(),
        _ => default_topic.to_string(),
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
        (
            "raydium_clmm/tickarray",
            "raydium_clmm/layout.tickarray.v1.json",
        ),
        ("meteora_dlmm/pool", "meteora_dlmm/layout.pool.v1.json"),
        ("meteora_dlmm/bin", "meteora_dlmm/layout.bin.v1.json"),
        ("orca_whirlpool/pool", "orca_whirlpool/layout.pool.v1.json"),
        (
            "orca_whirlpool/tickarray",
            "orca_whirlpool/layout.tickarray.v1.json",
        ),
    ];

    for (key, filename) in files {
        match reg.load_file(key, &root.join(filename)) {
            Ok(_) => println!("✅ Loaded {}", key),
            Err(e) => println!("❌ Failed to load {}: {}", key, e),
        }
    }

    Ok(reg)
}

// // ---------------- CLI ----------------

// #[derive(Parser, Debug)]
// #[command(name = "normalizer-service", version)]
// enum Cmd {
//     Fixture {
//         path: PathBuf,
//     }, // one instruction fixture
//     AccountFixture {
//         path: PathBuf, // base64 account.data
//         kind: String, // raydium:tickarray | meteora:pool | meteora:bin | orca:pool | orca:tickarray
//         pool_id: Option<String>,
//         slot: Option<u64>,
//     },
//     TestParsers,
// }

// // NOTE: use Tokio runtime because we send to Kafka asynchronously
// #[tokio::main]
// async fn main() -> Result<()> {
//     let cmd = Cmd::parse();
//     match cmd {
//         Cmd::Fixture { path } => run_fixture(path),
//         Cmd::AccountFixture {
//             path,
//             kind,
//             pool_id,
//             slot,
//         } => run_account_fixture(path, kind, pool_id, slot).await,
//         Cmd::TestParsers => test_parsers().await,
//     }
// }

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
    use base64::{engine::general_purpose, Engine as _};
    use rdkafka::producer::Producer; // for flush()

    // 1) load fixture
    let txt = fs::read_to_string(&path)?;
    let fx: Fixture = serde_json::from_str(&txt)?;

    // 2) ix data bytes
    let ix_bytes = general_purpose::STANDARD
        .decode(&fx.instruction.data_b64)
        .context("decode fx.instruction.data_b64 (base64)")?;

    // 3) token balance deltas
    let token_changes: Vec<rt::TokenBalanceChange> = fx
        .token_balance_changes
        .iter()
        .map(|c| rt::TokenBalanceChange {
            owner: c.owner.clone(),
            mint: c.mint.clone(),
            pre: parse_u128(&c.pre).unwrap_or(0),
            post: parse_u128(&c.post).unwrap_or(0),
        })
        .collect();

    // 4) optional: decode name/roles via generated registry
    let (decoded_name, decoded_roles) =
        if let Some(dec) = gen::decoder_for(&fx.instruction.program_id) {
            // NOTE: decoder currently takes &[u8] for account indices
            if let Some((name, roles)) = dec(
                &ix_bytes,
                &fx.instruction.account_indices,
                &fx.instruction.account_keys,
            ) {
                (Some(name), Some(roles))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

    // 5) build TxContext
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

    // 6) run extractors & publish
    let extractors: Vec<Box<dyn rt::SwapExtractor>> = gen::registry();
    let producer = crate::kafka::make_producer()?; // <- no arg
    let topic = crate::kafka::SWAP_TOPIC.clone();
    let mut hits = 0usize;

    for ex in extractors
        .into_iter()
        .filter(|e| e.program_id() == fx.instruction.program_id)
    {
        if let Some(mut ns) = ex.try_extract(&ctx) {
            hits += 1;
            println!(
                "✅ {} extracted: {} -> {} (slot {})",
                ns.dex, ns.token_in_mint, ns.token_out_mint, ns.slot
            );

            // Fill pool_id if extractor didn’t
            if ns.pool_id.is_empty() {
                if let Some(i) = fx.instruction.account_indices.first() {
                    if let Some(k) = fx.instruction.account_keys.get(*i as usize) {
                        ns.pool_id = k.clone();
                    }
                } else if let Some(k) = fx.instruction.account_keys.first() {
                    ns.pool_id = k.clone();
                }
            }

            let swap = normalizer_runtime::adapters::to_swap_event(&ns);
            let resolved_topic = resolve_swap_topic(&topic, &swap.dex);
            crate::kafka::publish_json_ff(
                producer.clone(),
                resolved_topic,
                swap.pool_id.clone(),
                swap.clone(),
            );
            println!("✅ Published to Kafka: {}", swap.pool_id);
            println!("{}", serde_json::to_string(&swap)?);
        }
    }

    // ensure delivery before exit
    let _ = producer.flush(std::time::Duration::from_secs(2));

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
    let data = BASE64_STANDARD.decode(b64.trim())?;

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
            let snap =
                meteora_map::decode_meteora_pool_snapshot(&layouts, &data, &pid, slot.unwrap_or(0))
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
            let snap =
                orca_map::decode_orca_whirlpool_snapshot(&layouts, &data, &pid, slot.unwrap_or(0))
                    .ok_or_else(|| anyhow::anyhow!("not an orca whirlpool"))?;
            println!("{}", serde_json::to_string_pretty(&snap)?);
        }
        "orca:tickarray" => {
            let tick_array = orca_map::decode_orca_whirlpool_tick_array(&layouts, &data)
                .ok_or_else(|| anyhow::anyhow!("not an orca tickarray"))?;
            println!(
                "tickarray start_tick={}, ticks={}",
                tick_array.start_tick_index,
                tick_array.ticks.len()
            );
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
