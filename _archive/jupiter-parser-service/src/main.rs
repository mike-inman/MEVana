//! Jupiter Parser Service
//!
//! Isolated microservice for parsing Jupiter V6 aggregator transactions
//! Processes ONLY Jupiter transactions with complete independence

use anyhow::Result;
use axum::{extract::State, response::Json, routing::get, Router};
use clap::Parser;
use prost::Message as ProstMessage;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
// Generated decoders (curated from IDLs)
// use normalizer_generated as generated;

// Jupiter program IDs we support
const JUPITER_AGGREGATOR_V4: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";
const JUPITER_AGGREGATOR_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const JUPITER_LIMIT_V1: &str = "jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZzyknqvzYNrNu";
const JUPITER_LIMIT_V2: &str = "j1o2qRpjcyUwEvwtcfhEQefh773ZgjxcVRry7LDqg5X";

#[derive(Parser)]
#[command(name = "jupiter-parser-service", version = "0.1.0")]
struct Args {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:29092")]
    kafka_brokers: String,

    #[arg(long, env = "KAFKA_TOPIC", default_value = "dex-transactions")]
    kafka_topic: String,

    #[arg(long, env = "REDIS_URL", default_value = "redis://localhost:6379")]
    redis_url: String,

    #[arg(long, env = "SERVICE_PORT", default_value = "8086")]
    service_port: u16,

    #[arg(long, env = "BIND_ADDRESS", default_value = "0.0.0.0")]
    bind_address: String,

    #[arg(long, env = "JUPITER_OUT_TOPIC", default_value = "jupiter.parsed")]
    out_topic_combined: String,

    #[arg(
        long,
        env = "JUPITER_OUT_SWAP_TOPIC",
        default_value = "jupiter.swap.parsed"
    )]
    out_topic_swap: String,

    #[arg(
        long,
        env = "JUPITER_OUT_LIMIT_TOPIC",
        default_value = "jupiter.limit.parsed"
    )]
    out_topic_limit: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct JupiterStats {
    total_transactions: u64,
    jupiter_transactions: u64,
    successful_parses: u64,
    failed_parses: u64,
    produced_messages: u64,
    instruction_types: HashMap<String, u64>,
    last_transaction_time: Option<std::time::SystemTime>,
    average_latency_ms: f64,
}

#[derive(Clone)]
struct AppState {
    stats: Arc<RwLock<JupiterStats>>,
    redis_client: redis::Client,
    idl_map: Arc<HashMap<String, HashMap<[u8; 8], String>>>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    program_ids: Vec<String>,
    uptime_seconds: u64,
    stats: JupiterStats,
}

#[derive(Serialize)]
struct MetricsResponse {
    jupiter_transactions_total: u64,
    jupiter_successful_parses_total: u64,
    jupiter_failed_parses_total: u64,
    jupiter_produced_messages_total: u64,
    jupiter_parse_latency_ms: f64,
    jupiter_instruction_types: HashMap<String, u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment and logging
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("🚀 Starting Jupiter Parser Service");
    info!(
        "Target Program IDs: {}, {}, {}, {}",
        JUPITER_AGGREGATOR_V4, JUPITER_AGGREGATOR_V6, JUPITER_LIMIT_V1, JUPITER_LIMIT_V2
    );
    info!("Kafka: {} -> {}", args.kafka_brokers, args.kafka_topic);
    info!("Service Port: {}", args.service_port);

    // Initialize Redis connection
    let redis_client = redis::Client::open(args.redis_url.as_str())?;

    // Test Redis connection
    {
        let mut conn = redis_client.get_multiplexed_tokio_connection().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        info!("✅ Redis connection verified");
    }

    // Initialize shared state
    // Load IDL discriminators for Jupiter programs
    let idl_map = Arc::new(load_idl_discriminators()?);

    let app_state = AppState {
        stats: Arc::new(RwLock::new(JupiterStats::default())),
        redis_client,
        idl_map,
    };

    // Start health/metrics server
    let health_server = start_health_server(
        app_state.clone(),
        args.bind_address.clone(),
        args.service_port,
    );

    // Start Jupiter transaction processor
    let processor = start_jupiter_processor(
        app_state.clone(),
        args.kafka_brokers.clone(),
        args.kafka_topic.clone(),
        args.out_topic_combined.clone(),
        args.out_topic_swap.clone(),
        args.out_topic_limit.clone(),
    );

    // Run both tasks concurrently
    tokio::select! {
        result = health_server => {
            error!("Health server failed: {:?}", result);
        }
        result = processor => {
            error!("Jupiter processor failed: {:?}", result);
        }
    }

    Ok(())
}

async fn start_health_server(state: AppState, bind_address: String, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", bind_address, port)).await?;
    info!("🏥 Health server listening on {}:{}", bind_address, port);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let stats = state.stats.read().await.clone();

    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "jupiter-parser-service".to_string(),
        program_ids: vec![
            JUPITER_AGGREGATOR_V4.to_string(),
            JUPITER_AGGREGATOR_V6.to_string(),
            JUPITER_LIMIT_V1.to_string(),
            JUPITER_LIMIT_V2.to_string(),
        ],
        uptime_seconds: 0, // TODO: Track actual uptime
        stats,
    })
}

