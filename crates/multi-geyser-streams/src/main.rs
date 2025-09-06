use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use dashmap::DashMap;
use futures::StreamExt;
use log::{error, info, warn};
use metrics::{counter, histogram};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tokio::time::Duration;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    geyser::SubscribeRequestFilterTransactions,
    prelude::{subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest},
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

// Rate counter for tracking messages per minute
#[derive(Debug)]
struct RateCounter {
    // Store timestamps of recent events (within last minute)
    timestamps: Mutex<VecDeque<u64>>,
    window_seconds: u64,
}

impl RateCounter {
    fn new(window_seconds: u64) -> Self {
        Self {
            timestamps: Mutex::new(VecDeque::new()),
            window_seconds,
        }
    }

    fn increment(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut timestamps = self.timestamps.lock().unwrap();
        timestamps.push_back(now);
        
        // Remove old entries outside the time window
        let cutoff = now - self.window_seconds;
        while let Some(&front) = timestamps.front() {
            if front <= cutoff {
                timestamps.pop_front();
            } else {
                break;
            }
        }
    }

    fn rate_per_minute(&self) -> f64 {
        let mut timestamps = self.timestamps.lock().unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Clean old entries
        let cutoff = now - self.window_seconds;
        while let Some(&front) = timestamps.front() {
            if front <= cutoff {
                timestamps.pop_front();
            } else {
                break;
            }
        }
        
        let count = timestamps.len() as f64;
        let window_minutes = self.window_seconds as f64 / 60.0;
        count / window_minutes
    }
}

// Global metrics tracking
lazy_static::lazy_static! {
    static ref STREAM_COUNTERS: DashMap<u8, AtomicU64> = DashMap::new();
    static ref DEX_COUNTERS: DashMap<String, AtomicU64> = DashMap::new();
    static ref STREAM_HEALTH: DashMap<u8, Arc<AtomicU64>> = DashMap::new();
    // Rate counters for messages per minute tracking
    static ref STREAM_RATE_COUNTERS: DashMap<u8, Arc<RateCounter>> = DashMap::new();
    static ref DEX_RATE_COUNTERS: DashMap<String, Arc<RateCounter>> = DashMap::new();
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
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(), // Jupiter V6 Main (CURRENT)
                "jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZzyknqvzYNrNu".to_string(), // Jupiter Limit Orders
            ],
            endpoint: endpoint.to_string(),
            token: token.to_string(),
        },
        StreamConfig {
            id: 2,
            name: "Whirlpool Stream".to_string(),
            program_ids: vec![
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(), // Whirlpool (Orca CLMM)
            ],
            endpoint: endpoint.to_string(),
            token: token.to_string(),
        },
        StreamConfig {
            id: 3,
            name: "Raydium Stream".to_string(),
            program_ids: vec![
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(), // Raydium AMM
            ],
            endpoint: endpoint.to_string(),
            token: token.to_string(),
        },
    ]
}

fn get_dex_name_from_program_id(program_id: &str) -> String {
    match program_id {
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => "jupiter-v4".to_string(), // Legacy
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "jupiter-v6".to_string(), // CURRENT MAIN
        "JUP6LkbZbafMNUBhDSDueet2ApgUKPKodDhqWFLVvZpg" => "jupiter-swap".to_string(),
        "jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZzyknqvzYNrNu" => "jupiter-limit".to_string(),
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "whirlpool".to_string(),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "raydium-amm".to_string(),
        _ => "unknown".to_string(),
    }
}

