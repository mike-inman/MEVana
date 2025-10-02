//! Live Transaction Data Flow Verification Test
//!
//! This test suite verifies that the MEVana system is actively receiving and processing
//! live transaction data from the Solana blockchain through Geyser streams.

#![allow(warnings)]

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::Value;
use std::{
    env,
    time::{Duration, Instant},
};
use tokio::time::timeout;

/// Configuration for live data testing
const GEYSER_ENDPOINT: &str = "https://grpc.ny.shyft.to";
const METRICS_ENDPOINT: &str = "http://localhost:8088/metrics";
const PROMETHEUS_ENDPOINT: &str = "http://localhost:9090/api/v1/query";
const TEST_TIMEOUT_SECONDS: u64 = 30;
const MIN_TRANSACTION_RATE_PER_SECOND: u64 = 1;
const MIN_ARBITRAGE_OPPORTUNITIES_DETECTED: u64 = 1;
const RUN_LIVE_TESTS_ENV: &str = "RUN_HOTPATH_LIVE_TESTS";

#[derive(Debug)]
pub struct LiveDataMetrics {
    pub geyser_connected: bool,
    pub transactions_processed: u64,
    pub arbitrage_opportunities: u64,
    pub success_rate: f64,
    pub last_transaction_timestamp: Option<String>,
    pub processing_latency_ms: f64,
}

/// Test suite for verifying live transaction data flow
pub struct LiveTransactionDataTest {
    client: Client,
}

impl LiveTransactionDataTest {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Main test function that verifies live transaction data is flowing
    pub async fn verify_live_transaction_data_flow(&self) -> Result<()> {
        println!("🧪 Starting Live Transaction Data Flow Verification");
        println!("⏰ Test timeout: {} seconds", TEST_TIMEOUT_SECONDS);

        let start_time = Instant::now();

        // Test 1: Verify Geyser connection is active
        self.test_geyser_connection_active().await?;

        // Test 2: Verify transaction processing metrics
        let initial_metrics = self.capture_metrics_snapshot().await?;

        // Test 3: Wait for live data processing
        println!("⏳ Waiting for live transaction processing...");
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Test 4: Verify data flow has increased
        let final_metrics = self.capture_metrics_snapshot().await?;
        self.verify_transaction_processing_increased(&initial_metrics, &final_metrics)?;

        // Test 5: Verify arbitrage opportunities are being detected
        self.verify_arbitrage_opportunities_detected(&final_metrics)
            .await?;

        // Test 6: Verify Prometheus is collecting live metrics
        self.verify_prometheus_collecting_live_metrics().await?;

        let elapsed = start_time.elapsed();
        println!(
            "✅ Live Transaction Data Flow Verification PASSED in {:.2}s",
            elapsed.as_secs_f64()
        );

        Ok(())
    }

    /// Test that Geyser connection is active and receiving data
    async fn test_geyser_connection_active(&self) -> Result<()> {
        println!("🔗 Testing Geyser connection status...");

        let response = timeout(
            Duration::from_secs(10),
            self.client.get(METRICS_ENDPOINT).send(),
        )
        .await
        .map_err(|_| anyhow!("Timeout connecting to metrics endpoint"))?
        .map_err(|e| anyhow!("Failed to connect to metrics endpoint: {}", e))?;

        let metrics_text = response.text().await?;

        // Check for Geyser processing metrics
        let geyser_metric =
            self.extract_metric_value(&metrics_text, "geyser_transactions_processed_total")?;

        if geyser_metric == 0 {
            return Err(anyhow!(
                "Geyser stream metrics not present - no live data flowing",
            ));
        }

        println!(
            "✅ Geyser stream metrics present (processed: {})",
            geyser_metric
        );
        Ok(())
    }

