//! Consensus Engine for Ground Truth Data
//! 
//! Builds consensus from multiple data sources to create reliable ground truth.

use super::{RawTransactionData, GroundTruthData};
use anyhow::Result;
use std::collections::HashMap;

/// Engine for building consensus from multiple ground truth sources
pub struct ConsensusEngine;

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new() -> Self {
        Self
    }
    
    /// Build consensus ground truth from multiple sources
    pub fn build_consensus(&self, sources: Vec<RawTransactionData>) -> Result<GroundTruthData> {
        if sources.is_empty() {
            anyhow::bail!("No sources provided for consensus building");
        }
        
        // For now, use the first source as base
        // TODO: Implement proper consensus logic
        let primary_source = &sources[0];
        
        let consensus_data = GroundTruthData {
            transaction_signature: primary_source.transaction_signature.clone(),
            program_id: primary_source.program_id.clone(),
            instruction_name: "unknown".to_string(), // TODO: Extract from instruction data
            block_time: primary_source.timestamp,
            slot: 0, // TODO: Extract from source data
            token_in: "".to_string(), // TODO: Extract from parsed data
            token_out: "".to_string(),
            amount_in: 0,
            amount_out: 0,
            user: "".to_string(),
            pool: None,
            price: None,
            price_impact: None,
            fee_amount: None,
            fee_mint: None,
            source_consensus: self.calculate_consensus_score(&sources),
            sources_agreeing: sources.iter().map(|s| s.source.clone()).collect(),
            sources_disagreeing: vec![],
            raw_data_sources: sources.into_iter()
                .map(|s| (s.source.clone(), s.instruction_data))
                .collect(),
        };
        
        Ok(consensus_data)
    }
    
    /// Calculate consensus confidence score
    fn calculate_consensus_score(&self, sources: &[RawTransactionData]) -> f64 {
        // Simple implementation: average of source confidence scores
        let total_confidence: f64 = sources.iter().map(|s| s.confidence_score).sum();
        total_confidence / sources.len() as f64
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_consensus_with_single_source() {
        let engine = ConsensusEngine::new();
        let source = RawTransactionData {
            source: "test".to_string(),
            transaction_signature: "test_sig".to_string(),
            program_id: "test_program".to_string(),
            instruction_data: serde_json::json!({}),
            account_data: serde_json::json!({}),
            timestamp: Utc::now(),
            confidence_score: 0.9,
        };
        
        let consensus = engine.build_consensus(vec![source]).unwrap();
        assert_eq!(consensus.transaction_signature, "test_sig");
        assert_eq!(consensus.source_consensus, 0.9);
    }
}
