use anyhow::Result;
use log::{error, info, warn};
use std::sync::Arc;
use tokio::signal;

mod bundle_builder;
mod config;
mod execution_engine;
mod flash_loan;
mod metrics;
mod opportunity_processor;
mod shyft_client;
mod wallet_service;

use config::ExecutorConfig;
use execution_engine::ExecutionEngine;
use metrics::MetricsServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🚀 Starting MEVana Executor Service");

    // Load configuration
    let config = ExecutorConfig::load()?;
    info!("Configuration loaded successfully");

    // Initialize metrics server
    let metrics_server = MetricsServer::new(config.metrics_port)?;
    let _metrics_handle = tokio::spawn(async move { metrics_server.start().await });

    // Initialize execution engine
    let execution_engine = ExecutionEngine::new(config).await?;
    info!("Execution engine initialized");

    // Start processing opportunities
    let engine_handle = tokio::spawn(async move { execution_engine.start().await });

    // Wait for shutdown signal
    info!("Executor service running. Press Ctrl+C to stop.");
    signal::ctrl_c().await?;
    info!("Shutdown signal received");

    // Graceful shutdown
    engine_handle.abort();
    info!("Executor service stopped");

    Ok(())
}
