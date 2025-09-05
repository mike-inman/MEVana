pub mod config;
pub mod engine_fast;
pub mod ai_hooks;
pub mod circuit;
pub mod edge_integration;
pub mod hotpath;
pub mod metrics_http;
pub mod models;
pub mod quoting;
pub mod sizing;
pub mod stats;

// Conditionally compile problematic modules based on features
#[cfg(feature = "whirlpool")]
pub mod whirlpool_adapter;

#[cfg(any(feature = "production", not(feature = "stubs")))]
pub mod executor;

// Always include plugin interface for extensibility
pub mod plugin;

// Compile-time feature validation
#[cfg(all(feature = "stubs", feature = "production"))]
compile_error!("Cannot enable both 'stubs' and 'production' features - choose one or the other");

// Development mode warnings
#[cfg(all(feature = "stubs", debug_assertions))]
const _: () = {
    const WARNING: &str = "
🟡 DEVELOPMENT MODE ACTIVE 🟡
==============================
Building with 'stubs' feature in debug mode.
- Transaction execution will be stubbed
- Some DEX integrations may be incomplete
- Metrics and monitoring are functional
- This is ONLY for development/testing
==============================
";
};

// Feature detection at runtime
pub fn get_build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        git_sha: option_env!("GIT_SHA").unwrap_or("unknown"),
        features: get_active_features(),
        build_timestamp: env!("BUILD_TIMESTAMP"),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildInfo {
    pub version: &'static str,
    pub git_sha: &'static str,
    pub features: Vec<&'static str>,
    pub build_timestamp: &'static str,
}

fn get_active_features() -> Vec<&'static str> {
    let mut features = Vec::new();
    
    #[cfg(feature = "stubs")]
    features.push("stubs");
    
    #[cfg(feature = "production")]
    features.push("production");
    
    #[cfg(feature = "whirlpool")]
    features.push("whirlpool");
    
    #[cfg(feature = "raydium")]
    features.push("raydium");
    
    #[cfg(feature = "meteora")]
    features.push("meteora");
    
    #[cfg(feature = "diagnostics")]
    features.push("diagnostics");
    
    #[cfg(feature = "profiling")]
    features.push("profiling");
    
    features
}

// Core re-exports (always available)
pub use config::{get_knobs, set_knobs, Knobs};
pub use engine_fast::FastEngine;
pub use hotpath::SignalQueue;
pub use models::{ArbSignal, Pair, Quote};

// Conditional re-exports based on features
#[cfg(feature = "whirlpool")]
pub use whirlpool_adapter::WhirlpoolAdapter;

#[cfg(any(feature = "production", not(feature = "stubs")))]
pub use executor::{
    build_leg_buy, build_leg_sell, build_payer_tx, BundleSubmitter, 
    Executor, FastExecutor, FlashLoanProvider,
};

// Test helpers (only available in test builds)
#[cfg(test)]
pub use edge_integration::EdgeArbitrageDetector;

// Development utilities
#[cfg(feature = "stubs")]
pub mod dev_utils {
    use super::*;
    
    pub fn create_stub_engine() -> FastEngine {
        let signal_queue = SignalQueue::new(100);
        FastEngine::new(signal_queue.sender())
    }
    
    pub fn log_feature_status() {
        let build_info = get_build_info();
        println!("🚀 Arb Engine Build Info:");
        println!("   Version: {}", build_info.version);
        println!("   Git SHA: {}", build_info.git_sha);
        println!("   Features: {:?}", build_info.features);
        println!("   Built: {}", build_info.build_timestamp);
    }
}

// Health check utilities (always available)
pub mod health {
    use crate::stats::compute_readiness;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    #[derive(Debug, serde::Serialize)]
    pub struct HealthStatus {
        pub service: &'static str,
        pub ready: bool,
        pub timestamp: u64,
        pub last_edge_ns: u64,
        pub staleness_ms: Option<u64>,
        pub error_ratio: f64,
        pub circuit_open: bool,
        pub build_info: super::BuildInfo,
    }
    
    pub fn get_health_status() -> HealthStatus {
        let (ready, err_ratio, staleness_ms, _total, _errs, circuit, _half_open, _max_stale, _err_thresh, last) = compute_readiness();
        
        let staleness = if staleness_ms == u64::MAX { 
            None 
        } else { 
            Some(staleness_ms) 
        };
        
        HealthStatus {
            service: "arb-engine",
            ready,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_edge_ns: last,
            staleness_ms: staleness,
            error_ratio: err_ratio,
            circuit_open: circuit,
            build_info: super::get_build_info(),
        }
    }
}

// Error handling utilities
pub mod errors {
    use thiserror::Error;
    
    #[derive(Error, Debug)]
    pub enum ArbEngineError {
        #[error("Edge processing failed: {0}")]
        EdgeProcessing(#[from] anyhow::Error),
        
        #[error("Signal queue full")]
        QueueFull,
        
        #[error("Circuit breaker open")]
        CircuitOpen,
        
        #[error("Feature not enabled: {feature}")]
        FeatureDisabled { feature: String },
        
        #[error("Configuration error: {0}")]
        Config(String),
        
        #[error("Execution error: {0}")]
        Execution(String),
    }
    
    pub type Result<T> = std::result::Result<T, ArbEngineError>;
}

// Integration test helpers
#[cfg(test)]
pub mod test_utils {
    use super::*;
    use edge_assembler::EdgeDelta;
    
    pub fn create_test_edge() -> EdgeDelta {
        EdgeDelta {
            edge_id: 12345,
            venue_id: 1,
            pool_pubkey: vec![1; 32],
            token_in: vec![1; 32],
            token_out: vec![2; 32],
            direction: 0,
            decimals_in: 9,
            decimals_out: 6,
            fee_bps_total: 30,
            slot: 123456,
            write_version: 1,
            updated_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            marginal_price_q64: (1u128 << 64).to_le_bytes().to_vec(), // 1.0 in Q64
            size_to_next_boundary_in: (1000u128).to_le_bytes().to_vec(),
            cu_estimate: 50000,
            state: None,
        }
    }
    
    pub async fn run_edge_processing_test() -> anyhow::Result<()> {
        let signal_queue = SignalQueue::new(100);
        let engine = FastEngine::new(signal_queue.sender());
        
        let test_edge = create_test_edge();
        engine.process_edge_update(test_edge)?;
        
        // Give processing time
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        println!("✅ Edge processing test completed");
        Ok(())
    }
}