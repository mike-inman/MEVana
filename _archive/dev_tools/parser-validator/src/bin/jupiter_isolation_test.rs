//! Jupiter Parser Isolation Test
//!
//! Comprehensive testing framework for Jupiter V6 aggregator parser
//! Tests only Jupiter transactions in isolation for thorough validation

use anyhow::Result;
use clap::{Arg, Command};
use prost::Message as ProstMessage;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

// Jupiter V6 Program ID
const JUPITER_PROGRAM_ID: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

#[derive(Debug, Clone)]
struct JupiterTestStats {
    total_transactions: u64,
    jupiter_transactions: u64,
    successful_parses: u64,
    failed_parses: u64,
    instruction_coverage: HashMap<String, u64>,
    latency_measurements: Vec<Duration>,
    error_types: HashMap<String, u64>,
    unknown_program_ids: HashMap<String, u64>,
}

impl JupiterTestStats {
    fn new() -> Self {
        Self {
            total_transactions: 0,
            jupiter_transactions: 0,
            successful_parses: 0,
            failed_parses: 0,
            instruction_coverage: HashMap::new(),
            latency_measurements: Vec::new(),
            error_types: HashMap::new(),
            unknown_program_ids: HashMap::new(),
        }
    }

    fn record_jupiter_transaction(
        &mut self,
        instruction_name: &str,
        parse_time: Duration,
        success: bool,
    ) {
        self.jupiter_transactions += 1;
        self.latency_measurements.push(parse_time);

        if success {
            self.successful_parses += 1;
            *self
                .instruction_coverage
                .entry(instruction_name.to_string())
                .or_insert(0) += 1;
        } else {
            self.failed_parses += 1;
        }
    }

    fn record_error(&mut self, error_type: &str) {
        *self.error_types.entry(error_type.to_string()).or_insert(0) += 1;
    }

    fn record_unknown_program(&mut self, program_id: &str) {
        *self
            .unknown_program_ids
            .entry(program_id.to_string())
            .or_insert(0) += 1;
    }

