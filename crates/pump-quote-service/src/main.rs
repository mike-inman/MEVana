

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

const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, help = "gRPC endpoint", env = "ENDPOINT", default_value = "https://grpc.mainnet-beta.solana.com")]
    endpoint: String,

    #[clap(long, help = "X-Token", env = "X_TOKEN", default_value = "efff2d23-97ae-4c5f-8b8b-9a6265001189")]
    x_token: String,

    #[clap(long, env = "KAFKA_BROKERS", default_value = "kafka:9092")]
    kafka_brokers: String,

    #[clap(long, env = "OUT_TOPIC", default_value = "quotes.pump.v1")]
    out_topic: String,

    #[clap(long, env = "RPC_URL", default_value = "https://api.mainnet-beta.solana.com")]
    rpc_url: String,
}

#[derive(Debug, Serialize)]
struct PumpQuote {
    token_mint: String,
    price_sol: f64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    market_cap_sol: f64,
    bonding_curve: String,
    timestamp: i64,
    signature: String,
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

    info!("🚀 Pump.fun Quote Service starting...");
    info!("📡 Kafka brokers: {}", args.kafka_brokers);
    info!("📤 Output topic: {}", args.out_topic);
    info!("🌐 RPC: {}", args.rpc_url);

    // Connect to Geyser stream
    let mut client = GeyserGrpcClient::build_from_shared(args.endpoint.clone())?
        .x_token(Some(args.x_token.clone()))?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;

    // Subscribe to pump.fun transactions
    let mut transactions = std::collections::HashMap::new();
    transactions.insert(
        "pump_client".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![PUMPFUN_PROGRAM_ID.to_string()],
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
    info!("Pump.fun quote stream opened - listening for transactions...");

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
                        // For now, generate quotes for all transactions (will refine program detection later)
                        if true {
                            let quote = PumpQuote {
                                token_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL mint as placeholder
                                price_sol: 0.0001, // Placeholder price
                                virtual_sol_reserves: 1_000_000_000, // 1 SOL in lamports
                                virtual_token_reserves: 1_000_000_000_000, // 1B tokens (assuming 6 decimals)
                                market_cap_sol: 100.0, // Placeholder market cap
                                bonding_curve: "PlaceholderCurve111111111111111111111111111111".to_string(),
                                timestamp: block_time,
                                signature: bs58::encode(&txn.signature).into_string(),
                            };
                            let payload = serde_json::to_string(&quote)?;
                            let record = FutureRecord::to(&args.out_topic)
                                .payload(&payload)
                                .key(&quote.token_mint);

                            if let Err(e) = producer.send(record, Duration::from_secs(5)).await {
                                error!("Failed to send quote to Kafka: {:?}", e);
                            } else {
                                info!("📤 Published Pump.fun quote for {}: {:.6} SOL (MC: {:.2} SOL)",
                                      &quote.token_mint[..8], quote.price_sol, quote.market_cap_sol);
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

    info!("Pump.fun quote stream closed");
    Ok(())
}


