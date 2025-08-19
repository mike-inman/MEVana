use anyhow::{Context, Result};
use log::{error, info, warn};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

mod program_ids;
mod swap_watcher;

use swap_watcher::{SwapWatcher, SwapWatcherConfig};

// Enhanced configuration structure
#[derive(Debug, Clone)]
struct ServiceConfig {
    api_key: String,
    network: String,
    health_port: u16,
    bind_address: String,
    kafka_broker: String,
}

impl ServiceConfig {
    fn from_env() -> Result<Self> {
        let api_key = env::var("SHYFT_API_KEY")
            .map_err(|_| anyhow::anyhow!("SHYFT_API_KEY environment variable is required"))?;

        let network = env::var("SHYFT_NETWORK").unwrap_or_else(|_| "mainnet-beta".to_string());

        let health_port = env::var("HEALTH_PORT")
            .unwrap_or_else(|_| "8086".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("HEALTH_PORT must be a valid port number"))?;

        let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());

        let kafka_broker =
            env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:9092".to_string());

        Ok(Self {
            api_key,
            network,
            health_port,
            bind_address,
            kafka_broker,
        })
    }

    fn log_config(&self) {
        info!("=== SHYFT VALIDATOR SERVICE CONFIGURATION ===");
        info!("Shyft Network: {}", self.network);
        info!("Health Port: {}", self.health_port);
        info!("Bind Address: {}", self.bind_address);
        info!("Kafka Broker: {}", self.kafka_broker);
        info!("API Key: {}...", &self.api_key[..8.min(self.api_key.len())]);
        info!("=============================================");
    }
}

