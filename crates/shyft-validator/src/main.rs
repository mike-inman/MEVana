use anyhow::{Context, Result};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

// NEW: Configuration structure with validation
#[derive(Debug, Clone)]
struct ServiceConfig {
    api_key: String,
    network: String,
    health_port: u16,
    bind_address: String,
}

impl ServiceConfig {
    fn from_env() -> Result<Self> {
        // Load environment variables with validation
        let api_key = env::var("SHYFT_API_KEY")
            .map_err(|_| anyhow::anyhow!("SHYFT_API_KEY environment variable is required"))?;
        
        let network = env::var("SHYFT_NETWORK")
            .unwrap_or_else(|_| "mainnet-beta".to_string());
        
        let health_port = env::var("HEALTH_PORT")
            .unwrap_or_else(|_| "8086".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("HEALTH_PORT must be a valid port number"))?;
        
        let bind_address = env::var("BIND_ADDRESS")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        Ok(Self {
            api_key,
            network,
            health_port,
            bind_address,
        })
    }

    fn log_config(&self) {
        info!("=== SHYFT VALIDATOR SERVICE CONFIGURATION ===");
        info!("Shyft Network: {}", self.network);
        info!("Health Port: {}", self.health_port);
        info!("Bind Address: {}", self.bind_address);
        info!("API Key: {}...", &self.api_key[..8.min(self.api_key.len())]);
        info!("=============================================");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShyftPoolData {
    pub tick_current_index: i32,
    pub sqrt_price: String,
    pub liquidity: String,
    pub fee_rate: i64,
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

#[derive(Debug, Clone)]
pub struct ShyftValidator {
    api_key: String,
    network: String,
}

impl ShyftValidator {
    pub fn new(api_key: String, network: String) -> Self {
        Self { api_key, network }
    }

    /// Validate our TickArray parsing by comparing with Shyft API results
    pub async fn validate_tickarray_parsing(
        &self,
        pool_address: &str,
        our_parsed_data: &str,
    ) -> Result<ValidationResult> {
        let start_time = std::time::Instant::now();

        // Get expected data from Shyft API
        let shyft_data = self.get_pool_data_from_shyft(pool_address).await?;

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

    /// Get pool data from Shyft API for comparison
    async fn get_pool_data_from_shyft(&self, pool_address: &str) -> Result<Option<ShyftPoolData>> {
        let query = r#"
            query GetPoolData($address: String!) {
                ORCA_WHIRLPOOLS_whirlpool(
                    where: { pubkey: { _eq: $address } }
                    limit: 1
                ) {
                    tickCurrentIndex
                    sqrtPrice
                    liquidity
                    feeRate
                }
            }
        "#;

        let client = reqwest::Client::new();
        let response: serde_json::Value = client
            .post(&self.graphql_endpoint())
            .json(&serde_json::json!({
                "query": query,
                "variables": { "address": pool_address }
            }))
            .send()
            .await
            .context("Failed to send GraphQL request for validation")?
            .json()
            .await
            .context("Failed to parse GraphQL response for validation")?;

        // Parse the response
        if let Some(data) = response.get("data") {
            if let Some(whirlpools) = data.get("ORCA_WHIRLPOOLS_whirlpool") {
                if let Some(rows) = whirlpools.as_array() {
                    if let Some(first_row) = rows.first() {
                        let pool_data = ShyftPoolData {
                            tick_current_index: first_row
                                .get("tickCurrentIndex")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i32,
                            sqrt_price: first_row
                                .get("sqrtPrice")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0")
                                .to_string(),
                            liquidity: first_row
                                .get("liquidity")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0")
                                .to_string(),
                            fee_rate: first_row
                                .get("feeRate")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0),
                        };
                        return Ok(Some(pool_data));
                    }
                }
            }
        }

        Ok(None)
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

    /// Get the GraphQL endpoint URL
    fn graphql_endpoint(&self) -> String {
        format!(
            "https://programs.shyft.to/v0/graphql/?api_key={}&network={}",
            self.api_key, self.network
        )
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

// NEW: Health check endpoint
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
            (StatusCode::OK, Json(json!({ "status": "healthy", "service": "shyft-validator" })))
        }))
        .route("/metrics", get(|| async { 
            (StatusCode::OK, Json(json!({ "status": "metrics endpoint ready" })))
        }));

    let listener = match TcpListener::bind((config.bind_address.as_str(), config.health_port)).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("❌ Failed to bind health server to {}:{} - {}", config.bind_address, config.health_port, e);
            error!("💡 Check if the bind address is valid and the port is available");
            return Err(anyhow::anyhow!("Health server binding failed: {}", e));
        }
    };

    info!("🚀 Health server started on {}:{}", config.bind_address, config.health_port);
    
    // Spawn the health server in a separate task
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

    info!("🚀 Starting Shyft Validator Service...");

    // Load and validate configuration
    let config = match ServiceConfig::from_env() {
        Ok(config) => {
            config.log_config();
            config
        },
        Err(e) => {
            error!("❌ Configuration error: {}", e);
            error!("💡 Please check all required environment variables are set");
            std::process::exit(1);
        }
    };

    // Start health server
    if let Err(e) = start_health_server(&config).await {
        error!("❌ Failed to start health server: {}", e);
        std::process::exit(1);
    }

    let validator = ShyftValidator::new(config.api_key.clone(), config.network.clone());

    // Test validation with a known pool address
    let test_pool = "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"; // Example Orca pool

    info!("🧪 Testing validation with pool: {}", test_pool);

    let test_parsing = "tick_index:12345, liquidity:1000000";
    let validation = validator
        .validate_tickarray_parsing(test_pool, test_parsing)
        .await?;

    info!("📊 Validation result: {:?}", validation);

    // Test batch validation
    let batch_validations = vec![
        (test_pool.to_string(), test_parsing.to_string()),
        (
            "AnotherPool123".to_string(),
            "tick_index:54321, liquidity:2000000".to_string(),
        ),
    ];

    info!("🧪 Testing batch validation...");
    let batch_results = validator.batch_validate_pools(batch_validations).await?;

    for result in batch_results {
        info!(
            "Batch validation result: {} - {}",
            result.pool_address,
            if result.matches {
                "✅ MATCHES"
            } else {
                "❌ MISMATCH"
            }
        );
    }

    info!("✅ Validation service ready! This service can be:");
    info!("   • Called independently for testing and debugging");
    info!("   • Used in production for validation when speed isn't critical");
    info!("   • Integrated with other services via library calls");
    info!("   • Deployed separately to avoid impacting core trading performance");

    // Keep the service running for health checks
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        info!("💓 Service heartbeat - running normally");
    }
}
