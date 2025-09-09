use anyhow::Result;
use std::sync::Arc;

use crate::live_trading_config::LiveTradingConfig;
use crate::flash_loan_integration::{HotpathFlashLoanManager, FlashLoanProvider, FlashLoanQuote};

// Import the full system when available
// use executor_service::flash_loan::{FlashLoanAggregator, FlashLoanProvider as FullFlashLoanProvider};
// use executor_service::flash_loan_optimizer::FlashLoanOptimizer;

/// Factory for creating the appropriate flash loan system
pub struct FlashLoanFactory;

pub enum FlashLoanSystem {
    Streamlined(Arc<HotpathFlashLoanManager>),
    // Full(Arc<FlashLoanAggregator>, Arc<FlashLoanOptimizer>),
}

impl FlashLoanFactory {
    /// Create the appropriate flash loan system based on configuration
    pub fn create_flash_loan_system(config: &LiveTradingConfig) -> Result<FlashLoanSystem> {
        match config.flash_loan_mode.as_str() {
            "streamlined" => {
                info!("🚀 Initializing STREAMLINED flash loan system");
                info!("   - Ultra-fast in-memory simulation");
                info!("   - Zero network calls for quotes");  
                info!("   - Perfect for development and testing");
                
                let manager = Arc::new(HotpathFlashLoanManager::new(config.clone()));
                Ok(FlashLoanSystem::Streamlined(manager))
            }
            "full" => {
                // For now, fall back to streamlined with a warning
                // In production, this would create the full system:
                warn!("🏗️  FULL flash loan system requested but not yet implemented");
                warn!("   - Would connect to real lending protocols");
                warn!("   - Would make live RPC calls for quotes");
                warn!("   - Would optimize across multiple providers");
                warn!("   - Falling back to streamlined mode...");
                
                let manager = Arc::new(HotpathFlashLoanManager::new(config.clone()));
                Ok(FlashLoanSystem::Streamlined(manager))
                
                // TODO: When ready for production, replace with:
                // info!("🏭 Initializing FULL flash loan system");
                // let aggregator = Arc::new(FlashLoanAggregator::new(executor_config, shyft_client)?);
                // let optimizer = Arc::new(FlashLoanOptimizer::new(executor_config, shyft_client)?);
                // Ok(FlashLoanSystem::Full(aggregator, optimizer))
            }
            mode => {
                warn!("❌ Unknown flash loan mode: '{}', defaulting to streamlined", mode);
                let manager = Arc::new(HotpathFlashLoanManager::new(config.clone()));
                Ok(FlashLoanSystem::Streamlined(manager))
            }
        }
    }
}

impl FlashLoanSystem {
    /// Get best flash loan provider (unified interface)
    pub async fn get_best_provider(
        &self,
        token_mint: &solana_sdk::pubkey::Pubkey,
        amount_sol: f64,
    ) -> Result<Option<FlashLoanQuote>> {
        match self {
            FlashLoanSystem::Streamlined(manager) => {
                manager.get_best_provider(token_mint, amount_sol).await
            }
            // FlashLoanSystem::Full(aggregator, optimizer) => {
            //     // Would call the full system's optimized provider selection
            //     todo!("Implement full system provider selection")
            // }
        }
    }
    
    /// Record flash loan execution result
    pub async fn record_flash_loan_result(&self, provider: FlashLoanProvider, success: bool) {
        match self {
            FlashLoanSystem::Streamlined(manager) => {
                manager.record_flash_loan_result(provider, success).await
            }
            // FlashLoanSystem::Full(aggregator, _) => {
            //     // Would update full system performance tracking
            //     todo!("Implement full system performance tracking")
            // }
        }
    }
    
    /// Get flash loan statistics
    pub async fn get_stats(&self) -> String {
        match self {
            FlashLoanSystem::Streamlined(manager) => {
                manager.get_stats().await
            }
            // FlashLoanSystem::Full(aggregator, optimizer) => {
            //     // Would return comprehensive full system stats
            //     todo!("Implement full system stats")
            // }
        }
    }
}

// Re-export for convenience
pub use crate::flash_loan_integration::{FlashLoanProvider, FlashLoanQuote, FlashLoanBundle};

use log::{info, warn};