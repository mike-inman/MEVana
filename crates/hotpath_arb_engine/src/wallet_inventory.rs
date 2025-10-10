use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use executor_service::shyft_client::ShyftClient;
use futures::future::join_all;
use log::warn;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone)]
pub struct WalletVaultClient {
    base_url: String,
    client: Client,
    authorization: Option<String>,
    user_id: String,
}

#[derive(Debug, Deserialize)]
struct WalletListResponse {
    wallets: Vec<WalletResponse>,
    #[allow(dead_code)]
    total: usize,
}

#[derive(Debug, Deserialize)]
struct WalletResponse {
    #[allow(dead_code)]
    id: Uuid,
    pubkey: String,
    #[serde(default)]
    tier: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    is_active: bool,
}

impl WalletVaultClient {
    pub fn from_env() -> Result<Option<Self>> {
        let base = std::env::var("WALLET_VAULT_URL")
            .unwrap_or_else(|_| "http://localhost:8090".to_string());

        // Ensure we end up with .../api/ as base
        let normalized = if base.ends_with('/') {
            base
        } else {
            format!("{}/", base)
        };
        let api_base = if normalized.contains("/api/") {
            normalized
        } else {
            format!("{}api/", normalized)
        };

        let timeout_secs = std::env::var("WALLET_VAULT_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        let auth_required = std::env::var("WALLET_VAULT_AUTH_REQUIRED")
            .unwrap_or_else(|_| "false".to_string())
            .eq_ignore_ascii_case("true");
        let secret = std::env::var("WALLET_VAULT_SECRET").ok();

        let authorization = match (auth_required, secret) {
            (true, Some(secret)) => Some(format!("Bearer {}", secret)),
            (true, None) => {
                warn!("Wallet vault auth required but WALLET_VAULT_SECRET not set – skipping integration");
                return Ok(None);
            }
            (false, Some(secret)) => Some(format!("Bearer {}", secret)),
            (false, None) => None,
        };

        let user_id =
            std::env::var("WALLET_VAULT_USER_ID").unwrap_or_else(|_| "hotpath-engine".to_string());

        Ok(Some(Self {
            base_url: api_base,
            client,
            authorization,
            user_id,
        }))
    }

    async fn list_wallets(&self) -> Result<Vec<WalletResponse>> {
        let url = format!("{}wallets", self.base_url);
        let mut request = self.client.get(&url);
        if let Some(auth) = &self.authorization {
            request = request.header("Authorization", auth);
        }
        request = request.header("X-User-ID", &self.user_id);

        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(anyhow!(
                "wallet-vault returned status {}",
                response.status()
            ));
        }

        let payload: WalletListResponse = response.json().await?;
        Ok(payload.wallets)
    }
}

#[derive(Clone)]
pub struct WalletInventoryFetcher {
    vault: WalletVaultClient,
    shyft_client: Arc<ShyftClient>,
}

impl WalletInventoryFetcher {
    pub fn from_env() -> Result<Option<Self>> {
        let vault = match WalletVaultClient::from_env()? {
            Some(client) => client,
            None => return Ok(None),
        };

        let api_key = std::env::var("X_TOKEN").unwrap_or_default();
        if api_key.trim().is_empty() {
            warn!("X_TOKEN not set – cannot query wallet balances via Shyft");
            return Ok(None);
        }

        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .or_else(|_| std::env::var("RPC_URL"))
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        let jito_url = std::env::var("JITO_API_URL")
            .unwrap_or_else(|_| "https://jito-api.jito.wtf".to_string());

        let shyft_client = Arc::new(ShyftClient::new(api_key, rpc_url, jito_url)?);

        Ok(Some(Self::new(vault, shyft_client)))
    }

    pub fn new(vault: WalletVaultClient, shyft_client: Arc<ShyftClient>) -> Self {
        Self {
            vault,
            shyft_client,
        }
    }

    pub async fn fetch_inventory(&self) -> Result<HashMap<String, f64>> {
        let wallets = self.vault.list_wallets().await?;
        if wallets.is_empty() {
            return Ok(HashMap::new());
        }

        let futures = wallets.into_iter().map(|wallet| {
            let client = Arc::clone(&self.shyft_client);
            async move {
                let balances = client.get_wallet_balances(&wallet.pubkey).await?;
                Ok::<_, anyhow::Error>((wallet.pubkey, balances))
            }
        });

        let mut totals: HashMap<String, f64> = HashMap::new();
        let results = join_all(futures).await;
        for result in results {
            match result {
                Ok((_pubkey, balances)) => {
                    accumulate_sol(&mut totals, balances.sol_balance);
                    accumulate_tokens(&mut totals, &balances.token_balances);
                }
                Err(err) => {
                    warn!("Failed to fetch wallet balance: {}", err);
                }
            }
        }

        Ok(totals)
    }
}

fn accumulate_sol(totals: &mut HashMap<String, f64>, lamports: u64) {
    let amount = lamports as f64 / 1_000_000_000.0;
    totals
        .entry("SOL".to_string())
        .and_modify(|value| *value += amount)
        .or_insert(amount);
}

fn accumulate_tokens(totals: &mut HashMap<String, f64>, token_balances: &HashMap<String, u64>) {
    for (mint, amount_raw) in token_balances {
        let metadata = KNOWN_TOKENS.get(mint.as_str());
        let (symbol, decimals) = metadata
            .map(|meta| (meta.symbol, meta.decimals))
            .unwrap_or((mint.as_str(), 9));

        let amount = if *amount_raw == 0 {
            0.0
        } else {
            *amount_raw as f64 / 10f64.powi(decimals as i32)
        };

        totals
            .entry(symbol.to_string())
            .and_modify(|value| *value += amount)
            .or_insert(amount);
    }
}

struct TokenMetadata {
    symbol: &'static str,
    decimals: u8,
}

static KNOWN_TOKENS: Lazy<HashMap<&'static str, TokenMetadata>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "EPjFWdd5AufqSSqeM2qZ3ET9z5tWd3daYFJ2Q8X9MJp",
        TokenMetadata {
            symbol: "USDC",
            decimals: 6,
        },
    );
    map.insert(
        "Es9vMFrzaCERzvVXkR7E62HEheG533CuJCXLcKn6A5k",
        TokenMetadata {
            symbol: "USDT",
            decimals: 6,
        },
    );
    map.insert(
        "So11111111111111111111111111111111111111112",
        TokenMetadata {
            symbol: "SOL",
            decimals: 9,
        },
    );
    map.insert(
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZzr7r6zYgm6X8",
        TokenMetadata {
            symbol: "MSOL",
            decimals: 9,
        },
    );
    map
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_token_lookup_includes_sol() {
        assert!(KNOWN_TOKENS.contains_key("So11111111111111111111111111111111111111112"));
    }
}
