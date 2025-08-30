use anyhow::Result;
use clap::Parser;
use log::{error, info, warn};
use normalizer_runtime::{NormalizedSwapV1, SwapExtractor, TxContext};
use normalizer_service::generated;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    ClientConfig, Message,
};
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Kafka bootstrap servers
    #[arg(short, long, default_value = "localhost:9092")]
    bootstrap_servers: String,

    /// Input Kafka topic for Raydium transactions
    #[arg(short, long, default_value = "raydium-transactions")]
    in_topic: String,

    /// Output Kafka topic for normalized swaps
    #[arg(short, long, default_value = "raydium-swaps.normalized")]
    out_topic: String,

    /// Dead letter queue topic for failed transactions
    #[arg(short, long, default_value = "raydium-swaps.dlq")]
    dlq_topic: String,

    /// Consumer group ID
    #[arg(short, long, default_value = "raydium-normalizer")]
    group_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    info!("Starting Raydium Normalizer...");
    info!("Input topic: {}", args.in_topic);
    info!("Output topic: {}", args.out_topic);
    info!("DLQ topic: {}", args.dlq_topic);

    // Create Kafka consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &args.group_id)
        .set("bootstrap.servers", &args.bootstrap_servers)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()?;

    consumer.subscribe(&[&args.in_topic])?;

    // Create Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &args.bootstrap_servers)
        .set("message.timeout.ms", "5000")
        .create()?;

    info!("Kafka connections established. Starting message processing...");

    let mut message_count = 0u64;
    let mut success_count = 0u64;
    let mut error_count = 0u64;

    loop {
        match consumer.recv().await {
            Ok(msg) => {
                message_count += 1;
                
                if let Some(payload) = msg.payload() {
                    match process_transaction(payload) {
                        Ok(Some(normalized_swap)) => {
                            // Send normalized swap to output topic
                            let json = serde_json::to_string(&normalized_swap)?;
                            let record = FutureRecord::to(&args.out_topic)
                                .payload(json.as_bytes())
                                .key(&normalized_swap.signature);
                            
                            match producer.send(record, Duration::from_secs(5)).await {
                                Ok(_) => {
                                    success_count += 1;
                                    if success_count % 100 == 0 {
                                        info!("Processed {} messages, {} successful, {} errors", 
                                              message_count, success_count, error_count);
                                    }
                                }
                                Err((e, _)) => {
                                    error!("Failed to send normalized swap: {}", e);
                                    error_count += 1;
                                }
                            }
                        }
                        Ok(None) => {
                            // Transaction processed but no swap extracted
                            if message_count % 1000 == 0 {
                                info!("Processed {} messages, {} successful, {} errors", 
                                      message_count, success_count, error_count);
                            }
                        }
                        Err(e) => {
                            error!("Failed to process transaction: {}", e);
                            error_count += 1;
                            
                            // Send to DLQ
                            let dlq_record = FutureRecord::to(&args.dlq_topic)
                                .payload(payload)
                                .key("processing_error");
                            
                            if let Err((e, _)) = producer.send(dlq_record, Duration::from_secs(5)).await {
                                warn!("Failed to send to DLQ: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Kafka consumer error: {}", e);
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

fn process_transaction(payload: &[u8]) -> Result<Option<NormalizedSwapV1>> {
    // Parse the transaction data from the stream service
    let tx_data: Value = serde_json::from_slice(payload)?;
    
    // Extract transaction details
    let signature = tx_data["signature"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing signature"))?;
    
    let slot = tx_data["slot"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing slot"))?;
    
    let block_time = tx_data["block_time"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("Missing block_time"))?;
    
    let program_id = tx_data["program_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing program_id"))?;
    
    // Use the generated normalizer service to decode and extract
    let decoder = generated::decoder_for(program_id)
        .ok_or_else(|| anyhow::anyhow!("No decoder for program {}", program_id))?;
    
    // Extract instruction data and account indices from the transaction
    let instruction_data = tx_data["instruction_data"]
        .as_str()
        .and_then(|s| base64::decode(s).ok())
        .unwrap_or_default();
    
    let account_indices = tx_data["account_indices"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_u8()).collect::<Vec<u8>>())
        .unwrap_or_default();
    
    let account_keys = tx_data["account_keys"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<String>>())
        .unwrap_or_default();
    
    // Decode the instruction using generated code
    let (instruction_name, roles_by_name) = decoder(&instruction_data, &account_indices, &account_keys)
        .ok_or_else(|| anyhow::anyhow!("Failed to decode instruction"))?;
    
    info!("Decoded instruction: {} for program: {}", instruction_name, program_id);
    
    // Find the appropriate extractor for this program
    let extractor = generated::registry()
        .iter()
        .find(|e| e.program_id() == program_id)
        .ok_or_else(|| anyhow::anyhow!("No extractor for program {}", program_id))?;
    
    // Create transaction context for extraction
    let token_balance_changes = extract_token_balance_changes(&tx_data)?;
    
    let tx_context = TxContext {
        slot,
        signature: signature.to_string(),
        token_balance_changes: &token_balance_changes,
        decoded_name: Some(instruction_name.clone()),
        decoded_accounts_by_role: Some(roles_by_name),
    };
    
    // Extract normalized swap using the generated extractor
    let normalized_swap = extractor.try_extract(&tx_context)
        .ok_or_else(|| anyhow::anyhow!("Failed to extract swap data"))?;
    
    info!("Successfully extracted swap: {} -> {} ({} -> {})", 
          normalized_swap.token_in_mint, normalized_swap.token_out_mint,
          normalized_swap.amount_in, normalized_swap.amount_out);
    
    Ok(Some(normalized_swap))
}

fn extract_token_balance_changes(tx_data: &Value) -> Result<Vec<normalizer_runtime::TokenBalanceChange>> {
    let mut changes = Vec::new();
    
    if let Some(balance_changes) = tx_data["token_balance_changes"].as_array() {
        for change in balance_changes {
            let mint = change["mint"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing mint in balance change"))?;
            
            let owner = change["owner"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing owner in balance change"))?;
            
            let pre_balance = change["pre_balance"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing pre_balance in balance change"))?;
            
            let post_balance = change["post_balance"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing post_balance in balance change"))?;
            
            changes.push(normalizer_runtime::TokenBalanceChange {
                mint: mint.to_string(),
                owner: owner.to_string(),
                pre_balance,
                post_balance,
            });
        }
    }
    
    Ok(changes)
}
