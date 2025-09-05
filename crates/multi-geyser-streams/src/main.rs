use anyhow::Result;
use axum::{response::Json, routing::get, Router};
use chrono::{DateTime, Utc};
use clap::Parser;
use dashmap::DashMap;
use futures::StreamExt;
use log::{error, info, warn};
use metrics::{counter, histogram};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::time::Duration;
use tokio::time::sleep;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    geyser::SubscribeRequestFilterTransactions,
    prelude::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    },
};

/// Multi-Geyser Stream Service
/// Runs 3 parallel Geyser connections for ultra-low latency DEX monitoring
#[derive(Parser)]
#[command(name = "multi-geyser-streams")]
struct Args {
    /// Geyser gRPC endpoint
    #[arg(long, default_value = "https://grpc.ny.shyft.to")]
    geyser_endpoint: String,

    /// Geyser auth token
    #[arg(long, default_value = "efff2d23-97ae-4c5f-8b8b-9a6265001189")]
    x_token: String,

    /// HTTP metrics port
    #[arg(long, default_value = "8086")]
    metrics_port: u16,

    /// Enable verbose logging
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamConfig {
    id: u8,
    name: String,
    program_ids: Vec<String>,
    endpoint: String,
    token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionMetrics {
    signature: String,
    slot: u64,
    stream_id: u8,
    dex_name: String,
    processing_time_us: u64,
    timestamp: DateTime<Utc>,
}

// Global metrics tracking
lazy_static::lazy_static! {
    static ref STREAM_COUNTERS: DashMap<u8, AtomicU64> = DashMap::new();
    static ref DEX_COUNTERS: DashMap<String, AtomicU64> = DashMap::new();
    static ref STREAM_HEALTH: DashMap<u8, Arc<AtomicU64>> = DashMap::new();
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            id: 0,
            name: "default".to_string(),
            program_ids: vec![],
            endpoint: "https://grpc.ny.shyft.to".to_string(),
            token: "efff2d23-97ae-4c5f-8b8b-9a6265001189".to_string(),
        }
    }
}

fn create_stream_configs(endpoint: &str, token: &str) -> Vec<StreamConfig> {
    vec![
        StreamConfig {
            id: 1,
            name: "Jupiter Stream".to_string(),
            program_ids: vec![
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), // Jupiter V6 Main
                "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB".to_string(),   // Jupiter V4
                "JUP6LkbZbafMNUBhDSDueet2ApgUKPKodDhqWFLVvZpg".to_string(),   // Jupiter Swap
                "jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZzyknqvzYNrNu".to_string(),   // Jupiter Limit Order
            ],
            endpoint: endpoint.to_string(),
            token: token.to_string(),
        },
        StreamConfig {
            id: 2,
            name: "Orca+Whirlpool Stream".to_string(),
            program_ids: vec![
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(), // Whirlpool (Orca CLMM)
                "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1".to_string(), // Orca Legacy AMM
            ],
            endpoint: endpoint.to_string(),
            token: token.to_string(),
        },
        StreamConfig {
            id: 3,
            name: "Other DEXs Stream".to_string(),
            program_ids: vec![
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(), // Raydium AMM
                "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqGnT".to_string(), // Raydium CLMM
                "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo".to_string(),   // Meteora DLMM
                "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY".to_string(),   // Phoenix
                "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX".to_string(),   // OpenBook
                "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string(),   // Pump.fun
                "2wT8Yq49kHgDzXuPxZSaeLaH1qbmGXtEyPy64bL7aD3c".to_string(),   // Mystery DEX
            ],
            endpoint: endpoint.to_string(),
            token: token.to_string(),
        },
    ]
}

fn get_dex_name_from_program_id(program_id: &str) -> String {
    match program_id {
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "jupiter-v6".to_string(),
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => "jupiter-v4".to_string(),
        "JUP6LkbZbafMNUBhDSDueet2ApgUKPKodDhqWFLVvZpg" => "jupiter-swap".to_string(),
        "jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZzyknqvzYNrNu" => "jupiter-limit".to_string(),
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "whirlpool".to_string(),
        "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1" => "orca-legacy".to_string(),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "raydium-amm".to_string(),
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqGnT" => "raydium-clmm".to_string(),
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "meteora".to_string(),
        "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY" => "phoenix".to_string(),
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX" => "openbook".to_string(),
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => "pump-fun".to_string(),
        "2wT8Yq49kHgDzXuPxZSaeLaH1qbmGXtEyPy64bL7aD3c" => "mystery-dex".to_string(),
        _ => "unknown".to_string(),
    }
}

