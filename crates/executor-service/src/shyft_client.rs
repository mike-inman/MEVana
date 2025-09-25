#![allow(dead_code)]

use anyhow::Result;
use log::{debug, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalances {
    pub sol_balance: u64,                     // in lamports
    pub token_balances: HashMap<String, u64>, // mint -> balance
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitoBundleRequest {
    pub transactions: Vec<String>, // base64 encoded transactions
    pub priority_fee: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitoBundleResponse {
    pub bundle_id: String,
    pub status: String,
    pub landed_slot: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShyftRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShyftRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}

/// High-performance Shyft.to client with premium endpoint access
pub struct ShyftClient {
    client: Client,
    api_key: String,
    rpc_url: String,
    jito_url: String,
    request_id_counter: std::sync::atomic::AtomicU64,
}

impl ShyftClient {
    pub fn new(api_key: String, rpc_url: String, jito_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()?;

        Ok(Self {
            client,
            api_key,
            rpc_url,
            jito_url,
            request_id_counter: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// Get wallet balances using Shyft.to premium RPC
    pub async fn get_wallet_balances(&self, wallet_address: &str) -> Result<WalletBalances> {
        // Get SOL balance
        let sol_balance = self.get_sol_balance(wallet_address).await?;

        // Get token balances
        let token_balances = self.get_token_balances(wallet_address).await?;

        Ok(WalletBalances {
            sol_balance,
            token_balances,
        })
    }

    /// Submit Jito bundle via Shyft.to premium endpoint
    pub async fn submit_jito_bundle(
        &self,
        transactions: Vec<String>,
        priority_fee: Option<u64>,
    ) -> Result<JitoBundleResponse> {
        let request = JitoBundleRequest {
            transactions,
            priority_fee,
        };

        let response = self
            .client
            .post(format!("{}/api/v1/bundle", self.jito_url))
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Jito bundle submission failed: {}", error_text);
        }

        let bundle_response: JitoBundleResponse = response.json().await?;

        info!(
            "Jito bundle submitted successfully: {}",
            bundle_response.bundle_id
        );

        Ok(bundle_response)
    }

    /// Get bundle status
    pub async fn get_bundle_status(&self, bundle_id: &str) -> Result<JitoBundleResponse> {
        let response = self
            .client
            .get(format!("{}/api/v1/bundle/{}", self.jito_url, bundle_id))
            .header("x-api-key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Bundle status check failed: {}", error_text);
        }

        let bundle_status: JitoBundleResponse = response.json().await?;

        Ok(bundle_status)
    }

    /// Get account info using Shyft.to RPC
    pub async fn get_account_info(&self, account_address: &str) -> Result<serde_json::Value> {
        let request = ShyftRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "getAccountInfo".to_string(),
            params: serde_json::json!([
                account_address,
                {
                    "encoding": "base64",
                    "commitment": "confirmed"
                }
            ]),
        };

        let response = self.send_rpc_request(request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("RPC error: {}", error);
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))
    }

    /// Get token accounts by owner
    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &str,
        mint: Option<&str>,
    ) -> Result<serde_json::Value> {
        let filter = if let Some(mint) = mint {
            serde_json::json!({
                "mint": mint
            })
        } else {
            serde_json::json!({
                "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            })
        };

        let request = ShyftRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "getTokenAccountsByOwner".to_string(),
            params: serde_json::json!([
                owner,
                filter,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]),
        };

        let response = self.send_rpc_request(request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("RPC error: {}", error);
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))
    }

    /// Simulate transaction
    pub async fn simulate_transaction(&self, transaction: &str) -> Result<serde_json::Value> {
        let request = ShyftRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "simulateTransaction".to_string(),
            params: serde_json::json!([
                transaction,
                {
                    "encoding": "base64",
                    "commitment": "confirmed",
                    "replaceRecentBlockhash": true,
                    "sigVerify": false,
                    "innerInstructions": false
                }
            ]),
        };

        let response = self.send_rpc_request(request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("Simulation error: {}", error);
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in simulation response"))
    }

    /// Submit a signed transaction (base64) to the RPC endpoint
    pub async fn send_transaction(
        &self,
        transaction: &str,
        skip_preflight: bool,
    ) -> Result<String> {
        let request = ShyftRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "sendTransaction".to_string(),
            params: serde_json::json!([
                transaction,
                {
                    "encoding": "base64",
                    "maxRetries": 2,
                    "skipPreflight": skip_preflight
                }
            ]),
        };

        let response = self.send_rpc_request(request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("sendTransaction RPC error: {}", error);
        }

        let result = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in sendTransaction response"))?;

        if let Some(signature) = result.as_str() {
            return Ok(signature.to_string());
        }

        if let Some(signature) = result.get("result").and_then(|value| value.as_str()) {
            return Ok(signature.to_string());
        }

        anyhow::bail!("unexpected sendTransaction response format: {}", result)
    }

    /// Get recent blockhash
    pub async fn get_recent_blockhash(&self) -> Result<String> {
        let request = ShyftRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "getLatestBlockhash".to_string(),
            params: serde_json::json!([{
                "commitment": "confirmed"
            }]),
        };

        let response = self.send_rpc_request(request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("RPC error: {}", error);
        }

        let result = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;

        let blockhash = result["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid blockhash format"))?;

        Ok(blockhash.to_string())
    }

    // Private helper methods

    async fn get_sol_balance(&self, wallet_address: &str) -> Result<u64> {
        let request = ShyftRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id(),
            method: "getBalance".to_string(),
            params: serde_json::json!([
                wallet_address,
                {
                    "commitment": "confirmed"
                }
            ]),
        };

        let response = self.send_rpc_request(request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("Balance RPC error: {}", error);
        }

        let result = response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in balance response"))?;

        let balance = result["value"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Invalid balance format"))?;

        Ok(balance)
    }

    async fn get_token_balances(&self, wallet_address: &str) -> Result<HashMap<String, u64>> {
        let token_accounts = self
            .get_token_accounts_by_owner(wallet_address, None)
            .await?;

        let mut token_balances = HashMap::new();

        if let Some(accounts) = token_accounts["value"].as_array() {
            for account in accounts {
                if let (Some(mint), Some(amount)) = (
                    account["account"]["data"]["parsed"]["info"]["mint"].as_str(),
                    account["account"]["data"]["parsed"]["info"]["tokenAmount"]["amount"].as_str(),
                ) {
                    if let Ok(balance) = amount.parse::<u64>() {
                        token_balances.insert(mint.to_string(), balance);
                    }
                }
            }
        }

        Ok(token_balances)
    }

    async fn send_rpc_request(&self, request: ShyftRpcRequest) -> Result<ShyftRpcResponse> {
        debug!(
            "Sending RPC request: {} (ID: {})",
            request.method, request.id
        );

        let response = self
            .client
            .post(&self.rpc_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("RPC request failed: {}", error_text);
        }

        let rpc_response: ShyftRpcResponse = response.json().await?;

        debug!("RPC response received for ID: {}", rpc_response.id);

        Ok(rpc_response)
    }

    fn next_request_id(&self) -> u64 {
        self.request_id_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}
