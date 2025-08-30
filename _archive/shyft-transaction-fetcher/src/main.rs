use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use dotenv::dotenv;
use env_logger;
use log::{error, info, warn};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "shyft-transaction-fetcher")]
#[command(about = "Fetches decoded transactions from Shyft API and sends to Kafka")]
struct Args {
    /// Transaction signature to fetch
    #[arg(short, long)]
    signature: Option<String>,

    /// Number of recent transactions to fetch from a wallet
    #[arg(short, long, default_value = "5")]
    count: u32,

    /// Wallet address to fetch recent transactions from
    #[arg(short, long)]
    wallet: Option<String>,

    /// Network to use (mainnet-beta, devnet, testnet)
    #[arg(short, long, default_value = "mainnet-beta")]
    network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceConfig {
    api_key: String,
    network: String,
    kafka_broker: String,
    output_topic: String,
    health_port: u16,
    bind_address: String,
}

impl ServiceConfig {
    fn from_env() -> Result<Self> {
        let api_key = std::env::var("SHYFT_API_KEY")
            .map_err(|_| anyhow::anyhow!("SHYFT_API_KEY environment variable is required"))?;

        let network = std::env::var("SHYFT_NETWORK")
            .unwrap_or_else(|_| "mainnet-beta".to_string());

        let kafka_broker = std::env::var("KAFKA_BROKER")
            .unwrap_or_else(|_| "localhost:9092".to_string());

        let output_topic = std::env::var("OUTPUT_TOPIC")
            .unwrap_or_else(|_| "shyft-transactions-parsed".to_string());

        let health_port = std::env::var("HEALTH_PORT")
            .unwrap_or_else(|_| "8087".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("HEALTH_PORT must be a valid port number"))?;

        let bind_address = std::env::var("BIND_ADDRESS")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        Ok(Self {
            api_key,
            network,
            kafka_broker,
            output_topic,
            health_port,
            bind_address,
        })
    }