async fn create_geyser_client(config: &StreamConfig) -> Result<yellowstone_grpc_client::GeyserGrpcClient<impl tonic::service::Interceptor>> {
    let client = GeyserGrpcClient::build_from_shared(config.endpoint.clone())
        .map_err(|e| anyhow::anyhow!(e))?
        .x_token(Some(config.token.clone()))
        .map_err(|e| anyhow::anyhow!(e))?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(10))
        .tls_config(tonic::transport::channel::ClientTlsConfig::new().with_native_roots())
        .map_err(|e| anyhow::anyhow!(e))?
        .max_decoding_message_size(1024 * 1024 * 1024)
        .connect()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    info!("✅ Connected to Geyser for stream {}: {}", config.id, config.name);
    Ok(client)
}

async fn create_geyser_subscription(config: &StreamConfig) -> Result<SubscribeRequest> {
    let mut transactions: HashMap<String, SubscribeRequestFilterTransactions> = HashMap::new();
    transactions.insert(
        format!("dex_transactions_{}", config.id),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: config.program_ids.clone(),
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );

    let request = SubscribeRequest {
        accounts: Default::default(),
        slots: Default::default(),
        transactions,
        transactions_status: Default::default(),
        blocks: Default::default(),
        blocks_meta: Default::default(),
        entry: Default::default(),
        accounts_data_slice: vec![],
        ping: None,
        commitment: Some(CommitmentLevel::Processed as i32),
        from_slot: None,
    };

    info!(
        "📡 Created subscription for stream {} with {} program IDs",
        config.id,
        config.program_ids.len()
    );

    Ok(request)
}

