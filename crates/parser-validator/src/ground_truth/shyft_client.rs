//! Shyft API Client for Ground Truth Data
//! 
//! Leverages premium Shyft.to access for high-quality transaction data
//! and parsing validation. Uses both RPC and API endpoints.

use super::{RawTransactionData, GroundTruthData};
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Shyft API client for transaction data retrieval
pub struct ShyftClient {
    client: Client,
    api_key: String,
    rpc_url: String,
    api_base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftTransactionResponse {
    pub result: ShyftTransaction,
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftTransaction {
    pub signature: String,
    pub slot: u64,
    #[serde(rename = "blockTime")]
    pub block_time: i64,
    pub instructions: Vec<ShyftInstruction>,
    pub accounts: Vec<ShyftAccount>,
    pub parsed_instructions: Option<Vec<ShyftParsedInstruction>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftInstruction {
    #[serde(rename = "programId")]
    pub program_id: String,
    pub data: String,
    pub accounts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftAccount {
    pub address: String,
    pub executable: bool,
    pub lamports: u64,
    pub owner: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftParsedInstruction {
    #[serde(rename = "programId")]
    pub program_id: String,
    #[serde(rename = "instructionName")]
    pub instruction_name: String,
    pub accounts: HashMap<String, String>,
    pub args: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftSwapInfo {
    #[serde(rename = "tokenIn")]
    pub token_in: String,
    #[serde(rename = "tokenOut")]
    pub token_out: String,
    #[serde(rename = "amountIn")]
    pub amount_in: String,
    #[serde(rename = "amountOut")]
    pub amount_out: String,
    pub user: String,
    pub pool: Option<String>,
    pub price: Option<f64>,
    #[serde(rename = "priceImpact")]
    pub price_impact: Option<f64>,
    pub fee: Option<ShyftFeeInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ShyftFeeInfo {
    pub amount: String,
    pub mint: String,
}

impl ShyftClient {
    /// Create new Shyft client with API key
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            rpc_url: "https://rpc.shyft.to".to_string(),
            api_base_url: "https://api.shyft.to".to_string(),
        }
    }
    
    /// Get detailed transaction data from Shyft API
    pub async fn get_transaction_details(&self, tx_signature: &str) -> Result<RawTransactionData> {
        debug!("🔍 Fetching Shyft data for transaction: {}", tx_signature);
        
        // Try enhanced transaction API first (premium feature)
        match self.get_enhanced_transaction(tx_signature).await {
            Ok(data) => return Ok(data),
            Err(e) => {
                warn!("Enhanced Shyft API failed, falling back to standard: {}", e);
            }
        }
        
        // Fallback to standard RPC call
        self.get_standard_transaction(tx_signature).await
    }
    
    /// Get enhanced transaction data with parsed instructions (premium)
    async fn get_enhanced_transaction(&self, tx_signature: &str) -> Result<RawTransactionData> {
        let url = format!(
            "{}/v1/transaction/parsed?network=mainnet-beta&txn_signature={}",
            self.api_base_url, tx_signature
        );
        
        let response = self.client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Shyft API request failed: {}", response.status());
        }
        
        let shyft_response: ShyftTransactionResponse = response.json().await?;
        
        if !shyft_response.success {
            anyhow::bail!("Shyft API returned success=false");
        }
        
        self.convert_shyft_data(shyft_response.result).await
    }
    
    /// Get standard transaction data via RPC
    async fn get_standard_transaction(&self, tx_signature: &str) -> Result<RawTransactionData> {
        let rpc_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                tx_signature,
                {
                    "encoding": "json",
                    "maxSupportedTransactionVersion": 0
                }
            ]
        });
        
        let response = self.client
            .post(&self.rpc_url)
            .header("x-api-key", &self.api_key)
            .json(&rpc_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            anyhow::bail!("Shyft RPC request failed: {}", response.status());
        }
        
        let rpc_response: serde_json::Value = response.json().await?;
        
        if let Some(error) = rpc_response.get("error") {
            anyhow::bail!("Shyft RPC error: {}", error);
        }
        
        self.convert_rpc_data(rpc_response).await
    }
    
    /// Convert enhanced Shyft data to standardized format
    async fn convert_shyft_data(&self, shyft_tx: ShyftTransaction) -> Result<RawTransactionData> {
        let mut instruction_data = serde_json::Map::new();
        let mut account_data = serde_json::Map::new();
        
        // Extract primary program ID (first instruction)
        let program_id = shyft_tx.instructions
            .first()
            .map(|i| i.program_id.clone())
            .unwrap_or_default();
        
        // Process parsed instructions if available
        if let Some(parsed_instructions) = &shyft_tx.parsed_instructions {
            for parsed in parsed_instructions {
                if self.is_swap_instruction(parsed) {
                    if let Ok(swap_info) = self.extract_swap_info(parsed) {
                        instruction_data.insert("swap_data".to_string(), serde_json::to_value(swap_info)?);
                    }
                }
            }
        }
        
        // Add raw instruction data
        instruction_data.insert("instructions".to_string(), serde_json::to_value(&shyft_tx.instructions)?);
        
        // Add account information
        account_data.insert("accounts".to_string(), serde_json::to_value(&shyft_tx.accounts)?);
        
        Ok(RawTransactionData {
            source: "shyft".to_string(),
            transaction_signature: shyft_tx.signature,
            program_id,
            instruction_data: serde_json::Value::Object(instruction_data),
            account_data: serde_json::Value::Object(account_data),
            timestamp: DateTime::from_timestamp(shyft_tx.block_time, 0)
                .unwrap_or_else(Utc::now),
            confidence_score: 0.9, // High confidence for enhanced API
        })
    }
    
    /// Convert standard RPC data to standardized format
    async fn convert_rpc_data(&self, rpc_response: serde_json::Value) -> Result<RawTransactionData> {
        let result = rpc_response.get("result")
            .and_then(|r| r.get("transaction"))
            .ok_or_else(|| anyhow::anyhow!("Invalid RPC response structure"))?;
        
        let signature = rpc_response
            .pointer("/result/transaction/signatures/0")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_string();
        
        let program_id = result
            .pointer("/message/instructions/0/programId")
            .and_then(|p| p.as_str())
            .unwrap_or_default()
            .to_string();
        
        let block_time = rpc_response
            .pointer("/result/blockTime")
            .and_then(|t| t.as_i64())
            .unwrap_or_else(|| Utc::now().timestamp());
        
        Ok(RawTransactionData {
            source: "shyft_rpc".to_string(),
            transaction_signature: signature,
            program_id,
            instruction_data: result.get("message").cloned().unwrap_or_default(),
            account_data: result.get("meta").cloned().unwrap_or_default(),
            timestamp: DateTime::from_timestamp(block_time, 0)
                .unwrap_or_else(Utc::now),
            confidence_score: 0.8, // Medium confidence for standard RPC
        })
    }
    
    /// Check if instruction is a swap-related instruction
    fn is_swap_instruction(&self, instruction: &ShyftParsedInstruction) -> bool {
        matches!(instruction.instruction_name.to_lowercase().as_str(),
            "swap" | "swapv2" | "swap_base_in" | "swap_base_out" | 
            "place_order" | "place_take_order" | "buy" | "sell"
        )
    }
    
    /// Extract swap information from parsed instruction
    fn extract_swap_info(&self, instruction: &ShyftParsedInstruction) -> Result<ShyftSwapInfo> {
        let args = &instruction.args;
        let accounts = &instruction.accounts;
        
        let token_in = accounts.get("tokenAccountIn")
            .or_else(|| accounts.get("userSourceTokenAccount"))
            .unwrap_or(&"".to_string())
            .to_string();
        
        let token_out = accounts.get("tokenAccountOut")
            .or_else(|| accounts.get("userDestinationTokenAccount"))
            .unwrap_or(&"".to_string())
            .to_string();
        
        let amount_in = args.get("amountIn")
            .or_else(|| args.get("amount"))
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string();
        
        let amount_out = args.get("amountOut")
            .or_else(|| args.get("minimumAmountOut"))
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string();
        
        let user = accounts.get("userTransferAuthority")
            .or_else(|| accounts.get("user"))
            .or_else(|| accounts.get("signer"))
            .unwrap_or(&"".to_string())
            .to_string();
        
        let pool = accounts.get("whirlpool")
            .or_else(|| accounts.get("pool"))
            .or_else(|| accounts.get("market"))
            .map(|s| s.to_string());
        
        Ok(ShyftSwapInfo {
            token_in,
            token_out,
            amount_in,
            amount_out,
            user,
            pool,
            price: None, // Will be calculated later
            price_impact: None,
            fee: None,
        })
    }
    
    /// Test connection to Shyft API
    pub async fn test_connection(&self) -> Result<()> {
        let url = format!("{}/v1/rpc", self.api_base_url);
        
        let test_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSlot"
        });
        
        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&test_request)
            .send()
            .await?;
        
        if response.status().is_success() {
            debug!("✅ Shyft connectivity test passed");
            Ok(())
        } else {
            anyhow::bail!("Shyft connectivity test failed: {}", response.status());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shyft_client_creation() {
        let client = ShyftClient::new("test_key".to_string());
        assert_eq!(client.api_key, "test_key");
        assert!(client.rpc_url.contains("shyft.to"));
    }
    
    #[test]
    fn test_is_swap_instruction() {
        let client = ShyftClient::new("test".to_string());
        
        let swap_instruction = ShyftParsedInstruction {
            program_id: "test".to_string(),
            instruction_name: "swap".to_string(),
            accounts: HashMap::new(),
            args: HashMap::new(),
        };
        
        assert!(client.is_swap_instruction(&swap_instruction));
        
        let non_swap_instruction = ShyftParsedInstruction {
            program_id: "test".to_string(),
            instruction_name: "transfer".to_string(),
            accounts: HashMap::new(),
            args: HashMap::new(),
        };
        
        assert!(!client.is_swap_instruction(&non_swap_instruction));
    }
}
