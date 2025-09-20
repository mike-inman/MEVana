use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    // Database configuration
    pub database_url: String,
    pub redis_url: String,
    pub connection_pool_size: u32,

    // Kafka configuration
    pub kafka_broker: String,
    pub discovery_topic: String,
    pub consumer_group: String,

    // API configuration
    pub http_port: u16,
    pub metrics_port: u16,

    // IDL fetching configuration
    pub anchor_cli_path: String,
    pub solana_cli_path: String,
    pub idl_cache_dir: String,
    pub manual_idl_dir: String,

    // External APIs
    pub coingecko_api_key: Option<String>,
    pub pyth_api_url: String,
    pub jupiter_api_url: String,

    // Plugin system
    pub plugin_dir: String,
    pub auto_load_plugins: bool,

    // Performance tuning
    pub cache_ttl_seconds: u64,
    pub discovery_batch_size: usize,
    pub max_concurrent_fetches: usize,

    // System configuration
    pub log_level: String,
}

impl RegistryConfig {
    pub fn load() -> Result<Self> {
        Ok(Self {
            // Database configuration
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgresql://mev:mevpassword@localhost:5433/mevdb".to_string()
            }),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            connection_pool_size: env::var("CONNECTION_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,

            // Kafka configuration
            kafka_broker: env::var("KAFKA_BROKER")
                .unwrap_or_else(|_| "localhost:29092".to_string()),
            discovery_topic: env::var("DISCOVERY_TOPIC")
                .unwrap_or_else(|_| "program.discovery.v1".to_string()),
            consumer_group: env::var("CONSUMER_GROUP")
                .unwrap_or_else(|_| "dex-registry".to_string()),

            // API configuration
            http_port: env::var("HTTP_PORT")
                .unwrap_or_else(|_| "8085".to_string())
                .parse()?,
            metrics_port: env::var("METRICS_PORT")
                .unwrap_or_else(|_| "9091".to_string())
                .parse()?,

            // IDL fetching configuration
            anchor_cli_path: env::var("ANCHOR_CLI_PATH").unwrap_or_else(|_| "anchor".to_string()),
            solana_cli_path: env::var("SOLANA_CLI_PATH").unwrap_or_else(|_| "solana".to_string()),
            idl_cache_dir: env::var("IDL_CACHE_DIR")
                .unwrap_or_else(|_| "data/idl_cache".to_string()),
            manual_idl_dir: env::var("MANUAL_IDL_DIR")
                .unwrap_or_else(|_| "idls/manual".to_string()),

            // External APIs
            coingecko_api_key: env::var("COINGECKO_API_KEY").ok(),
            pyth_api_url: env::var("PYTH_API_URL")
                .unwrap_or_else(|_| "https://api.pyth.network".to_string()),
            jupiter_api_url: env::var("JUPITER_API_URL")
                .unwrap_or_else(|_| "https://quote-api.jup.ag".to_string()),

            // Plugin system
            plugin_dir: env::var("PLUGIN_DIR").unwrap_or_else(|_| "plugins".to_string()),
            auto_load_plugins: env::var("AUTO_LOAD_PLUGINS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,

            // Performance tuning
            cache_ttl_seconds: env::var("CACHE_TTL_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()?,
            discovery_batch_size: env::var("DISCOVERY_BATCH_SIZE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,
            max_concurrent_fetches: env::var("MAX_CONCURRENT_FETCHES")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,

            // System configuration
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        })
    }

    pub fn validate(&self) -> Result<()> {
        // Validate database paths
        if self.database_url.is_empty() {
            anyhow::bail!("DATABASE_URL cannot be empty");
        }

        // Validate ports
        if self.http_port == 0 {
            anyhow::bail!("HTTP_PORT must be greater than 0");
        }

        if self.metrics_port == 0 {
            anyhow::bail!("METRICS_PORT must be greater than 0");
        }

        // Validate performance settings
        if self.discovery_batch_size == 0 {
            anyhow::bail!("DISCOVERY_BATCH_SIZE must be greater than 0");
        }

        if self.max_concurrent_fetches == 0 {
            anyhow::bail!("MAX_CONCURRENT_FETCHES must be greater than 0");
        }

        Ok(())
    }
}
