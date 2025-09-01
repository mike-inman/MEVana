//! Ultra-Low Latency DEX Stream Integrated Service
//!
//! This service demonstrates the single-process, in-memory architecture
//! for ultra-low latency arbitrage detection

use anyhow::Result;
use axum::{response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use clap::Parser;
use crossbeam::channel::{bounded, Receiver, Sender};
use dashmap::DashMap;
use log::{error, info};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::time::Duration;
use uuid::Uuid;

// Global shared state
lazy_static::lazy_static! {
    static ref POOL_STATES: DashMap<String, PoolState> = DashMap::new();
    static ref TX_CHANNEL: (Sender<TransactionData>, Receiver<TransactionData>) = bounded(10000);
    static ref ARB_CHANNEL: (Sender<ArbitrageSignal>, Receiver<ArbitrageSignal>) = bounded(1000);
    static ref HEALTH_STATUS: Arc<tokio::sync::RwLock<HealthStatus>> = Arc::new(tokio::sync::RwLock::new(HealthStatus::default()));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceConfig {
    endpoint: String,
    x_token: String,
    dex_filtering: bool,
    program_ids: Vec<String>,
    health_port: u16,
    bind_address: String,
    min_profit_bps: u64,
    max_slippage_bps: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://grpc.ny.shyft.to".to_string(),
            x_token: "efff2d23-97ae-4c5f-8b8b-9a6265001189".to_string(),
            dex_filtering: false, // Temporarily disable filtering to see ALL transactions
            program_ids: vec![
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(), // Whirlpool
                "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB".to_string(), // Jupiter V6
                "JUP6LkbZbafMNUBhDSDueet2ApgUKPKodDhqWFLVvZpg".to_string(), // Jupiter Swap
                "jupoNjAxXgZ4rjzxzPMP4oxduvQsQtZzyknqvzYNrNu".to_string(), // Jupiter Limit Order
            ],
            health_port: 8084,
            bind_address: "0.0.0.0".to_string(),
            min_profit_bps: 50,
            max_slippage_bps: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoolState {
    pool_address: String,
    token_a: String,
    token_b: String,
    liquidity: u128,
    sqrt_price: u128,
    tick_index: i32,
    fee_rate: u64,
    last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionData {
    signature: String,
    slot: u64,
    program_id: String,
    dex_name: String,
    raw_data: String,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArbitrageSignal {
    id: String,
    timestamp: DateTime<Utc>,
    profit_bps: u64,
    route: Vec<String>,
    confidence: f64,
}

#[derive(Debug)]
struct HealthStatus {
    last_geyser_update: Option<DateTime<Utc>>,
    last_arbitrage_signal: Option<DateTime<Utc>>,
    total_transactions_processed: u64,
    total_arbitrage_signals: u64,
    uptime_start: DateTime<Utc>,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            last_geyser_update: None,
            last_arbitrage_signal: None,
            total_transactions_processed: 0,
            total_arbitrage_signals: 0,
            uptime_start: Utc::now(),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "ENDPOINT")]
    endpoint: Option<String>,

    #[arg(long, env = "X_TOKEN")]
    x_token: Option<String>,

    #[arg(long, env = "DEX_FILTERING", default_value = "true")]
    dex_filtering: bool,

    #[arg(long, env = "PROGRAM_IDS")]
    program_ids: Option<String>,

    #[arg(long, env = "HEALTH_PORT", default_value = "8084")]
    health_port: u16,

    #[arg(long, env = "BIND_ADDRESS", default_value = "0.0.0.0")]
    bind_address: String,

    #[arg(long, env = "MIN_PROFIT_BPS", default_value = "50")]
    min_profit_bps: u64,

    #[arg(long, env = "MAX_SLIPPAGE_BPS", default_value = "100")]
    max_slippage_bps: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    // Initialize configuration
    let mut config = ServiceConfig::default();

    if let Some(endpoint) = args.endpoint {
        config.endpoint = endpoint;
    }

    if let Some(x_token) = args.x_token {
        config.x_token = x_token;
    }

    config.dex_filtering = args.dex_filtering;
    config.health_port = args.health_port;
    config.bind_address = args.bind_address;
    config.min_profit_bps = args.min_profit_bps;
    config.max_slippage_bps = args.max_slippage_bps;

    if let Some(program_ids) = args.program_ids {
        config.program_ids = program_ids
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
    }

    info!("🚀 Starting Ultra-Low Latency DEX Stream Integrated Service");
    info!("🔧 Configuration: {:?}", config);

    // Clone values before spawning tasks
    let bind_address = config.bind_address.clone();
    let health_port = config.health_port;
    let config_arc = Arc::new(config);

    // Spawn background tasks
    let geyser_handle = {
        let config = config_arc.clone();
        tokio::spawn(async move {
            real_geyser_stream_loop(config).await;
        })
    };

    let health_server_handle = {
        let config = config_arc.clone();
        tokio::spawn(async move {
            start_health_server(config).await;
        })
    };

    let arbitrage_handle = {
        let config = config_arc.clone();
        tokio::spawn(async move {
            arbitrage_engine_loop(config).await;
        })
    };

    let persistence_handle = tokio::spawn(async move {
        persistence_layer().await;
    });

    let health_checker_handle = tokio::spawn(async move {
        background_health_checker().await;
    });

    info!(
        "📈 Metrics endpoint: http://{}:{}/metrics",
        bind_address, health_port
    );
    info!(
        "🏥 Health endpoint: http://{}:{}/health",
        bind_address, health_port
    );

    // Wait for all tasks
    tokio::try_join!(
        geyser_handle,
        health_server_handle,
        arbitrage_handle,
        persistence_handle,
        health_checker_handle
    )?;

    Ok(())
}

async fn real_geyser_stream_loop(config: Arc<ServiceConfig>) -> Result<()> {
    info!("📡 Starting real Geyser stream loop...");

    use std::collections::HashMap;
    use yellowstone_grpc_client::GeyserGrpcClient;
    use yellowstone_grpc_proto::{
        geyser::SubscribeRequestFilterTransactions,
        prelude::{subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest},
    };

    let mut client = GeyserGrpcClient::build_from_shared(config.endpoint.clone())
        .map_err(|e| anyhow::anyhow!(e))?
        .x_token(Some(config.x_token.clone()))
        .map_err(|e| anyhow::anyhow!(e))?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(10))
        .connect()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // Build request for ALL transactions (no filtering)
    let mut transactions: HashMap<String, SubscribeRequestFilterTransactions> = HashMap::new();
    transactions.insert(
        "client".to_owned(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![], // Empty means ALL accounts
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
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: vec![],
        ping: None,
        from_slot: None,
    };

    let (_, mut stream) = client.subscribe_with_request(Some(request)).await?;

    println!(
        "✅ SUCCESSFULLY CONNECTED to Geyser stream at {}",
        config.endpoint
    );
    info!("🔗 Connected to Geyser stream at {}", config.endpoint);

    use futures_util::StreamExt;

    println!("🚀 Starting real Geyser stream - watching for ALL transactions...");
    println!("🔍 Monitoring program IDs: {:?}", config.program_ids);
    println!("⚡ DEX filtering: {}", config.dex_filtering);
    println!("🔗 Connecting to: {}", config.endpoint);

    let mut message_count = 0;
    println!("🔄 Starting message loop...");

    while let Some(message) = stream.next().await {
        message_count += 1;
        if message_count % 100 == 0 {
            println!("📊 Processed {} messages so far...", message_count);
        }

        match message {
            Ok(update_msg) => {
                match update_msg.update_oneof {
                    Some(UpdateOneof::Transaction(transaction_update)) => {
                        // Extract signature from transaction
                        let signature = if let Some(tx_info) = &transaction_update.transaction {
                            bs58::encode(&tx_info.signature).into_string()
                        } else {
                            "unknown".to_string()
                        };

                        // Extract program IDs from instructions
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

                        // Filter by DEX program IDs if enabled
                        let should_process = if config.dex_filtering {
                            program_ids
                                .iter()
                                .any(|pid| config.program_ids.contains(pid))
                        } else {
                            true
                        };

                        if should_process {
                            let dex_name = if program_ids
                                .iter()
                                .any(|pid| pid == "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc")
                            {
                                "whirlpool".to_string()
                            } else if program_ids
                                .iter()
                                .any(|pid| pid == "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB")
                            {
                                "jupiter".to_string()
                            } else {
                                "unknown".to_string()
                            };

                            let tx_data = TransactionData {
                                signature: signature.clone(),
                                slot: transaction_update.slot,
                                program_id: program_ids
                                    .first()
                                    .unwrap_or(&"unknown".to_string())
                                    .clone(),
                                dex_name: dex_name.clone(),
                                raw_data: format!("Transaction data for {}", signature),
                                timestamp: Utc::now(),
                            };

                            // Send to arbitrage engine via in-memory channel
                            if let Err(e) = TX_CHANNEL.0.send(tx_data.clone()) {
                                error!("Failed to send transaction to arbitrage engine: {}", e);
                            }

                            // Update health status
                            {
                                let mut health = HEALTH_STATUS.write().await;
                                health.last_geyser_update = Some(Utc::now());
                                health.total_transactions_processed += 1;
                            }

                            // Console output for development feedback
                            println!(
                                "🔥 LIVE TRANSACTION: {} | Slot: {} | DEX: {} | Programs: {:?}",
                                signature, transaction_update.slot, dex_name, program_ids
                            );

                            info!(
                                "📊 Real transaction: {} from {} (DEX: {})",
                                tx_data.signature, tx_data.program_id, tx_data.dex_name
                            );
                        } else {
                            // Console output for filtered transactions
                            if program_ids.len() > 0 {
                                println!(
                                    "⏭️  FILTERED: {} | Slot: {} | Programs: {:?}",
                                    signature, transaction_update.slot, program_ids
                                );
                            }
                        }

                        // Show raw message for all transactions
                        println!(
                            "📊 RAW TX: {} | Slot: {} | Raw: {:?}",
                            signature, transaction_update.slot, transaction_update
                        );
                    }
                    Some(UpdateOneof::Ping(_ping)) => {
                        println!("🏓 Ping received");
                        info!("🏓 Ping received");
                    }
                    Some(UpdateOneof::Pong(pong)) => {
                        println!("🏓 Pong received: id {}", pong.id);
                        info!("🏓 Pong received: id {}", pong.id);
                    }
                    _ => {
                        // Other message types
                        println!("📡 Other message type received");
                    }
                }
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
                println!("❌ Error receiving message: {}", e);
            }
        }
    }

    Ok(())
}

async fn arbitrage_engine_loop(config: Arc<ServiceConfig>) -> Result<()> {
    info!("🧠 Starting arbitrage engine loop...");

    let mut detector = ArbitrageDetector::new(config.min_profit_bps, config.max_slippage_bps);

    while let Ok(transaction) = TX_CHANNEL.1.recv() {
        if let Some(signal) = detector.process_transaction(transaction)? {
            if let Err(e) = ARB_CHANNEL.0.send(signal.clone()) {
                error!("Failed to send arbitrage signal: {}", e);
            }

            // Update health status
            {
                let mut health = HEALTH_STATUS.write().await;
                health.last_arbitrage_signal = Some(Utc::now());
                health.total_arbitrage_signals += 1;
            }

            info!(
                "💰 Arbitrage signal detected: {} bps profit",
                signal.profit_bps
            );
        }
    }

    Ok(())
}

async fn persistence_layer() -> Result<()> {
    info!("💾 Starting persistence layer...");

    while let Ok(signal) = ARB_CHANNEL.1.recv() {
        // TODO: Implement async persistence to Kafka/Redis/PostgreSQL
        // This runs in the background and doesn't block the hotpath

        info!("💾 Persisting arbitrage signal: {}", signal.id);

        // Simulate async persistence
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            // TODO: Send to Kafka, Redis, PostgreSQL
        });
    }

    Ok(())
}

async fn background_health_checker() -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        let health = HEALTH_STATUS.read().await;
        let uptime = Utc::now().signed_duration_since(health.uptime_start);

        info!(
            "🏥 Health check - Uptime: {}s, TX processed: {}, Signals: {}",
            uptime.num_seconds(),
            health.total_transactions_processed,
            health.total_arbitrage_signals
        );
    }
}