async fn create_geyser_client(
    config: &StreamConfig,
) -> Result<yellowstone_grpc_client::GeyserGrpcClient<impl tonic::service::Interceptor>> {
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

    info!(
        "✅ Connected to Geyser for stream {}: {}",
        config.id, config.name
    );
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
    STREAM_RATE_COUNTERS.insert(stream_id, Arc::new(RateCounter::new(60))); // 1-minute window

    loop {
        match run_stream_with_retry(&config).await {
            Ok(_) => {
                warn!("Stream {} ended normally, restarting...", stream_id);
            }
            Err(e) => {
                error!(
                    "Stream {} error: {}. Retrying in 5 seconds...",
                    stream_id, e
                );
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
                println!("📥 Stream {} received message", stream_id);
                if let Some(UpdateOneof::Transaction(transaction_update)) = update_msg.update_oneof
                {
                    println!("💰 Stream {} processing transaction at slot {}", stream_id, transaction_update.slot);
                    let processing_result =
                        process_transaction_update(config, transaction_update).await;
                    let processing_time_us = start_time.elapsed().as_micros() as u64;

                    // Record metrics
                    histogram!("geyser_transaction_processing_time_us")
                        .record(processing_time_us as f64);
                    counter!("geyser_transactions_processed_total").increment(1);

                    if let Some(counter) = STREAM_COUNTERS.get(&stream_id) {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                    
                    // Increment rate counter
                    if let Some(rate_counter) = STREAM_RATE_COUNTERS.get(&stream_id) {
                        rate_counter.increment();
                    }

                    // Update health
                    if let Some(health) = STREAM_HEALTH.get(&stream_id) {
                        health.store(chrono::Utc::now().timestamp() as u64, Ordering::Relaxed);
                    }

                    match processing_result {
                        Ok(Some(metrics)) => {
                            counter!("geyser_transactions_successful_total").increment(1);
                            println!("✅ Stream {} SUCCESS: {} tx {} ({}μs)", stream_id, metrics.dex_name, &metrics.signature[0..8], processing_time_us);

                            // Update DEX counters
                            DEX_COUNTERS
                                .entry(metrics.dex_name.clone())
                                .or_insert_with(|| AtomicU64::new(0))
                                .fetch_add(1, Ordering::Relaxed);
                                
                            // Update DEX rate counters
                            DEX_RATE_COUNTERS
                                .entry(metrics.dex_name.clone())
                                .or_insert_with(|| Arc::new(RateCounter::new(60)))
                                .increment();
                        }
                        Ok(None) => {
                            counter!("geyser_transactions_filtered_total").increment(1);
                        }
                        Err(e) => {
                            counter!("geyser_transactions_failed_total").increment(1);
                            error!(
                                "Failed to process transaction on stream {}: {}",
                                stream_id, e
                            );
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
                    if let Some(program_id) =
                        account_keys.get(instruction.program_id_index as usize)
                    {
                        program_ids.push(program_id.clone());
                    }
                }
            }
        }
    }

    Ok(program_ids)
}

// Update metrics continuously in background
async fn update_metrics_task() {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    
    loop {
        interval.tick().await;
        
        // Update stream counters as gauges
        for entry in STREAM_COUNTERS.iter() {
            let stream_id = entry.key();
            let count = entry.value().load(Ordering::Relaxed) as f64;
            metrics::gauge!("multi_geyser_stream_transactions_total", "stream_id" => stream_id.to_string()).set(count);
            
            // Update rate counter metrics
            if let Some(rate_counter) = STREAM_RATE_COUNTERS.get(stream_id) {
                let rate = rate_counter.rate_per_minute();
                metrics::gauge!("multi_geyser_stream_messages_per_minute", "stream_id" => stream_id.to_string()).set(rate);
            }
        }

        // Update DEX counters as gauges
        for entry in DEX_COUNTERS.iter() {
            let dex_name = entry.key();
            let count = entry.value().load(Ordering::Relaxed) as f64;
            metrics::gauge!("multi_geyser_dex_transactions_total", "dex" => dex_name.clone()).set(count);
            
            // Update DEX rate counter metrics
            if let Some(rate_counter) = DEX_RATE_COUNTERS.get(dex_name) {
                let rate = rate_counter.rate_per_minute();
                metrics::gauge!("multi_geyser_dex_messages_per_minute", "dex" => dex_name.clone()).set(rate);
            }
        }

        // Update health status
        let now = chrono::Utc::now().timestamp() as u64;
        for entry in STREAM_HEALTH.iter() {
            let stream_id = entry.key();
            let last_seen = entry.value().load(Ordering::Relaxed);
            let seconds_since_last = now.saturating_sub(last_seen) as f64;
            let healthy = if seconds_since_last < 30.0 { 1.0 } else { 0.0 };
            
            metrics::gauge!("multi_geyser_stream_seconds_since_last_message", "stream_id" => stream_id.to_string()).set(seconds_since_last);
            metrics::gauge!("multi_geyser_stream_healthy", "stream_id" => stream_id.to_string()).set(healthy);
        }
    }
}

async fn start_simple_metrics_server(port: u16) {
    use axum::{routing::get, Router};
    
    let bind_addr = format!("0.0.0.0:{}", port);
    info!("🔧 Starting simple metrics server on {}", bind_addr);
    
    let app = Router::new()
        .route("/metrics", get(serve_prometheus_metrics));
    
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_prometheus_metrics() -> String {
    let mut output = String::new();
    
    // Stream counters and rates
    for entry in STREAM_COUNTERS.iter() {
        let stream_id = entry.key();
        let count = entry.value().load(Ordering::Relaxed);
        
        // Total messages
        output.push_str(&format!("# TYPE multi_geyser_stream_messages_total counter\n"));
        output.push_str(&format!("multi_geyser_stream_messages_total{{stream_id=\"{}\"}} {}\n", stream_id, count));
        
        // Messages per minute rate
        if let Some(rate_counter) = STREAM_RATE_COUNTERS.get(stream_id) {
            let rate = rate_counter.rate_per_minute();
            output.push_str(&format!("# TYPE multi_geyser_stream_messages_per_minute gauge\n"));
            output.push_str(&format!("multi_geyser_stream_messages_per_minute{{stream_id=\"{}\"}} {:.2}\n", stream_id, rate));
        }
    }
    
    // DEX counters and rates
    for entry in DEX_COUNTERS.iter() {
        let dex_name = entry.key();
        let count = entry.value().load(Ordering::Relaxed);
        
        // Total DEX messages
        output.push_str(&format!("# TYPE multi_geyser_dex_messages_total counter\n"));
        output.push_str(&format!("multi_geyser_dex_messages_total{{dex=\"{}\"}} {}\n", dex_name, count));
        
        // DEX messages per minute rate
        if let Some(rate_counter) = DEX_RATE_COUNTERS.get(dex_name) {
            let rate = rate_counter.rate_per_minute();
            output.push_str(&format!("# TYPE multi_geyser_dex_messages_per_minute gauge\n"));
            output.push_str(&format!("multi_geyser_dex_messages_per_minute{{dex=\"{}\"}} {:.2}\n", dex_name, rate));
        }
    }
    
    // Stream health
    for entry in STREAM_HEALTH.iter() {
        let stream_id = entry.key();
        let last_seen = entry.value().load(Ordering::Relaxed);
        let now = chrono::Utc::now().timestamp() as u64;
        let seconds_since_last = if last_seen > 0 { now - last_seen } else { u64::MAX };
        let healthy = if seconds_since_last < 30 { 1.0 } else { 0.0 };
        
        output.push_str(&format!("# TYPE multi_geyser_stream_healthy gauge\n"));
        output.push_str(&format!("multi_geyser_stream_healthy{{stream_id=\"{}\"}} {}\n", stream_id, healthy));
        
        output.push_str(&format!("# TYPE multi_geyser_stream_seconds_since_last_message gauge\n"));
        output.push_str(&format!("multi_geyser_stream_seconds_since_last_message{{stream_id=\"{}\"}} {}\n", stream_id, seconds_since_last));
    }
    
    output
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenv::dotenv().ok();
    
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    let mut args = Args::parse();
    
    // Override with environment variables if they exist
    if let Ok(endpoint) = std::env::var("GEYSER_ENDPOINT") {
        args.geyser_endpoint = endpoint;
    }
    if let Ok(token) = std::env::var("X_TOKEN") {
        args.x_token = token;
    }
    if let Ok(port) = std::env::var("METRICS_PORT") {
        if let Ok(port_num) = port.parse::<u16>() {
            args.metrics_port = port_num;
        }
    }

    env_logger::Builder::from_default_env()
        .filter_level(if args.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    // Initialize metrics FIRST, before everything else
    info!("🔧 Initializing metrics server on port {}", args.metrics_port);
    
    // Start simple custom metrics server instead of broken PrometheusBuilder
    let metrics_port = args.metrics_port;
    tokio::spawn(async move {
        start_simple_metrics_server(metrics_port).await;
    });
    
    info!("✅ Metrics server initialized and running on port {}", args.metrics_port);

    info!("🚀 Starting Multi-Geyser Streams Service");
    info!("📡 Endpoint: {}", args.geyser_endpoint);
    info!("📊 Metrics port: {}", args.metrics_port);

    // Metrics will be initialized by start_metrics_server

    // Create stream configurations
    let stream_configs = create_stream_configs(&args.geyser_endpoint, &args.x_token);

    info!("🔧 Configured {} Geyser streams:", stream_configs.len());
    for config in &stream_configs {
        info!(
            "  Stream {}: {} ({} program IDs)",
            config.id,
            config.name,
            config.program_ids.len()
        );
    }

    // Metrics server is already initialized above

    // Start background metrics update task
    let metrics_update_handle = tokio::spawn(async move {
        update_metrics_task().await;
    });

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

    // Start a test metric counter to verify metrics are working
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let mut counter = 0u64;
        loop {
            interval.tick().await;
            counter += 1;
            metrics::counter!("test_heartbeat_total").increment(1);
            metrics::gauge!("test_uptime_seconds").set(counter as f64);
            if counter % 30 == 0 {
                info!("Heartbeat: {} seconds, emitted test metrics", counter);
                info!("Emitted metrics: heartbeat={}, uptime={}", counter, counter);
            }
        }
    });

    // Wait for all tasks (they run forever)
    tokio::select! {
        result = futures::future::try_join_all(stream_handles) => {
            if let Err(e) = result {
                error!("Stream error: {}", e);
            }
        }
        result = metrics_update_handle => {
            if let Err(e) = result {
                error!("Metrics update error: {}", e);
            }
        }
    }

    Ok(())
}
