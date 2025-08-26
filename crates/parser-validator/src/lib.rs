//! Parser Validation Framework
//!
//! Comprehensive testing system for validating parser accuracy, completeness,
//! reliability, latency, and DEX coverage against live data sources.

pub mod database;
pub mod ground_truth;
pub mod validation;
pub mod streaming;
pub mod pipeline;
// TODO: Implement remaining modules
// pub mod dashboard;
// pub mod reporting;
// pub mod metrics;

pub use ground_truth::GroundTruthAggregator;
pub use validation::ValidationFramework;
pub use pipeline::{ValidationPipeline, ValidationPipelineConfig};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the validation framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Shyft API configuration
    pub shyft_api_key: String,
    pub shyft_rpc_url: String,

    /// Helius API configuration  
    pub helius_api_key: String,
    pub helius_rpc_url: String,

    /// Solscan API configuration
    pub solscan_api_key: String,

    /// Database configuration
    pub database_url: String,

    /// Validation parameters
    pub validation_duration: Duration,
    pub accuracy_threshold: f64,
    pub completeness_threshold: f64,
    pub latency_threshold_ms: u64,

    /// Target DEXs to validate
    pub target_dexes: Vec<String>,

    /// Load testing configuration
    pub load_test_levels: Vec<u32>, // TPS levels

    /// Dashboard configuration
    pub dashboard_host: String,
    pub dashboard_port: u16,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            shyft_api_key: std::env::var("SHYFT_API_KEY").unwrap_or_default(),
            shyft_rpc_url: "https://rpc.shyft.to".to_string(),
            helius_api_key: std::env::var("HELIUS_API_KEY").unwrap_or_default(),
            helius_rpc_url: "https://rpc.helius.xyz".to_string(),
            solscan_api_key: std::env::var("SOLSCAN_API_KEY").unwrap_or_default(),
            database_url: "postgresql://postgres:password@localhost:5432/mevana_validation"
                .to_string(),
            validation_duration: Duration::from_secs(3600), // 1 hour
            accuracy_threshold: 0.99,
            completeness_threshold: 0.95,
            latency_threshold_ms: 10,
            target_dexes: vec![
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(), // Orca Whirlpools
                "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY".to_string(), // Phoenix
                "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string(), // Pump.fun
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(), // Raydium AMM
                "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1".to_string(), // Raydium CLMM
                "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG".to_string(), // Meteora
                "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb".to_string(), // OpenBook
                "GFXsSL5sSaDfNFQUYsHekbWBW1TsFdjDYzACh62tEHxn".to_string(), // GooseFX
            ],
            load_test_levels: vec![10, 50, 100, 500, 1000],
            dashboard_host: "0.0.0.0".to_string(),
            dashboard_port: 8085, // Using next available port after dex-stream-universal (8084)
        }
    }
}

/// Main entry point for the validation framework
pub async fn run_validation_framework(config: ValidationConfig) -> Result<()> {
    tracing::info!("🧪 Starting Parser Validation Framework");

    // Initialize database
    let db = database::ValidationDatabase::new(&config.database_url).await?;
    db.run_migrations().await?;
    tracing::info!("✅ Database initialized");

    // Set up ground truth sources
    let ground_truth = GroundTruthAggregator::new(
        config.shyft_api_key.clone(),
        config.helius_api_key.clone(),
        config.solscan_api_key.clone(),
    );
    tracing::info!("✅ Ground truth sources configured");

    // Initialize validation framework
    let framework = ValidationFramework::new(ground_truth, db.clone());
    tracing::info!("✅ Validation framework initialized");

    // Start validation pipeline
    let pipeline_config = ValidationPipelineConfig::default();
    let pipeline = ValidationPipeline::new(framework, db, pipeline_config);
    tracing::info!("🚀 Starting validation pipeline");
    
    pipeline.start().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ValidationConfig::default();
        assert_eq!(config.accuracy_threshold, 0.99);
        assert_eq!(config.completeness_threshold, 0.95);
        assert_eq!(config.latency_threshold_ms, 10);
        assert_eq!(config.dashboard_port, 8085);
    }
}