async fn process_geyser_stream(config: StreamConfig) -> Result<()> {
    let stream_id = config.id;
    info!("🚀 Starting Geyser stream {}: {}", stream_id, config.name);

    // Initialize metrics
    STREAM_COUNTERS.insert(stream_id, AtomicU64::new(0));
    STREAM_HEALTH.insert(stream_id, Arc::new(AtomicU64::new(0)));

    loop {
        match run_stream_with_retry(&config).await {
            Ok(_) => {
                warn!("Stream {} ended normally, restarting...", stream_id);
            }
            Err(e) => {
                error!("Stream {} error: {}. Retrying in 5 seconds...", stream_id, e);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn run_stream_with_retry(config: &StreamConfig) -> Result<()> {
    let mut client = create_geyser_client(config).await?;
    let request = create_geyser_subscription(config).await?;

    let (_, mut stream) = client.subscribe_with_request(Some(request)).await?;
    let stream_id = config.id;

    // Update health status
    if let Some(health) = STREAM_HEALTH.get(&stream_id) {
        health.store(chrono::Utc::now().timestamp() as u64, Ordering::Relaxed);
    }

    info!("🔄 Stream {} processing messages...", stream_id);

    while let Some(message) = stream.next().await {
        let start_time = Instant::now();

        match message {
            Ok(update_msg) => {
                if let Some(UpdateOneof::Transaction(transaction_update)) = update_msg.update_oneof {
                    let processing_result = process_transaction_update(config, transaction_update).await;
                    let processing_time_us = start_time.elapsed().as_micros() as u64;

                    // Record metrics
                    histogram!("geyser_transaction_processing_time_us").record(processing_time_us as f64);
                    counter!("geyser_transactions_processed_total").increment(1);

                    if let Some(counter) = STREAM_COUNTERS.get(&stream_id) {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }

                    // Update health
                    if let Some(health) = STREAM_HEALTH.get(&stream_id) {
                        health.store(chrono::Utc::now().timestamp() as u64, Ordering::Relaxed);
                    }

                    match processing_result {
                        Ok(Some(metrics)) => {
                            counter!("geyser_transactions_successful_total").increment(1);

                            // Update DEX counters
                            DEX_COUNTERS.entry(metrics.dex_name.clone())
                                .or_insert_with(|| AtomicU64::new(0))
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(None) => {
                            counter!("geyser_transactions_filtered_total").increment(1);
                        }
                        Err(e) => {
                            counter!("geyser_transactions_failed_total").increment(1);
                            error!("Failed to process transaction on stream {}: {}", stream_id, e);
                        }
                    }
                }
            }
            Err(e) => {
                counter!("geyser_stream_errors_total").increment(1);
                error!("Stream {} gRPC error: {}", stream_id, e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}

async fn process_transaction_update(
    config: &StreamConfig,
    transaction_update: yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
) -> Result<Option<TransactionMetrics>> {
    let signature = if let Some(tx_info) = &transaction_update.transaction {
        bs58::encode(&tx_info.signature).into_string()
    } else {
        return Ok(None);
    };

    // Extract program IDs from transaction
    let program_ids = extract_program_ids(&transaction_update)?;

    // Find matching DEX
    let mut dex_name = "unknown".to_string();
    for program_id in &program_ids {
        if config.program_ids.contains(program_id) {
            dex_name = get_dex_name_from_program_id(program_id);
            break;
        }
    }

    if dex_name == "unknown" {
        return Ok(None); // Filtered out
    }

    let metrics = TransactionMetrics {
        signature,
        slot: transaction_update.slot,
        stream_id: config.id,
        dex_name,
        processing_time_us: 0, // Will be set by caller
        timestamp: Utc::now(),
    };

    Ok(Some(metrics))
}

fn extract_program_ids(
    transaction_update: &yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
) -> Result<Vec<String>> {
    let mut program_ids = Vec::new();

    if let Some(tx_info) = &transaction_update.transaction {
        if let Some(inner_transaction) = &tx_info.transaction {
            if let Some(message) = &inner_transaction.message {
                let account_keys: Vec<String> = message
                    .account_keys
                    .iter()
                    .map(|key| bs58::encode(key).into_string())
                    .collect();

                for instruction in &message.instructions {
                    if let Some(program_id) = account_keys.get(instruction.program_id_index as usize) {
                        program_ids.push(program_id.clone());
                    }
                }
            }
        }
    }

    Ok(program_ids)
}

async fn metrics_handler() -> Json<serde_json::Value> {
    let mut stream_metrics = json!({});
    let mut dex_metrics = json!({});

    // Collect stream metrics
    for entry in STREAM_COUNTERS.iter() {
        let stream_id = entry.key();
        let count = entry.value().load(Ordering::Relaxed);
        stream_metrics[format!("stream_{}", stream_id)] = json!(count);
    }

    // Collect DEX metrics
    for entry in DEX_COUNTERS.iter() {
        let dex_name = entry.key();
        let count = entry.value().load(Ordering::Relaxed);
        dex_metrics[dex_name.clone()] = json!(count);
    }

    // Health status
    let mut health_status = json!({});
    let now = chrono::Utc::now().timestamp() as u64;
    for entry in STREAM_HEALTH.iter() {
        let stream_id = entry.key();
        let last_seen = entry.value().load(Ordering::Relaxed);
        let seconds_since_last = now.saturating_sub(last_seen);
        health_status[format!("stream_{}_seconds_since_last_message", stream_id)] = json!(seconds_since_last);
        health_status[format!("stream_{}_healthy", stream_id)] = json!(seconds_since_last < 30);
    }

    Json(json!({
        "service": "multi-geyser-streams",
        "timestamp": chrono::Utc::now(),
        "streams": stream_metrics,
        "dex_counters": dex_metrics,
        "health": health_status
    }))
}

async fn start_metrics_server(port: u16) -> Result<()> {
    let app = Router::new().route("/metrics", get(metrics_handler));

    let bind_addr = format!("0.0.0.0:{}", port);
    info!("🔧 Starting metrics server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    env_logger::Builder::from_default_env()
        .filter_level(if args.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    info!("🚀 Starting Multi-Geyser Streams Service");
    info!("📡 Endpoint: {}", args.geyser_endpoint);
    info!("📊 Metrics port: {}", args.metrics_port);

    // Initialize Prometheus metrics
    let _prometheus_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install()
        .expect("Failed to install Prometheus recorder");

    // Create stream configurations
    let stream_configs = create_stream_configs(&args.geyser_endpoint, &args.x_token);

    info!("🔧 Configured {} Geyser streams:", stream_configs.len());
    for config in &stream_configs {
        info!("  Stream {}: {} ({} program IDs)", config.id, config.name, config.program_ids.len());
    }

    // Start metrics server
    let metrics_server_handle = {
        let port = args.metrics_port;
        tokio::spawn(async move {
            if let Err(e) = start_metrics_server(port).await {
                error!("Metrics server error: {}", e);
            }
        })
    };

    // Start all Geyser streams in parallel
    let stream_handles: Vec<_> = stream_configs
        .into_iter()
        .map(|config| {
            tokio::spawn(async move {
                if let Err(e) = process_geyser_stream(config.clone()).await {
                    error!("Stream {} terminated with error: {}", config.id, e);
                }
            })
        })
        .collect();

    info!("✅ All streams started successfully");

    // Wait for all streams (they run forever)
    futures::future::try_join_all(stream_handles).await?;

    // Wait for metrics server
    metrics_server_handle.await?;

    Ok(())
}