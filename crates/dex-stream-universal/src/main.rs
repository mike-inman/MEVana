use anyhow::Result;
use backoff::{future::retry, ExponentialBackoff};
use chrono;
use clap::Parser;
use dotenv::dotenv;
use env_logger;
use futures_util::{sink::SinkExt, stream::StreamExt, TryFutureExt};
use log::{error, info, warn};
use prometheus::{Counter, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry};
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use reqwest;
use serde_json::json;
use shared_core::NetworkConfig;
use std::collections::HashSet;
use std::time::Duration;
use tonic::metadata::MetadataValue;
use tonic::transport::channel::ClientTlsConfig;
use warp::Filter;
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::{
    geyser::SubscribeRequestFilterTransactions,
    prelude::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
    },
    prost::Message as _,
};

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(short, long, help = "Kafka broker address (REQUIRED)")]
    broker: String,
    #[arg(short, long, help = "Output topic (REQUIRED)")]
    topic: String,
    #[arg(short, long, help = "Geyser endpoint URL (REQUIRED)")]
    endpoint: String,
    #[arg(long, help = "Geyser authentication token (REQUIRED)")]
    x_token: String,
    #[arg(long, help = "Comma-separated list of program IDs to filter by")]
    program_ids: Option<String>,
    #[arg(long, help = "Disable program ID filtering (wide open mode)")]
    pub no_filter: bool,
}

// Prometheus metrics
lazy_static::lazy_static! {
    static ref REGISTRY: Registry = Registry::new();

    // Transaction processing metrics
    pub static ref TRANSACTIONS_PROCESSED: IntCounter = IntCounter::new(
        "dex_stream_transactions_processed_total",
        "Total number of transactions processed"
    ).expect("Failed to create transactions_processed metric");

    pub static ref TRANSACTIONS_PROCESSED_RATE: IntGauge = IntGauge::new(
        "dex_stream_transactions_per_second",
        "Current transactions processed per second"
    ).expect("Failed to create transactions_per_second metric");

    pub static ref TRANSACTION_PROCESSING_TIME: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "dex_stream_transaction_processing_seconds",
            "Time spent processing individual transactions"
        )
    ).expect("Failed to create transaction_processing_time metric");

    // Kafka metrics
    pub static ref KAFKA_MESSAGES_SENT: IntCounter = IntCounter::new(
        "dex_stream_kafka_messages_sent_total",
        "Total number of Kafka messages sent"
    ).expect("Failed to create kafka_messages_sent metric");

    pub static ref KAFKA_SEND_ERRORS: IntCounter = IntCounter::new(
        "dex_stream_kafka_send_errors_total",
        "Total number of Kafka send errors"
    ).expect("Failed to create kafka_send_errors metric");

    // Geyser stream metrics
    pub static ref GEYSER_STREAM_UP: IntGauge = IntGauge::new(
        "dex_stream_geyser_stream_up",
        "Geyser stream connection status (1=up, 0=down)"
    ).expect("Failed to create geyser_stream_up metric");

    pub static ref GEYSER_STREAM_ERRORS: IntCounter = IntCounter::new(
        "dex_stream_geyser_stream_errors_total",
        "Total number of Geyser stream errors"
    ).expect("Failed to create geyser_stream_errors metric");

    // Service health metrics
    pub static ref SERVICE_UPTIME: IntGauge = IntGauge::new(
        "dex_stream_service_uptime_seconds",
        "Service uptime in seconds"
    ).expect("Failed to create service_uptime metric");
}

// Initialize metrics
fn init_metrics() {
    REGISTRY
        .register(Box::new(TRANSACTIONS_PROCESSED.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(TRANSACTIONS_PROCESSED_RATE.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(TRANSACTION_PROCESSING_TIME.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(KAFKA_MESSAGES_SENT.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(KAFKA_SEND_ERRORS.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(GEYSER_STREAM_UP.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(GEYSER_STREAM_ERRORS.clone()))
        .unwrap();
    REGISTRY.register(Box::new(SERVICE_UPTIME.clone())).unwrap();
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
            if let Some(message) = &transaction.message {
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

    // Extract program IDs from account keys
    if let Some(transaction_info) = &update.transaction {
        if let Some(transaction) = &transaction_info.transaction {
            if let Some(message) = &transaction.message {
                for account_key in &message.account_keys {
                    let account_key_str = bs58::encode(account_key).into_string();
                    if !known_program_ids.contains(&account_key_str) {
                        // This might be a program ID, add it for discovery
                        unknown_programs.push(account_key_str);
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

pub fn build_request(program_ids: Vec<String>) -> SubscribeRequest {
    let mut transactions: TxnFilterMap = std::collections::HashMap::new();
    transactions.insert(
        "client".to_owned(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: program_ids,
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

/// Test Kafka broker connectivity before starting the service
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
            info!("   Response status: {}", resp.status());
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

/// Service configuration that combines command line arguments and environment variables
#[derive(Debug, Clone)]
struct ServiceConfig {
    broker: String,
    topic: String,
    endpoint: String,
    x_token: String,
    program_ids: Vec<String>,
    no_filter: bool,
}

impl ServiceConfig {
    /// Create service configuration from command line arguments and environment variables
    fn from_args_and_env(args: &Args, _network_config: &NetworkConfig) -> Result<Self> {
        let broker = args.broker.clone();
        let topic = args.topic.clone();
        let endpoint = args.endpoint.clone();
        let x_token = args.x_token.clone();

        let program_ids_csv = if args.no_filter {
            // If no_filter is set, ignore program IDs and create wide-open request
            String::new()
        } else {
            args.program_ids
                .clone()
                .or_else(|| std::env::var("PROGRAM_IDS").ok())
                .unwrap_or_default()
        };

        let program_ids: Vec<String> = program_ids_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            broker,
            topic,
            endpoint,
            x_token,
            program_ids,
            no_filter: args.no_filter,
        })
    }

    /// Log the startup configuration
    fn log_startup_configuration(&self) {
        info!(
            "🚀 DEX Stream Universal - Broker: {}, Topic: {}, Mode: {}",
            self.broker,
            self.topic,
            if self.program_ids.is_empty() {
                "WIDE-OPEN"
            } else {
                "FILTERED"
            }
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    // 🚀 USE SHARED NETWORK CONFIGURATION
    let mut network_config = NetworkConfig::from_env();

    // 🔍 Network audit and self-healing
    match network_config.self_heal().await {
        Ok(()) => info!("✅ Network self-healing completed"),
        Err(e) => warn!("⚠️  Network self-healing failed: {}", e),
    }

    // Clone args to avoid borrow checker issues
    let args_clone = args.clone();

    let broker = args_clone.broker.clone();

    // Build service configuration
    let config = ServiceConfig::from_args_and_env(&args, &network_config)?;

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
    info!("  No-Filter Mode: {}", config.no_filter);
    info!("  Program IDs Count: {}", config.program_ids.len());
    if !config.program_ids.is_empty() {
        info!("  Program IDs: {:?}", config.program_ids);
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
        Ok(_) => info!("✅ Geyser: PASSED"),
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

    // Create a set of known program IDs for fast lookup
    let known_program_ids: HashSet<String> = config.program_ids.iter().cloned().collect();

    // Start the main processing loop
    retry(ExponentialBackoff::default(), move || {
        let producer = producer.clone();
        let discovery_producer = discovery_producer.clone();
        let config = config.clone();
        let known_program_ids = known_program_ids.clone();

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

            let request = build_request(config.program_ids.clone());
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
                                                    FutureRecord::<(), _>::to("unknown-programs")
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
    .await?;

    Ok(())
}
