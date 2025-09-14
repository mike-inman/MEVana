use std::env;
use tokio::time::{sleep, Duration};
use log::{info, warn};
use hotpath_arb_engine::synthetic_data_mode::SyntheticDataGenerator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .format_timestamp_micros()
        .init();

    info!("🎭 MEVana Synthetic Data Mode Starting");
    info!("📊 This mode generates realistic arbitrage data for dashboard testing");
    info!("🚧 Use when ISP throttles real Geyser streams");

    // Check environment variables
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://mev:mevpassword@localhost:5433/mevdb".to_string());
    let metrics_port = env::var("METRICS_PORT")
        .unwrap_or_else(|_| "8088".to_string())
        .parse::<u16>()
        .unwrap_or(8088);

    info!("📊 Metrics will be available at: http://localhost:{}/metrics", metrics_port);
    info!("💾 Database: {}", database_url);

    // Start HTTP server for metrics (minimal)
    let metrics_server = tokio::spawn(async move {
        use warp::Filter;
        
        let metrics = warp::path("metrics")
            .map(|| {
                // Basic metrics for Grafana testing
                format!(
                    "# HELP synthetic_mode Synthetic data generation mode\n\
                     # TYPE synthetic_mode gauge\n\
                     synthetic_mode 1\n\
                     # HELP arbitrage_opportunities_detected Total arbitrage opportunities detected\n\
                     # TYPE arbitrage_opportunities_detected counter\n\
                     arbitrage_opportunities_detected {}\n",
                    hotpath_arb_engine::orderbook_tracker::ARBITRAGE_DETECTED.len()
                )
            });

        let health = warp::path("health")
            .map(|| "OK");

        let routes = metrics.or(health);
        
        info!("🌐 Starting metrics server on port {}", metrics_port);
        warp::serve(routes)
            .run(([127, 0, 0, 1], metrics_port))
            .await;
    });

    // Start synthetic data generator
    let data_generator = tokio::spawn(async move {
        let mut generator = SyntheticDataGenerator::new();
        generator.run_synthetic_stream().await;
    });

    // Run both tasks concurrently
    tokio::select! {
        _ = metrics_server => warn!("Metrics server ended"),
        _ = data_generator => warn!("Data generator ended"),
    }

    Ok(())
}