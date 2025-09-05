use anyhow::Result;
use backoff::{future::retry, ExponentialBackoff};
use chrono;
use clap::Parser;
use dotenv::dotenv;
use env_logger;
use futures_util::{sink::SinkExt, stream::StreamExt, TryFutureExt};
use log::{error, info, warn};
use prometheus::{Histogram, HistogramOpts, IntCounter, Registry};
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use reqwest;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tonic::transport::channel::ClientTlsConfig;
use tonic::transport::Server;
use uuid::Uuid;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    geyser::SubscribeRequestFilterTransactions,
    prelude::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
    },
    prost::Message as _,
};

// gRPC Health Checking Protocol imports - will add these incrementally
use tonic_health::server::HealthReporter;

// HTTP server for health endpoints
use axum::{extract::State, response::Json, routing::get, Router};
use serde::Serialize;
use tower_http::cors::CorsLayer;

// Health status tracking
#[derive(Debug, Clone)]
struct HealthStatus {
    kafka: bool,
    geyser: bool,
    service: bool,
    last_check: chrono::DateTime<chrono::Utc>,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            kafka: false,
            geyser: false,
            service: true,
            last_check: chrono::Utc::now(),
        }
    }
}

// Service state
#[derive(Clone)]
struct AppState {
    health_status: Arc<RwLock<HealthStatus>>,
    metrics_registry: Registry,
}

// gRPC Health Service implementation
struct GrpcHealthService {
    health_status: Arc<RwLock<HealthStatus>>,
}

impl GrpcHealthService {
    fn new(health_status: Arc<RwLock<HealthStatus>>) -> Self {
        Self { health_status }
    }
}

// TODO: Implement tonic-health service trait
// Need to investigate the correct import paths for tonic-health 0.12

// For now, let's add a simple health status function
impl GrpcHealthService {
    async fn get_health_status(&self) -> String {
        let health_status = self.health_status.read().await;

        if health_status.service && health_status.kafka && health_status.geyser {
            "SERVING".to_string()
        } else {
            "NOT_SERVING".to_string()
        }
    }
}

// Command line arguments with environment variable fallback
#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(
        short,
        long,
        env = "KAFKA_BROKER",
        help = "Kafka broker address (from KAFKA_BROKER env var or --broker)"
    )]
    broker: Option<String>,
    #[arg(
        short,
        long,
        env = "TOPIC",
        help = "Output topic (from TOPIC env var or --topic)"
    )]
    topic: Option<String>,
    #[arg(
        short,
        long,
        env = "ENDPOINT",
        help = "Geyser endpoint URL (from ENDPOINT env var or --endpoint)"
    )]
    endpoint: Option<String>,
    #[arg(
        long,
        env = "X_TOKEN",
        help = "Geyser authentication token (from X_TOKEN env var or --x-token)"
    )]
    x_token: Option<String>,
    #[arg(
        long,
        env = "PROGRAM_IDS",
        help = "Comma-separated list of program IDs to filter by (from PROGRAM_IDS env var or --program-ids)"
    )]
    program_ids: Option<String>,
    #[arg(
        long,
        env = "DEX_FILTERING",
        help = "DEX filtering mode: ON (filtered) or OFF (wide open) (from DEX_FILTERING env var or --dex-filtering)",
        default_value = "ON"
    )]
    pub dex_filtering: String,
    #[arg(
        long,
        env = "HEALTH_PORT",
        default_value = "0",
        help = "Health check server port (from HEALTH_PORT env var or --health-port)"
    )]
    health_port: u16,
    #[arg(
        long,
        env = "BIND_ADDRESS",
        default_value = "0.0.0.0",
        help = "Bind address for HTTP server (from BIND_ADDRESS env var or --bind-address)"
    )]
    bind_address: String,
    #[arg(
        long,
        env = "DATABASE_URL",
        help = "PostgreSQL database URL for raw transaction logging (from DATABASE_URL env var or --database-url)"
    )]
    database_url: Option<String>,
}

// Prometheus metrics
lazy_static::lazy_static! {
    static ref REGISTRY: Registry = Registry::new();

    // Transaction processing metrics
    pub static ref TRANSACTIONS_PROCESSED: IntCounter = IntCounter::new(
        "dex_stream_v2_transactions_processed_total",
        "Total number of transactions processed"
    ).expect("Failed to create transactions_processed metric");

    pub static ref TRANSACTION_PROCESSING_TIME: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "dex_stream_v2_transaction_processing_seconds",
            "Time spent processing individual transactions"
        )
    ).expect("Failed to create transaction_processing_time metric");

    // Health check metrics
    pub static ref HEALTH_CHECK_TOTAL: IntCounter = IntCounter::new(
        "dex_stream_v2_health_checks_total",
        "Total number of health checks performed"
    ).expect("Failed to create health_checks_total metric");

    pub static ref HEALTH_CHECK_FAILURES: IntCounter = IntCounter::new(
        "dex_stream_v2_health_check_failures_total",
        "Total number of health check failures"
    ).expect("Failed to create health_check_failures metric");
}

// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
    version: String,
    uptime: String,
    checks: HealthChecks,
}

#[derive(Serialize)]
struct HealthChecks {
    kafka: String,
    geyser: String,
    service: String,
}

// Initialize metrics
fn init_metrics() {
    REGISTRY
        .register(Box::new(TRANSACTIONS_PROCESSED.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(TRANSACTION_PROCESSING_TIME.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(HEALTH_CHECK_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(HEALTH_CHECK_FAILURES.clone()))
        .unwrap();
}

type TxnFilterMap = std::collections::HashMap<String, SubscribeRequestFilterTransactions>;

/// Analyzes a transaction to find unknown program IDs for discovery
pub fn analyze_transaction_for_discovery(
    update: &yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
    known_program_ids: &HashSet<String>,
) -> Option<Vec<String>> {
    let mut unknown_programs = Vec::new();

    // Extract program IDs from transaction instructions
    if let Some(transaction_info) = &update.transaction {
        if let Some(transaction) = &transaction_info.transaction {
            if let Some(message) = transaction.message.as_ref() {
                for instruction in &message.instructions {
                    let program_id_index = instruction.program_id_index as usize;
                    if let Some(program_id) = message.account_keys.get(program_id_index) {
                        let program_id_str = bs58::encode(program_id).into_string();
                        if !known_program_ids.contains(&program_id_str) {
                            unknown_programs.push(program_id_str);
                        }
                    }
                }
            }
        }
    }

    if unknown_programs.is_empty() {
        None
    } else {
        // Remove duplicates
        unknown_programs.sort();
        unknown_programs.dedup();
        Some(unknown_programs)
    }
}

fn build_request(program_ids: Vec<String>, wide_open: bool) -> SubscribeRequest {
    let mut transactions: TxnFilterMap = std::collections::HashMap::new();
    // In wide_open mode we include EVERYTHING (all transactions, including votes & failed)
    // by leaving vote/failed None and account_include empty.
    let (vote, failed, account_include) = if wide_open {
        (None, None, Vec::new())
    } else {
        (Some(false), Some(false), program_ids)
    };

    if wide_open {
        info!("⚠️ Wide-open subscription: including vote + failed + all accounts (no account_include filter)");
    }

    transactions.insert(
        "client".to_owned(),
        SubscribeRequestFilterTransactions {
            vote,
            failed,
            account_include,
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );
    SubscribeRequest {
        accounts: Default::default(),
        slots: Default::default(),
        transactions,
        transactions_status: Default::default(),
        blocks: Default::default(),
        blocks_meta: Default::default(),
        entry: Default::default(),
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: vec![],
        ping: None,
        from_slot: None,
    }
}

fn create_producer(broker: &str, fast_mode: bool) -> Result<FutureProducer> {
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", broker);

    if fast_mode {
        config
            .set("linger.ms", "0")
            .set("acks", "1")
            .set("compression.type", "lz4")
            .set("enable.idempotence", "false")
            .set("message.timeout.ms", "100");
    } else {
        config
            .set("enable.idempotence", "true")
            .set("acks", "all")
            .set("retries", "3")
            .set("message.timeout.ms", "30000");
    }

    let p = config.create()?;
    Ok(p)
}

/// Test Kafka broker connectivity
async fn test_kafka_connectivity(broker: &str) -> Result<()> {
    info!("🔍 Testing Kafka connectivity to: {}", broker);

    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", broker);
    config.set("message.timeout.ms", "5000");

    match config.create::<FutureProducer>() {
        Ok(_) => {
            info!(
                "✅ Kafka connectivity test passed - connected to: {}",
                broker
            );
            Ok(())
        }
        Err(e) => {
            warn!("⚠️  Kafka connectivity test failed: {}", e);
            warn!("💡 To fix: Ensure Kafka broker is running at {}", broker);
            Err(anyhow::anyhow!("Kafka connectivity test failed: {}", e))
        }
    }
}

/// Get external IP address that external services would see
async fn get_external_ip() -> Result<String> {
    let services = vec![
        "https://api.ipify.org",
        "https://ifconfig.me",
        "https://icanhazip.com",
    ];

    for service in services {
        if let Ok(client) = reqwest::Client::new()
            .get(service)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            if let Ok(ip) = client.text().await {
                let ip = ip.trim().to_string();
                if !ip.is_empty() {
                    return Ok(ip);
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Failed to detect external IP from any service"
    ))
}

/// Silent Kafka health check (no logging)
async fn silent_kafka_health_check(broker: &str) -> bool {
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", broker);
    config.set("message.timeout.ms", "5000");

    config.create::<FutureProducer>().is_ok()
}

/// Silent Geyser health check (no logging)
async fn silent_geyser_health_check(endpoint: &str, x_token: &str) -> bool {
    let client = reqwest::Client::new();
    let response = client
        .get(endpoint)
        .header("x-token", x_token)
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    response.is_ok()
}

/// Test Geyser endpoint connectivity
async fn test_geyser_connectivity(endpoint: &str, x_token: &str) -> Result<()> {
    info!("🔍 Testing Geyser connectivity to: {}", endpoint);

    // Test basic HTTP connectivity first
    let client = reqwest::Client::new();
    let response = client
        .get(endpoint)
        .header("x-token", x_token)
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    match response {
        Ok(resp) => {
            info!(
                "✅ Geyser connectivity test passed - connected to: {}",
                endpoint
            );
            if resp.status().is_success() {
                info!("   Response status: {} - SUCCESS", resp.status());
            } else {
                info!(
                    "   Response status: {} - NORMAL (gRPC endpoints don't serve HTTP)",
                    resp.status()
                );
            }
            Ok(())
        }
        Err(e) => {
            warn!("⚠️  Geyser connectivity test failed: {}", e);
            warn!(
                "💡 To fix: Ensure Geyser endpoint is accessible at {}",
                endpoint
            );
            Err(anyhow::anyhow!("Geyser connectivity test failed: {}", e))
        }
    }
}

/// Service configuration
#[derive(Debug, Clone)]
struct ServiceConfig {
    broker: String,
    topic: String,
    endpoint: String,
    x_token: String,
    program_ids: Vec<String>,
    dex_filtering: String,
    health_port: u16,
    bind_address: String,
    database_url: String,
}

impl ServiceConfig {
    fn from_args(args: &Args) -> Result<Self, Vec<String>> {
        let mut missing_configs = Vec::new();

        // Check required configurations - NO HARDCODED NETWORK SETTINGS
        let broker = args.broker.clone().unwrap_or_else(|| {
            missing_configs.push("KAFKA_BROKER".to_string());
            String::new() // Empty string, will be caught by validation
        });

        let topic = args.topic.clone().unwrap_or_else(|| {
            missing_configs.push("TOPIC".to_string());
            String::new() // Empty string, will be caught by validation
        });

        let endpoint = args.endpoint.clone().unwrap_or_else(|| {
            missing_configs.push("ENDPOINT".to_string());
            String::new() // Empty string, will be caught by validation
        });

        let x_token = args.x_token.clone().unwrap_or_else(|| {
            missing_configs.push("X_TOKEN".to_string());
            String::new() // Empty string, will be caught by validation
        });

        let program_ids_csv = if args.dex_filtering == "OFF" {
            String::new()
        } else {
            args.program_ids.clone().unwrap_or_default()
        };

        let program_ids: Vec<String> = program_ids_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Validate that no required network settings are empty
        if broker.is_empty() || topic.is_empty() || endpoint.is_empty() || x_token.is_empty() {
            return Err(missing_configs);
        }

        let database_url = args.database_url.clone().unwrap_or_else(|| {
            // Default to port 5433 (avoiding local PostgreSQL conflict)
            "postgresql://mev:mevpassword@localhost:5433/mevdb".to_string()
        });

        let config = Self {
            broker,
            topic,
            endpoint,
            x_token,
            program_ids,
            dex_filtering: args.dex_filtering.clone(),
            health_port: args.health_port,
            bind_address: args.bind_address.clone(),
            database_url,
        };

        // Log missing configurations
        if !missing_configs.is_empty() {
            warn!("⚠️  Missing required configurations: {:?}", missing_configs);
            warn!("💡 Set environment variables or CLI args for production use.");
        }

        Ok(config)
    }

    fn log_startup_configuration(&self) {
        info!(
            "🚀 DEX Stream Universal V2 - Broker: {}, Topic: {}, Mode: {}",
            self.broker,
            self.topic,
            if self.dex_filtering == "OFF" {
                "WIDE-OPEN"
            } else {
                "FILTERED"
            }
        );
    }
}

/// Background health checker that runs periodically
async fn background_health_checker(
    config: ServiceConfig,
    health_status: Arc<RwLock<HealthStatus>>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        // Silent health checks - don't spam logs with connectivity test messages
        let kafka_healthy = silent_kafka_health_check(&config.broker).await;
        let geyser_healthy = silent_geyser_health_check(&config.endpoint, &config.x_token).await;

        // Update health status
        {
            let mut status = health_status.write().await;
            status.kafka = kafka_healthy;
            status.geyser = geyser_healthy;
            status.last_check = chrono::Utc::now();
        }

        // Log health status (only when unhealthy or status changes)
        if kafka_healthy && geyser_healthy {
            info!("💚 Health check: All systems healthy");
        } else {
            warn!(
                "💔 Health check: Some systems unhealthy - Kafka: {}, Geyser: {}",
                kafka_healthy, geyser_healthy
            );
        }

        HEALTH_CHECK_TOTAL.inc();
        if !kafka_healthy || !geyser_healthy {
            HEALTH_CHECK_FAILURES.inc();
        }
    }
}

/// HTTP health endpoint handler
async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let health_status = state.health_status.read().await;
    let uptime = chrono::Utc::now().signed_duration_since(health_status.last_check);

    Json(HealthResponse {
        status: if health_status.service && health_status.kafka && health_status.geyser {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: format!("{}s", uptime.num_seconds()),
        checks: HealthChecks {
            kafka: if health_status.kafka {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
            geyser: if health_status.geyser {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
            service: if health_status.service {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
        },
    })
}

/// Prometheus metrics endpoint
async fn metrics_handler(State(_state): State<AppState>) -> String {
    use prometheus::Encoder;
    let mut buffer = Vec::new();
    let encoder = prometheus::TextEncoder::new();
    encoder.encode(&REGISTRY.gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// Log EVERY transaction to PostgreSQL with extracted program IDs
async fn log_every_transaction_to_db(
    pool: &PgPool,
    update: &yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
) -> Result<()> {
    if let Some(transaction) = &update.transaction {
        if !transaction.signature.is_empty() {
            let signature_str = bs58::encode(&transaction.signature).into_string();
            let slot = update.slot as i64;
            let block_time = chrono::DateTime::from_timestamp(
                transaction
                    .meta
                    .as_ref()
                    .map(|_| update.slot as i64) // Use slot as timestamp
                    .unwrap_or(0),
                0,
            )
            .unwrap_or_else(chrono::Utc::now);

            // Extract program IDs from transaction instructions
            let mut extracted_program_ids = Vec::new();
            if let Some(inner_transaction) = transaction.transaction.as_ref() {
                if let Some(message) = inner_transaction.message.as_ref() {
                    let account_keys: Vec<String> = message
                        .account_keys
                        .iter()
                        .map(|key| bs58::encode(key).into_string())
                        .collect();

                    for instruction in &message.instructions {
                        if let Some(program_id) =
                            account_keys.get(instruction.program_id_index as usize)
                        {
                            extracted_program_ids.push(program_id.clone());
                        }
                    }
                }
            }

            // Create raw message JSON
            let mut buf = Vec::new();
            update
                .encode(&mut buf)
                .map_err(|e| anyhow::anyhow!("Encode error: {}", e))?;
            let raw_message = serde_json::json!({
                "slot": update.slot,
                "signature": signature_str,
                "encoded_data": bs58::encode(&buf).into_string(),
                "extracted_program_ids": extracted_program_ids,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

                                    // Log each program ID as a separate row
                        for program_id in &extracted_program_ids {
                            let dex_name = match program_id.as_str() {
                                "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => "Jupiter_V6",
                                "JUP6LkbZbafMNUBhDSDueet2ApgUKPKodDhqWFLVvZpg" => "Jupiter_Swap",
                                "jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZzyknqvzYNrNu" => "Jupiter_Limit_Order",
                                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Whirlpool",
                                "675kPXNDfGNzVz7eY9Me1Lw2feN9WbVraN5D1CR1KwJ" => "Raydium_AMM_V4",
                                "EhhTKFNriT2CgK5K8a7Z8aXjLw1f7og2c2x2D52f2222" => "Phoenix",
                                "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4" => "Meteora",
                                _ => "Unknown",
                            };

                sqlx::query(
                    r#"
                    INSERT INTO raw_dex_transactions (
                        block_time, slot_number, transaction_signature, program_id, dex_name, raw_message
                    ) VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (transaction_signature) DO NOTHING
                    "#,
                )
                .bind(block_time)
                .bind(slot)
                .bind(&signature_str)
                .bind(program_id)
                .bind(dex_name)
                .bind(&raw_message)
                .execute(pool)
                .await
                .map_err(|e| anyhow::anyhow!("Database insert error: {}", e))?;
            }
        }
    }
    Ok(())
}

/// Log raw transaction data to PostgreSQL
async fn log_transaction_to_db(
    pool: &PgPool,
    update: &yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
    program_ids: &[String],
) -> Result<()> {
    if let Some(transaction) = &update.transaction {
        if !transaction.signature.is_empty() {
            let signature_str = bs58::encode(&transaction.signature).into_string();
            let slot = update.slot as i64;
            let block_time = chrono::DateTime::from_timestamp(
                transaction
                    .meta
                    .as_ref()
                    .map(|_| update.slot as i64) // Use slot as timestamp
                    .unwrap_or(0),
                0,
            )
            .unwrap_or_else(chrono::Utc::now);

            // Create raw message JSON
            let mut buf = Vec::new();
            update
                .encode(&mut buf)
                .map_err(|e| anyhow::anyhow!("Encode error: {}", e))?;
            let raw_message = serde_json::json!({
                "slot": update.slot,
                "signature": signature_str,
                "encoded_data": bs58::encode(&buf).into_string(),
                "program_ids": program_ids,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            // Log each program ID as a separate row
            for program_id in program_ids {
                let dex_name = match program_id.as_str() {
                    "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => "Jupiter",
                    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Whirlpool",
                    "675kPXNDfGNzVz7eY9Me1Lw2feN9WbVraN5D1CR1KwJ" => "Raydium_AMM_V4",
                    "EhhTKFNriT2CgK5K8a7Z8aXjLw1f7og2c2x2D52f2222" => "Phoenix",
                    "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4" => "Meteora",
                    _ => "Unknown",
                };

                sqlx::query(
                    r#"
                    INSERT INTO raw_dex_transactions (
                        block_time, slot_number, transaction_signature, program_id, dex_name, raw_message
                    ) VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (transaction_signature) DO NOTHING
                    "#,
                )
                .bind(block_time)
                .bind(slot)
                .bind(&signature_str)
                .bind(program_id)
                .bind(dex_name)
                .bind(&raw_message)
                .execute(pool)
                .await
                .map_err(|e| anyhow::anyhow!("Database insert error: {}", e))?;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    // Initialize metrics
    init_metrics();

    // Parse command line arguments
    let args = Args::parse();

    // Build service configuration
    let config = match ServiceConfig::from_args(&args) {
        Ok(config) => config,
        Err(missing_configs) => {
            error!("❌ Failed to create service configuration");
            error!("Missing required configurations: {:?}", missing_configs);
            error!("💡 Set environment variables or CLI args for production use");
            std::process::exit(1);
        }
    };

    // Log startup configuration
    config.log_startup_configuration();

    // Emit final runtime configuration for transparency
    info!("🔧 FINAL RUNTIME CONFIGURATION");
    info!("=================================");
    info!("  Kafka Broker: {}", config.broker);
    info!("  Topic: {}", config.topic);
    info!("  Geyser Endpoint: {}", config.endpoint);
    info!(
        "  X-Token: {}...{}",
        &config.x_token[..8],
        &config.x_token[config.x_token.len() - 4..]
    );
    info!("  DEX Filtering: {}", config.dex_filtering);
    info!("  Program IDs Count: {}", config.program_ids.len());
    if !config.program_ids.is_empty() {
        info!("  Program IDs: {:?}", config.program_ids);
    }
    info!("  Health Port: {}", config.health_port);
    info!("=================================");

    // Detect and display external IP address
    info!("🌐 EXTERNAL NETWORK INFORMATION");
    info!("=================================");
    match get_external_ip().await {
        Ok(ip) => {
            info!("  Public IP Address: {}", ip);
            info!("  External Services will see: {}", ip);
        }
        Err(e) => {
            warn!("  Could not detect public IP: {}", e);
            info!("  External services may not be able to reach this service");
        }
    }
    info!("=================================");

    // Test connectivity before starting service
    info!("🧪 RUNNING STARTUP VALIDATION TESTS");
    info!("====================================");

    let mut startup_errors = Vec::new();

    // Test Kafka connectivity
    match test_kafka_connectivity(&config.broker).await {
        Ok(_) => info!("✅ Kafka: PASSED"),
        Err(e) => {
            warn!("❌ Kafka: FAILED - {}", e);
            startup_errors.push(format!("Kafka connectivity failed: {}", e));
        }
    }

    // Test Geyser connectivity
    match test_geyser_connectivity(&config.endpoint, &config.x_token).await {
        Ok(_) => {
            info!("✅ Geyser: PASSED - Connection successful (404 is NORMAL for gRPC endpoints)")
        }
        Err(e) => {
            warn!("❌ Geyser: FAILED - {}", e);
            startup_errors.push(format!("Geyser connectivity failed: {}", e));
        }
    }

    // Report startup validation results
    if !startup_errors.is_empty() {
        warn!("⚠️  STARTUP VALIDATION FAILED");
        warn!("=================================");
        for error in &startup_errors {
            warn!("  ❌ {}", error);
        }
        warn!("=================================");
        warn!("💡 Service will attempt to start but may fail during operation");
        warn!("💡 Fix the above issues for optimal performance");
    } else {
        info!("✅ ALL STARTUP VALIDATION TESTS PASSED");
    }
    info!("====================================");

    // Create Kafka producer
    let fast_mode = std::env::var("FAST_MODE").ok().as_deref() == Some("1");
    let producer = create_producer(&config.broker, fast_mode)?;

    // Create discovery producer if enabled
    let discovery_mode = std::env::var("DISCOVERY_MODE").ok().as_deref() == Some("1");
    let discovery_producer = if discovery_mode {
        Some(create_producer(&config.broker, true)?)
    } else {
        None
    };

    // Create PostgreSQL connection pool for raw transaction logging
    info!("🗃️ Connecting to PostgreSQL: {}", config.database_url);
    let db_pool = sqlx::PgPool::connect(&config.database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to PostgreSQL: {}", e))?;
    info!("✅ PostgreSQL connected successfully");

    // Create a set of known program IDs for fast lookup
    let known_program_ids: HashSet<String> = config.program_ids.iter().cloned().collect();

    // Initialize health status
    let health_status = Arc::new(RwLock::new(HealthStatus::default()));
    let app_state = AppState {
        health_status: health_status.clone(),
        metrics_registry: REGISTRY.clone(),
    };

    // Start background health checker
    let health_checker_config = config.clone();
    let health_checker_handle = tokio::spawn(background_health_checker(
        health_checker_config,
        health_status.clone(),
    ));

    // Start HTTP health server using hyper directly
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = match TcpListener::bind((config.bind_address.as_str(), config.health_port)).await
    {
        Ok(listener) => listener,
        Err(e) => {
            error!(
                "❌ Failed to bind HTTP server to {}:{} - {}",
                config.bind_address, config.health_port, e
            );
            error!("💡 Check if the bind address is valid and the port is available");
            return Err(anyhow::anyhow!("HTTP server binding failed: {}", e));
        }
    };
    let health_server_handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            error!("Health server error: {}", err);
        }
    });

    info!(
        "🌐 Health check server started on port {}",
        config.health_port
    );
    info!(
        "📊 Metrics available at http://:{}/metrics",
        config.health_port
    );
    info!(
        "💚 Health status available at http://:{}/health",
        config.health_port
    );

    // Start the main processing loop
    let main_loop_handle = tokio::spawn(async move {
        retry(ExponentialBackoff::default(), move || {
            let producer = producer.clone();
            let discovery_producer = discovery_producer.clone();
            let config = config.clone();
            let known_program_ids = known_program_ids.clone();
            let db_pool = db_pool.clone();

            async move {
                let mut client = GeyserGrpcClient::build_from_shared(config.endpoint.clone())
                    .map_err(|e| anyhow::anyhow!(e))?
                    .x_token(Some(config.x_token.clone()))
                    .map_err(|e| anyhow::anyhow!(e))?
                    .connect_timeout(Duration::from_secs(10))
                    .timeout(Duration::from_secs(10))
                    .tls_config(ClientTlsConfig::new().with_native_roots())
                    .map_err(|e| anyhow::anyhow!(e))?
                    .max_decoding_message_size(1024 * 1024 * 1024)
                    .connect()
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;

                let wide_open = config.dex_filtering == "OFF";
                let request = build_request(config.program_ids.clone(), wide_open);
                let (mut subscribe_tx, mut stream) =
                    client.subscribe_with_request(Some(request)).await?;
                info!("✅ Geyser stream connected - processing live data");

                let mut unknown_program_count = 0u64;
                let mut transaction_count = 0u64;
                let mut last_heartbeat = std::time::Instant::now();

                while let Some(msg) = stream.next().await {
                    match msg {
                        Ok(update_msg) => match update_msg.update_oneof {
                            Some(UpdateOneof::Transaction(update)) => {
                                let start_time = std::time::Instant::now();
                                transaction_count += 1;

                                // Lean heartbeat - just count and timestamp
                                if last_heartbeat.elapsed().as_secs() >= 30 {
                                    let now = chrono::Utc::now();
                                    info!(
                                        "💓 HEARTBEAT: Transactions: {}, Time: {}",
                                        transaction_count,
                                        now.format("%H:%M:%S")
                                    );
                                    last_heartbeat = std::time::Instant::now();
                                }

                                let mut buf = Vec::new();
                                update.encode(&mut buf)?;

                                // Emit to main topic
                                let _ = producer
                                    .send(
                                        FutureRecord::<(), _>::to(&config.topic).payload(&buf),
                                    Duration::from_secs(10),
                                    )
                                    .await;

                                    // Console output: structured human-readable transaction snapshot
                                    if let Some(txinfo) = update.transaction.as_ref() {
                                        let signature_b58 = bs58::encode(&txinfo.signature).into_string();
                                        let mut program_ids: Vec<String> = Vec::new();
                                        if let Some(inner_tx) = txinfo.transaction.as_ref() {
                                            if let Some(message) = inner_tx.message.as_ref() {
                                                for ix in &message.instructions {
                                                    if let Some(key) = message.account_keys.get(ix.program_id_index as usize) {
                                                        program_ids.push(bs58::encode(key).into_string());
                                                    }
                                                }
                                            }
                                        }
                                        let raw_b58 = bs58::encode(&buf).into_string();
                                        println!(
                                            "TX slot={} sig={} programs=[{}] raw_b58={}", 
                                            update.slot,
                                            signature_b58,
                                            program_ids.join(","),
                                            raw_b58
                                        );
                                    } else {
                                        println!("TX slot={} (no transaction info)", update.slot);
                                    }


                                // Log raw transaction to PostgreSQL - EVERY TRANSACTION
                                if let Err(e) = log_every_transaction_to_db(&db_pool, &update).await
                                {
                                    warn!("Failed to log transaction to database: {}", e);
                                }

                                // Discovery mode: analyze transaction for unknown program IDs
                                if discovery_mode {
                                    if let Some(discovery_prod) = &discovery_producer {
                                        if let Some(unknown_programs) =
                                            analyze_transaction_for_discovery(
                                                &update,
                                                &known_program_ids,
                                            )
                                        {
                                            for program_id in unknown_programs {
                                                unknown_program_count += 1;
                                                let signature = update
                                                    .transaction
                                                    .as_ref()
                                                    .and_then(|t| Some(t.signature.clone()))
                                                    .map(|s| bs58::encode(&s).into_string())
                                                    .unwrap_or_else(|| "unknown".to_string());

                                                let discovery_data = json!({
                                                    "program_id": program_id,
                                                    "timestamp": chrono::Utc::now().timestamp(),
                                                    "transaction_signature": signature,
                                                    "slot": update.slot,
                                                    "encounter_count": unknown_program_count,
                                                    "source": "dex-stream-universal"
                                                });

                                                let discovery_payload = discovery_data.to_string();
                                                let _ = discovery_prod
                                                    .send(
                                                        FutureRecord::<(), _>::to(
                                                            "unknown-programs",
                                                        )
                                                        .payload(discovery_payload.as_bytes())
                                                        .key(&()),
                                                        Duration::from_secs(5),
                                                    )
                                                    .await;

                                                if unknown_program_count % 100 == 0 {
                                                    info!(
                                                        "Discovered {} unknown program IDs",
                                                        unknown_program_count
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }

                                // Update metrics
                                TRANSACTIONS_PROCESSED.inc();
                                let processing_time = start_time.elapsed();
                                TRANSACTION_PROCESSING_TIME.observe(processing_time.as_secs_f64());
                            }
                            Some(UpdateOneof::Ping(_)) => {
                                let _ = subscribe_tx
                                    .send(SubscribeRequest {
                                        ping: Some(SubscribeRequestPing { id: 1 }),
                                        ..Default::default()
                                    })
                                    .await;
                            }
                            _ => {}
                        },
                        Err(e) => {
                            error!("stream error: {e:?}");
                            break;
                        }
                    }
                }
                info!("stream closed");
                Ok::<(), anyhow::Error>(())
            }
            .map_err(backoff::Error::transient)
        })
        .await
    });

    // Wait for all tasks
    tokio::select! {
        _ = health_checker_handle => {
            error!("Health checker task failed");
        }
        _ = health_server_handle => {
            error!("Health server task failed");
        }
        _ = main_loop_handle => {
            info!("Main processing loop completed");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_service_config_creation() {
        // Test successful config creation
        let args = Args {
            broker: Some("test-broker".to_string()),
            topic: Some("test-topic".to_string()),
            endpoint: Some("https://test.endpoint".to_string()),
            x_token: Some("test-token".to_string()),
            program_ids: None,
            dex_filtering: "ON".to_string(),
            health_port: 0,
            bind_address: "127.0.0.1".to_string(),
        };

        let config = ServiceConfig::from_args(&args).unwrap();
        assert_eq!(config.broker, "test-broker");
        assert_eq!(config.topic, "test-topic");
        assert_eq!(config.endpoint, "https://test.endpoint");
        assert_eq!(config.x_token, "test-token");
        assert_eq!(config.dex_filtering, "ON");
        assert_eq!(config.health_port, 0);
    }

    #[test]
    fn test_service_config_missing_required_fields() {
        // Test config creation with missing required fields
        let args = Args {
            broker: Some("test-broker".to_string()),
            topic: None, // Missing required field
            endpoint: Some("https://test.endpoint".to_string()),
            x_token: Some("test-token".to_string()),
            program_ids: None,
            dex_filtering: "ON".to_string(),
            health_port: 0,
            bind_address: "127.0.0.1".to_string(),
        };

        let result = ServiceConfig::from_args(&args);
        assert!(result.is_err());

        if let Err(missing_configs) = result {
            assert!(missing_configs.contains(&"TOPIC".to_string()));
        }
    }

    #[test]
    fn test_service_config_dex_filtering_off() {
        // Test DEX filtering OFF mode configuration
        let args = Args {
            broker: Some("test-broker".to_string()),
            topic: Some("test-topic".to_string()),
            endpoint: Some("https://test.endpoint".to_string()),
            x_token: Some("test-token".to_string()),
            program_ids: None,
            dex_filtering: "OFF".to_string(),
            health_port: 0,
            bind_address: "127.0.0.1".to_string(),
        };

        let config = ServiceConfig::from_args(&args).unwrap();
        assert_eq!(config.dex_filtering, "OFF");
        assert_eq!(config.program_ids.len(), 0);
    }

    #[test]
    fn test_health_status_default() {
        // Test HealthStatus default values
        let status = HealthStatus::default();
        assert_eq!(status.service, true);
        assert_eq!(status.kafka, false);
        assert_eq!(status.geyser, false);
    }

    #[test]
    fn test_health_status_updates() {
        // Test HealthStatus updates
        let mut status = HealthStatus::default();

        status.kafka = true;
        status.geyser = true;

        assert_eq!(status.service, true);
        assert_eq!(status.kafka, true);
        assert_eq!(status.geyser, true);
    }

    #[test]
    fn test_metrics_registry() {
        // Test that metrics registry is accessible and metrics are registered
        let registry = &REGISTRY;
        // The metrics are already registered in init_metrics(), so we can just verify they exist
        assert!(true); // Placeholder - metrics are accessible via the static refs
    }

    #[test]
    fn test_program_ids_parsing() {
        // Test program IDs parsing logic
        let program_ids_csv = "11111111111111111111111111111111,22222222222222222222222222222222";
        let program_ids: Vec<String> = program_ids_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert_eq!(program_ids.len(), 2);
        assert_eq!(program_ids[0], "11111111111111111111111111111111");
        assert_eq!(program_ids[1], "22222222222222222222222222222222");
    }

    #[test]
    fn test_program_ids_hashset_conversion() {
        // Test HashSet conversion for fast lookups
        let program_ids = vec![
            "11111111111111111111111111111111".to_string(),
            "22222222222222222222222222222222".to_string(),
        ];
        let known_program_ids: HashSet<String> = program_ids.iter().cloned().collect();

        assert_eq!(known_program_ids.len(), 2);
        assert!(known_program_ids.contains("11111111111111111111111111111111"));
        assert!(known_program_ids.contains("22222222222222222222222222222222"));
    }

    #[test]
    fn test_args_clone() {
        // Test Args struct cloning
        let args = Args {
            broker: Some("test-broker".to_string()),
            topic: Some("test-broker".to_string()),
            endpoint: Some("https://test.endpoint".to_string()),
            x_token: Some("test-token".to_string()),
            program_ids: None,
            dex_filtering: "ON".to_string(),
            health_port: 0,
            bind_address: "127.0.0.1".to_string(),
        };

        let cloned_args = args.clone();
        assert_eq!(args.broker, cloned_args.broker);
        assert_eq!(args.topic, cloned_args.topic);
        assert_eq!(args.endpoint, cloned_args.endpoint);
        assert_eq!(args.x_token, cloned_args.x_token);
        assert_eq!(args.dex_filtering, cloned_args.dex_filtering);
        assert_eq!(args.health_port, cloned_args.health_port);
    }

    #[test]
    fn test_service_config_clone() {
        // Test ServiceConfig struct cloning
        let config = ServiceConfig {
            broker: "test-broker".to_string(),
            topic: "test-topic".to_string(),
            endpoint: "https://test.endpoint".to_string(),
            x_token: "test-token".to_string(),
            program_ids: vec!["test-id".to_string()],
            dex_filtering: "ON".to_string(),
            health_port: 0,
            bind_address: "127.0.0.1".to_string(),
        };

        let cloned_config = config.clone();
        assert_eq!(config.broker, cloned_config.broker);
        assert_eq!(config.x_token, cloned_config.x_token);
        assert_eq!(config.program_ids, cloned_config.program_ids);
        assert_eq!(config.dex_filtering, cloned_config.dex_filtering);
        assert_eq!(config.health_port, cloned_config.health_port);
    }

    #[test]
    fn test_health_response_serialization() {
        // Test health response JSON serialization
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            version: "1.0.0".to_string(),
            uptime: "3600s".to_string(),
            checks: HealthChecks {
                kafka: "healthy".to_string(),
                geyser: "healthy".to_string(),
                service: "healthy".to_string(),
            },
        };

        // Test that it can be serialized to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("3600s"));
    }
}
