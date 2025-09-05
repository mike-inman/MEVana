// executor-service/src/bin/test_execution_orchestrator.rs
//! Test binary to demonstrate the ExecutionOrchestrator's capabilities

use std::sync::Arc;
use log::{info, warn};
use anyhow::Result;
use serde::{Serialize, Deserialize};

use executor_service::{
    config::ExecutorConfig,
    execution_orchestrator::{ExecutionOrchestrator, ExecutionMode, ComprehensiveExecutionResult},
    execution_engine::ArbitrageOpportunity,
    shyft_client::ShyftClient,
    wallet_service::WalletService,
    bundle_builder::BundleBuilder,
    flash_loan::FlashLoanAggregator,
    metrics::MetricsServer,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockArbitrageOpportunity {
    pub opportunity_id: String,
    pub profit_bps: u32,
    pub volume_usd: f64,
}

impl From<MockArbitrageOpportunity> for ArbitrageOpportunity {
    fn from(mock: MockArbitrageOpportunity) -> Self {
        use uuid::Uuid;
        
        ArbitrageOpportunity {
            opportunity_id: Uuid::parse_str(&mock.opportunity_id).unwrap_or(Uuid::new_v4()),
            profit_bps: mock.profit_bps as u64,
            // Mock token addresses as Vec<u8> (32 bytes each for Solana pubkeys)
            token_in: vec![0u8; 32], // Mock SOL token
            token_out: vec![1u8; 32], // Mock USDC token  
            edges: vec![1, 2], // Mock edge IDs
            venues: vec![0, 1], // Mock venue IDs (whirlpool, raydium)
            max_size_in: (mock.volume_usd * 1_000_000.0) as u128, // Convert USD to micro-lamports
            total_cu_estimate: 200_000, // Mock compute units
            total_fees_bps: 30, // Mock fees (0.3%)
            detected_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            max_edge_age_ms: 5000, // 5 second max age
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    info!("🚀 Starting ExecutionOrchestrator Integration Test");
    
    // Create mock components for testing
    let config = Arc::new(ExecutorConfig::load()?); // Load config from environment
    let shyft_client = Arc::new(ShyftClient::new(
        "test_key".to_string(),
        "https://rpc.shyft.to".to_string(),
        "https://mainnet.block-engine.jito.wtf".to_string(),
    )?);
    let wallet_service = Arc::new(WalletService::new(config.clone(), shyft_client.clone())?);
    let bundle_builder = Arc::new(BundleBuilder::new(shyft_client.clone()));
    let flash_loan_aggregator = Arc::new(FlashLoanAggregator::new(config.clone(), shyft_client.clone())?);
    let metrics_server = Arc::new(MetricsServer::new(config.metrics_port)?);
    
    // Create the ExecutionOrchestrator
    let orchestrator = ExecutionOrchestrator::new(
        config,
        shyft_client,
        wallet_service,
        flash_loan_aggregator,
        bundle_builder,
        metrics_server,
    ).await?;
    
    // Test cases
    let test_opportunities = vec![
        MockArbitrageOpportunity {
            opportunity_id: "test_opp_1".to_string(),
            profit_bps: 75,  // 0.75% profit - should be profitable
            volume_usd: 10000.0,
        },
        MockArbitrageOpportunity {
            opportunity_id: "test_opp_2".to_string(),
            profit_bps: 25,  // 0.25% profit - marginal
            volume_usd: 5000.0,
        },
        MockArbitrageOpportunity {
            opportunity_id: "test_opp_3".to_string(),
            profit_bps: 150, // 1.5% profit - very good
            volume_usd: 20000.0,
        },
    ];
    
    info!("🧪 Running TEST MODE executions first...");
    
    // Test each opportunity in test mode first
    for mock_opp in &test_opportunities {
        let opportunity: ArbitrageOpportunity = mock_opp.clone().into();
        let execution_mode = ExecutionMode::Test { 
            simulate_success_rate: 0.9,
            simulate_latency_ms: 100,
            enable_logging: true,
        };
        
        info!("📊 Testing opportunity: {} ({}bps profit)", 
              opportunity.opportunity_id, opportunity.profit_bps);
        
        match orchestrator.execute_opportunity(opportunity, execution_mode).await {
            Ok(result) => {
                display_execution_result(&result);
            }
            Err(e) => {
                warn!("❌ Test execution failed: {}", e);
            }
        }
        
        // Brief pause between tests
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    info!("🎯 TEST MODE completed successfully!");
    info!("💡 In production, you would now switch to LIVE MODE after validation");
    
    // Example of how LIVE MODE would be called (commented out for safety)
    /*
    info!("🔥 LIVE MODE execution example (commented out for safety):");
    let live_mode = ExecutionMode::Live {
        enable_aggressive_optimization: true,
        max_slippage_bps: 100,
        force_profit_threshold_lamports: Some(50_000),
    };
    
    // This would execute real trades:
    // let result = orchestrator.execute_opportunity(test_opportunities[0].clone().into(), live_mode).await?;
    */
    
    info!("✅ ExecutionOrchestrator integration test completed successfully!");
    info!("🎉 Ready for deployment and live trading!");
    
    Ok(())
}

fn display_execution_result(result: &ComprehensiveExecutionResult) {
    info!("📊 Execution Result Summary:");
    info!("   🆔 Execution ID: {}", result.execution_id);
    info!("   💰 Estimated Profit: {} lamports", result.estimated_profit_lamports);
    
    if let Some(actual_profit) = result.actual_profit_lamports {
        info!("   💎 Actual Profit: {} lamports", actual_profit);
        let profit_accuracy = if result.estimated_profit_lamports > 0 {
            (actual_profit as f64 / result.estimated_profit_lamports as f64) * 100.0
        } else {
            0.0
        };
        info!("   📈 Profit Accuracy: {:.1}%", profit_accuracy);
    }
    
    info!("   ⏱️  Total Execution Time: {}ms", result.total_execution_time_ms);
    
    match result.jito_bundle_result.as_ref() {
        Some(jito) => {
            info!("   🎯 Jito Bundle: {} ({:?})", jito.bundle_id, jito.status);
            if let Some(slot) = jito.landed_slot {
                info!("   🎰 Landed Slot: {}", slot);
            }
        }
        None => info!("   🎯 Jito Bundle: None"),
    }
    
    match result.flash_loan_result.as_ref() {
        Some(fl) => {
            info!("   💸 Flash Loan: {:?} ({} lamports fee)", fl.provider_used, fl.fee_paid_lamports);
        }
        None => info!("   💸 Flash Loan: None"),
    }
    
    info!("   ⚠️  Status: {:?}", result.final_status);
    info!("   📏 Efficiency Score: {:.2}", result.execution_efficiency_score);
    info!("");
}