async fn metrics_handler(State(state): State<AppState>) -> Json<MetricsResponse> {
    let stats = state.stats.read().await;

    Json(MetricsResponse {
        jupiter_transactions_total: stats.jupiter_transactions,
        jupiter_successful_parses_total: stats.successful_parses,
        jupiter_failed_parses_total: stats.failed_parses,
        jupiter_produced_messages_total: stats.produced_messages,
        jupiter_parse_latency_ms: stats.average_latency_ms,
        jupiter_instruction_types: stats.instruction_types.clone(),
    })
}

async fn start_jupiter_processor(
    state: AppState,
    kafka_brokers: String,
    kafka_topic: String,
    out_topic_combined: String,
    out_topic_swap: String,
    out_topic_limit: String,
) -> Result<()> {
    // Create Kafka consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_brokers)
        .set("group.id", "jupiter-parser-service")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest")
        .create()?;

    consumer.subscribe(&[&kafka_topic])?;
    info!("✅ Connected to Kafka topic: {}", kafka_topic);

    // Create Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_brokers)
        .create()?;

    // Periodic heartbeat for operator feedback
    {
        let stats_ref = state.stats.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let snapshot = stats_ref.read().await.clone();
                let last_age = snapshot
                    .last_transaction_time
                    .and_then(|t| t.elapsed().ok())
                    .map(|d| format!("{}s ago", d.as_secs()))
                    .unwrap_or_else(|| "never".to_string());
                info!(
                    "📈 Jupiter parser heartbeat: total={}, jupiter={}, produced={}, avg_ms={:.2}, last_seen={}",
                    snapshot.total_transactions,
                    snapshot.jupiter_transactions,
                    snapshot.produced_messages,
                    snapshot.average_latency_ms,
                    last_age
                );
            }
        });
    }

    loop {
        match consumer.recv().await {
            Ok(message) => {
                if let Some(payload) = message.payload() {
                    let mut stats = state.stats.write().await;
                    stats.total_transactions += 1;

                    if let Err(e) = process_transaction(
                        payload,
                        &mut stats,
                        &state.redis_client,
                        &producer,
                        &out_topic_combined,
                        &out_topic_swap,
                        &out_topic_limit,
                        &state.idl_map,
                    )
                    .await
                    {
                        warn!("Failed to process transaction: {}", e);
                        stats.failed_parses += 1;
                    }
                }
            }
            Err(e) => {
                error!("Kafka error: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_transaction(
    payload: &[u8],
    stats: &mut JupiterStats,
    redis_client: &redis::Client,
    producer: &FutureProducer,
    out_topic_combined: &str,
    out_topic_swap: &str,
    out_topic_limit: &str,
    idl_map: &Arc<HashMap<String, HashMap<[u8; 8], String>>>,
) -> Result<()> {
    let parse_start = Instant::now();

    // Decode yellowstone transaction
    let transaction_data =
        yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction::decode(payload)?;

    let Some(info) = transaction_data.transaction.as_ref() else {
        return Ok(());
    };

    let Some(transaction) = info.transaction.as_ref() else {
        return Ok(());
    };

    let Some(message) = transaction.message.as_ref() else {
        return Ok(());
    };

    // Build account keys
    let account_keys: Vec<String> = message
        .account_keys
        .iter()
        .map(|key| bs58::encode(key).into_string())
        .collect();

    // Capture signature once for enrichment
    let signature = if let Some(info) = transaction_data.transaction.as_ref() {
        bs58::encode(&info.signature).into_string()
    } else {
        String::new()
    };

    // Process each instruction - look for Jupiter
    for (ix_index, instruction) in message.instructions.iter().enumerate() {
        let program_id_index = instruction.program_id_index as usize;

        if let Some(program_id) = account_keys.get(program_id_index) {
            // Process only Jupiter program IDs
            let is_jupiter = matches!(
                program_id.as_str(),
                JUPITER_AGGREGATOR_V4 | JUPITER_AGGREGATOR_V6 | JUPITER_LIMIT_V1 | JUPITER_LIMIT_V2
            );
            if !is_jupiter {
                continue;
            }

            stats.jupiter_transactions += 1;

            // Decode via runtime IDL (discriminator → instruction name)
            let mut instruction_name = "unknown".to_string();
            let roles: HashMap<String, String> = HashMap::new();
            if instruction.data.len() >= 8 {
                let mut disc: [u8; 8] = [0u8; 8];
                disc.copy_from_slice(&instruction.data[..8]);
                if let Some(map) = idl_map.get(program_id) {
                    if let Some(name) = map.get(&disc) {
                        instruction_name = name.clone();
                    }
                }
            }

            let parse_time = parse_start.elapsed();

            // Update stats
            stats.successful_parses += 1;
            *stats
                .instruction_types
                .entry(instruction_name.clone())
                .or_insert(0) += 1;
            stats.average_latency_ms = parse_time.as_secs_f64() * 1000.0;
            stats.last_transaction_time = Some(std::time::SystemTime::now());

            // Store to Redis
            if let Err(e) = store_jupiter_transaction(
                redis_client,
                &transaction_data,
                ix_index,
                &instruction_name,
                &roles,
                program_id,
            )
            .await
            {
                warn!("Failed to store to Redis: {}", e);
            }

            // Prepare instruction accounts summary
            let ix_accounts: Vec<serde_json::Value> = instruction
                .accounts
                .iter()
                .map(|i| *i as usize)
                .filter_map(|idx| account_keys.get(idx).cloned().map(|k| (idx, k)))
                .map(|(idx, key)| serde_json::json!({ "index": idx, "key": key }))
                .collect();

            // Produce to Kafka topics (enriched)
            let payload_json = serde_json::json!({
                "slot": transaction_data.slot,
                "signature": signature,
                "ix_index": ix_index,
                "instruction_name": instruction_name,
                "program_id": program_id,
                "roles": roles,
                "accounts": ix_accounts,
            })
            .to_string();

            let result_combined = producer
                .send(
                    FutureRecord::<(), _>::to(out_topic_combined).payload(&payload_json),
                    Duration::from_secs(5),
                )
                .await;

            let topic = if matches!(program_id.as_str(), JUPITER_LIMIT_V1 | JUPITER_LIMIT_V2) {
                out_topic_limit
            } else {
                out_topic_swap
            };

            let result_specific = producer
                .send(
                    FutureRecord::<(), _>::to(topic).payload(&payload_json),
                    Duration::from_secs(5),
                )
                .await;

            // Count only successful deliveries
            if result_combined.is_ok() {
                stats.produced_messages += 1;
            }
            if result_specific.is_ok() {
                stats.produced_messages += 1;
            }
        }
    }

    Ok(())
}

async fn store_jupiter_transaction(
    redis_client: &redis::Client,
    transaction_data: &yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
    ix_index: usize,
    instruction_name: &str,
    roles: &HashMap<String, String>,
    program_id: &str,
) -> Result<()> {
    let mut conn = redis_client.get_multiplexed_tokio_connection().await?;

    let signature = if let Some(info) = transaction_data.transaction.as_ref() {
        bs58::encode(&info.signature).into_string()
    } else {
        return Ok(());
    };

    let jupiter_key = format!("jupiter:transaction:{}", signature);

    // Store Jupiter transaction data
    let jupiter_data = serde_json::json!({
        "signature": signature,
        "slot": transaction_data.slot,
        "ix_index": ix_index,
        "instruction_name": instruction_name,
        "program_id": program_id,
        "roles": roles,
        "parsed_at": chrono::Utc::now().timestamp(),
        "parser_service": "jupiter-parser-service"
    });

    let _: () = redis::cmd("HSET")
        .arg(&jupiter_key)
        .arg("data")
        .arg(jupiter_data.to_string())
        .query_async(&mut conn)
        .await?;

    // Set expiration (24 hours)
    let _: () = redis::cmd("EXPIRE")
        .arg(&jupiter_key)
        .arg(86400)
        .query_async(&mut conn)
        .await?;

    debug!("💾 Stored Jupiter transaction: {}", signature);

    Ok(())
}

fn load_idl_discriminators() -> anyhow::Result<HashMap<String, HashMap<[u8; 8], String>>> {
    use std::fs;
    use std::path::PathBuf;

    let idl_dir = std::env::var("IDL_DIR").unwrap_or_else(|_| "/app/idls".to_string());
    let mut result: HashMap<String, HashMap<[u8; 8], String>> = HashMap::new();

    // Map program_id to expected IDL file names present in idls/
    let targets: Vec<(&str, &str)> = vec![
        (JUPITER_AGGREGATOR_V4, "jupiter_aggregator_v4.json"),
        (JUPITER_AGGREGATOR_V6, "jupiter_aggregator_v6.json"),
        (JUPITER_LIMIT_V1, "jupiter_limit_order.json"),
        (JUPITER_LIMIT_V2, "jupiter_limit_order_v2.json"),
    ];

    for (program_id, file_name) in targets {
        let mut path = PathBuf::from(&idl_dir);
        path.push(file_name);
        if !path.exists() {
            continue;
        }
        let txt = fs::read_to_string(&path)?;
        let idl: serde_json::Value = serde_json::from_str(&txt)?;
        let mut map: HashMap<[u8; 8], String> = HashMap::new();

        if let Some(ixs) = idl.get("instructions").and_then(|v| v.as_array()) {
            for ix in ixs {
                let name = ix.get("name").and_then(|v| v.as_str()).unwrap_or("");
                // Anchor discriminator = first 8 bytes of sha256("global:<name>")
                let mut hasher = Sha256::new();
                hasher.update(format!("global:{}", name).as_bytes());
                let digest = hasher.finalize();
                let mut disc: [u8; 8] = [0u8; 8];
                disc.copy_from_slice(&digest[..8]);
                map.insert(disc, name.to_string());
            }
        }

        result.insert(program_id.to_string(), map);
    }

    Ok(result)
}
