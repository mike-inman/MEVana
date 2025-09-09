use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

/// Complete live trading configuration - everything needed for one-click activation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveTradingConfig {
    // Core execution flags
    pub enable_live_execution: bool,
    pub dry_run: bool,
    pub simulation_mode: bool,
    
    // Position sizing and risk
    pub min_position_size_sol: f64,
    pub max_position_size_sol: f64,
    pub max_daily_loss_sol: f64,
    pub min_confidence_for_execution: f64,
    
    // Wallet configuration
    pub hot_wallet_max_balance: f64,
    pub warm_wallet_max_balance: f64,
    pub cold_wallet_max_balance: f64,
    pub wallet_security_tier: String,
    
    // Execution provider settings
    pub jito_enabled: bool,
    pub jito_tip_lamports: u64,
    pub jito_auth_keypair_path: Option<String>,
    
    // RPC configuration
    pub solana_rpc_url: String,
    pub rpc_commitment: String,
    pub rpc_timeout_seconds: u64,
    
    // Flash loan settings
    pub flash_loan_enabled: bool,
    pub flash_loan_mode: String, // "streamlined" or "full"
    pub preferred_flash_loan_provider: String,
    pub max_flash_loan_amount_sol: f64,
    
    // Risk management
    pub circuit_breaker_enabled: bool,
    pub max_consecutive_failures: u32,
    pub cooling_off_period_seconds: u64,
    pub max_trades_per_minute: u32,
    
    // Venue-specific limits
    pub max_position_raydium: f64,
    pub max_position_orca: f64,
    pub max_position_meteora: f64,
    pub max_position_phoenix: f64,
    
    // Monitoring and alerts
    pub profit_tracking_enabled: bool,
    pub log_all_executions: bool,
    pub alert_on_negative_profit: bool,
    pub slack_webhook_url: Option<String>,
    
    // Performance settings
    pub track_slippage: bool,
    pub track_gas_costs: bool,
    pub max_slippage_bps: u32,
    pub priority_fee_multiplier: f64,
}

impl Default for LiveTradingConfig {
    fn default() -> Self {
        Self {
            // SAFE DEFAULTS - Live execution DISABLED
            enable_live_execution: false,
            dry_run: true,
            simulation_mode: true,
            
            // Conservative position sizing
            min_position_size_sol: 0.001,  // $0.20
            max_position_size_sol: 0.01,   // $2.00
            max_daily_loss_sol: 1.0,       // $200 daily limit
            min_confidence_for_execution: 0.8,
            
            // Wallet safety limits
            hot_wallet_max_balance: 0.1,   // 0.1 SOL max in hot wallet
            warm_wallet_max_balance: 5.0,  // 5 SOL max in warm wallet
            cold_wallet_max_balance: 100.0, // 100 SOL max in cold wallet
            wallet_security_tier: "hot".to_string(),
            
            // Jito bundle configuration
            jito_enabled: true,
            jito_tip_lamports: 10000,      // 0.00001 SOL tip
            jito_auth_keypair_path: None,
            
            // RPC settings
            solana_rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            rpc_commitment: "confirmed".to_string(),
            rpc_timeout_seconds: 10,
            
            // Flash loan defaults
            flash_loan_enabled: true,
            flash_loan_mode: "streamlined".to_string(),
            preferred_flash_loan_provider: "marginfi".to_string(),
            max_flash_loan_amount_sol: 10.0,
            
            // Risk management (aggressive safety)
            circuit_breaker_enabled: true,
            max_consecutive_failures: 3,
            cooling_off_period_seconds: 30,
            max_trades_per_minute: 10,
            
            // Conservative venue limits
            max_position_raydium: 0.005,   // $1 max per Raydium trade
            max_position_orca: 0.005,      // $1 max per Orca trade
            max_position_meteora: 0.002,   // $0.40 max per Meteora trade
            max_position_phoenix: 0.001,   // $0.20 max per Phoenix trade
            
            // Full monitoring enabled
            profit_tracking_enabled: true,
            log_all_executions: true,
            alert_on_negative_profit: true,
            slack_webhook_url: None,
            
            // Performance tracking
            track_slippage: true,
            track_gas_costs: true,
            max_slippage_bps: 50,          // 0.5% max slippage
            priority_fee_multiplier: 1.5,  // 1.5x priority fees
        }
    }
}

