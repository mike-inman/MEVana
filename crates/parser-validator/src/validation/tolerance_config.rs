//! Tolerance Configuration for Validation
//! 
//! Configurable thresholds and tolerances for validation accuracy,
//! completeness, and performance metrics.

use serde::{Deserialize, Serialize};

/// Configuration for validation tolerances and thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToleranceConfig {
    // Accuracy thresholds
    pub min_accuracy_threshold: f64,           // Overall accuracy minimum (0.99 = 99%)
    pub price_tolerance_bps: u16,              // Price deviation tolerance in basis points
    pub amount_tolerance_bps: u16,             // Amount deviation tolerance in basis points
    
    // Completeness thresholds
    pub min_completeness_threshold: f64,       // Minimum field completeness (0.95 = 95%)
    pub critical_fields: Vec<String>,          // Fields that must always be present
    
    // Performance thresholds
    pub max_latency_threshold_us: u64,         // Maximum parse time in microseconds
    pub max_validation_time_us: u64,           // Maximum validation time in microseconds
    
    // Address validation
    pub address_exact_match: bool,             // Require exact address matches
    pub case_sensitive_symbols: bool,          // Case sensitivity for token symbols
    
    // Timestamp tolerances
    pub timestamp_tolerance_ms: u64,           // Block time variation tolerance
    
    // Numeric precision
    pub decimal_precision: u8,                 // Decimal places for price comparisons
    pub rounding_tolerance: f64,               // Tolerance for rounding differences
    
    // Error thresholds
    pub max_error_rate: f64,                   // Maximum acceptable error rate
    pub max_warning_rate: f64,                 // Maximum acceptable warning rate
    
    // Confidence requirements
    pub min_ground_truth_confidence: f64,      // Minimum ground truth consensus
    pub min_parser_confidence: f64,            // Minimum parser confidence score
}

impl Default for ToleranceConfig {
    fn default() -> Self {
        Self {
            // Accuracy: 99% overall, tight tolerances for financial data
            min_accuracy_threshold: 0.99,
            price_tolerance_bps: 1,      // 0.01% price deviation
            amount_tolerance_bps: 1,     // 0.01% amount deviation
            
            // Completeness: 95% field coverage minimum
            min_completeness_threshold: 0.95,
            critical_fields: vec![
                "transaction_signature".to_string(),
                "program_id".to_string(),
                "instruction_name".to_string(),
                "token_in".to_string(),
                "token_out".to_string(),
                "amount_in".to_string(),
                "amount_out".to_string(),
                "user".to_string(),
            ],
            
            // Performance: Sub-10ms parse time target
            max_latency_threshold_us: 10_000,      // 10ms
            max_validation_time_us: 50_000,        // 50ms
            
            // String validation
            address_exact_match: true,
            case_sensitive_symbols: false,
            
            // Time tolerance: 1 second for block time variations
            timestamp_tolerance_ms: 1_000,
            
            // Numeric precision: 6 decimal places, 0.01% rounding tolerance
            decimal_precision: 6,
            rounding_tolerance: 0.0001,
            
            // Error rates: 1% error, 5% warning maximums
            max_error_rate: 0.01,
            max_warning_rate: 0.05,
            
            // Confidence: 80% minimums for ground truth and parser
            min_ground_truth_confidence: 0.8,
            min_parser_confidence: 0.8,
        }
    }
}

impl ToleranceConfig {
    /// Create configuration optimized for high-frequency trading
    pub fn high_frequency_trading() -> Self {
        Self {
            min_accuracy_threshold: 0.999,        // 99.9% accuracy required
            price_tolerance_bps: 0,               // Zero price deviation tolerance
            amount_tolerance_bps: 0,              // Zero amount deviation tolerance
            max_latency_threshold_us: 5_000,      // 5ms maximum parse time
            max_validation_time_us: 20_000,       // 20ms maximum validation time
            min_ground_truth_confidence: 0.95,    // 95% ground truth confidence
            min_parser_confidence: 0.95,          // 95% parser confidence
            max_error_rate: 0.001,                // 0.1% error rate maximum
            ..Default::default()
        }
    }
    
    /// Create configuration for development/testing with relaxed tolerances
    pub fn development() -> Self {
        Self {
            min_accuracy_threshold: 0.90,         // 90% accuracy acceptable
            price_tolerance_bps: 10,              // 0.1% price deviation
            amount_tolerance_bps: 10,             // 0.1% amount deviation
            max_latency_threshold_us: 100_000,    // 100ms parse time
            max_validation_time_us: 500_000,      // 500ms validation time
            min_ground_truth_confidence: 0.5,     // 50% ground truth confidence
            min_parser_confidence: 0.5,           // 50% parser confidence
            max_error_rate: 0.1,                  // 10% error rate acceptable
            max_warning_rate: 0.2,                // 20% warning rate acceptable
            ..Default::default()
        }
    }
    
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.min_accuracy_threshold < 0.0 || self.min_accuracy_threshold > 1.0 {
            return Err("min_accuracy_threshold must be between 0.0 and 1.0".to_string());
        }
        
        if self.min_completeness_threshold < 0.0 || self.min_completeness_threshold > 1.0 {
            return Err("min_completeness_threshold must be between 0.0 and 1.0".to_string());
        }
        
        if self.max_latency_threshold_us == 0 {
            return Err("max_latency_threshold_us must be greater than 0".to_string());
        }
        
        if self.max_validation_time_us == 0 {
            return Err("max_validation_time_us must be greater than 0".to_string());
        }
        
        if self.min_ground_truth_confidence < 0.0 || self.min_ground_truth_confidence > 1.0 {
            return Err("min_ground_truth_confidence must be between 0.0 and 1.0".to_string());
        }
        
