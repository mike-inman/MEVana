use {
    clap::Parser,
    futures::{stream::StreamExt, SinkExt},
    log::{error, info},
    rdkafka::{
        config::ClientConfig,
        producer::{FutureProducer, FutureRecord},
    },
    serde::Serialize,
    std::{
        env,
        time::Duration,
    },
    tonic::transport::channel::ClientTlsConfig,
    yellowstone_grpc_client::GeyserGrpcClient,
    yellowstone_grpc_proto::prelude::*,
    yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof,
};

// Note: These modules are available but not currently used in this simplified version
// mod instruction_account_mapper;
// mod token_serializable;
// mod serialization;

const METEORA_DAMM_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, help = "gRPC endpoint", env = "ENDPOINT", default_value = "https://grpc.ny.shyft.to")]
    endpoint: String,

    #[clap(long, help = "X-Token", env = "X_TOKEN", default_value = "efff2d23-97ae-4c5f-8b8b-9a6265001189")]
    x_token: String,

    #[clap(long, env = "KAFKA_BROKERS", default_value = "kafka:9092")]
    kafka_brokers: String,

    #[clap(long, env = "OUT_TOPIC", default_value = "quotes.meteora.v1")]
    out_topic: String,
}

#[derive(Serialize)]
struct MeteoraQuote {
    signature: String,
    slot: u64,
    block_time: i64,
    pool_address: String,
    amount_in: u64,
    minimum_amount_out: u64,
    actual_amount_out: Option<u64>,
    token_in: String,
    token_out: String,
    fee_amount: Option<u64>,
    dex_name: String,
    program_id: String,
}



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env::set_var(
        env_logger::DEFAULT_FILTER_ENV,
        env::var_os(env_logger::DEFAULT_FILTER_ENV).unwrap_or_else(|| "info".into()),
    );
    env_logger::init();

    let args = Args::parse();

    // Create Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &args.kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()?;

    info!("Starting Meteora quote service...");
    info!("Program ID: {}", METEORA_DAMM_PROGRAM_ID);
    info!("Kafka brokers: {}", args.kafka_brokers);
    info!("Output topic: {}", args.out_topic);

    let mut client = GeyserGrpcClient::build_from_shared(args.endpoint.clone())?
        .x_token(Some(args.x_token.clone()))?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;
    info!("Connected to gRPC endpoint: {}", args.endpoint);

    let request = yellowstone_grpc_proto::prelude::SubscribeRequest {
        accounts: std::collections::HashMap::default(),
        slots: std::collections::HashMap::default(),
        transactions: {
            let mut transactions = std::collections::HashMap::new();
            transactions.insert(
                "meteora".to_string(),
                yellowstone_grpc_proto::prelude::SubscribeRequestFilterTransactions {
                    vote: Some(false),
                    failed: Some(false),
                    account_include: vec![METEORA_DAMM_PROGRAM_ID.to_string()],
                    account_exclude: vec![],
                    account_required: vec![],
                    signature: None,
                },
            );
            transactions
        },
        transactions_status: std::collections::HashMap::default(),
        blocks: std::collections::HashMap::default(),
        blocks_meta: std::collections::HashMap::default(),
        entry: std::collections::HashMap::default(),
        commitment: Some(yellowstone_grpc_proto::prelude::CommitmentLevel::Processed as i32),
        accounts_data_slice: Vec::default(),
        ping: None,
        from_slot: None,
    };

    let (mut subscribe_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;

    info!("Stream opened, listening for Meteora transactions...");

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => match msg.update_oneof {
                Some(UpdateOneof::Transaction(update)) => {
                    let slot = update.slot;
                    let block_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs() as i64;

                    if let Some(txn_info) = update.transaction {
                        let raw_signature = txn_info.signature.clone();
                        let signature_str = bs58::encode(&raw_signature).into_string();

                        if let Some(ref raw_transaction) = txn_info.transaction {
                            if let Some(ref raw_message) = raw_transaction.message {
                                // Process the transaction for swap quotes
                                match extract_meteora_quote(&txn_info, slot, block_time, &signature_str) {
                                    Ok(Some(quote)) => {
                                        let quote_json = serde_json::to_string(&quote).unwrap();
                                        let record = FutureRecord::to(&args.out_topic)
                                            .payload(&quote_json)
                                            .key(&signature_str);

                                        if let Err(e) = producer.send(record, Duration::from_secs(5)).await {
                                            error!("Failed to send Meteora quote to Kafka: {:?}", e);
                                        } else {
                                            info!("📤 Published Meteora quote: {} -> {} tokens", quote.amount_in, quote.actual_amount_out.unwrap_or(0));
                                        }
                                    }
                                    Ok(None) => {
                                        // Not a swap transaction, skip silently
                                    }
                                    Err(e) => {
                                        error!("Failed to extract Meteora quote from {}: {}", signature_str, e);
                                    }
                                }
                            }
                        }
                    }
                }
                Some(UpdateOneof::Ping(_)) => {
                    subscribe_tx
                        .send(yellowstone_grpc_proto::prelude::SubscribeRequest {
                            ping: Some(yellowstone_grpc_proto::prelude::SubscribeRequestPing { id: 1 }),
                            ..Default::default()
                        })
                        .await?;
                }
                Some(UpdateOneof::Pong(_)) => {}
                None => {
                    error!("Update not found in message");
                    break;
                }
                _ => {}
            },
            Err(error) => {
                error!("Stream error: {error:?}");
                break;
            }
        }
    }

    info!("Stream closed");
    Ok(())
}

fn extract_meteora_quote(
    txn_info: &SubscribeUpdateTransactionInfo,
    slot: u64,
    block_time: i64,
    signature: &str,
) -> anyhow::Result<Option<MeteoraQuote>> {
    // Check if this transaction involves Meteora DAMM
    if let Some(transaction) = &txn_info.transaction {
        if let Some(message) = &transaction.message {
            // Check if any account is the Meteora DAMM program
            let involves_meteora = message.account_keys.iter().any(|account_key| {
                let pubkey = solana_sdk::pubkey::Pubkey::new_from_array(account_key.clone().try_into().unwrap_or([0; 32]));
                pubkey.to_string() == METEORA_DAMM_PROGRAM_ID
            });

            if involves_meteora {
                // This is a Meteora transaction - create a basic quote
                // For now, we'll create placeholder quotes for any Meteora transaction
                // TODO: Parse actual swap instructions and extract real trade data
                let quote = MeteoraQuote {
                    signature: signature.to_string(),
                    slot,
                    block_time,
                    pool_address: "unknown".to_string(), // TODO: Extract from instruction accounts
                    amount_in: 0, // TODO: Extract from swap instruction
                    minimum_amount_out: 0, // TODO: Extract from swap instruction
                    actual_amount_out: None, // TODO: Extract from transaction meta
                    token_in: "unknown".to_string(), // TODO: Extract from pool state
                    token_out: "unknown".to_string(), // TODO: Extract from pool state
                    fee_amount: None, // TODO: Calculate from transaction meta
                    dex_name: "meteora".to_string(),
                    program_id: METEORA_DAMM_PROGRAM_ID.to_string(),
                };

                return Ok(Some(quote));
            }
        }
    }

    Ok(None)
}