impl LiveTradingConfig {
    /// Load configuration from environment variables with safe defaults
    pub fn load_from_env() -> Self {
        let mut config = Self::default();
        
        // Core execution flags
        if let Ok(val) = env::var("ENABLE_LIVE_EXECUTION") {
            config.enable_live_execution = val == "true";
        }
        if let Ok(val) = env::var("DRY_RUN") {
            config.dry_run = val == "true";
        }
        if let Ok(val) = env::var("SIMULATION_MODE") {
            config.simulation_mode = val == "true";
        }
        
        // Position sizing
        if let Ok(val) = env::var("MIN_POSITION_SIZE_SOL") {
            if let Ok(size) = val.parse::<f64>() {
                config.min_position_size_sol = size;
            }
        }
        if let Ok(val) = env::var("MAX_POSITION_SIZE_SOL") {
            if let Ok(size) = val.parse::<f64>() {
                config.max_position_size_sol = size;
            }
        }
        if let Ok(val) = env::var("MAX_DAILY_LOSS_SOL") {
            if let Ok(loss) = val.parse::<f64>() {
                config.max_daily_loss_sol = loss;
            }
        }
        if let Ok(val) = env::var("MIN_CONFIDENCE_FOR_EXECUTION") {
            if let Ok(conf) = val.parse::<f64>() {
                config.min_confidence_for_execution = conf;
            }
        }
        
        // Wallet configuration
        if let Ok(val) = env::var("HOT_WALLET_MAX_BALANCE") {
            if let Ok(balance) = val.parse::<f64>() {
                config.hot_wallet_max_balance = balance;
            }
        }
        if let Ok(val) = env::var("WARM_WALLET_MAX_BALANCE") {
            if let Ok(balance) = val.parse::<f64>() {
                config.warm_wallet_max_balance = balance;
            }
        }
        
        // Jito configuration
        if let Ok(val) = env::var("JITO_ENABLED") {
            config.jito_enabled = val == "true";
        }
        if let Ok(val) = env::var("JITO_TIP_LAMPORTS") {
            if let Ok(tip) = val.parse::<u64>() {
                config.jito_tip_lamports = tip;
            }
        }
        if let Ok(val) = env::var("JITO_AUTH_KEYPAIR_PATH") {
            config.jito_auth_keypair_path = Some(val);
        }
        
        // RPC configuration
        if let Ok(val) = env::var("SOLANA_RPC_URL") {
            config.solana_rpc_url = val;
        }
        if let Ok(val) = env::var("RPC_COMMITMENT") {
            config.rpc_commitment = val;
        }
        
        // Flash loan configuration
        if let Ok(val) = env::var("FLASH_LOAN_ENABLED") {
            config.flash_loan_enabled = val == "true";
        }
        if let Ok(val) = env::var("PREFERRED_FLASH_LOAN_PROVIDER") {
            config.preferred_flash_loan_provider = val;
        }
        
        // Risk management
        if let Ok(val) = env::var("MAX_CONSECUTIVE_FAILURES") {
            if let Ok(failures) = val.parse::<u32>() {
                config.max_consecutive_failures = failures;
            }
        }
        if let Ok(val) = env::var("MAX_TRADES_PER_MINUTE") {
            if let Ok(trades) = val.parse::<u32>() {
                config.max_trades_per_minute = trades;
            }
        }
        
        // Venue limits
        if let Ok(val) = env::var("MAX_POSITION_RAYDIUM") {
            if let Ok(pos) = val.parse::<f64>() {
                config.max_position_raydium = pos;
            }
        }
        if let Ok(val) = env::var("MAX_POSITION_ORCA") {
            if let Ok(pos) = val.parse::<f64>() {
                config.max_position_orca = pos;
            }
        }
        if let Ok(val) = env::var("MAX_POSITION_METEORA") {
            if let Ok(pos) = val.parse::<f64>() {
                config.max_position_meteora = pos;
            }
        }
        
        // Monitoring
        if let Ok(val) = env::var("SLACK_WEBHOOK_URL") {
            config.slack_webhook_url = Some(val);
        }
        
        config
    }
    
    /// Validate configuration for live trading readiness
    pub fn validate_for_live_trading(&self) -> Result<()> {
        if self.enable_live_execution {
            // Critical validations for live trading
            if self.dry_run {
                return Err(anyhow::anyhow!("Cannot enable live execution while dry_run=true"));
            }
            if self.simulation_mode {
                return Err(anyhow::anyhow!("Cannot enable live execution while simulation_mode=true"));
            }
            if self.max_position_size_sol <= 0.0 {
                return Err(anyhow::anyhow!("max_position_size_sol must be > 0"));
            }
            if self.min_confidence_for_execution < 0.5 {
                return Err(anyhow::anyhow!("min_confidence_for_execution should be >= 0.5 for safety"));
            }
            if self.jito_enabled && self.jito_auth_keypair_path.is_none() {
                return Err(anyhow::anyhow!("Jito enabled but no auth keypair path provided"));
            }
        }
        
        Ok(())
    }
    