        if self.min_parser_confidence < 0.0 || self.min_parser_confidence > 1.0 {
            return Err("min_parser_confidence must be between 0.0 and 1.0".to_string());
        }
        
        if self.max_error_rate < 0.0 || self.max_error_rate > 1.0 {
            return Err("max_error_rate must be between 0.0 and 1.0".to_string());
        }
        
        if self.max_warning_rate < 0.0 || self.max_warning_rate > 1.0 {
            return Err("max_warning_rate must be between 0.0 and 1.0".to_string());
        }
        
        Ok(())
    }
    
    /// Check if a price deviation is within tolerance
    pub fn is_price_within_tolerance(&self, actual: f64, expected: f64) -> bool {
        if expected == 0.0 {
            return actual == 0.0;
        }
        
        let deviation_bps = ((actual - expected).abs() / expected) * 10_000.0;
        deviation_bps <= self.price_tolerance_bps as f64
    }
    
    /// Check if an amount deviation is within tolerance
    pub fn is_amount_within_tolerance(&self, actual: u64, expected: u64) -> bool {
        if expected == 0 {
            return actual == 0;
        }
        
        let deviation_bps = ((actual as f64 - expected as f64).abs() / expected as f64) * 10_000.0;
        deviation_bps <= self.amount_tolerance_bps as f64
    }
    
    /// Check if a timestamp is within tolerance
    pub fn is_timestamp_within_tolerance(
        &self,
        actual: chrono::DateTime<chrono::Utc>,
        expected: chrono::DateTime<chrono::Utc>,
    ) -> bool {
        let diff_ms = (actual - expected).num_milliseconds().abs() as u64;
        diff_ms <= self.timestamp_tolerance_ms
    }
    
    /// Calculate price deviation in basis points
    pub fn calculate_price_deviation_bps(&self, actual: f64, expected: f64) -> u16 {
        if expected == 0.0 {
            return if actual == 0.0 { 0 } else { u16::MAX };
        }
        
        let deviation_bps = ((actual - expected).abs() / expected) * 10_000.0;
        deviation_bps.round() as u16
    }
    
    /// Calculate amount deviation in basis points
    pub fn calculate_amount_deviation_bps(&self, actual: u64, expected: u64) -> u16 {
        if expected == 0 {
            return if actual == 0 { 0 } else { u16::MAX };
        }
        
        let deviation_bps = ((actual as f64 - expected as f64).abs() / expected as f64) * 10_000.0;
        deviation_bps.round() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_default_config() {
        let config = ToleranceConfig::default();
        assert_eq!(config.min_accuracy_threshold, 0.99);
        assert_eq!(config.price_tolerance_bps, 1);
        assert_eq!(config.amount_tolerance_bps, 1);
        assert_eq!(config.max_latency_threshold_us, 10_000);
    }
    
    #[test]
    fn test_high_frequency_config() {
        let config = ToleranceConfig::high_frequency_trading();
        assert_eq!(config.min_accuracy_threshold, 0.999);
        assert_eq!(config.price_tolerance_bps, 0);
        assert_eq!(config.amount_tolerance_bps, 0);
        assert_eq!(config.max_latency_threshold_us, 5_000);
    }
    
    #[test]
    fn test_development_config() {
        let config = ToleranceConfig::development();
        assert_eq!(config.min_accuracy_threshold, 0.90);
        assert_eq!(config.price_tolerance_bps, 10);
        assert_eq!(config.max_latency_threshold_us, 100_000);
    }
    
    #[test]
    fn test_price_tolerance() {
        let config = ToleranceConfig::default();
        
        // Within tolerance (1 basis point = 0.01%)
        assert!(config.is_price_within_tolerance(100.0, 100.01));
        assert!(config.is_price_within_tolerance(100.0, 99.99));
        
        // Outside tolerance
        assert!(!config.is_price_within_tolerance(100.0, 100.02));
        assert!(!config.is_price_within_tolerance(100.0, 99.98));
    }
    
    #[test]
    fn test_amount_tolerance() {
        let config = ToleranceConfig::default();
        
        // Within tolerance (1 basis point = 0.01%)
        assert!(config.is_amount_within_tolerance(1000000, 1000010));
        assert!(config.is_amount_within_tolerance(1000000, 999990));
        
        // Outside tolerance
        assert!(!config.is_amount_within_tolerance(1000000, 1000020));
        assert!(!config.is_amount_within_tolerance(1000000, 999980));
    }
    
    #[test]
    fn test_timestamp_tolerance() {
        let config = ToleranceConfig::default();
        let base_time = Utc::now();
        
        // Within tolerance (1 second)
        let within_tolerance = base_time + chrono::Duration::milliseconds(500);
        assert!(config.is_timestamp_within_tolerance(within_tolerance, base_time));
        
        // Outside tolerance
        let outside_tolerance = base_time + chrono::Duration::milliseconds(1500);
        assert!(!config.is_timestamp_within_tolerance(outside_tolerance, base_time));
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = ToleranceConfig::default();
        assert!(config.validate().is_ok());
        
        // Invalid accuracy threshold
        config.min_accuracy_threshold = 1.5;
        assert!(config.validate().is_err());
        
        config.min_accuracy_threshold = -0.1;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_deviation_calculations() {
        let config = ToleranceConfig::default();
        
        // Price deviation
        assert_eq!(config.calculate_price_deviation_bps(100.0, 100.01), 1);
        assert_eq!(config.calculate_price_deviation_bps(100.0, 101.0), 100);
        
        // Amount deviation
        assert_eq!(config.calculate_amount_deviation_bps(1000000, 1000100), 10);
        assert_eq!(config.calculate_amount_deviation_bps(1000000, 1010000), 100);
    }
}
