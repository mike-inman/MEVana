//! Validation Engines
//! 
//! Core validation logic for accuracy, completeness, and performance testing
//! of parser output against ground truth data.

pub mod accuracy_validator;
pub mod completeness_validator;
pub mod latency_validator;
pub mod tolerance_config;

pub use accuracy_validator::AccuracyValidator;
pub use completeness_validator::CompletenessValidator;
pub use latency_validator::LatencyValidator;
pub use tolerance_config::ToleranceConfig;

use crate::ground_truth::{GroundTruthAggregator, GroundTruthData};
use crate::database::ValidationDatabase;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Comprehensive validation result for a single transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub id: String,
    pub transaction_signature: String,
    pub program_id: String,
    pub instruction_name: String,
    
    // Accuracy metrics
    pub overall_accuracy: f64,
    pub field_accuracies: HashMap<String, f64>,
    pub price_deviation_bps: Option<u16>,
    pub amount_deviation_bps: Option<u16>,
    
    // Completeness metrics
    pub completeness_ratio: f64,
    pub fields_parsed: u32,
    pub fields_expected: u32,
    pub missing_fields: Vec<String>,
    
    // Performance metrics
    pub parse_time_us: u64,
    pub validation_time_us: u64,
    
    // Ground truth metadata
    pub ground_truth_confidence: f64,
    pub ground_truth_sources: Vec<String>,
    
    // Error information
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    
    // Timestamps
    pub block_time: DateTime<Utc>,
    pub validation_time: DateTime<Utc>,
}

/// Validation error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub category: String,
    pub field: Option<String>,
    pub message: String,
    pub severity: ErrorSeverity,
    pub details: HashMap<String, serde_json::Value>,
}

/// Validation warning information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub category: String,
    pub field: Option<String>,
    pub message: String,
    pub recommendation: Option<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Parsed transaction data from our system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTransactionData {
    pub transaction_signature: String,
    pub program_id: String,
    pub instruction_name: String,
    pub block_time: DateTime<Utc>,
    pub slot: u64,
    
    // Swap data
    pub token_in: Option<String>,
    pub token_out: Option<String>,
    pub amount_in: Option<u64>,
    pub amount_out: Option<u64>,
    pub user: Option<String>,
    pub pool: Option<String>,
    
    // Additional fields
    pub price: Option<f64>,
    pub price_impact: Option<f64>,
    pub fee_amount: Option<u64>,
    pub fee_mint: Option<String>,
    
    // Parsing metadata
    pub parser_version: String,
    pub parse_duration_us: u64,
    pub confidence_score: f64,
}

/// Main validation framework coordinating all validation engines
pub struct ValidationFramework {
    ground_truth: GroundTruthAggregator,
    database: ValidationDatabase,
    accuracy_validator: AccuracyValidator,
    completeness_validator: CompletenessValidator,
    latency_validator: LatencyValidator,
    tolerance_config: ToleranceConfig,
}

impl ValidationFramework {
    /// Create new validation framework
    pub fn new(ground_truth: GroundTruthAggregator, database: ValidationDatabase) -> Self {
        let tolerance_config = ToleranceConfig::default();
        
        Self {
            ground_truth,
            database,
            accuracy_validator: AccuracyValidator::new(tolerance_config.clone()),
            completeness_validator: CompletenessValidator::new(),
            latency_validator: LatencyValidator::new(),
            tolerance_config,
        }
    }
    