    /// Get position size limit for a specific venue
    pub fn get_venue_position_limit(&self, venue: &str) -> f64 {
        match venue.to_lowercase().as_str() {
            v if v.contains("raydium") => self.max_position_raydium,
            v if v.contains("orca") => self.max_position_orca,
            v if v.contains("meteora") => self.max_position_meteora,
            v if v.contains("phoenix") => self.max_position_phoenix,
            _ => self.min_position_size_sol, // Default to minimum for unknown venues
        }
    }
    
    /// Check if we're ready for live trading (all systems configured)
    pub fn is_live_trading_ready(&self) -> bool {
        self.enable_live_execution && 
        !self.dry_run && 
        !self.simulation_mode &&
        self.jito_enabled &&
        self.jito_auth_keypair_path.is_some()
    }
    
    /// Get a summary of current configuration
    pub fn summary(&self) -> String {
        format!(
            "Live Trading Config:\n\
             🔹 Live Execution: {} (dry_run: {}, simulation: {})\n\
             🔹 Position Size: {:.4} - {:.4} SOL\n\
             🔹 Daily Loss Limit: {:.2} SOL\n\
             🔹 Min Confidence: {:.1}%\n\
             🔹 Jito: {} (tip: {} lamports)\n\
             🔹 Circuit Breaker: {} (max failures: {})\n\
             🔹 Venue Limits: Raydium={:.3}, Orca={:.3}, Meteora={:.3} SOL\n\
             🔹 Ready for Live: {}",
            self.enable_live_execution,
            self.dry_run,
            self.simulation_mode,
            self.min_position_size_sol,
            self.max_position_size_sol,
            self.max_daily_loss_sol,
            self.min_confidence_for_execution * 100.0,
            self.jito_enabled,
            self.jito_tip_lamports,
            self.circuit_breaker_enabled,
            self.max_consecutive_failures,
            self.max_position_raydium,
            self.max_position_orca,
            self.max_position_meteora,
            self.is_live_trading_ready()
        )
    }
}

/// Environment setup helper
pub fn create_env_template() -> String {
    r#"# MEVana Live Trading Configuration
# Copy to .env and customize for your setup

# 🚨 CRITICAL: Set to true ONLY when ready for live trading
ENABLE_LIVE_EXECUTION=false
DRY_RUN=true
SIMULATION_MODE=true

# Position Sizing (start small!)
MIN_POSITION_SIZE_SOL=0.001    # $0.20 minimum
MAX_POSITION_SIZE_SOL=0.01     # $2.00 maximum  
MAX_DAILY_LOSS_SOL=1.0         # $200 daily limit
MIN_CONFIDENCE_FOR_EXECUTION=0.8

# Wallet Safety Limits
HOT_WALLET_MAX_BALANCE=0.1     # 0.1 SOL max in hot wallet
WARM_WALLET_MAX_BALANCE=5.0    # 5 SOL max in warm wallet
COLD_WALLET_MAX_BALANCE=100.0  # 100 SOL max in cold wallet

# Jito Bundle Configuration (REQUIRED for live trading)
JITO_ENABLED=true
JITO_TIP_LAMPORTS=10000        # 0.00001 SOL tip per bundle
JITO_AUTH_KEYPAIR_PATH=/path/to/your/jito-keypair.json

# RPC Configuration
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
RPC_COMMITMENT=confirmed
RPC_TIMEOUT_SECONDS=10

# Flash Loan Configuration
FLASH_LOAN_ENABLED=true
PREFERRED_FLASH_LOAN_PROVIDER=marginfi
MAX_FLASH_LOAN_AMOUNT_SOL=10.0

# Risk Management (CRITICAL)
CIRCUIT_BREAKER_ENABLED=true
MAX_CONSECUTIVE_FAILURES=3
COOLING_OFF_PERIOD_SECONDS=30
MAX_TRADES_PER_MINUTE=10

# Venue-Specific Position Limits
MAX_POSITION_RAYDIUM=0.005     # $1 max per Raydium trade
MAX_POSITION_ORCA=0.005        # $1 max per Orca trade  
MAX_POSITION_METEORA=0.002     # $0.40 max per Meteora trade
MAX_POSITION_PHOENIX=0.001     # $0.20 max per Phoenix trade

# Monitoring & Alerts
PROFIT_TRACKING_ENABLED=true
LOG_ALL_EXECUTIONS=true
ALERT_ON_NEGATIVE_PROFIT=true
SLACK_WEBHOOK_URL=https://hooks.slack.com/your-webhook-url

# Performance Settings
TRACK_SLIPPAGE=true
TRACK_GAS_COSTS=true
MAX_SLIPPAGE_BPS=50           # 0.5% max slippage
PRIORITY_FEE_MULTIPLIER=1.5   # 1.5x priority fees
"#.to_string()
}