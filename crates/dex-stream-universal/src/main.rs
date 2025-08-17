use anyhow::Result;
use backoff::{future::retry, ExponentialBackoff};
use clap::Parser;
use dotenv::dotenv;
use env_logger;
use futures_util::{sink::SinkExt, stream::StreamExt, TryFutureExt};
use log::{error, info};
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

fn create_producer(broker: &str) -> Result<FutureProducer> {
    let p: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("enable.idempotence", "true")
        .set("compression.type", "lz4")
        .set("acks", "all")
        .set("linger.ms", "5")
        .create()?;
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

    let producer = create_producer(&broker)?;

    retry(ExponentialBackoff::default(), move || {
        let producer = producer.clone();
        let endpoint = endpoint.clone();
        let x_token = x_token.clone();
        let topic = topic.clone();
        let request = build_request(program_ids.clone());

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
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(update_msg) => match update_msg.update_oneof {
                        Some(UpdateOneof::Transaction(update)) => {
                            let key = update
                                .transaction
                                .as_ref()
                                .map(|t| bs58::encode(&t.signature).into_string())
                                .unwrap_or_else(|| format!("slot:{}", update.slot));
                            let mut buf = Vec::new();
                            update.encode(&mut buf)?;
                            let _ = producer
                                .send(
                                    FutureRecord::to(&topic).key(&key).payload(&buf),
                                    Duration::from_secs(10),
                                )
                                .await;
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
