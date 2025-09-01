use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    // Kafka configuration
    pub kafka_broker: String,
    pub opportunities_topic: String,
    pub results_topic: String,
    pub consumer_group: String,

    // Shyft.to configuration
    pub shyft_api_key: String,
    pub shyft_rpc_url: String,
    pub shyft_geyser_url: String,
    pub shyft_jito_url: String,
    pub shyft_geyser_x_token: String, // auth token (x_token) for geyser stream
    pub shyft_graphql_url: Option<String>, // optional GraphQL endpoint
    pub shyft_meta_api_key: Option<String>, // optional separate meta/key service key

    // Wallet configuration
    pub wallet_keypair_path: String,
    pub wallet_backup_paths: Vec<String>,
    pub max_wallet_balance_sol: f64,
    pub min_wallet_balance_sol: f64,

    // Trading configuration
    pub max_position_size_sol: f64,
    pub min_profit_threshold_bps: u64,
    pub max_slippage_bps: u64,
    pub max_execution_time_ms: u64,

    // Flash loan configuration
    pub flash_loan_providers: Vec<String>,
    pub max_flash_loan_amount_sol: f64,
    pub flash_loan_timeout_ms: u64,

    // Risk management
    pub daily_loss_limit_sol: f64,
    pub max_concurrent_trades: u32,
    pub cool_down_period_ms: u64,

    // System configuration
    pub metrics_port: u16,
    pub health_check_port: u16,
    pub log_level: String,
}

impl ExecutorConfig {
    pub fn load() -> Result<Self> {
        Ok(Self {
            // Kafka configuration
            kafka_broker: env::var("KAFKA_BROKER")
                .unwrap_or_else(|_| "localhost:29092".to_string()),
            opportunities_topic: env::var("OPPORTUNITIES_TOPIC")
                .unwrap_or_else(|_| "arb.opportunities.v1".to_string()),
            results_topic: env::var("RESULTS_TOPIC")
                .unwrap_or_else(|_| "execution.results.v1".to_string()),
            consumer_group: env::var("CONSUMER_GROUP")
                .unwrap_or_else(|_| "executor-service".to_string()),

            // Shyft.to configuration
            shyft_api_key: env::var("SHYFT_API_KEY")
                .expect("SHYFT_API_KEY environment variable required"),
            shyft_rpc_url: env::var("SHYFT_RPC_URL")
                .unwrap_or_else(|_| "https://rpc.shyft.to".to_string()),
            shyft_geyser_url: env::var("SHYFT_GEYSER_URL")
                .unwrap_or_else(|_| "https://geyser.shyft.to".to_string()),
            shyft_jito_url: env::var("SHYFT_JITO_URL")
                .unwrap_or_else(|_| "https://jito.shyft.to".to_string()),
            shyft_geyser_x_token: env::var("SHYFT_GEYSER_X_TOKEN")
                .unwrap_or_else(|_| env::var("X_TOKEN").unwrap_or_default()),
            shyft_graphql_url: env::var("SHYFT_GRAPHQL_URL").ok(),
            shyft_meta_api_key: env::var("SHYFT_META_API_KEY").ok(),

            // Wallet configuration
            wallet_keypair_path: env::var("WALLET_KEYPAIR_PATH")
                .unwrap_or_else(|_| "~/.config/solana/id.json".to_string()),
            wallet_backup_paths: env::var("WALLET_BACKUP_PATHS")
                .unwrap_or_else(|_| "".to_string())
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
            max_wallet_balance_sol: env::var("MAX_WALLET_BALANCE_SOL")
                .unwrap_or_else(|_| "100.0".to_string())
                .parse()?,
            min_wallet_balance_sol: env::var("MIN_WALLET_BALANCE_SOL")
                .unwrap_or_else(|_| "1.0".to_string())
                .parse()?,

            // Trading configuration
            max_position_size_sol: env::var("MAX_POSITION_SIZE_SOL")
                .unwrap_or_else(|_| "10.0".to_string())
                .parse()?,
            min_profit_threshold_bps: env::var("MIN_PROFIT_THRESHOLD_BPS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
            max_slippage_bps: env::var("MAX_SLIPPAGE_BPS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()?,
            max_execution_time_ms: env::var("MAX_EXECUTION_TIME_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()?,

            // Flash loan configuration
            flash_loan_providers: env::var("FLASH_LOAN_PROVIDERS")
                .unwrap_or_else(|_| "solend,marginfi,tulip".to_string())
                .split(',')
                .map(|s| s.to_string())
                .collect(),
            max_flash_loan_amount_sol: env::var("MAX_FLASH_LOAN_AMOUNT_SOL")
                .unwrap_or_else(|_| "1000.0".to_string())
                .parse()?,
            flash_loan_timeout_ms: env::var("FLASH_LOAN_TIMEOUT_MS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()?,

            // Risk management
            daily_loss_limit_sol: env::var("DAILY_LOSS_LIMIT_SOL")
                .unwrap_or_else(|_| "5.0".to_string())
                .parse()?,
            max_concurrent_trades: env::var("MAX_CONCURRENT_TRADES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()?,
            cool_down_period_ms: env::var("COOL_DOWN_PERIOD_MS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()?,

            // System configuration
            metrics_port: env::var("METRICS_PORT")
                .unwrap_or_else(|_| "9090".to_string())
                .parse()?,
            health_check_port: env::var("HEALTH_CHECK_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        })
    }

    pub fn validate(&self) -> Result<()> {
        // Validate critical configuration
        if self.shyft_api_key.is_empty() {
            anyhow::bail!("SHYFT_API_KEY is required");
        }

        if self.max_position_size_sol <= 0.0 {
            anyhow::bail!("MAX_POSITION_SIZE_SOL must be positive");
        }

        if self.min_wallet_balance_sol >= self.max_wallet_balance_sol {
            anyhow::bail!("MIN_WALLET_BALANCE_SOL must be less than MAX_WALLET_BALANCE_SOL");
        }

        if self.flash_loan_providers.is_empty() {
            anyhow::bail!("At least one flash loan provider must be configured");
        }

        Ok(())
    }
}
