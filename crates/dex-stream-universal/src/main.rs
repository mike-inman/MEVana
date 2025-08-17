use anyhow::Result;
use backoff::{future::retry, ExponentialBackoff};
use clap::Parser;
use dotenv::dotenv;
use env_logger;
use futures_util::{sink::SinkExt, stream::StreamExt, TryFutureExt};
use log::{error, info, warn};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::time::Duration;
use tonic::transport::channel::ClientTlsConfig;
use yellowstone_grpc_client::{GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::{
    geyser::SubscribeRequestFilterTransactions,
    prelude::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
    },
    prost::Message as _,
};
use serde_json::json;
use std::collections::HashSet;

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    broker: Option<String>,
    #[arg(short, long, help = "Output topic", default_value = "geyser.raw")]
    topic: String,
    #[arg(short, long)]
    endpoint: Option<String>,
    #[arg(long)]
    x_token: Option<String>,
    #[arg(long, help = "Comma-separated list of program ids")]
    program_ids: Option<String>,
}

type TxnFilterMap = std::collections::HashMap<String, SubscribeRequestFilterTransactions>;

/// Analyzes a transaction to find unknown program IDs for discovery
fn analyze_transaction_for_discovery(
    update: &yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
    known_program_ids: &HashSet<String>,
) -> Option<Vec<String>> {
    let mut unknown_programs = Vec::new();
    
    // Extract program IDs from transaction instructions
    if let Some(transaction_info) = &update.transaction {
        if let Some(transaction) = &transaction_info.transaction {
            if let Some(message) = &transaction.message {
                for instruction in &message.instructions {
                    let program_id_index = instruction.program_id_index as usize;
                    if let Some(program_id) = message.account_keys.get(program_id_index) {
                        let program_id_str = bs58::encode(program_id).into_string();
                        if !known_program_ids.contains(&program_id_str) {
                            unknown_programs.push(program_id_str);
                        }
                    }
                }
            }
        }
    }
    
    // Extract program IDs from account keys
    if let Some(transaction_info) = &update.transaction {
        if let Some(transaction) = &transaction_info.transaction {
            if let Some(message) = &transaction.message {
                for account_key in &message.account_keys {
                    let account_key_str = bs58::encode(account_key).into_string();
                    if !known_program_ids.contains(&account_key_str) {
                        // This might be a program ID, add it for discovery
                        unknown_programs.push(account_key_str);
                    }
                }
            }
        }
    }
    
    if unknown_programs.is_empty() {
        None
    } else {
        // Remove duplicates
        unknown_programs.sort();
        unknown_programs.dedup();
        Some(unknown_programs)
    }
}

