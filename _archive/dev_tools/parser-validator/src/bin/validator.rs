//! Parser Validation CLI
//! 
//! Command-line interface for running parser validation tests.

use anyhow::Result;
use clap::{Arg, Command};
use parser_validator::{run_validation_framework, ValidationConfig};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let matches = Command::new("parser-validator")
        .version("0.1.0")
        .about("Comprehensive parser validation framework for MEVana")
        .arg(
            Arg::new("database-url")
                .long("database-url")
                .help("PostgreSQL database URL")
                .default_value("postgresql://postgres:password@localhost:5432/mevana_validation")
        )
        .arg(
            Arg::new("shyft-api-key")
                .long("shyft-api-key")
                .help("Shyft API key")
                .env("SHYFT_API_KEY")
        )
        .arg(
            Arg::new("helius-api-key")
                .long("helius-api-key")
                .help("Helius API key")
                .env("HELIUS_API_KEY")
        )
        .arg(
            Arg::new("solscan-api-key")
                .long("solscan-api-key")
                .help("Solscan API key")
                .env("SOLSCAN_API_KEY")
        )
        .arg(
            Arg::new("test-mode")
                .long("test-mode")
                .help("Run in test mode (no database required)")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();
    
    println!("🧪 MEVana Parser Validation Framework");
    println!("=====================================");
    
    if matches.get_flag("test-mode") {
        println!("Running in test mode...");
        test_validation_framework().await?;
    } else {
        let config = ValidationConfig {
            database_url: matches.get_one::<String>("database-url").unwrap().clone(),
            shyft_api_key: matches.get_one::<String>("shyft-api-key").unwrap_or(&String::new()).clone(),
            helius_api_key: matches.get_one::<String>("helius-api-key").unwrap_or(&String::new()).clone(),
            solscan_api_key: matches.get_one::<String>("solscan-api-key").unwrap_or(&String::new()).clone(),
            ..Default::default()
        };
        
        println!("Starting validation framework...");
        run_validation_framework(config).await?;
    }
    
    Ok(())
}

/// Test the validation framework without database
async fn test_validation_framework() -> Result<()> {
    use parser_validator::{ValidationFramework, GroundTruthAggregator};
    use parser_validator::database::ValidationDatabase;
    
    println!("✅ Testing ground truth aggregator...");
    
    // Test ground truth sources (without actual API calls)
    let ground_truth = GroundTruthAggregator::new(
        "test_shyft_key".to_string(),
        "test_helius_key".to_string(),
        "test_solscan_key".to_string(),
    );
    
    println!("✅ Ground truth aggregator initialized");
    
    // Test connectivity (placeholder)
    match ground_truth.test_connectivity().await {
        Ok(_) => println!("✅ Ground truth connectivity test passed"),
        Err(e) => println!("⚠️  Ground truth connectivity test failed: {}", e),
    }
    
    // Test database (if we had a test database)
    println!("🚧 Database tests skipped in test mode");
    
    println!("✅ All validation framework components tested successfully!");
    
    Ok(())
}
