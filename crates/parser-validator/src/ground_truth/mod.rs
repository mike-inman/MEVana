//! Ground Truth Data Sources
//! 
//! Multi-source validation system that aggregates transaction data from
//! Shyft, Helius, and Solscan APIs to build consensus ground truth.

pub mod shyft_client;
pub mod helius_client;
pub mod solscan_client;
pub mod consensus_engine;

pub use shyft_client::ShyftClient;
pub use helius_client::HeliusClient;
pub use solscan_client::SolscanClient;
pub use consensus_engine::ConsensusEngine;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Ground truth data for a parsed transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthData {
    pub transaction_signature: String,
    pub program_id: String,
    pub instruction_name: String,
    pub block_time: DateTime<Utc>,
    pub slot: u64,
    
    // Swap-specific data
    pub token_in: String,
    pub token_out: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub user: String,
    pub pool: Option<String>,
    
    // Price information
    pub price: Option<f64>,
    pub price_impact: Option<f64>,
    
    // Fee information
    pub fee_amount: Option<u64>,
    pub fee_mint: Option<String>,
    
    // Metadata
    pub source_consensus: f64, // 0.0-1.0 confidence score
    pub sources_agreeing: Vec<String>,
    pub sources_disagreeing: Vec<String>,
    pub raw_data_sources: HashMap<String, serde_json::Value>,
}

/// Raw transaction data from external sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTransactionData {
    pub source: String,
    pub transaction_signature: String,
    pub program_id: String,
    pub instruction_data: serde_json::Value,
    pub account_data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub confidence_score: f64,
}

/// Aggregator for building consensus ground truth from multiple sources
pub struct GroundTruthAggregator {
    shyft: ShyftClient,
    helius: HeliusClient,
    solscan: SolscanClient,
    consensus: ConsensusEngine,
}

impl GroundTruthAggregator {
    /// Create new aggregator with API clients
    pub fn new(shyft_key: String, helius_key: String, solscan_key: String) -> Self {
        Self {
            shyft: ShyftClient::new(shyft_key),
            helius: HeliusClient::new(helius_key),
            solscan: SolscanClient::new(solscan_key),
            consensus: ConsensusEngine::new(),
        }
    }
    
    /// Get consensus ground truth for a transaction
    pub async fn get_ground_truth(&self, tx_signature: &str) -> Result<GroundTruthData> {
        tracing::debug!("Fetching ground truth for transaction: {}", tx_signature);
        
        // Fetch from all sources in parallel
        let (shyft_result, helius_result, solscan_result): (Result<RawTransactionData>, Result<RawTransactionData>, Result<RawTransactionData>) = tokio::join!(
            self.shyft.get_transaction_details(tx_signature),
            self.helius.get_transaction_details(tx_signature),
            self.solscan.get_transaction_details(tx_signature)
        );
        
        // Collect successful results
        let mut sources = Vec::new();
        
        if let Ok(data) = shyft_result {
            tracing::debug!("✅ Shyft data retrieved for {}", tx_signature);
            sources.push(data);
        } else {
            tracing::warn!("❌ Shyft failed for {}: {:?}", tx_signature, shyft_result.err());
        }
        
        if let Ok(data) = helius_result {
            tracing::debug!("✅ Helius data retrieved for {}", tx_signature);
            sources.push(data);
        } else {
            tracing::warn!("❌ Helius failed for {}: {:?}", tx_signature, helius_result.err());
        }
        
        if let Ok(data) = solscan_result {
            tracing::debug!("✅ Solscan data retrieved for {}", tx_signature);
            sources.push(data);
        } else {
            tracing::warn!("❌ Solscan failed for {}: {:?}", tx_signature, solscan_result.err());
        }
        
        if sources.is_empty() {
            anyhow::bail!("No ground truth sources available for transaction: {}", tx_signature);
        }
        
        // Build consensus from available sources
        let ground_truth = self.consensus.build_consensus(sources)?;
        
        tracing::info!(
            "📊 Built ground truth for {} with {:.1}% consensus from {} sources",
            tx_signature,
            ground_truth.source_consensus * 100.0,
            ground_truth.sources_agreeing.len()
        );
        
        Ok(ground_truth)
    }
    
    /// Get multiple ground truths in batch
    pub async fn get_ground_truth_batch(&self, tx_signatures: &[String]) -> Result<Vec<GroundTruthData>> {
        let mut results = Vec::new();
        
        // Process in chunks to avoid overwhelming APIs
        const BATCH_SIZE: usize = 10;
        
        for chunk in tx_signatures.chunks(BATCH_SIZE) {
            let mut batch_results = Vec::new();
            
            for signature in chunk {
                match self.get_ground_truth(signature).await {
                    Ok(data) => batch_results.push(data),
                    Err(e) => {
                        tracing::warn!("Failed to get ground truth for {}: {}", signature, e);
                    }
                }
            }
            
            results.extend(batch_results);
            
            // Rate limiting pause
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        Ok(results)
    }
    
    /// Test connectivity to all ground truth sources
    pub async fn test_connectivity(&self) -> Result<()> {
        tracing::info!("🔧 Testing ground truth source connectivity");
        
        let shyft_test = self.shyft.test_connection().await;
        let helius_test = self.helius.test_connection().await;
        let solscan_test = self.solscan.test_connection().await;
        
        match (shyft_test, helius_test, solscan_test) {
            (Ok(_), Ok(_), Ok(_)) => {
                tracing::info!("✅ All ground truth sources connected successfully");
                Ok(())
            },
            _ => {
                tracing::error!("❌ Some ground truth sources failed connectivity test");
                anyhow::bail!("Ground truth source connectivity issues detected");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ground_truth_data_creation() {
        let data = GroundTruthData {
            transaction_signature: "test_sig".to_string(),
            program_id: "test_program".to_string(),
            instruction_name: "swap".to_string(),
            block_time: Utc::now(),
            slot: 12345,
            token_in: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            token_out: "So11111111111111111111111111111111111111112".to_string(),
            amount_in: 1000000,
            amount_out: 2000000,
            user: "user_address".to_string(),
            pool: Some("pool_address".to_string()),
            price: Some(2.0),
            price_impact: Some(0.01),
            fee_amount: Some(1000),
            fee_mint: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
            source_consensus: 1.0,
            sources_agreeing: vec!["shyft".to_string(), "helius".to_string()],
            sources_disagreeing: vec![],
            raw_data_sources: HashMap::new(),
        };
        
        assert_eq!(data.transaction_signature, "test_sig");
        assert_eq!(data.source_consensus, 1.0);
    }
}
