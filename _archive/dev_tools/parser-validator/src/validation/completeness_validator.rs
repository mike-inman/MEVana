//! Completeness Validation Engine
//! 
//! Validates how completely transactions are parsed (field coverage).

use super::ParsedTransactionData;
use crate::ground_truth::GroundTruthData;
use anyhow::Result;

/// Result of completeness validation
#[derive(Debug, Clone)]
pub struct CompletenessResult {
    pub ratio: f64,
    pub fields_parsed: u32,
    pub fields_expected: u32,
    pub missing_fields: Vec<String>,
}

/// Engine for validating parsing completeness
pub struct CompletenessValidator;

impl CompletenessValidator {
    /// Create new completeness validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate completeness of parsed data
    pub async fn validate_completeness(
        &self,
        parsed: &ParsedTransactionData,
        _ground_truth: &GroundTruthData,
    ) -> Result<CompletenessResult> {
        let mut fields_parsed = 0u32;
        let mut missing_fields = Vec::new();
        
        // Core fields
        if !parsed.transaction_signature.is_empty() {
            fields_parsed += 1;
        } else {
            missing_fields.push("transaction_signature".to_string());
        }
        
        if !parsed.program_id.is_empty() {
            fields_parsed += 1;
        } else {
            missing_fields.push("program_id".to_string());
        }
        
        if !parsed.instruction_name.is_empty() {
            fields_parsed += 1;
        } else {
            missing_fields.push("instruction_name".to_string());
        }
        
        // Swap-specific fields
        if parsed.token_in.is_some() {
            fields_parsed += 1;
        } else {
            missing_fields.push("token_in".to_string());
        }
        
        if parsed.token_out.is_some() {
            fields_parsed += 1;
        } else {
            missing_fields.push("token_out".to_string());
        }
        
        if parsed.amount_in.is_some() {
            fields_parsed += 1;
        } else {
            missing_fields.push("amount_in".to_string());
        }
        
        if parsed.amount_out.is_some() {
            fields_parsed += 1;
        } else {
            missing_fields.push("amount_out".to_string());
        }
        
        if parsed.user.is_some() {
            fields_parsed += 1;
        } else {
            missing_fields.push("user".to_string());
        }
        
        // Optional fields (count but don't penalize for missing)
        if parsed.pool.is_some() {
            fields_parsed += 1;
        }
        
        if parsed.price.is_some() {
            fields_parsed += 1;
        }
        
        if parsed.fee_amount.is_some() {
            fields_parsed += 1;
        }
        
        let fields_expected = 11u32; // Total possible fields
        let ratio = fields_parsed as f64 / fields_expected as f64;
        
        Ok(CompletenessResult {
            ratio,
            fields_parsed,
            fields_expected,
            missing_fields,
        })
    }
}

impl Default for CompletenessValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_complete_parsing() {
        let validator = CompletenessValidator::new();
        
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
        let result = runtime.block_on(validator.validate_completeness(&parsed, &ground_truth)).unwrap();
        
        // Should have high completeness ratio
        assert!(result.ratio > 0.9);
        assert_eq!(result.fields_parsed, 11);
    }
}