fn build_request(program_ids: Vec<String>) -> SubscribeRequest {
    let mut transactions: TxnFilterMap = std::collections::HashMap::new();
    transactions.insert(
        "client".to_owned(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: program_ids,
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );
    SubscribeRequest {
        accounts: Default::default(),
        slots: Default::default(),
        transactions,
        transactions_status: Default::default(),
        blocks: Default::default(),
        blocks_meta: Default::default(),
        entry: Default::default(),
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: vec![],
        ping: None,
        from_slot: None,
    }
}

fn create_producer(broker: &str, fast_mode: bool) -> Result<FutureProducer> {
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", broker);
    if fast_mode {
        config
            .set("linger.ms", "0")
            .set("acks", "1")
            .set("compression.type", "lz4")
            .set("enable.idempotence", "false")
            .set("message.timeout.ms", "100");
    } else {
        config
            .set("enable.idempotence", "true")
            .set("compression.type", "lz4")
            .set("acks", "all")
            .set("linger.ms", "5");
    }
    let p: FutureProducer = config.create()?;
    Ok(p)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();
    let args = Args::parse();

    let broker = args
        .broker
        .or_else(|| std::env::var("KAFKA_BROKER").ok())
        .expect("KAFKA_BROKER");
    let topic = args.topic;
    let endpoint = args
        .endpoint
        .or_else(|| std::env::var("ENDPOINT").ok())
        .expect("ENDPOINT");
    let x_token = args
        .x_token
        .or_else(|| std::env::var("X_TOKEN").ok())
        .expect("X_TOKEN");
    let program_ids_csv = args
        .program_ids
        .or_else(|| std::env::var("PROGRAM_IDS").ok())
        .unwrap_or_default();
    let program_ids: Vec<String> = program_ids_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    info!(
        "universal stream: broker={broker} topic={topic} endpoint={endpoint} pids={:?}",
        program_ids
    );

    let fast_mode = std::env::var("FAST_MODE").ok().as_deref() == Some("1");
    let producer = create_producer(&broker, fast_mode)?;
    
    // Create a set of known program IDs for fast lookup
    let known_program_ids: HashSet<String> = program_ids.iter().cloned().collect();
    
    // Discovery mode: emit unknown program IDs for AI analysis
    let discovery_mode = std::env::var("DISCOVERY_MODE").ok().as_deref() == Some("1");
    let discovery_producer = if discovery_mode {
        Some(create_producer(&broker, true)?)
    } else {
        None
    };

    retry(ExponentialBackoff::default(), move || {
        let producer = producer.clone();
        let discovery_producer = discovery_producer.clone();
        let endpoint = endpoint.clone();
        let x_token = x_token.clone();
        let topic = topic.clone();
        let request = build_request(program_ids.clone());
        let known_program_ids = known_program_ids.clone();

        async move {
            let mut client = GeyserGrpcClient::build_from_shared(endpoint.clone())
                .map_err(|e| anyhow::anyhow!(e))?
                .x_token(Some(x_token.clone()))
                .map_err(|e| anyhow::anyhow!(e))?
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(10))
                .tls_config(ClientTlsConfig::new().with_native_roots())
                .map_err(|e| anyhow::anyhow!(e))?
                .max_decoding_message_size(1024 * 1024 * 1024)
                .connect()
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            let (mut subscribe_tx, mut stream) =
                client.subscribe_with_request(Some(request)).await?;
            info!("stream opened");
            
            let mut unknown_program_count = 0u64;
            
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(update_msg) => match update_msg.update_oneof {
                        Some(UpdateOneof::Transaction(update)) => {
                            let mut buf = Vec::new();
                            update.encode(&mut buf)?;
                            
                            // Emit to main topic
                            let _ = producer
                                .send(
                                    FutureRecord::<(), _>::to(&topic).payload(&buf),
                                    Duration::from_secs(10),
                                )
                                .await;
                            
                            // Discovery mode: analyze transaction for unknown program IDs
                            if discovery_mode {
                                if let Some(discovery_prod) = &discovery_producer {
                                    if let Some(unknown_programs) = analyze_transaction_for_discovery(&update, &known_program_ids) {
                                        for program_id in unknown_programs {
                                            unknown_program_count += 1;
                                            let signature = update.transaction
                                                .as_ref()
                                                .and_then(|t| t.signature.as_ref())
                                                .map(|s| bs58::encode(s).into_string())
                                                .unwrap_or_else(|| "unknown".to_string());
                                            
                                            let discovery_data = json!({
                                                "program_id": program_id,
                                                "timestamp": chrono::Utc::now().timestamp(),
                                                "transaction_signature": signature,
                                                "slot": update.slot,
                                                "encounter_count": unknown_program_count,
                                                "source": "dex-stream-universal"
                                            });
                                            
                                            let discovery_payload = discovery_data.to_string();
                                            let _ = discovery_prod
                                                .send(
                                                    FutureRecord::<(), _>::to("unknown-programs")
                                                        .payload(discovery_payload.as_bytes())
                                                        .key(&()),
                                                    Duration::from_secs(5),
                                                )
                                                .await;
                                            
                                            if unknown_program_count % 100 == 0 {
                                                info!("Discovered {} unknown program IDs", unknown_program_count);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some(UpdateOneof::Ping(_)) => {
                            let _ = subscribe_tx
                                .send(SubscribeRequest {
                                    ping: Some(SubscribeRequestPing { id: 1 }),
                                    ..Default::default()
                                })
                                .await;
                        }
                        _ => {}
                    },
                    Err(e) => {
                        error!("stream error: {e:?}");
                        break;
                    }
                }
            }
            info!("stream closed");
            Ok::<(), anyhow::Error>(())
        }
        .map_err(backoff::Error::transient)
    })
    .await?;

    Ok(())
}
