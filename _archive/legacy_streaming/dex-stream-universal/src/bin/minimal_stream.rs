use anyhow::Result;
use clap::Parser;
use futures_util::StreamExt;
use log::{info, warn};
use std::time::{Duration, Instant};
use tonic::transport::channel::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::{CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions, subscribe_update::UpdateOneof};

#[derive(Debug, Parser, Clone)]
#[command(about = "Minimal Geyser stream (signatures + program IDs)")]
struct Args {
    #[arg(long, env = "ENDPOINT")] endpoint: String,
    #[arg(long, env = "X_TOKEN")] x_token: String,
    #[arg(long, env = "PROGRAM_IDS", help = "Comma separated program IDs when filtering ON")] program_ids: Option<String>,
    #[arg(long, env = "DEX_FILTERING", default_value = "OFF", help = "ON = filter by PROGRAM_IDS, OFF = wide open")] dex_filtering: String,
    #[arg(long, env = "RUN_SECS", default_value = "4", help = "Max seconds to run before exiting")] run_secs: u64,
    #[arg(long, env = "MAX_TX", default_value = "120", help = "Stop after this many transactions if reached earlier")] max_tx: usize,
}

fn build_request(program_ids: Vec<String>, wide_open: bool) -> SubscribeRequest {
    let mut map = std::collections::HashMap::new();
    let (vote, failed, account_include) = if wide_open { (None, None, Vec::new()) } else { (Some(false), Some(false), program_ids) };
    map.insert("minimal".to_string(), SubscribeRequestFilterTransactions { vote, failed, account_include, account_exclude: vec![], account_required: vec![], signature: None });
    SubscribeRequest { transactions: map, commitment: Some(CommitmentLevel::Processed as i32), ..Default::default() }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    let wide_open = args.dex_filtering.eq_ignore_ascii_case("OFF");
    let program_ids: Vec<String> = if wide_open { vec![] } else { args.program_ids.unwrap_or_default().split(',').filter(|s| !s.is_empty()).map(|s| s.trim().to_string()).collect() };
    info!("🚀 minimal_stream starting | mode={} | run_secs={} | max_tx={}", if wide_open {"WIDE-OPEN"} else {"FILTERED"}, args.run_secs, args.max_tx);
    if !wide_open { info!("Program IDs: {:?}", program_ids); }

    let mut client = GeyserGrpcClient::build_from_shared(args.endpoint.clone())?
        .x_token(Some(args.x_token.clone()))?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(10))
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect().await?;

    let request = build_request(program_ids, wide_open);
    let (_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;
    info!("✅ connected – streaming...");
    let start = Instant::now();
    let mut count = 0usize;

    while let Some(msg) = stream.next().await {
        if start.elapsed().as_secs() >= args.run_secs || count >= args.max_tx { break; }
        let Ok(update) = msg else { break }; 
        if let Some(UpdateOneof::Transaction(txu)) = update.update_oneof { 
            if let Some(txinfo) = txu.transaction.as_ref() { 
                let sig = bs58::encode(&txinfo.signature).into_string();
                let mut programs = Vec::new();
                if let Some(inner) = txinfo.transaction.as_ref() { if let Some(message) = inner.message.as_ref() { for ix in &message.instructions { if let Some(k) = message.account_keys.get(ix.program_id_index as usize) { programs.push(bs58::encode(k).into_string()); } } } }
                println!("slot={} sig={} programs=[{}]", txu.slot, sig, programs.join(","));
                count += 1;
            }
        }
    }

    info!("🛑 stopping after {} tx (elapsed {}s)", count, start.elapsed().as_secs());
    Ok(())
}
