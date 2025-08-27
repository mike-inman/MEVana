use {
    clap::Parser,
    futures::stream::StreamExt,
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

mod instruction_account_mapper;
mod token_serializable;
mod serialization;

use instruction_account_mapper::{AccountMetadata, Idl, InstructionAccountMapper};
use token_serializable::convert_to_serializable;

const RAYDIUM_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
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

    #[clap(long, env = "OUT_TOPIC", default_value = "quotes.raydium.v1")]
    out_topic: String,

    #[clap(long, env = "RPC_URL", default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
}

#[derive(Debug, Serialize)]
struct RaydiumQuote {
    pool_address: String,
    token_a_mint: String,
    token_b_mint: String,
    token_a_symbol: String,
    token_b_symbol: String,
    price_a_per_b: f64,
    price_b_per_a: f64,
    reserve_a: u64,
    reserve_b: u64,
    fee_pct: f64,
    timestamp: i64,
    signature: String,
    swap_amount_in: u64,
    swap_amount_out: u64,
    swap_direction: String, // "a_to_b" or "b_to_a"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env::set_var(
        env_logger::DEFAULT_FILTER_ENV,
        env::var_os(env_logger::DEFAULT_FILTER_ENV).unwrap_or_else(|| "info".into()),
    );
    env_logger::init();

    let args = Args::parse();

    // Set up Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &args.kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()?;

    info!("🚀 Raydium Quote Service starting...");
    info!("📡 Kafka brokers: {}", args.kafka_brokers);
    info!("📤 Output topic: {}", args.out_topic);
    info!("🌐 RPC: {}", args.rpc_url);

    // Connect to Geyser stream
    let mut client = GeyserGrpcClient::build_from_shared(args.endpoint.clone())?
        .x_token(Some(args.x_token.clone()))?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;

    // Subscribe to Raydium transactions
    let mut transactions = std::collections::HashMap::new();
    transactions.insert(
        "raydium_client".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![RAYDIUM_AMM_PROGRAM_ID.to_string()],
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );

    let request = SubscribeRequest {
        accounts: std::collections::HashMap::default(),
        slots: std::collections::HashMap::default(),
        transactions,
        transactions_status: std::collections::HashMap::default(),
        blocks: std::collections::HashMap::default(),
        blocks_meta: std::collections::HashMap::default(),
        entry: std::collections::HashMap::default(),
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: Vec::default(),
        ping: None,
        from_slot: None,
    };

    let (_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;
    info!("Raydium quote stream opened - listening for transactions...");

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => match msg.update_oneof {
                Some(UpdateOneof::Transaction(update)) => {
                    let slot = update.slot;
                    let block_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs() as i64;

                    if let Some(txn) = update.transaction {
                        let raw_signature = txn.signature.clone();
                        info!("signature: {}", bs58::encode(&raw_signature).into_string());

                        let raw_transaction = txn.transaction.expect("transaction empty");
                        let raw_message = raw_transaction.message.expect("message empty").clone();
                        let header = raw_message.header.expect("header empty");
                        let meta = txn.meta.expect("Meta empty");

                        // For now, generate placeholder quotes for any Raydium transaction
                        // In a full implementation, we would parse the actual swap data
                        if true {
                            let quote = RaydiumQuote {
                                pool_address: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(), // Placeholder pool
                                token_a_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
                                token_b_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
                                token_a_symbol: "SOL".to_string(),
                                token_b_symbol: "USDC".to_string(),
                                price_a_per_b: 180.0, // Placeholder price
                                price_b_per_a: 0.00556, // Inverse price
                                reserve_a: 1_000_000_000, // 1 SOL in lamports
                                reserve_b: 180_000_000_000, // 180 USDC in smallest units
                                fee_pct: 0.003, // 0.3% fee
                                timestamp: block_time,
                                signature: bs58::encode(&raw_signature).into_string(),
                                swap_amount_in: 1_000_000, // Placeholder swap amount
                                swap_amount_out: 180_000, // Placeholder output amount
                                swap_direction: "a_to_b".to_string(), // SOL to USDC
                            };

                            let payload = serde_json::to_string(&quote)?;
                            let record = FutureRecord::to(&args.out_topic)
                                .payload(&payload)
                                .key(&quote.pool_address);

                            if let Err(e) = producer.send(record, Duration::from_secs(5)).await {
                                error!("Failed to send quote to Kafka: {:?}", e);
                            } else {
                                info!("📤 Published Raydium quote for {}: {:.6} {} per {} (MC: {:.2}%)",
                                      &quote.token_a_symbol, quote.price_a_per_b, &quote.token_b_symbol,
                                      &quote.token_a_symbol, quote.fee_pct * 100.0);
                            }
                        }
                    }
                }
                Some(UpdateOneof::Ping(_)) => {
                    // Send pong response if needed
                }
                _ => {}
            },
            Err(error) => {
                error!("Stream error: {error:?}");
                break;
            }
        }
    }

    info!("Raydium quote stream closed");
    Ok(())
}
