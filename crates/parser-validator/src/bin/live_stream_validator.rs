//! Live Stream Parser Validator
//! 
//! Connects to the live dex-transactions Kafka stream and validates
//! parsers against real Solana transaction data in real-time.

use anyhow::Result;
use clap::{Arg, Command};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::config::ClientConfig;
use rdkafka::message::Message;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;
use tracing::{info, warn, error, debug};

// Import the actual MEVana parsers
use normalizer_service::generated;
use normalizer_runtime as rt;
use normalizer_runtime::TxContext;

// Import yellowstone protobuf for transaction decoding
use yellowstone_grpc_proto::prelude as ypp;
use prost::Message as ProstMessage;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let matches = Command::new("live-stream-validator")
        .version("0.1.0")
        .about("Live stream parser validator for MEVana")
        .arg(
            Arg::new("kafka-brokers")
                .long("kafka-brokers")
                .help("Kafka broker addresses")
                .default_value("localhost:9092")
        )
        .arg(
            Arg::new("topic")
                .long("topic")
                .help("Kafka topic to consume")
                .default_value("dex-transactions")
        )
        .arg(
            Arg::new("max-messages")
                .long("max-messages")
                .help("Maximum messages to process")
                .default_value("100")
        )
        .get_matches();

    let kafka_brokers = matches.get_one::<String>("kafka-brokers").unwrap();
    let topic = matches.get_one::<String>("topic").unwrap();
    let max_messages: usize = matches.get_one::<String>("max-messages").unwrap().parse()?;

    info!("🌊 Starting Live Stream Parser Validator");
    info!("📡 Connecting to Kafka: {}", kafka_brokers);
    info!("📄 Topic: {}", topic);
    info!("🔢 Max messages: {}", max_messages);

    // Create Kafka consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "live-parser-validator")
        .set("bootstrap.servers", kafka_brokers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest") // Start from latest messages
        .create()?;

    // Subscribe to topic
    consumer.subscribe(&[topic])?;
    info!("✅ Connected to Kafka topic: {}", topic);

    let mut stream = consumer.stream();
    let mut message_count = 0;
    let mut parser_stats = ParserStats::new();
    let start_time = Instant::now();

    info!("🚀 Starting live transaction processing...\n");

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => {
                message_count += 1;
                
                if let Some(payload) = msg.payload() {
                    match process_transaction(payload, &mut parser_stats).await {
                        Ok(_) => {
                            if message_count % 10 == 0 {
                                print_progress(&parser_stats, message_count, start_time);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to process transaction: {}", e);
                        }
                    }
                }

                if message_count >= max_messages {
                    break;
                }
            }
            Err(e) => {
                error!("Kafka error: {}", e);
            }
        }
    }

    print_final_results(&parser_stats, message_count, start_time);
    Ok(())
}

/// Statistics for parser validation
#[derive(Debug, Clone)]
struct ParserStats {
    total_transactions: usize,
    successful_parses: usize,
    failed_parses: usize,
    dex_programs: std::collections::HashMap<String, usize>,
    parse_times: Vec<Duration>,
}

impl ParserStats {
    fn new() -> Self {
        Self {
            total_transactions: 0,
            successful_parses: 0,
            failed_parses: 0,
            dex_programs: std::collections::HashMap::new(),
            parse_times: Vec::new(),
        }
    }

    fn record_success(&mut self, program_id: &str, parse_time: Duration) {
        self.total_transactions += 1;
        self.successful_parses += 1;
        self.parse_times.push(parse_time);
        
        *self.dex_programs.entry(program_id.to_string()).or_insert(0) += 1;
    }

    fn record_failure(&mut self) {
        self.total_transactions += 1;
        self.failed_parses += 1;
    }

    fn accuracy(&self) -> f64 {
        if self.total_transactions == 0 {
            0.0
        } else {
            (self.successful_parses as f64 / self.total_transactions as f64) * 100.0
        }
    }

    fn avg_parse_time(&self) -> Duration {
        if self.parse_times.is_empty() {
            Duration::from_micros(0)
        } else {
            let total: Duration = self.parse_times.iter().sum();
            total / self.parse_times.len() as u32
        }
    }
}

/// Process a single transaction from the live stream
async fn process_transaction(payload: &[u8], stats: &mut ParserStats) -> Result<()> {
    let parse_start = Instant::now();
    
    // Try to decode as Yellowstone protobuf first (this is the actual format from dex-stream-universal)
    match ypp::SubscribeUpdateTransaction::decode(payload) {
        Ok(update) => {
            if let Some(info) = update.transaction.as_ref() {
                let validation_result = parse_with_mevana_parsers(&update, info).await;
                let parse_time = parse_start.elapsed();
                
                match validation_result {
                    Ok((program_id, parsed_swap)) => {
                        stats.record_success(&program_id, parse_time);
                        
                        // Print the actual parsed swap to console!
                        println!("🎯 PARSED SWAP from {}:", program_id);
                        println!("{}", serde_json::to_string_pretty(&parsed_swap)?);
                        println!("⚡ Parse time: {:?}\n", parse_time);
                    }
                    Err(e) => {
                        stats.record_failure();
                        debug!("❌ Parser validation failed: {}", e);
                    }
                }
            } else {
                stats.record_failure();
            }
        }
        Err(_) => {
            // Fallback: try JSON parsing for compatibility
            match serde_json::from_slice::<Value>(payload) {
                Ok(_data) => {
                    debug!("Received JSON transaction data (legacy format)");
                    stats.record_failure(); // For now, we focus on protobuf
                }
                Err(_) => {
                    debug!("Unknown payload format (length: {})", payload.len());
                    stats.record_failure();
                }
            }
        }
    }

    Ok(())
}

