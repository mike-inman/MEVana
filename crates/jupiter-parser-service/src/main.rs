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
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

// Jupiter V6 Program ID - ONLY program we process
const JUPITER_PROGRAM_ID: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

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
}

#[derive(Debug, Clone, Default)]
struct JupiterStats {
    total_transactions: u64,
    jupiter_transactions: u64,
    successful_parses: u64,
    failed_parses: u64,
    instruction_types: HashMap<String, u64>,
    last_transaction_time: Option<std::time::SystemTime>,
    average_latency_ms: f64,
}

#[derive(Clone)]
struct AppState {
    stats: Arc<RwLock<JupiterStats>>,
    redis_client: redis::Client,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    program_id: String,
    uptime_seconds: u64,
    stats: JupiterStats,
}

#[derive(Serialize)]
struct MetricsResponse {
    jupiter_transactions_total: u64,
    jupiter_successful_parses_total: u64,
    jupiter_failed_parses_total: u64,
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
    info!("Target Program ID: {}", JUPITER_PROGRAM_ID);
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
    let app_state = AppState {
        stats: Arc::new(RwLock::new(JupiterStats::default())),
        redis_client,
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
        program_id: JUPITER_PROGRAM_ID.to_string(),
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
        jupiter_parse_latency_ms: stats.average_latency_ms,
        jupiter_instruction_types: stats.instruction_types.clone(),
    })
}

async fn start_jupiter_processor(
    state: AppState,
    kafka_brokers: String,
    kafka_topic: String,
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

    loop {
        match consumer.recv().await {
            Ok(message) => {
                if let Some(payload) = message.payload() {
                    let mut stats = state.stats.write().await;
                    stats.total_transactions += 1;

                    if let Err(e) =
                        process_transaction(payload, &mut stats, &state.redis_client).await
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

    // Process each instruction - look for Jupiter
    for (ix_index, instruction) in message.instructions.iter().enumerate() {
        let program_id_index = instruction.program_id_index as usize;

        if let Some(program_id) = account_keys.get(program_id_index) {
            // ONLY process Jupiter transactions
            if program_id != JUPITER_PROGRAM_ID {
                continue;
            }

            stats.jupiter_transactions += 1;

            // Use Jupiter parser from normalizer-generated
            if let Some(decoder) = normalizer_generated::decoder_for(program_id) {
                if let Some((instruction_name, roles)) =
                    decoder(&instruction.data, &instruction.accounts, &account_keys)
                {
                    let parse_time = parse_start.elapsed();

                    // Update stats
                    stats.successful_parses += 1;
                    *stats
                        .instruction_types
                        .entry(instruction_name.clone())
                        .or_insert(0) += 1;
                    stats.average_latency_ms = parse_time.as_secs_f64() * 1000.0;
                    stats.last_transaction_time = Some(std::time::SystemTime::now());

                    // Store parsed data to Redis
                    if let Err(e) = store_jupiter_transaction(
                        redis_client,
                        &transaction_data,
                        ix_index,
                        &instruction_name,
                        &roles,
                    )
                    .await
                    {
                        warn!("Failed to store to Redis: {}", e);
                    }

                    info!(
                        "✅ Parsed Jupiter {} instruction in {:.2}ms",
                        instruction_name,
                        parse_time.as_secs_f64() * 1000.0
                    );
                } else {
                    stats.failed_parses += 1;
                    debug!(
                        "❌ Failed to decode Jupiter instruction at index {}",
                        ix_index
                    );
                }
            } else {
                stats.failed_parses += 1;
                debug!("❌ No decoder found for Jupiter program ID");
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
        "program_id": JUPITER_PROGRAM_ID,
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