    fn log_config(&self) {
        info!("=== SHYFT TRANSACTION FETCHER CONFIGURATION ===");
        info!("Shyft Network: {}", self.network);
        info!("Kafka Broker: {}", self.kafka_broker);
        info!("Output Topic: {}", self.output_topic);
        info!("Health Port: {}", self.health_port);
        info!("Bind Address: {}", self.bind_address);
        info!("API Key: {}...", &self.api_key[..8.min(self.api_key.len())]);
        info!("===============================================");
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftTransactionResponse {
    success: bool,
    result: Option<ShyftTransaction>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftTransaction {
    signature: String,
    slot: u64,
    block_time: i64,
    fee: u64,
    status: String,
    err: Option<serde_json::Value>,
    memo: Option<String>,
    account_keys: Vec<ShyftAccountKey>,
    instructions: Vec<ShyftInstruction>,
    inner_instructions: Option<Vec<ShyftInnerInstruction>>,
    pre_balances: Vec<i64>,
    post_balances: Vec<i64>,
    pre_token_balances: Option<Vec<ShyftTokenBalance>>,
    post_token_balances: Option<Vec<ShyftTokenBalance>>,
    log_messages: Option<Vec<String>>,
    rewards: Option<Vec<ShyftReward>>,
    loaded_addresses: Option<ShyftLoadedAddresses>,
    return_data: Option<ShyftReturnData>,
    compute_units_consumed: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftAccountKey {
    pubkey: String,
    writable: bool,
    signer: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftInstruction {
    program_id: String,
    program: Option<String>,
    instruction: Option<String>,
    accounts: Vec<ShyftAccountMeta>,
    data: Option<String>,
    data_parsed: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftAccountMeta {
    pubkey: String,
    writable: bool,
    signer: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftInnerInstruction {
    index: u32,
    instructions: Vec<ShyftInstruction>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftTokenBalance {
    account_index: u32,
    mint: String,
    owner: Option<String>,
    program_id: Option<String>,
    ui_token_amount: ShyftUiTokenAmount,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftUiTokenAmount {
    token_amount: String,
    decimals: u8,
    ui_amount: Option<f64>,
    ui_amount_string: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftReward {
    pubkey: String,
    lamports: i64,
    post_balance: u64,
    reward_type: Option<String>,
    commission: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftLoadedAddresses {
    writable: Vec<String>,
    readonly: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftReturnData {
    program_id: String,
    data: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftTransactionHistoryResponse {
    success: bool,
    result: Option<Vec<ShyftTransaction>>,
    error: Option<String>,
}

/// Builds a complete, prettified JSON representation of the Shyft transaction
/// matching our debug output format for easy comparison
fn build_shyft_transaction_json_debug(
    tx: &ShyftTransaction,
    config: &ServiceConfig,
) -> Result<serde_json::Value> {
    let full_tx = serde_json::json!({
        "debug_label": "SHYFT_API_COMPLETE_TRANSACTION_PARSED",
        "timestamp": Utc::now().to_rfc3339(),
        "source": "shyft_api",
        "network": config.network,
        "slot": tx.slot,
        "signature": tx.signature,
        "block_time": tx.block_time,
        "status": tx.status,
        "fee": tx.fee,
        "err": tx.err,
        "memo": tx.memo,
        
        // COMPLETE TRANSACTION DATA
        "transaction": {
            "account_keys": tx.account_keys.iter().map(|ak| {
                serde_json::json!({
                    "address": ak.pubkey,
                    "is_writable": ak.writable,
                    "is_signer": ak.signer,
                })
            }).collect::<Vec<_>>(),
            
            "instructions": tx.instructions.iter().enumerate().map(|(idx, inst)| {
                serde_json::json!({
                    "index": idx,
                    "program_id": inst.program_id,
                    "program_name": inst.program,
                    "instruction_name": inst.instruction,
                    "accounts": inst.accounts.iter().map(|acc| {
                        serde_json::json!({
                            "address": acc.pubkey,
                            "is_writable": acc.writable,
                            "is_signer": acc.signer,
                        })
                    }).collect::<Vec<_>>(),
                    "data": inst.data,
                    "data_parsed": inst.data_parsed,
                })
            }).collect::<Vec<_>>(),
            
            "inner_instructions": tx.inner_instructions.as_ref().map(|inner| {
                inner.iter().map(|inner_inst| {
                    serde_json::json!({
                        "index": inner_inst.index,
                        "instructions": inner_inst.instructions.iter().enumerate().map(|(idx, inst)| {
                            serde_json::json!({
                                "index": idx,
                                "program_id": inst.program_id,
                                "program_name": inst.program,
                                "instruction_name": inst.instruction,
                                "accounts": inst.accounts.iter().map(|acc| {
                                    serde_json::json!({
                                        "address": acc.pubkey,
                                        "is_writable": acc.writable,
                                        "is_signer": acc.signer,
                                    })
                                }).collect::<Vec<_>>(),
                                "data": inst.data,
                                "data_parsed": inst.data_parsed,
                            })
                        }).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>()
            }),
        },
        
        // COMPLETE METADATA
        "meta": {
            "pre_balances": tx.pre_balances,
            "post_balances": tx.post_balances,
            "pre_token_balances": tx.pre_token_balances,
            "post_token_balances": tx.post_token_balances,
            "log_messages": tx.log_messages,
            "rewards": tx.rewards,
            "loaded_addresses": tx.loaded_addresses,
            "return_data": tx.return_data,
            "compute_units_consumed": tx.compute_units_consumed,
        },
        
        // COMPLETE ACCOUNT INFORMATION
        "account_details": tx.account_keys.iter().enumerate().map(|(idx, ak)| {
            serde_json::json!({
                "index": idx,
                "address": ak.pubkey,
                "is_writable": ak.writable,
                "is_signer": ak.signer,
            })
        }).collect::<Vec<_>>(),
    });

    Ok(full_tx)
}

/// Fetch a single transaction by signature from Shyft API
async fn fetch_transaction_by_signature(
    client: &Client,
    config: &ServiceConfig,
    signature: &str,
) -> Result<Option<ShyftTransaction>> {
    let url = format!(
        "https://api.shyft.to/sol/v1/transaction/parsed?api_key={}&network={}&tx_signature={}",
        config.api_key, config.network, signature
    );

    info!("🔍 Fetching transaction from Shyft: {}", signature);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to Shyft API")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        warn!("⚠️  Shyft API error ({}): {}", status, error_text);
        return Ok(None);
    }

    let tx_response: ShyftTransactionResponse = response
        .json()
        .await
        .context("Failed to parse Shyft API response")?;

    if !tx_response.success {
        warn!("⚠️  Shyft API returned success=false: {:?}", tx_response.error);
        return Ok(None);
    }

    Ok(tx_response.result)
}

/// Fetch recent transaction history from a wallet
async fn fetch_transaction_history(
    client: &Client,
    config: &ServiceConfig,
    wallet: &str,
    count: u32,
) -> Result<Vec<ShyftTransaction>> {
    let url = format!(
        "https://api.shyft.to/sol/v1/transaction/history?api_key={}&network={}&account={}&tx_num={}",
        config.api_key, config.network, wallet, count
    );

    info!("🔍 Fetching {} recent transactions from wallet: {}", count, wallet);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to Shyft API")?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        warn!("⚠️  Shyft API error ({}): {}", status, error_text);
        return Ok(vec![]);
    }

    let history_response: ShyftTransactionHistoryResponse = response
        .json()
        .await
        .context("Failed to parse Shyft API response")?;

    if !history_response.success {
        warn!("⚠️  Shyft API returned success=false: {:?}", history_response.error);
        return Ok(vec![]);
    }

    Ok(history_response.result.unwrap_or_default())
}

/// Send transaction data to Kafka topic
async fn send_to_kafka(
    producer: &FutureProducer,
    topic: &str,
    signature: &str,
    data: &serde_json::Value,
) -> Result<()> {
    let key = format!("shyft_{}", signature);
    let payload = serde_json::to_vec(data)?;

    let _ = producer
        .send(
            FutureRecord::to(topic).key(&key).payload(&payload),
            Duration::from_secs(5),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send to Kafka: {:?}", e))?;

    info!("📤 Sent Shyft transaction to Kafka topic '{}' for signature: {}", topic, signature);
    Ok(())
}

/// Start health check server
async fn start_health_server(config: &ServiceConfig) -> Result<()> {
    use axum::{
        routing::get,
        Router,
        http::StatusCode,
        response::Json,
    };
    use serde_json::json;
    use tokio::net::TcpListener;

    let app = Router::new()
        .route("/health", get(|| async { 
            (StatusCode::OK, Json(json!({ "status": "healthy", "service": "shyft-transaction-fetcher" })))
        }))
        .route("/metrics", get(|| async { 
            (StatusCode::OK, Json(json!({ "status": "metrics endpoint ready" })))
        }));

    let listener = TcpListener::bind((config.bind_address.as_str(), config.health_port)).await
        .context("Failed to bind health server")?;

    info!("🚀 Health server started on {}:{}", config.bind_address, config.health_port);
    
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Health server error: {}", e);
        }
    });

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    info!("🚀 Starting Shyft Transaction Fetcher Service...");

    // Load configuration
    let config = ServiceConfig::from_env()
        .context("Failed to load configuration")?;
    
    config.log_config();

    // Start health server
    start_health_server(&config).await?;

    // Parse command line arguments
    let args = Args::parse();

    // Create HTTP client
    let client = Client::new();

    // Create Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_broker)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka producer")?;

    info!("✅ Kafka producer created successfully");

    // Process based on arguments
    if let Some(signature) = args.signature {
        // Fetch single transaction by signature
        if let Some(tx) = fetch_transaction_by_signature(&client, &config, &signature).await? {
            let json_data = build_shyft_transaction_json_debug(&tx, &config)?;
            
            // Send to Kafka
            send_to_kafka(&producer, &config.output_topic, &signature, &json_data).await?;
            
            // Also print to stdout for inspection
            println!("{}", serde_json::to_string_pretty(&json_data)?);
        } else {
            warn!("⚠️  No transaction found for signature: {}", signature);
        }
    } else if let Some(wallet) = args.wallet {
        // Fetch recent transaction history from wallet
        let transactions = fetch_transaction_history(&client, &config, &wallet, args.count).await?;
        
        info!("📊 Found {} transactions for wallet: {}", transactions.len(), wallet);
        
        for tx in transactions {
            let json_data = build_shyft_transaction_json_debug(&tx, &config)?;
            
            // Send to Kafka
            send_to_kafka(&producer, &config.output_topic, &tx.signature, &json_data).await?;
            
            // Also print to stdout for inspection
            println!("{}", serde_json::to_string_pretty(&json_data)?);
            
            // Rate limiting - be respectful to Shyft API
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    } else {
        info!("💡 No specific transaction or wallet specified. Use --signature or --wallet to fetch data.");
        info!("💡 Examples:");
        info!("   --signature <TX_SIGNATURE>     # Fetch specific transaction");
        info!("   --wallet <WALLET_ADDRESS>      # Fetch recent transactions from wallet");
        info!("   --count 10                     # Number of transactions to fetch (default: 5)");
    }

    info!("✅ Shyft Transaction Fetcher service ready!");
    info!("💡 Service can be called independently or integrated with other services");

    // Keep the service running for health checks
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        info!("💓 Service heartbeat - running normally");
    }
}