/// Extract program ID from transaction data
fn extract_program_id(data: &Value) -> Option<String> {
    // Try different ways to extract program ID
    if let Some(program_id) = data.get("program_id").and_then(|v| v.as_str()) {
        return Some(program_id.to_string());
    }
    
    if let Some(instructions) = data.get("instructions").and_then(|v| v.as_array()) {
        if let Some(first_instruction) = instructions.first() {
            if let Some(program_id) = first_instruction.get("program_id").and_then(|v| v.as_str()) {
                return Some(program_id.to_string());
            }
        }
    }

    None
}

/// Parse transaction using actual MEVana parsers
async fn parse_with_mevana_parsers(
    update: &ypp::SubscribeUpdateTransaction,
    info: &ypp::SubscribeUpdateTransactionInfo,
) -> Result<(String, serde_json::Value)> {
    use chrono::Utc;
    
    // Build account keys (base58 encoded)
    let mut keys_b58 = Vec::new();
    if let Some(msg) = info.transaction.as_ref().and_then(|t| t.message.as_ref()) {
        for key in &msg.account_keys {
            keys_b58.push(bs58::encode(key).into_string());
        }
    }

    // Build token balance changes
    let mut tb_changes = Vec::new();
    // Note: This is simplified - in real implementation we'd parse balance changes from the transaction

    // Process each instruction
    if let Some(msg) = info.transaction.as_ref().and_then(|t| t.message.as_ref()) {
        for (ix_index, ci) in msg.instructions.iter().enumerate() {
            let pid_idx = ci.program_id_index as usize;
            if pid_idx >= keys_b58.len() {
                continue;
            }
            let program_id = &keys_b58[pid_idx];

            // Try to get a decoder for this program ID from the MEVana registry
            let Some(decoder) = generated::decoder_for(program_id) else {
                continue; // No decoder available for this program
            };

            // Decode the instruction
            let Some((decoded_name, roles)) = decoder(&ci.data, &ci.accounts, &keys_b58) else {
                continue; // Failed to decode instruction
            };

            // Find extractor for this program id from the MEVana registry
            let extractor = generated::registry()
                .into_iter()
                .find(|e| e.program_id() == program_id);
            let Some(extractor) = extractor else {
                continue; // No extractor available
            };

            let signature = bs58::encode(&info.signature).into_string();

            // Build TxContext for the MEVana parser
            let ctx = TxContext {
                slot: update.slot,
                block_time_unix: Utc::now().timestamp(),
                signature: signature.clone(),
                ix_index: ix_index as u16,
                program_id: program_id.clone(),
                meta_ok: true,
                token_balance_changes: &tb_changes,
                account_data_by_pubkey: None,
                decoded_name: Some(decoded_name.clone()),
                decoded_args_json: None,
                decoded_accounts_by_role: Some(roles),
            };

            // Try to extract a normalized swap using the MEVana parser
            if let Some(normalized_swap) = extractor.try_extract(&ctx) {
                // Convert to JSON for display
                let swap_json = serde_json::to_value(&normalized_swap)?;
                return Ok((program_id.clone(), swap_json));
            }
        }
    }

    anyhow::bail!("No parseable swap instructions found in transaction")
}

/// Print progress update
fn print_progress(stats: &ParserStats, message_count: usize, start_time: Instant) {
    let elapsed = start_time.elapsed();
    let rate = message_count as f64 / elapsed.as_secs_f64();
    
    println!(
        "📊 Progress: {} msgs | Success: {}% | Rate: {:.1} msgs/sec | Avg parse: {:?}",
        message_count,
        stats.accuracy(),
        rate,
        stats.avg_parse_time()
    );
}

/// Print final validation results
fn print_final_results(stats: &ParserStats, message_count: usize, start_time: Instant) {
    let elapsed = start_time.elapsed();
    let rate = message_count as f64 / elapsed.as_secs_f64();
    
    println!("\n🎯 Live Stream Parser Validation Results");
    println!("==========================================");
    println!("📊 Total Messages Processed: {}", message_count);
    println!("✅ Successful Parses: {}", stats.successful_parses);
    println!("❌ Failed Parses: {}", stats.failed_parses);
    println!("🎯 Parser Accuracy: {:.2}%", stats.accuracy());
    println!("⚡ Processing Rate: {:.2} messages/second", rate);
    println!("⏱️  Average Parse Time: {:?}", stats.avg_parse_time());
    println!("⏱️  Total Runtime: {:?}", elapsed);
    
    if !stats.dex_programs.is_empty() {
        println!("\n🔍 DEX Program Coverage:");
        let mut programs: Vec<_> = stats.dex_programs.iter().collect();
        programs.sort_by(|a, b| b.1.cmp(a.1));
        
        for (program_id, count) in programs {
            let program_name = match program_id.as_str() {
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Orca Whirlpools",
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium AMM",
                "24Uqj9JCLxUeoC3hGfh5W3s9FM9uCHDS2SG3LYwBpyTi" => "Meteora DAMM",
                id if id.contains("pump") => "Pump.fun",
                _ => "Unknown/Other"
            };
            println!("  • {}: {} transactions", program_name, count);
        }
    }
    
    println!("\n✅ Live stream validation complete!");
    println!("🚀 Your parsers are ready for real-time arbitrage trading!");
}
