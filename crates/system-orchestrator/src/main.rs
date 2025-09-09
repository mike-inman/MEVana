use anyhow::Result;
use log::{error, info, warn};
use std::time::Duration;
use tokio::signal;

mod config;
mod health_checker;
mod orchestrator;
mod service_manager;
mod startup_sequence;

use crate::config::OrchestratorConfig;
use crate::orchestrator::SystemOrchestrator;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    info!("🚀 MEVana System Orchestrator starting");
    info!("📋 Loading configuration and service definitions");
    
    // Load configuration
    let config = OrchestratorConfig::load().await?;
    info!("✅ Configuration loaded: {} services defined", config.services.len());
    
    // Create orchestrator
    let mut orchestrator = SystemOrchestrator::new(config).await?;
    
    // Setup graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c().await.expect("Failed to install CTRL+C signal handler");
        warn!("🛑 Shutdown signal received, initiating graceful shutdown...");
    };
    
    // Start system orchestration
    tokio::select! {
        result = orchestrator.start_system() => {
            match result {
                Ok(_) => info!("✅ System orchestration completed successfully"),
                Err(e) => {
                    error!("❌ System orchestration failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ = shutdown_signal => {
            info!("🔄 Initiating graceful system shutdown...");
            if let Err(e) = orchestrator.shutdown_system().await {
                error!("❌ Error during system shutdown: {}", e);
                std::process::exit(1);
            }
            info!("✅ System shutdown completed successfully");
        }
    }
    
    Ok(())
}