//! Accuracy Validation Engine
//! 
//! Validates field-level accuracy of parsed data against ground truth.

use super::{ParsedTransactionData, ToleranceConfig};
use crate::ground_truth::GroundTruthData;
use anyhow::Result;
use std::collections::HashMap;

/// Result of accuracy validation
#[derive(Debug, Clone)]
pub struct AccuracyResult {
    pub overall_accuracy: f64,
    pub field_accuracies: HashMap<String, f64>,
    pub price_deviation_bps: Option<u16>,
    pub amount_deviation_bps: Option<u16>,
}

/// Engine for validating parsing accuracy
pub struct AccuracyValidator {
    tolerance_config: ToleranceConfig,
}

impl AccuracyValidator {
    /// Create new accuracy validator
    pub fn new(tolerance_config: ToleranceConfig) -> Self {
        Self { tolerance_config }
    }
    
    /// Validate parsed data against ground truth
    pub async fn validate_parsed_data(
        &self,
        parsed: &ParsedTransactionData,
        ground_truth: &GroundTruthData,
    ) -> Result<AccuracyResult> {
        let mut field_accuracies = HashMap::new();
        
        // Validate transaction signature
        field_accuracies.insert(
            "transaction_signature".to_string(),
            if parsed.transaction_signature == ground_truth.transaction_signature { 1.0 } else { 0.0 },
        );
        
        // Validate program ID
        field_accuracies.insert(
            "program_id".to_string(),
            if parsed.program_id == ground_truth.program_id { 1.0 } else { 0.0 },
        );
        
        // Validate token addresses
        if let (Some(parsed_token_in), token_in) = (&parsed.token_in, &ground_truth.token_in) {
            field_accuracies.insert(
                "token_in".to_string(),
                if parsed_token_in == token_in { 1.0 } else { 0.0 },
            );
        }
        
        if let (Some(parsed_token_out), token_out) = (&parsed.token_out, &ground_truth.token_out) {
            field_accuracies.insert(
                "token_out".to_string(),
                if parsed_token_out == token_out { 1.0 } else { 0.0 },
            );
        }
        
        // Validate amounts with tolerance
        let amount_in_accuracy = if let Some(parsed_amount) = parsed.amount_in {
            if self.tolerance_config.is_amount_within_tolerance(parsed_amount, ground_truth.amount_in) {
                1.0
            } else {
                0.0
            }
        } else {
            0.5 // Partial credit for missing data
        };
        field_accuracies.insert("amount_in".to_string(), amount_in_accuracy);
        
        let amount_out_accuracy = if let Some(parsed_amount) = parsed.amount_out {
            if self.tolerance_config.is_amount_within_tolerance(parsed_amount, ground_truth.amount_out) {
                1.0
            } else {
                0.0
            }
        } else {
            0.5 // Partial credit for missing data
        };
        field_accuracies.insert("amount_out".to_string(), amount_out_accuracy);
        
        // Calculate price deviations
        let price_deviation_bps = if let (Some(parsed_price), Some(truth_price)) = (parsed.price, ground_truth.price) {
            Some(self.tolerance_config.calculate_price_deviation_bps(parsed_price, truth_price))
        } else {
            None
        };
        
        // Calculate amount deviations
        let amount_deviation_bps = if let Some(parsed_amount) = parsed.amount_in {
            Some(self.tolerance_config.calculate_amount_deviation_bps(parsed_amount, ground_truth.amount_in))
        } else {
            None
        };
        
        // Calculate overall accuracy
        let overall_accuracy = field_accuracies.values().sum::<f64>() / field_accuracies.len() as f64;
        
        Ok(AccuracyResult {
            overall_accuracy,
            field_accuracies,
            price_deviation_bps,
            amount_deviation_bps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_perfect_accuracy() {
        let config = ToleranceConfig::default();
        let validator = AccuracyValidator::new(config);
        
        let parsed = ParsedTransactionData {
            transaction_signature: "test_sig".to_string(),
            program_id: "test_program".to_string(),
            instruction_name: "swap".to_string(),
            block_time: Utc::now(),
            slot: 12345,
            token_in: Some("token_a".to_string()),
            token_out: Some("token_b".to_string()),
            amount_in: Some(1000000),
            amount_out: Some(2000000),
            user: Some("user_address".to_string()),
            pool: Some("pool_address".to_string()),
            price: Some(2.0),
            price_impact: Some(0.01),
            fee_amount: Some(1000),
            fee_mint: Some("token_a".to_string()),
            parser_version: "0.1.0".to_string(),
            parse_duration_us: 5000,
            confidence_score: 0.9,
        };
        
        let ground_truth = GroundTruthData {
            transaction_signature: "test_sig".to_string(),
            program_id: "test_program".to_string(),
            instruction_name: "swap".to_string(),
            block_time: Utc::now(),
            slot: 12345,
            token_in: "token_a".to_string(),
            token_out: "token_b".to_string(),
            amount_in: 1000000,
            amount_out: 2000000,
            user: "user_address".to_string(),
            pool: Some("pool_address".to_string()),
            price: Some(2.0),
            price_impact: Some(0.01),
            fee_amount: Some(1000),
            fee_mint: Some("token_a".to_string()),
            source_consensus: 1.0,
            sources_agreeing: vec!["shyft".to_string()],
            sources_disagreeing: vec![],
            raw_data_sources: std::collections::HashMap::new(),
        };
        
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(validator.validate_parsed_data(&parsed, &ground_truth)).unwrap();
        
        // Should have high accuracy for matching data
        assert!(result.overall_accuracy > 0.8);
    }
}