    /// Validate a single transaction against ground truth
    pub async fn validate_transaction(
        &self,
        parsed_data: ParsedTransactionData,
        ground_truth: GroundTruthData,
    ) -> Result<ValidationResult> {
        let validation_start = std::time::Instant::now();
        let validation_id = Uuid::new_v4().to_string();
        
        tracing::debug!("🔍 Validating transaction: {}", parsed_data.transaction_signature);
        
        // Run accuracy validation
        let accuracy_result = self.accuracy_validator
            .validate_parsed_data(&parsed_data, &ground_truth)
            .await?;
        
        // Run completeness validation
        let completeness_result = self.completeness_validator
            .validate_completeness(&parsed_data, &ground_truth)
            .await?;
        
        // Collect errors and warnings
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Check accuracy thresholds
        if accuracy_result.overall_accuracy < self.tolerance_config.min_accuracy_threshold {
            errors.push(ValidationError {
                category: "accuracy".to_string(),
                field: None,
                message: format!(
                    "Overall accuracy {:.2}% below threshold {:.2}%",
                    accuracy_result.overall_accuracy * 100.0,
                    self.tolerance_config.min_accuracy_threshold * 100.0
                ),
                severity: ErrorSeverity::High,
                details: HashMap::new(),
            });
        }
        
        // Check completeness thresholds
        if completeness_result.ratio < self.tolerance_config.min_completeness_threshold {
            errors.push(ValidationError {
                category: "completeness".to_string(),
                field: None,
                message: format!(
                    "Completeness {:.2}% below threshold {:.2}%",
                    completeness_result.ratio * 100.0,
                    self.tolerance_config.min_completeness_threshold * 100.0
                ),
                severity: ErrorSeverity::Medium,
                details: HashMap::new(),
            });
        }
        
        // Check latency thresholds
        if parsed_data.parse_duration_us > self.tolerance_config.max_latency_threshold_us {
            warnings.push(ValidationWarning {
                category: "performance".to_string(),
                field: Some("parse_time".to_string()),
                message: format!(
                    "Parse time {}μs exceeds threshold {}μs",
                    parsed_data.parse_duration_us,
                    self.tolerance_config.max_latency_threshold_us
                ),
                recommendation: Some("Consider parser optimization".to_string()),
            });
        }
        
        let validation_duration = validation_start.elapsed();
        
        let result = ValidationResult {
            id: validation_id,
            transaction_signature: parsed_data.transaction_signature.clone(),
            program_id: parsed_data.program_id.clone(),
            instruction_name: parsed_data.instruction_name.clone(),
            overall_accuracy: accuracy_result.overall_accuracy,
            field_accuracies: accuracy_result.field_accuracies,
            price_deviation_bps: accuracy_result.price_deviation_bps,
            amount_deviation_bps: accuracy_result.amount_deviation_bps,
            completeness_ratio: completeness_result.ratio,
            fields_parsed: completeness_result.fields_parsed,
            fields_expected: completeness_result.fields_expected,
            missing_fields: completeness_result.missing_fields,
            parse_time_us: parsed_data.parse_duration_us,
            validation_time_us: validation_duration.as_micros() as u64,
            ground_truth_confidence: ground_truth.source_consensus,
            ground_truth_sources: ground_truth.sources_agreeing,
            errors,
            warnings,
            block_time: parsed_data.block_time,
            validation_time: Utc::now(),
        };
        
        // Store result in database
        self.database.store_validation_result(&result).await?;
        
        tracing::info!(
            "✅ Validation complete for {}: {:.1}% accuracy, {:.1}% completeness",
            parsed_data.transaction_signature,
            result.overall_accuracy * 100.0,
            result.completeness_ratio * 100.0
        );
        
        Ok(result)
    }
    
    /// Get ground truth for a transaction
    pub async fn get_ground_truth(&self, tx_signature: &str) -> Result<GroundTruthData> {
        self.ground_truth.get_ground_truth(tx_signature).await
    }
    
    /// Parse transaction with our system (placeholder - connects to normalizer-service)
    pub async fn parse_transaction(&self, transaction: &Transaction) -> Result<ParsedTransactionData> {
        // TODO: Integrate with actual normalizer-service parsing
        // For now, return a placeholder
        Ok(ParsedTransactionData {
            transaction_signature: transaction.signature.clone(),
            program_id: "placeholder".to_string(),
            instruction_name: "placeholder".to_string(),
            block_time: Utc::now(),
            slot: 0,
            token_in: None,
            token_out: None,
            amount_in: None,
            amount_out: None,
            user: None,
            pool: None,
            price: None,
            price_impact: None,
            fee_amount: None,
            fee_mint: None,
            parser_version: "0.1.0".to_string(),
            parse_duration_us: 1000,
            confidence_score: 0.9,
        })
    }
}

/// Placeholder transaction structure
#[derive(Debug, Clone)]
pub struct Transaction {
    pub signature: String,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_creation() {
        let result = ValidationResult {
            id: "test_id".to_string(),
            transaction_signature: "test_sig".to_string(),
            program_id: "test_program".to_string(),
            instruction_name: "swap".to_string(),
            overall_accuracy: 0.99,
            field_accuracies: HashMap::new(),
            price_deviation_bps: Some(5),
            amount_deviation_bps: Some(1),
            completeness_ratio: 0.95,
            fields_parsed: 8,
            fields_expected: 8,
            missing_fields: vec![],
            parse_time_us: 5000,
            validation_time_us: 10000,
            ground_truth_confidence: 0.9,
            ground_truth_sources: vec!["shyft".to_string()],
            errors: vec![],
            warnings: vec![],
            block_time: Utc::now(),
            validation_time: Utc::now(),
        };
        
        assert_eq!(result.overall_accuracy, 0.99);
        assert_eq!(result.completeness_ratio, 0.95);
        assert_eq!(result.fields_parsed, 8);
    }
    
    #[test]
    fn test_error_severity() {
        let error = ValidationError {
            category: "test".to_string(),
            field: None,
            message: "Test error".to_string(),
            severity: ErrorSeverity::High,
            details: HashMap::new(),
        };
        
        assert!(matches!(error.severity, ErrorSeverity::High));
    }
}
