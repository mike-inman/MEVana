use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::metrics::MetricsServer;

#[allow(dead_code)]
pub const MISSING_REQUIRED: &[&str] = &[
    // Placeholder examples (fill with real keys as registry matures)
    // "solend.USDC.reserve",
    // "raydium_clmm.SOLUSDC.vault_a",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountRegistryConfig {
    pub strict: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanReserveAccounts {
    pub reserve: String,
    pub liquidity_supply: String,
    pub fee_receiver: String,
    pub oracle: String,
    pub mint: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClmmPoolAccounts {
    pub pool: String,
    pub token_a_vault: String,
    pub token_b_vault: String,
    pub oracle_or_observation: Option<String>,
    pub tick_arrays: Vec<String>,
    pub token_a_mint: String,
    pub token_b_mint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookMarketAccounts {
    pub market: String,
    pub bids: String,
    pub asks: String,
    pub event_queue: String,
    pub base_vault: String,
    pub quote_vault: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleAccount {
    pub price: String,
    pub price_decimals: Option<u8>, // number of decimals the price is scaled by (e.g. 9 for lamports-style)
    pub confidence_limit_bps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JitoConfig {
    pub tip_account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountRegistryData {
    pub flash_loans: HashMap<String, HashMap<String, FlashLoanReserveAccounts>>, // provider -> mint -> accounts
    pub clmm_pools: HashMap<String, ClmmPoolAccounts>, // pool_id -> accounts
    pub orderbooks: HashMap<String, OrderbookMarketAccounts>, // market_id -> accounts
    pub oracles: HashMap<String, OracleAccount>,       // symbol/mint -> oracle
    pub jito: JitoConfig,
}

impl AccountRegistryData {
    pub fn has_key(&self, key: &str) -> bool {
        // Key patterns: provider.mint.reserve, clmm.pool_id.vault_a, etc.
        let parts: Vec<&str> = key.split('.').collect();
        if parts.is_empty() {
            return false;
        }
        match parts[0] {
            "flash" if parts.len() == 3 => {
                if let Some(provider) = self.flash_loans.get(parts[1]) {
                    provider.contains_key(parts[2])
                } else {
                    false
                }
            }
            "clmm" if parts.len() >= 2 => self.clmm_pools.contains_key(parts[1]),
            "orderbook" if parts.len() >= 2 => self.orderbooks.contains_key(parts[1]),
            "oracle" if parts.len() >= 2 => self.oracles.contains_key(parts[1]),
            "jito" => self.jito.tip_account.is_some(),
            _ => false,
        }
    }

    pub fn dynamic_required_keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (prov, mints) in &self.flash_loans {
            for (mint, _r) in mints {
                out.push(format!("flash.{prov}.{mint}"));
            }
        }
        for (pool_id, _p) in &self.clmm_pools {
            out.push(format!("clmm.{pool_id}"));
        }
        for (sym, _o) in &self.oracles {
            out.push(format!("oracle.{sym}"));
        }
        if self.jito.tip_account.is_some() {
            out.push("jito.tip_account".into());
        }
        out
    }
}

#[derive(Debug)]
pub struct AccountRegistry {
    pub data: AccountRegistryData,
    pub missing_keys: Vec<String>,
    pub validation_errors: Vec<String>,
    // Aggregated mint -> decimals map (derived from flash loan reserves currently)
    pub mint_decimals: HashMap<String, u8>,
}

impl Default for AccountRegistry {
    fn default() -> Self {
        Self {
            data: AccountRegistryData::default(),
            missing_keys: Vec::new(),
            validation_errors: Vec::new(),
            mint_decimals: HashMap::new(),
        }
    }
}

impl AccountRegistry {
    pub fn load(cfg: &AccountRegistryConfig) -> Result<Self> {
        let data = if let Some(path) = &cfg.path {
            if Path::new(path).exists() {
                let raw = fs::read_to_string(path)?;
                match serde_json::from_str::<AccountRegistryData>(&raw) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        warn!("account_registry_parse_error path={} err={}", path, e);
                        AccountRegistryData::default()
                    }
                }
            } else {
                warn!(
                    "account_registry_path_missing path={} -> starting empty",
                    path
                );
                AccountRegistryData::default()
            }
        } else {
            AccountRegistryData::default()
        };
        let mut registry = Self {
            data,
            missing_keys: Vec::new(),
            validation_errors: Vec::new(),
            mint_decimals: HashMap::new(),
        };
        // Derive mint decimals map from flash loan reserves (can be extended later)
        for (_prov, mints) in &registry.data.flash_loans {
            for (_mint_key, reserve) in mints {
                registry
                    .mint_decimals
                    .entry(reserve.mint.clone())
                    .or_insert(reserve.decimals);
            }
        }
        // Future: extend mint_decimals by scanning CLMM pools or explicit token registry section.
        registry.compute_missing();
        registry.compute_validation_errors();
        // Strict feature gate: if feature enabled and missing required placeholders, panic early.
        #[cfg(feature = "strict-accounts")]
        {
            let mut missing_required: Vec<&str> = Vec::new();
            for key in MISSING_REQUIRED {
                if !registry.data.has_key(key) {
                    missing_required.push(*key);
                }
            }
            if !missing_required.is_empty() {
                return Err(anyhow!(
                    "strict_accounts_missing required={:?}",
                    missing_required
                ));
            }
        }
        if cfg.strict
            && (!registry.missing_keys.is_empty() || !registry.validation_errors.is_empty())
        {
            return Err(anyhow!(
                "strict_account_registry_issues missing={:?} validation={:?}",
                registry.missing_keys,
                registry.validation_errors
            ));
        }
        Ok(registry)
    }

    pub fn compute_missing(&mut self) {
        let required = self.data.dynamic_required_keys();
        self.missing_keys = required
            .into_iter()
            .filter(|k| !self.data.has_key(k))
            .collect();
    }

    pub fn compute_validation_errors(&mut self) {
        let mut errs = Vec::new();
        // Sanity: decimals range, tick arrays non-empty, naive placeholder detection.
        for (prov, mints) in &self.data.flash_loans {
            for (mint, reserve) in mints {
                if reserve.decimals > 12 {
                    errs.push(format!("flash.{prov}.{mint}.decimals_out_of_range"));
                }
                if reserve.reserve.contains("Example") {
                    errs.push(format!("flash.{prov}.{mint}.reserve_placeholder"));
                }
            }
        }
        for (pool_id, pool) in &self.data.clmm_pools {
            if pool.tick_arrays.is_empty() {
                errs.push(format!("clmm.{pool_id}.no_tick_arrays"));
            }
            if pool.pool.contains("Example") {
                errs.push(format!("clmm.{pool_id}.pool_placeholder"));
            }
        }
        if let Some(tip) = &self.data.jito.tip_account {
            if tip.contains("Example") {
                errs.push("jito.tip_account.placeholder".to_string());
            }
        }
        self.validation_errors = errs;
    }
}

/// Utility: convert token deltas to lamports using supplied mint_decimals & oracle map (pure function).
#[allow(dead_code)]
pub fn convert_token_deltas_to_lamports_generic(
    deltas: &std::collections::HashMap<String, i64>,
    oracles: &HashMap<String, OracleAccount>,
    mint_decimals: &HashMap<String, u8>,
) -> i64 {
    let mut total: i64 = 0;
    for (mint, amount) in deltas {
        if mint == "So11111111111111111111111111111111111111112" {
            total += *amount;
            continue;
        }
        if let Some(oracle) = oracles.get(mint) {
            if let Ok(raw_price) = oracle.price.parse::<u128>() {
                let price_decimals = oracle.price_decimals.unwrap_or(9) as u32;
                let m_dec = mint_decimals.get(mint).copied().unwrap_or(9) as i32;
                let target = 9i32;
                let normalized: i128 = if m_dec < target {
                    (*amount as i128) * 10i128.pow((target - m_dec) as u32)
                } else if m_dec > target {
                    (*amount as i128) / 10i128.pow((m_dec - target) as u32)
                } else {
                    *amount as i128
                };
                let div = 10u128.pow(price_decimals);
                let delta = (normalized * raw_price as i128) / div as i128;
                total += delta as i64;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_convert_mixed_decimals() {
        let mut deltas: HashMap<String, i64> = HashMap::new();
        // +1 USDC (6 dec) and +1 SOL (9 dec) with simplistic prices
        // Assume oracle price scaled with 9 decimals: price = 100 * 10^9 lamports per token for both (arbitrary)
        deltas.insert("USDC_MINT".to_string(), 1_000_000); // 1.0 USDC in 6-dec base units
        deltas.insert("SOL_MINT".to_string(), 1_000_000_000); // 1.0 SOL in 9-dec base units
        let mut oracles: HashMap<String, OracleAccount> = HashMap::new();
        oracles.insert(
            "USDC_MINT".to_string(),
            OracleAccount {
                price: (100u64 * 1_000_000_000u64).to_string(),
                price_decimals: Some(9),
                confidence_limit_bps: None,
            },
        );
        oracles.insert(
            "SOL_MINT".to_string(),
            OracleAccount {
                price: (100u64 * 1_000_000_000u64).to_string(),
                price_decimals: Some(9),
                confidence_limit_bps: None,
            },
        );
        let mut mint_decimals: HashMap<String, u8> = HashMap::new();
        mint_decimals.insert("USDC_MINT".to_string(), 6);
        mint_decimals.insert("SOL_MINT".to_string(), 9);

        let total = convert_token_deltas_to_lamports_generic(&deltas, &oracles, &mint_decimals);
        // Expected: both tokens worth 100 * 1 SOL-equivalent each => 2 * 100 * 1 SOL price units
        // Price expression simplified: normalized amounts both become 1e9 units then * raw_price / 1e9 => 100 * 1e9 lamports each.
        let expected_each = 100i64 * 1_000_000_000i64; // 100 SOL in lamports per token, but lamports internal 1 SOL = 1e9
        assert_eq!(total, expected_each * 2);
    }
}

pub fn report_missing_accounts(registry: &AccountRegistry, metrics: Option<&MetricsServer>) {
    let missing_count = registry.missing_keys.len();
    // Compute diff vs. hard-coded required (placeholder list) for visibility
    let mut required_missing: Vec<&str> = Vec::new();
    for key in MISSING_REQUIRED {
        if !registry.data.has_key(key) {
            required_missing.push(*key);
        }
    }
    if missing_count > 0 {
        warn!(
            "account_registry_missing count={} dynamic_missing_keys={:?} required_missing_count={} required_missing={:?}",
            missing_count, registry.missing_keys, required_missing.len(), required_missing
        );
    } else {
        info!("account_registry_all_present");
    }
    if !registry.validation_errors.is_empty() {
        warn!(
            "account_registry_validation_errors count={} errs={:?}",
            registry.validation_errors.len(),
            registry.validation_errors
        );
    }
    if let Some(m) = metrics {
        m.update_account_registry_stats(missing_count, registry.data.clmm_pools.len());
        m.update_account_registry_validation_errors(registry.validation_errors.len());
        // Expose placeholder diff indirectly via existing gauge (could add separate one later)
        if !required_missing.is_empty() {
            // reuse account_sets_missing as baseline; no additional gauge yet
        }
        m.update_required_missing_accounts(required_missing.len());
    }
}