// Enhanced Shyft API response structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShyftPoolData {
    pub tick_current_index: i32,
    pub sqrt_price: String,
    pub liquidity: String,
    pub fee_rate: i64,
    pub protocol: String,
    pub pool_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShyftTokenBalance {
    pub mint: String,
    pub balance: String,
    pub decimals: u8,
    pub ui_balance: f64,
    pub owner: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShyftPoolDiscovery {
    pub pool_address: String,
    pub protocol: String,
    pub token_a: String,
    pub token_b: String,
    pub liquidity: String,
    pub fee_rate: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShyftTransactionParsed {
    pub signature: String,
    pub slot: u64,
    pub block_time: i64,
    pub fee: u64,
    pub status: String,
    pub instructions: Vec<ShyftInstruction>,
    pub account_keys: Vec<ShyftAccountKey>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShyftInstruction {
    pub program_id: String,
    pub program: Option<String>,
    pub instruction: Option<String>,
    pub accounts: Vec<ShyftAccountMeta>,
    pub data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShyftAccountKey {
    pub pubkey: String,
    pub writable: bool,
    pub signer: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShyftAccountMeta {
    pub pubkey: String,
    pub writable: bool,
    pub signer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub pool_address: String,
    pub our_parsing: String,
    pub shyft_data: Option<ShyftPoolData>,
    pub matches: bool,
    pub timestamp: i64,
    pub validation_duration_ms: u64,
}

#[derive(Clone)]
pub struct ShyftValidator {
    api_key: String,
    network: String,
    client: Client,
    producer: FutureProducer,
}

impl ShyftValidator {
    pub fn new(api_key: String, network: String, producer: FutureProducer) -> Self {
        Self {
            api_key,
            network,
            client: Client::new(),
            producer,
        }
    }

    /// Get pool liquidity details from Shyft API
    pub async fn get_pool_liquidity_details(
        &self,
        pool_address: &str,
    ) -> Result<Option<ShyftPoolData>> {
        let url = format!(
            "https://api.shyft.to/sol/v1/pool/get_pool?network={}&pool_address={}",
            self.network, pool_address
        );

        info!("🔍 Fetching pool liquidity details: {}", pool_address);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .context("Failed to send request to Shyft pool API")?;

        if !response.status().is_success() {
            warn!(
                "⚠️  Shyft pool API error ({}): {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
            return Ok(None);
        }

        // Parse response and extract pool data
        let pool_data = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse Shyft pool API response")?;

        // Extract relevant pool information
        if let Some(result) = pool_data.get("result") {
            let pool = ShyftPoolData {
                tick_current_index: result
                    .get("tickCurrentIndex")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                sqrt_price: result
                    .get("sqrtPrice")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                liquidity: result
                    .get("liquidity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                fee_rate: result.get("feeRate").and_then(|v| v.as_i64()).unwrap_or(0),
                protocol: result
                    .get("protocol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                pool_address: pool_address.to_string(),
            };
            return Ok(Some(pool));
        }

        Ok(None)
    }

    /// Get token balance from Shyft API
    pub async fn get_token_balance(
        &self,
        mint: &str,
        owner: &str,
    ) -> Result<Option<ShyftTokenBalance>> {
        let url = format!(
            "https://api.shyft.to/sol/v1/token/get_balance?network={}&mint={}&owner={}",
            self.network, mint, owner
        );

        info!("🔍 Fetching token balance: mint={}, owner={}", mint, owner);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .context("Failed to send request to Shyft token API")?;

        if !response.status().is_success() {
            warn!(
                "⚠️  Shyft token API error ({}): {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
            return Ok(None);
        }

        let balance_data = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse Shyft token API response")?;

        if let Some(result) = balance_data.get("result") {
            let balance = ShyftTokenBalance {
                mint: mint.to_string(),
                balance: result
                    .get("balance")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string(),
                decimals: result.get("decimals").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                ui_balance: result
                    .get("ui_balance")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                owner: owner.to_string(),
            };
            return Ok(Some(balance));
        }

        Ok(None)
    }

    /// Discover pools for a token pair
    pub async fn discover_pools(
        &self,
        token_a: &str,
        token_b: &str,
    ) -> Result<Vec<ShyftPoolDiscovery>> {
        let url = format!(
            "https://api.shyft.to/sol/v1/pool/get_pools_by_liquidity_pair?network={}&token_a={}&token_b={}",
            self.network, token_a, token_b
        );

        info!("🔍 Discovering pools for pair: {}/{}", token_a, token_b);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .context("Failed to send request to Shyft pool discovery API")?;

        if !response.status().is_success() {
            warn!(
                "⚠️  Shyft pool discovery API error ({}): {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
            return Ok(vec![]);
        }

        let discovery_data = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse Shyft pool discovery API response")?;

        let mut pools = Vec::new();
        if let Some(result) = discovery_data.get("result") {
            if let Some(pools_array) = result.as_array() {
                for pool in pools_array {
                    let pool_discovery = ShyftPoolDiscovery {
                        pool_address: pool
                            .get("pool_address")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        protocol: pool
                            .get("protocol")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        token_a: token_a.to_string(),
                        token_b: token_b.to_string(),
                        liquidity: pool
                            .get("liquidity")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0")
                            .to_string(),
                        fee_rate: pool.get("fee_rate").and_then(|v| v.as_i64()).unwrap_or(0),
                    };
                    pools.push(pool_discovery);
                }
            }
        }

        Ok(pools)
    }

    /// Get parsed transaction data
    pub async fn get_transaction_parsed(
        &self,
        signature: &str,
    ) -> Result<Option<ShyftTransactionParsed>> {
        let url = format!(
            "https://api.shyft.to/sol/v1/transaction/parsed?network={}&txn_signature={}&commitment=finalized",
            self.network, signature
        );

        info!("🔍 Fetching parsed transaction: {}", signature);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .context("Failed to send request to Shyft transaction API")?;

        if !response.status().is_success() {
            warn!(
                "⚠️  Shyft transaction API error ({}): {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
            return Ok(None);
        }

        let tx_data = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse Shyft transaction API response")?;

        if let Some(result) = tx_data.get("result") {
            let tx = ShyftTransactionParsed {
                signature: signature.to_string(),
                slot: result.get("slot").and_then(|v| v.as_u64()).unwrap_or(0),
                block_time: result
                    .get("block_time")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                fee: (result.get("fee").and_then(|v| v.as_f64()).unwrap_or(0.0) * 1_000_000_000.0)
                    as u64, // Convert SOL to lamports
                status: result
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                instructions: self.parse_instructions(result.get("instructions"))?,
                account_keys: self.parse_account_keys(result.get("account_keys"))?,
            };
            return Ok(Some(tx));
        }

        Ok(None)
    }

    fn parse_instructions(
        &self,
        instructions: Option<&serde_json::Value>,
    ) -> Result<Vec<ShyftInstruction>> {
        let mut parsed = Vec::new();
        if let Some(instructions_array) = instructions.and_then(|v| v.as_array()) {
            for inst in instructions_array {
                let instruction = ShyftInstruction {
                    program_id: inst
                        .get("program_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    program: inst
                        .get("program")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    instruction: inst
                        .get("instruction")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    accounts: self.parse_account_meta(inst.get("accounts"))?,
                    data: inst
                        .get("data")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };
                parsed.push(instruction);
            }
        }
        Ok(parsed)
    }

    fn parse_account_keys(
        &self,
        account_keys: Option<&serde_json::Value>,
    ) -> Result<Vec<ShyftAccountKey>> {
        let mut parsed = Vec::new();
        if let Some(accounts_array) = account_keys.and_then(|v| v.as_array()) {
            for acc in accounts_array {
                let account = ShyftAccountKey {
                    pubkey: acc
                        .get("pubkey")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    writable: acc
                        .get("writable")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    signer: acc.get("signer").and_then(|v| v.as_bool()).unwrap_or(false),
                };
                parsed.push(account);
            }
        }
        Ok(parsed)
    }

    fn parse_account_meta(
        &self,
        accounts: Option<&serde_json::Value>,
    ) -> Result<Vec<ShyftAccountMeta>> {
        let mut parsed = Vec::new();
        if let Some(accounts_array) = accounts.and_then(|v| v.as_array()) {
            for acc in accounts_array {
                let account = ShyftAccountMeta {
                    pubkey: acc
                        .get("pubkey")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    writable: acc
                        .get("writable")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    signer: acc.get("signer").and_then(|v| v.as_bool()).unwrap_or(false),
                };
                parsed.push(account);
            }
        }
        Ok(parsed)
    }

    /// Get the FULL raw JSON response from Shyft API
    pub async fn get_full_transaction_json(
        &self,
        signature: &str,
    ) -> Result<Option<serde_json::Value>> {
        let url = format!(
            "https://api.shyft.to/sol/v1/transaction/parsed?network={}&txn_signature={}&commitment=finalized",
            self.network, signature
        );

        info!("🔍 Fetching FULL transaction JSON: {}", signature);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .context("Failed to send request to Shyft transaction API")?;

        if !response.status().is_success() {
            warn!(
                "⚠️  Shyft transaction API error ({}): {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
            return Ok(None);
        }

        let full_json = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse Shyft transaction API response")?;

        Ok(Some(full_json))
    }

    /// Get the RAW transaction JSON from Shyft API
    pub async fn get_transaction_raw(&self, signature: &str) -> Result<Option<serde_json::Value>> {
        let url = format!(
            "https://api.shyft.to/sol/v1/transaction/raw?network={}&txn_signature={}&commitment=finalized",
            self.network, signature
        );

        info!("🔍 Fetching RAW transaction JSON: {}", signature);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .context("Failed to send request to Shyft RAW transaction API")?;

        if !response.status().is_success() {
            warn!(
                "⚠️  Shyft RAW transaction API error ({}): {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
            return Ok(None);
        }

        let raw_json = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse Shyft RAW transaction API response")?;

        Ok(Some(raw_json))
    }

    /// getTransaction (Shyft RPC) - Returns transaction details for a confirmed transaction
    ///
    /// Notes (from Shyft docs):
    /// - All the specifications for the getTransaction RPC method on Solana
    /// - Returns transaction details for a confirmed transaction
    ///
    /// Parameters required for this RPC call:
    /// - The pubkey of account delegate to query, as base-58 encoded string.
    /// - A JSON object with one of the following fields:
    ///   - mint: Pubkey of the specific token Mint to limit accounts to, as base-58 encoded string; or
    ///   - programId: Pubkey of the Token program that owns the accounts, as base-58 encoded string
    ///
    /// Configuration (optional fields):
    /// - commitment: How finalized a block is at that point in time. Supported: processed, confirmed, finalized. Defaults to finalized.
    /// - dataSlice: Request a slice of the account's data.
    ///   - length: <usize> — number of bytes to return
    ///   - offset: <usize> — byte offset from which to start reading
    /// - maxSupportedTransactionVersion: Currently, the only valid value is 0. Setting it to 0 allows fetching both Versioned and legacy transactions.
    /// - encoding: Encoding format for each returned transaction. Supported: json, jsonParsed, base58, base64.
    ///
    /// This method calls Shyft RPC `getTransaction` and publishes the FULL JSON-RPC response.
    pub async fn get_transaction_via_rpc(
        &self,
        signature: &str,
    ) -> Result<Option<serde_json::Value>> {
        // Shyft RPC uses api_key in the query string per docs
        let url = format!("https://rpc.shyft.to?api_key={}", self.api_key);

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                signature,
                {
                    "commitment": "finalized",
                    "maxSupportedTransactionVersion": 0,
                    "encoding": "json"
                }
            ]
        });

        info!("🔍 RPC getTransaction for signature: {}", signature);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Shyft RPC getTransaction")?;

        if !response.status().is_success() {
            warn!(
                "⚠️  Shyft RPC getTransaction error ({}): {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
            return Ok(None);
        }

        let rpc_json = response
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse Shyft RPC getTransaction response")?;

        Ok(Some(rpc_json))
    }

    /// Send data to Kafka with proper topic naming
    pub async fn send_to_kafka(
        &self,
        topic: &str,
        key: &str,
        data: &serde_json::Value,
    ) -> Result<()> {
        let payload = serde_json::to_vec(data)?;

        let _ = self
            .producer
            .send(
                FutureRecord::to(topic).key(key).payload(&payload),
                Duration::from_secs(5),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send to Kafka: {:?}", e))?;

        info!("📤 Sent to Kafka topic '{}' with key: {}", topic, key);
        Ok(())
    }

    /// Validate our TickArray parsing by comparing with Shyft API results
    pub async fn validate_tickarray_parsing(
        &self,
        pool_address: &str,
        our_parsed_data: &str,
    ) -> Result<ValidationResult> {
        let start_time = std::time::Instant::now();

        // Get expected data from Shyft API
        let shyft_data = self.get_pool_liquidity_details(pool_address).await?;

        let validation_duration = start_time.elapsed();

        // Compare our parsing with Shyft's data
        let validation = ValidationResult {
            pool_address: pool_address.to_string(),
            our_parsing: our_parsed_data.to_string(),
            shyft_data: shyft_data.clone(),
            matches: self.compare_parsing_results(our_parsed_data, &shyft_data),
            timestamp: chrono::Utc::now().timestamp(),
            validation_duration_ms: validation_duration.as_millis() as u64,
        };

        info!(
            "🔍 Validation result for pool {}: {} (took {}ms)",
            pool_address,
            if validation.matches {
                "✅ MATCHES"
            } else {
                "❌ MISMATCH"
            },
            validation.validation_duration_ms
        );

        Ok(validation)
    }

    /// Compare our parsing results with Shyft data
    fn compare_parsing_results(
        &self,
        our_parsing: &str,
        shyft_data: &Option<ShyftPoolData>,
    ) -> bool {
        if let Some(shyft) = shyft_data {
            // Check if our parsing contains key data points that match Shyft
            our_parsing.contains(&shyft.tick_current_index.to_string())
                || our_parsing.contains(&shyft.sqrt_price)
                || our_parsing.contains(&shyft.liquidity)
        } else {
            false
        }
    }

    /// Batch validate multiple pools for efficiency
    pub async fn batch_validate_pools(
        &self,
        validations: Vec<(String, String)>, // (pool_address, our_parsing)
    ) -> Result<Vec<ValidationResult>> {
        let mut results = Vec::new();

        for (pool_address, our_parsing) in validations {
            match self
                .validate_tickarray_parsing(&pool_address, &our_parsing)
                .await
            {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("⚠️  Failed to validate pool {}: {}", pool_address, e);
                    // Continue with other validations
                }
            }

            // Rate limiting - be respectful to Shyft API
            sleep(Duration::from_millis(100)).await;
        }

        Ok(results)
    }
}

// Health check endpoint
async fn start_health_server(config: &ServiceConfig) -> Result<()> {
    use axum::{http::StatusCode, response::Json, routing::get, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    let app = Router::new()
        .route(
            "/health",
            get(|| async {
                (
                    StatusCode::OK,
                    Json(json!({ "status": "healthy", "service": "shyft-validator" })),
                )
            }),
        )
        .route(
            "/metrics",
            get(|| async {
                (
                    StatusCode::OK,
                    Json(json!({ "status": "metrics endpoint ready" })),
                )
            }),
        );

    let listener = TcpListener::bind((config.bind_address.as_str(), config.health_port))
        .await
        .context("Failed to bind health server")?;

    info!(
        "🚀 Health server started on {}:{}",
        config.bind_address, config.health_port
    );

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Health server error: {}", e);
        }
    });

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    info!("🚀 Starting Enhanced Shyft Validator Service...");

    // Load and validate configuration
    let config = ServiceConfig::from_env().context("Failed to load configuration")?;

    config.log_config();

    // Start health server
    start_health_server(&config).await?;

    // Create Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.kafka_broker)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka producer")?;

    let validator = ShyftValidator::new(config.api_key.clone(), config.network.clone(), producer);

    // Create and start the swap watcher
    let watcher_config = SwapWatcherConfig {
        kafka_broker: config.kafka_broker.clone(),
        input_topic: "orca-swaps.parsed.v2".to_string(),
        output_topic: "shyft-swaps-parsed".to_string(),
        group_id: "shyft-swap-watcher".to_string(),
        initial_delay_ms: 2000, // Start with 2 second delay for API readiness
        max_delay_ms: 30000,    // Max 30 second delay
        delay_increment_ms: 1000, // Increase by 1 second each time
        batch_size: 10,
    };

    let mut swap_watcher = SwapWatcher::new(watcher_config, validator);

    info!("🚀 Starting Enhanced Shyft Validator with Swap Watcher...");
    info!("📺 Watching for Orca/Raydium swaps in topic: orca-swaps.parsed.v2");
    info!("📤 Outputting enriched data to topic: shyft-swaps-parsed");
    info!("⏱️  Using sliding window approach for API readiness");

    // Start the swap watcher
    if let Err(e) = swap_watcher.start_watching().await {
        error!("❌ Swap watcher failed: {}", e);
        return Err(e);
    }

    info!("✅ Enhanced Shyft Validator service ready!");
    info!("🚀 Now supporting:");
    info!("   • Pool liquidity details (Orca + Raydium)");
    info!("   • Token balance queries");
    info!("   • Pool discovery for token pairs");
    info!("   • Enhanced transaction parsing");
    info!("   • Kafka integration for all data");
    info!("   • Real-time swap monitoring with sliding window API readiness");

    // Keep the service running for health checks
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        info!("💓 Service heartbeat - running normally");
    }
}
