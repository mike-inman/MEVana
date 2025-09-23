use once_cell::sync::OnceLock;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use log::{debug, info, warn};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ProviderReserveConfig {
    #[serde(default)]
    pub available_amount: Option<u64>,
    #[serde(default)]
    pub fee_bps: Option<u16>,
    #[serde(default)]
    pub fee_lamports: Option<u64>,
    #[serde(default)]
    pub reserve: Option<String>,
    #[serde(default)]
    pub liquidity_vault: Option<String>,
    #[serde(default)]
    pub fee_receiver: Option<String>,
    #[serde(default)]
    pub host_fee_receiver: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub program_id: Option<String>,
    #[serde(default)]
    pub reserves: HashMap<String, ProviderReserveConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct FlashLoanProvidersConfig {
    #[serde(default, alias = "solend")]
    pub save: ProviderConfig,
    #[serde(default)]
    pub port: ProviderConfig,
    #[serde(default)]
    pub kamino: ProviderConfig,
    #[serde(default)]
    pub marginfi: ProviderConfig,
}

fn load_config_from(path: &PathBuf) -> Option<FlashLoanProvidersConfig> {
    match fs::read_to_string(path) {
        Ok(data) => match serde_json::from_str::<FlashLoanProvidersConfig>(&data) {
            Ok(cfg) => {
                info!("Loaded flash-loan provider config from {}", path.display());
                Some(cfg)
            }
            Err(err) => {
                warn!(
                    "Failed to parse flash-loan provider config {}: {}",
                    path.display(),
                    err
                );
                None
            }
        },
        Err(err) => {
            debug!(
                "Flash-loan provider config {} not readable: {}",
                path.display(),
                err
            );
            None
        }
    }
}

fn resolve_config() -> FlashLoanProvidersConfig {
    if let Ok(path) = env::var("FLASH_LOAN_PROVIDERS_CONFIG") {
        let path = PathBuf::from(path);
        if let Some(cfg) = load_config_from(&path) {
            return cfg;
        }
        warn!(
            "FLASH_LOAN_PROVIDERS_CONFIG was set to {}, but loading failed. Falling back to defaults.",
            path.display()
        );
    }

    let default_path = PathBuf::from("config/flash_loan_providers.devnet.json");
    if let Some(cfg) = load_config_from(&default_path) {
        return cfg;
    }

    warn!("Flash-loan provider config not found; using empty defaults");
    FlashLoanProvidersConfig::default()
}

static CONFIG: OnceLock<FlashLoanProvidersConfig> = OnceLock::new();

pub fn providers_config() -> FlashLoanProvidersConfig {
    CONFIG.get_or_init(resolve_config).clone()
}

pub fn providers_config_ref() -> &'static FlashLoanProvidersConfig {
    CONFIG.get_or_init(resolve_config)
}

pub fn parse_pubkey(value: &Option<String>) -> Option<Pubkey> {
    value
        .as_ref()
        .and_then(|s| s.parse::<Pubkey>().ok())
}