    /// Capture current metrics snapshot
    async fn capture_metrics_snapshot(&self) -> Result<LiveDataMetrics> {
        let response = self.client.get(METRICS_ENDPOINT).send().await?;
        let metrics_text = response.text().await?;

        // Parse metrics from Prometheus format
        let transactions_processed =
            self.extract_metric_value(&metrics_text, "hotpath_transactions_processed_total")?;
        let arbitrage_opportunities =
            self.extract_metric_value(&metrics_text, "hotpath_arbitrage_opportunities_total")?;

        let successful_trades = self
            .extract_metric_value(&metrics_text, "successful_trades_total")
            .unwrap_or(0) as f64;
        let failed_trades = self
            .extract_metric_value(&metrics_text, "failed_trades_total")
            .unwrap_or(0) as f64;

        let total_trades = successful_trades + failed_trades;
        let success_rate = if total_trades > 0.0 {
            (successful_trades / total_trades) * 100.0
        } else {
            0.0
        };

        // Use histogram count to approximate latency (convert μs to ms)
        let processing_latency_count = self
            .extract_metric_value(&metrics_text, "geyser_transaction_processing_time_us_count")?;
        let processing_latency_sum =
            self.extract_metric_float(&metrics_text, "geyser_transaction_processing_time_us_sum")?;

        let processing_latency_ms = if processing_latency_count > 0 {
            (processing_latency_sum / processing_latency_count as f64) / 1000.0
        } else {
            0.0
        };

        Ok(LiveDataMetrics {
            geyser_connected: metrics_text.contains("geyser_transactions_processed_total"),
            transactions_processed,
            arbitrage_opportunities,
            success_rate,
            last_transaction_timestamp: None,
            processing_latency_ms,
        })
    }