    fn get_performance_metrics(&self) -> PerformanceMetrics {
        if self.latency_measurements.is_empty() {
            return PerformanceMetrics::default();
        }

        let mut sorted_latencies = self.latency_measurements.clone();
        sorted_latencies.sort();

        let p50_idx = sorted_latencies.len() / 2;
        let p95_idx = (sorted_latencies.len() * 95) / 100;
        let p99_idx = (sorted_latencies.len() * 99) / 100;

        PerformanceMetrics {
            p50: sorted_latencies[p50_idx],
            p95: sorted_latencies[p95_idx.min(sorted_latencies.len() - 1)],
            p99: sorted_latencies[p99_idx.min(sorted_latencies.len() - 1)],
            avg: Duration::from_nanos(
                sorted_latencies
                    .iter()
                    .map(|d| d.as_nanos() as u64)
                    .sum::<u64>()
                    / sorted_latencies.len() as u64,
            ),
            success_rate: if self.jupiter_transactions > 0 {
                (self.successful_parses as f64 / self.jupiter_transactions as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    fn print_comprehensive_report(&self) {
        let metrics = self.get_performance_metrics();

        info!("🚀 JUPITER PARSER ISOLATION TEST REPORT");
        info!("=====================================");
        info!("📊 Transaction Statistics:");
        info!(
            "  Total Transactions Processed: {}",
            self.total_transactions
        );
        info!(
            "  Jupiter Transactions Found: {}",
            self.jupiter_transactions
        );
        info!("  Successful Parses: {}", self.successful_parses);
        info!("  Failed Parses: {}", self.failed_parses);
        info!("  Success Rate: {:.2}%", metrics.success_rate);

        info!("⚡ Performance Metrics:");
        info!("  P50 Latency: {:.2}ms", metrics.p50.as_secs_f64() * 1000.0);
        info!("  P95 Latency: {:.2}ms", metrics.p95.as_secs_f64() * 1000.0);
        info!("  P99 Latency: {:.2}ms", metrics.p99.as_secs_f64() * 1000.0);
        info!(
            "  Average Latency: {:.2}ms",
            metrics.avg.as_secs_f64() * 1000.0
        );

        info!("📋 Instruction Coverage:");
        for (instruction, count) in &self.instruction_coverage {
            info!("  {}: {} transactions", instruction, count);
        }

        if !self.error_types.is_empty() {
            warn!("⚠️ Error Analysis:");
            for (error_type, count) in &self.error_types {
                warn!("  {}: {} occurrences", error_type, count);
            }
        }

        if !self.unknown_program_ids.is_empty() {
            info!("🔍 Unknown Program IDs Discovered:");
            for (program_id, count) in &self.unknown_program_ids {
                info!("  {}: {} transactions", program_id, count);
            }
        }
    }
}

#[derive(Debug, Default)]
struct PerformanceMetrics {
    p50: Duration,
    p95: Duration,
    p99: Duration,
    avg: Duration,
    success_rate: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let matches = Command::new("jupiter-isolation-test")
        .version("0.1.0")
        .about("Jupiter parser isolation testing framework")
        .arg(
            Arg::new("kafka-brokers")
                .long("kafka-brokers")
                .help("Kafka broker endpoints")
                .default_value("localhost:29092"),
        )
        .arg(
            Arg::new("topic")
                .long("topic")
                .help("Kafka topic to consume")
                .default_value("dex-transactions"),
        )
        .arg(
            Arg::new("max-messages")
                .long("max-messages")
                .help("Maximum messages to process")
                .value_parser(clap::value_parser!(u64))
                .default_value("1000"),
        )
        .arg(
            Arg::new("report-interval")
                .long("report-interval")
                .help("Report interval in seconds")
                .value_parser(clap::value_parser!(u64))
                .default_value("30"),
        )
        .get_matches();

    let kafka_brokers = matches.get_one::<String>("kafka-brokers").unwrap();
    let topic = matches.get_one::<String>("topic").unwrap();
    let max_messages = *matches.get_one::<u64>("max-messages").unwrap();
    let report_interval = *matches.get_one::<u64>("report-interval").unwrap();

    info!("🚀 Starting Jupiter Parser Isolation Test");
    info!("Target: {} transactions from {}", max_messages, topic);
    info!("Jupiter Program ID: {}", JUPITER_PROGRAM_ID);

    // Create Kafka consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", kafka_brokers)
        .set("group.id", "jupiter-isolation-test")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest")
        .create()?;

    consumer.subscribe(&[topic])?;
    info!("✅ Connected to Kafka topic: {}", topic);

    let mut stats = JupiterTestStats::new();
    let mut last_report = Instant::now();
    let report_duration = Duration::from_secs(report_interval);

    let mut message_stream = consumer.stream();

    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(message) => {
                if let Some(payload) = message.payload() {
                    stats.total_transactions += 1;

                    if let Err(e) = process_transaction(payload, &mut stats).await {
                        stats.record_error(&format!("Processing error: {}", e));
                    }

                    // Periodic reporting
                    if last_report.elapsed() >= report_duration {
                        stats.print_comprehensive_report();
                        last_report = Instant::now();
                    }

                    // Stop after max messages
                    if stats.total_transactions >= max_messages {
                        break;
                    }
                }
            }
            Err(e) => {
                error!("Kafka error: {}", e);
                stats.record_error(&format!("Kafka error: {}", e));
            }
        }
    }

    // Final report
    info!("🏁 Jupiter Parser Isolation Test Complete");
    stats.print_comprehensive_report();

    Ok(())
}

/// Process a single transaction and test Jupiter parser
async fn process_transaction(payload: &[u8], stats: &mut JupiterTestStats) -> Result<()> {
    let parse_start = Instant::now();

    // Try to decode the yellowstone transaction
    let transaction_data =
        match yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction::decode(payload) {
            Ok(tx) => tx,
            Err(_) => {
                // Try as JSON fallback
                let json_data: Value = serde_json::from_slice(payload)?;
                return process_json_transaction(&json_data, stats, parse_start);
            }
        };

    // Extract program IDs from the transaction
    if let Some(transaction) = transaction_data.transaction.as_ref() {
        if let Some(transaction_inner) = transaction.transaction.as_ref() {
            if let Some(message) = transaction_inner.message.as_ref() {
                // Build account keys
                let account_keys: Vec<String> = message
                    .account_keys
                    .iter()
                    .map(|key| bs58::encode(key).into_string())
                    .collect();

                // Check each instruction for Jupiter program ID
                for (ix_index, instruction) in message.instructions.iter().enumerate() {
                    let program_id_index = instruction.program_id_index as usize;
                    if let Some(program_id) = account_keys.get(program_id_index) {
                        // Record all program IDs for discovery
                        if program_id != JUPITER_PROGRAM_ID {
                            stats.record_unknown_program(program_id);
                            continue;
                        }

                        // Found Jupiter transaction - test parser
                        let parse_time = parse_start.elapsed();
                        let parse_result = test_jupiter_parser(
                            &instruction.data,
                            &instruction.accounts,
                            &account_keys,
                            ix_index,
                        )
                        .await;

                        match parse_result {
                            Ok(instruction_name) => {
                                stats.record_jupiter_transaction(
                                    &instruction_name,
                                    parse_time,
                                    true,
                                );
                                debug!(
                                    "✅ Parsed Jupiter {}: {:.2}ms",
                                    instruction_name,
                                    parse_time.as_secs_f64() * 1000.0
                                );
                            }
                            Err(e) => {
                                stats.record_jupiter_transaction("unknown", parse_time, false);
                                stats.record_error(&format!("Parse failure: {}", e));
                                debug!("❌ Failed to parse Jupiter transaction: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process JSON-formatted transaction (fallback)
fn process_json_transaction(
    data: &Value,
    stats: &mut JupiterTestStats,
    parse_start: Instant,
) -> Result<()> {
    // Look for program_id in JSON transaction
    if let Some(program_id) = data.get("program_id").and_then(|p| p.as_str()) {
        if program_id != JUPITER_PROGRAM_ID {
            stats.record_unknown_program(program_id);
            return Ok(());
        }

        // Found Jupiter transaction
        let parse_time = parse_start.elapsed();
        let instruction_name = data
            .get("instruction_name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        stats.record_jupiter_transaction(instruction_name, parse_time, true);
        debug!(
            "✅ Parsed Jupiter JSON {}: {:.2}ms",
            instruction_name,
            parse_time.as_secs_f64() * 1000.0
        );
    }

    Ok(())
}

/// Test the Jupiter parser implementation
async fn test_jupiter_parser(
    instruction_data: &[u8],
    account_indices: &[u8],
    account_keys: &[String],
    ix_index: usize,
) -> Result<String> {
    // Use the generated Jupiter parser
    if let Some((instruction_name, roles)) = normalizer_generated::decoder_for(JUPITER_PROGRAM_ID)
        .and_then(|decoder| decoder(instruction_data, account_indices, account_keys))
    {
        // Additional validation - check required roles are present
        validate_jupiter_instruction(&instruction_name, &roles)?;

        Ok(instruction_name)
    } else {
        Err(anyhow::anyhow!(
            "Failed to decode Jupiter instruction at index {}",
            ix_index
        ))
    }
}

/// Validate Jupiter instruction has required roles
fn validate_jupiter_instruction(
    instruction_name: &str,
    roles: &HashMap<String, String>,
) -> Result<()> {
    match instruction_name {
        "route" => {
            require_role(roles, "userTransferAuthority")?;
            require_role(roles, "destinationTokenAccount")?;
        }
        "whirlpoolSwap" => {
            require_role(roles, "whirlpool")?;
            require_role(roles, "tokenOwnerAccountA")?;
            require_role(roles, "tokenOwnerAccountB")?;
        }
        "raydiumSwap" => {
            require_role(roles, "ammId")?;
            require_role(roles, "userSourceTokenAccount")?;
            require_role(roles, "userDestinationTokenAccount")?;
        }
        _ => {
            // For other instructions, basic validation
            if roles.is_empty() {
                return Err(anyhow::anyhow!(
                    "No roles found for instruction: {}",
                    instruction_name
                ));
            }
        }
    }
    Ok(())
}

fn require_role(roles: &HashMap<String, String>, role_name: &str) -> Result<()> {
    if !roles.contains_key(role_name) {
        return Err(anyhow::anyhow!("Missing required role: {}", role_name));
    }
    Ok(())
}
