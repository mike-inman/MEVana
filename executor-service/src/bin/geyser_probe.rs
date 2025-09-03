use futures_util::StreamExt;
use std::time::Duration;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    SubscribeRequestFilterTransactions,
};

// Minimal standalone probe to validate GEYSER endpoint + token.
// Does not touch DB, Kafka, wallet, metrics. Prints raw account update summaries for ~N seconds.
// Run with:
//   GEYSER_ENDPOINT=... GEYSER_X_TOKEN=... cargo run -p executor-service --bin geyser_probe
// Optional env:
//   GEYSER_OWNER_FILTERS=prog1,prog2   (comma separated owners)
//   GEYSER_PROBE_SECS=10              (duration, default 10)
//   RUST_LOG=info                     (for logs)

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let endpoint = std::env::var("GEYSER_ENDPOINT").or_else(|_| std::env::var("ENDPOINT"))?;
    let x_token = std::env::var("GEYSER_X_TOKEN").or_else(|_| std::env::var("X_TOKEN"))?;
    let secs: u64 = std::env::var("GEYSER_PROBE_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    log::info!(
        "geyser_probe_start endpoint={} duration_secs={}",
        endpoint,
        secs
    );

    let mut client = GeyserGrpcClient::build_from_shared(endpoint.clone())?
        .x_token(Some(x_token.clone()))?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(10))
        .tls_config(tonic::transport::channel::ClientTlsConfig::new().with_native_roots())?
        .max_decoding_message_size(1024 * 1024 * 1024)
        .connect()
        .await?;
    log::info!("geyser_probe_connected");

    // Filtering configuration (mirrors dex-stream-universal simplified)
    let dex_filtering = std::env::var("DEX_FILTERING").unwrap_or_else(|_| "ON".to_string());
    // Support both PROGRAM_IDS and GEYSER_PROGRAM_IDS env vars
    let program_ids_csv = std::env::var("GEYSER_PROGRAM_IDS")
        .or_else(|_| std::env::var("PROGRAM_IDS"))
        .unwrap_or_default();
    let program_ids: Vec<String> = program_ids_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if dex_filtering == "OFF" {
        log::info!("probe_filter_mode=wide_open (vote+failed+all accounts)");
    } else {
        log::info!(
            "probe_filter_mode=filtered program_ids_count={} program_ids={:?}",
            program_ids.len(),
            program_ids
        );
    }

    let mut transactions_filters = std::collections::HashMap::new();
    let (vote, failed, account_include) = if dex_filtering == "OFF" {
        (None, None, Vec::new())
    } else {
        (Some(false), Some(false), program_ids)
    };
    transactions_filters.insert(
        "probe".to_string(),
        SubscribeRequestFilterTransactions {
            vote,
            failed,
            account_include,
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );

    let req = SubscribeRequest {
        commitment: Some(CommitmentLevel::Processed as i32),
        transactions: transactions_filters,
        ..Default::default()
    };
    let (_tx, mut stream) = client.subscribe_with_request(Some(req)).await?;
    log::info!("geyser_probe_stream_opened");

    use std::time::Instant;
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        match stream.next().await {
            Some(Ok(m)) => match m.update_oneof {
                Some(UpdateOneof::Account(a)) => {
                    if let Some(ai) = a.account {
                        log::info!("acct slot={} len={}", a.slot, ai.data.len());
                    }
                }
                Some(UpdateOneof::Transaction(t)) => {
                    if let Some(txinfo) = t.transaction.as_ref() {
                        let signature = bs58::encode(&txinfo.signature).into_string();
                        // Extract program IDs
                        let mut programs = Vec::new();
                        if let Some(inner) = txinfo.transaction.as_ref() {
                            if let Some(msg) = inner.message.as_ref() {
                                for ix in &msg.instructions {
                                    if let Some(key) =
                                        msg.account_keys.get(ix.program_id_index as usize)
                                    {
                                        programs.push(bs58::encode(key).into_string());
                                    }
                                }
                            }
                        }
                        log::info!(
                            "txn slot={} sig={} programs=[{}]",
                            t.slot,
                            signature,
                            programs.join(",")
                        );
                    } else {
                        log::info!("txn slot={} (no txinfo)", t.slot);
                    }
                }
                Some(UpdateOneof::Ping(_)) => {}
                Some(UpdateOneof::Pong(_)) => {}
                _ => {}
            },
            Some(Err(e)) => {
                log::error!("stream_error {:?}", e);
                break;
            }
            None => {
                log::warn!("stream_closed");
                break;
            }
        }
    }
    log::info!("done");
    Ok(())
}
