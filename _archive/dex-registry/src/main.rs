use anyhow::Result;
use log::info;
use std::sync::Arc;
use tokio::signal;

mod api;
mod cache;
mod config;
mod database;
mod discovery;
mod plugin;
mod registry;

use config::RegistryConfig;
use registry::DexRegistry;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🚀 Starting DEX Registry Service");

    // Load configuration
    let config = RegistryConfig::load()?;
    info!("Configuration loaded successfully");

    // Initialize registry
    let registry = DexRegistry::new(config).await?;
    info!("DEX Registry initialized");

    // Start background services
    let registry_handle = tokio::spawn(async move { registry.start().await });

    // Wait for shutdown signal
    info!("DEX Registry service running. Press Ctrl+C to stop.");
    signal::ctrl_c().await?;
    info!("Shutdown signal received");

    // Graceful shutdown
    registry_handle.abort();
    info!("DEX Registry service stopped");

    Ok(())
}