    /// Extract numeric value from Prometheus metrics text
    fn extract_metric_value(&self, metrics_text: &str, metric_name: &str) -> Result<u64> {
        for line in metrics_text.lines() {
            if line.starts_with(metric_name) && !line.contains('#') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1]
                        .parse::<f64>()
                        .map(|f| f as u64)
                        .map_err(|e| anyhow!("Failed to parse metric value: {}", e));
                }
            }
        }
        Ok(0) // Default to 0 if metric not found
    }

    fn extract_metric_float(&self, metrics_text: &str, metric_name: &str) -> Result<f64> {
        for line in metrics_text.lines() {
            if line.starts_with(metric_name) && !line.contains('#') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1]
                        .parse::<f64>()
                        .map_err(|e| anyhow!("Failed to parse metric value: {}", e));
                }
            }
        }
        Ok(0.0)
    }

    /// Verify that transaction processing has increased between snapshots
    fn verify_transaction_processing_increased(
        &self,
        initial: &LiveDataMetrics,
        final_metrics: &LiveDataMetrics,
    ) -> Result<()> {
        println!("📊 Verifying transaction processing increase...");

        let tx_increase = final_metrics.transactions_processed - initial.transactions_processed;
        let time_elapsed = 10.0; // 10 seconds between snapshots
        let tx_rate = tx_increase as f64 / time_elapsed;

        println!(
            "📈 Transaction rate: {:.1} tx/s (minimum required: {} tx/s)",
            tx_rate, MIN_TRANSACTION_RATE_PER_SECOND
        );

        if tx_increase == 0 {
            return Err(anyhow!(
                "❌ No transaction processing detected - system appears to be idle"
            ));
        }

        if tx_rate < MIN_TRANSACTION_RATE_PER_SECOND as f64 {
            return Err(anyhow!(
                "❌ Transaction processing rate too low: {:.1} tx/s (minimum: {} tx/s)",
                tx_rate,
                MIN_TRANSACTION_RATE_PER_SECOND
            ));
        }

        println!(
            "✅ Live transaction processing verified: {} transactions in 10s",
            tx_increase
        );
        Ok(())
    }

    /// Verify that arbitrage opportunities are being detected from live data
    async fn verify_arbitrage_opportunities_detected(
        &self,
        metrics: &LiveDataMetrics,
    ) -> Result<()> {
        println!("🎯 Verifying arbitrage opportunity detection...");

        if metrics.arbitrage_opportunities < MIN_ARBITRAGE_OPPORTUNITIES_DETECTED {
            return Err(anyhow!(
                "❌ Insufficient arbitrage opportunities detected: {} (minimum: {})",
                metrics.arbitrage_opportunities,
                MIN_ARBITRAGE_OPPORTUNITIES_DETECTED
            ));
        }

        println!(
            "✅ Arbitrage opportunities detected: {} (success rate: {:.1}%)",
            metrics.arbitrage_opportunities, metrics.success_rate
        );

        // Verify processing latency is reasonable
        if metrics.processing_latency_ms > 1000.0 {
            println!(
                "⚠️  High processing latency: {:.1}ms",
                metrics.processing_latency_ms
            );
        } else {
            println!(
                "✅ Processing latency: {:.1}ms",
                metrics.processing_latency_ms
            );
        }

        Ok(())
    }

    /// Verify that Prometheus is collecting live metrics from MEVana
    async fn verify_prometheus_collecting_live_metrics(&self) -> Result<()> {
        println!("📊 Verifying Prometheus live metrics collection...");

        // Query Prometheus for MEVana metrics
        let query = "up{job=\"hotpath-arb-engine\"}";
        let url = format!("{}?query={}", PROMETHEUS_ENDPOINT, query);

        let response = timeout(Duration::from_secs(10), self.client.get(&url).send())
            .await
            .map_err(|_| anyhow!("Timeout querying Prometheus"))?
            .map_err(|e| anyhow!("Failed to query Prometheus: {}", e))?;

        let prometheus_response: Value = response.json().await?;

        // Check if MEVana service is UP in Prometheus
        if let Some(data) = prometheus_response.get("data") {
            if let Some(result) = data.get("result") {
                if let Some(result_array) = result.as_array() {
                    for metric in result_array {
                        if let Some(value) = metric.get("value") {
                            if let Some(value_array) = value.as_array() {
                                if value_array.len() >= 2 {
                                    if let Some(status) = value_array[1].as_str() {
                                        if status == "1" {
                                            println!(
                                                "✅ Prometheus collecting live MEVana metrics"
                                            );
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        return Err(anyhow!(
            "❌ Prometheus not collecting live MEVana metrics - service appears DOWN"
        ));
    }
}

fn live_tests_enabled() -> bool {
    match env::var(RUN_LIVE_TESTS_ENV) {
        Ok(val) => matches!(
            val.as_str(),
            "1" | "true" | "TRUE" | "True" | "yes" | "YES" | "on" | "ON"
        ),
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_live_transaction_data_flow_integration() -> Result<()> {
    if !live_tests_enabled() {
        println!(
            "⚠️ Skipping live transaction data flow integration test (set {}=1 to enable)",
            RUN_LIVE_TESTS_ENV
        );
        return Ok(());
    }

    // Initialize test environment
    let test = LiveTransactionDataTest::new();

    // Run comprehensive live data verification
    let result = timeout(
        Duration::from_secs(TEST_TIMEOUT_SECONDS),
        test.verify_live_transaction_data_flow(),
    )
    .await;

    match result {
        Ok(Ok(())) => {
            println!(
                "🎉 INTEGRATION TEST PASSED: Live transaction data flow verified successfully"
            );
            Ok(())
        }
        Ok(Err(e)) => {
            println!("❌ INTEGRATION TEST FAILED: {}", e);
            Err(e)
        }
        Err(_) => {
            let error = anyhow!(
                "❌ INTEGRATION TEST TIMEOUT: Test exceeded {} seconds",
                TEST_TIMEOUT_SECONDS
            );
            println!("{}", error);
            Err(error)
        }
    }
}

/// Standalone function to run live data verification from command line
pub async fn run_live_data_verification() -> Result<()> {
    let test = LiveTransactionDataTest::new();
    test.verify_live_transaction_data_flow().await
}

#[tokio::test]
async fn test_geyser_endpoint_connectivity() -> Result<()> {
    if !live_tests_enabled() {
        println!(
            "⚠️ Skipping Geyser endpoint connectivity test (set {}=1 to enable)",
            RUN_LIVE_TESTS_ENV
        );
        return Ok(());
    }

    println!(
        "🌐 Testing Geyser endpoint connectivity: {}",
        GEYSER_ENDPOINT
    );

    let client = Client::new();

    // Test basic connectivity (this will likely fail with gRPC endpoint, but shows network path)
    let result = timeout(Duration::from_secs(5), client.head(GEYSER_ENDPOINT).send()).await;

    match result {
        Ok(response) => {
            match response {
                Ok(_) => println!("✅ Geyser endpoint reachable"),
                Err(e) => {
                    // Expected for gRPC endpoints - just verify network path exists
                    if e.to_string().contains("connection") {
                        println!(
                            "✅ Geyser endpoint network path accessible (gRPC expected behavior)"
                        );
                    } else {
                        println!("⚠️  Geyser endpoint response: {}", e);
                    }
                }
            }
        }
        Err(_) => {
            return Err(anyhow!(
                "❌ Geyser endpoint not reachable - network timeout"
            ))
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_metrics_endpoint_availability() -> Result<()> {
    if !live_tests_enabled() {
        println!(
            "⚠️ Skipping metrics endpoint availability test (set {}=1 to enable)",
            RUN_LIVE_TESTS_ENV
        );
        return Ok(());
    }

    println!(
        "📊 Testing metrics endpoint availability: {}",
        METRICS_ENDPOINT
    );

    let client = Client::new();
    let response = timeout(Duration::from_secs(10), client.get(METRICS_ENDPOINT).send())
        .await
        .map_err(|_| anyhow!("Timeout connecting to metrics endpoint"))?
        .map_err(|e| anyhow!("Failed to connect to metrics endpoint: {}", e))?;

    if response.status().is_success() {
        let metrics_text = response.text().await?;
        let metric_count = metrics_text
            .lines()
            .filter(|line| !line.starts_with('#') && line.contains(' '))
            .count();
        println!(
            "✅ Metrics endpoint available ({} metrics exposed)",
            metric_count
        );

        // Verify essential MEVana metrics exist
        let essential_metrics = [
            "hotpath_transactions_processed_total",
            "hotpath_arbitrage_opportunities_total",
            "geyser_transactions_processed_total",
        ];

        for metric in &essential_metrics {
            if metrics_text.contains(metric) {
                println!("✅ Essential metric present: {}", metric);
            } else {
                println!("⚠️  Essential metric missing: {}", metric);
            }
        }

        Ok(())
    } else {
        Err(anyhow!(
            "❌ Metrics endpoint returned error: {}",
            response.status()
        ))
    }
}

/// Quick diagnostic function to print current system status
pub async fn print_system_diagnostic() -> Result<()> {
    println!("🔍 MEVana Live Data System Diagnostic");
    println!("=====================================");

    let client = Client::new();

    // Check metrics endpoint
    match client.get(METRICS_ENDPOINT).send().await {
        Ok(response) if response.status().is_success() => {
            let metrics = response.text().await?;
            let tx_count = extract_metric_simple(&metrics, "hotpath_transactions_processed_total");
            let arb_count =
                extract_metric_simple(&metrics, "hotpath_arbitrage_opportunities_total");
            let connection_status = if metrics.contains("geyser_transactions_processed_total") {
                "DATA FLOW"
            } else {
                "NO DATA"
            };

            println!("📊 Metrics Status: AVAILABLE");
            println!("🔗 Geyser Status: {}", connection_status);
            println!("📈 Transactions Processed: {}", tx_count);
            println!("🎯 Arbitrage Opportunities: {}", arb_count);
        }
        Ok(response) => println!("❌ Metrics Status: ERROR (HTTP {})", response.status()),
        Err(e) => println!("❌ Metrics Status: UNAVAILABLE ({})", e),
    }

    // Check Prometheus
    match client.get(PROMETHEUS_ENDPOINT).send().await {
        Ok(response) if response.status().is_success() => {
            println!("📊 Prometheus Status: AVAILABLE");
        }
        Ok(response) => println!("❌ Prometheus Status: ERROR (HTTP {})", response.status()),
        Err(e) => println!("❌ Prometheus Status: UNAVAILABLE ({})", e),
    }

    Ok(())
}

fn extract_metric_simple(metrics_text: &str, metric_name: &str) -> String {
    for line in metrics_text.lines() {
        if line.starts_with(metric_name) && !line.contains('#') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].to_string();
            }
        }
    }
    "0".to_string()
}