async fn start_health_server(config: Arc<ServiceConfig>) -> Result<()> {
    use axum::{routing::get, Router};
    use tokio::net::TcpListener;

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler));

    let bind_address = config.bind_address.clone();
    let health_port = config.health_port;

    let listener = TcpListener::bind(format!("{}:{}", bind_address, health_port)).await?;
    let addr = listener.local_addr()?;

    info!("🏥 Health server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> impl IntoResponse {
    let health = HEALTH_STATUS.read().await;
    let uptime = Utc::now().signed_duration_since(health.uptime_start);

    Json(json!({
        "status": "healthy",
        "uptime_seconds": uptime.num_seconds(),
        "last_geyser_update": health.last_geyser_update,
        "last_arbitrage_signal": health.last_arbitrage_signal,
        "total_transactions_processed": health.total_transactions_processed,
        "total_arbitrage_signals": health.total_arbitrage_signals
    }))
}

async fn metrics_handler() -> impl IntoResponse {
    let health = HEALTH_STATUS.read().await;
    let uptime = Utc::now().signed_duration_since(health.uptime_start);

    format!(
        "# HELP mevana_uptime_seconds Total uptime in seconds\n\
         # TYPE mevana_uptime_seconds counter\n\
         mevana_uptime_seconds {}\n\
         # HELP mevana_transactions_processed_total Total transactions processed\n\
         # TYPE mevana_transactions_processed_total counter\n\
         mevana_transactions_processed_total {}\n\
         # HELP mevana_arbitrage_signals_total Total arbitrage signals detected\n\
         # TYPE mevana_arbitrage_signals_total counter\n\
         mevana_arbitrage_signals_total {}\n",
        uptime.num_seconds(),
        health.total_transactions_processed,
        health.total_arbitrage_signals
    )
}

struct ArbitrageDetector {
    min_profit_bps: u64,
    max_slippage_bps: u64,
}

impl ArbitrageDetector {
    fn new(min_profit_bps: u64, max_slippage_bps: u64) -> Self {
        Self {
            min_profit_bps,
            max_slippage_bps,
        }
    }

    fn process_transaction(
        &mut self,
        _transaction: TransactionData,
    ) -> Result<Option<ArbitrageSignal>> {
        // TODO: Implement actual arbitrage detection logic
        // This is a stub that will be replaced with real arbitrage detection

        // Simulate occasional arbitrage signals for testing
        let mut rng = rand::thread_rng();
        if rng.gen::<u8>() < 5 {
            // 5% chance
            Ok(Some(ArbitrageSignal {
                id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                profit_bps: self.min_profit_bps + rng.gen::<u64>() % 100,
                route: vec!["whirlpool".to_string(), "orca".to_string()],
                confidence: 0.85,
            }))
        } else {
            Ok(None)
        }
    }
}
