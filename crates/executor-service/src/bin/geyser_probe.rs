//! Read-only Yellowstone subscription benchmark. No wallet or submission path.
use futures_util::StreamExt;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::Write,
    time::{Duration, Instant},
};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    SubscribeRequestFilterTransactions,
};

fn row(
    file: &mut std::fs::File,
    label: &str,
    endpoint: &str,
    trial: &str,
    metric: &str,
    value: f64,
    unit: &str,
    status: &str,
) -> anyhow::Result<()> {
    writeln!(
        file,
        "{},{},{},{},{},{:.3},{},{}",
        label,
        chrono::Utc::now().to_rfc3339(),
        endpoint,
        trial,
        metric,
        value,
        unit,
        status
    )?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint = std::env::var("GEYSER_ENDPOINT")?;
    let token = std::env::var("GEYSER_X_TOKEN")?;
    let label = std::env::var("BENCH_SERVER_LABEL")?;
    let trial = std::env::var("BENCH_TRIAL")?;
    let output = std::env::var("BENCH_OUTPUT")?;
    anyhow::ensure!(
        matches!(label.as_str(), "vultr" | "hetzner"),
        "invalid server label"
    );
    anyhow::ensure!(
        trial
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "invalid trial label"
    );
    let host = url::Url::parse(&endpoint)
        .map_err(|_| anyhow::anyhow!("invalid Geyser endpoint"))?
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("endpoint host missing"))?
        .to_string();
    let secs: u64 = std::env::var("GEYSER_PROBE_SECS")
        .unwrap_or_else(|_| "120".into())
        .parse()?;
    let stall_secs: u64 = std::env::var("GEYSER_STALL_SECS")
        .unwrap_or_else(|_| "10".into())
        .parse()?;
    anyhow::ensure!(secs > 0 && stall_secs > 0, "durations must be positive");
    let program_ids: Vec<String> = std::env::var("GEYSER_PROGRAM_IDS")?
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    anyhow::ensure!(
        !program_ids.is_empty(),
        "GEYSER_PROGRAM_IDS must contain at least one program"
    );

    let mut file = OpenOptions::new().append(true).create(true).open(output)?;
    if file.metadata()?.len() == 0 {
        writeln!(
            file,
            "server_label,utc_time,endpoint,trial,metric,value,unit,status"
        )?;
    }
    let started = Instant::now();
    let mut client = GeyserGrpcClient::build_from_shared(endpoint)
        .map_err(|_| anyhow::anyhow!("invalid Geyser endpoint"))?
        .x_token(Some(token))
        .map_err(|_| anyhow::anyhow!("invalid Geyser token"))?
        .connect_timeout(Duration::from_secs(10))
        .tls_config(tonic::transport::channel::ClientTlsConfig::new().with_native_roots())?
        .max_decoding_message_size(16 * 1024 * 1024)
        .connect()
        .await
        .map_err(|_| anyhow::anyhow!("Geyser connection failed"))?;
    row(
        &mut file,
        &label,
        &host,
        &trial,
        "connect_ms",
        started.elapsed().as_secs_f64() * 1000.0,
        "ms",
        "ok",
    )?;

    let mut transactions = HashMap::new();
    transactions.insert(
        "benchmark".into(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: program_ids,
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );
    let request = SubscribeRequest {
        commitment: Some(CommitmentLevel::Processed as i32),
        transactions,
        ..Default::default()
    };
    let (_sender, mut stream) = client
        .subscribe_with_request(Some(request))
        .await
        .map_err(|_| anyhow::anyhow!("Geyser subscription failed"))?;
    let opened = Instant::now();
    let deadline = opened + Duration::from_secs(secs);
    let (mut updates, mut stalls, mut disconnects) = (0u64, 0u64, 0u64);
    let mut first_ms = None;
    let mut last_update = opened;
    let mut status = "ok";
    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        if now.duration_since(last_update) >= Duration::from_secs(stall_secs) {
            stalls += 1;
            last_update = now;
        }
        let wait = std::cmp::min(Duration::from_secs(stall_secs), deadline - now);
        match tokio::time::timeout(wait, stream.next()).await {
            Ok(Some(Ok(message))) => {
                if matches!(message.update_oneof, Some(UpdateOneof::Transaction(_))) {
                    updates += 1;
                    first_ms.get_or_insert_with(|| opened.elapsed().as_secs_f64() * 1000.0);
                    last_update = Instant::now();
                }
            }
            Ok(Some(Err(_))) | Ok(None) => {
                disconnects += 1;
                status = "disconnected";
                break;
            }
            Err(_) => {
                if Instant::now() >= deadline {
                    break;
                }
            }
        }
    }
    let elapsed = opened.elapsed().as_secs_f64();
    if first_ms.is_none() && status == "ok" {
        status = "no_updates";
    }
    row(
        &mut file,
        &label,
        &host,
        &trial,
        "first_update_ms",
        first_ms.unwrap_or(0.0),
        "ms",
        if first_ms.is_some() { "ok" } else { status },
    )?;
    row(
        &mut file,
        &label,
        &host,
        &trial,
        "updates_per_sec",
        updates as f64 / elapsed,
        "per_sec",
        status,
    )?;
    row(
        &mut file,
        &label,
        &host,
        &trial,
        "updates",
        updates as f64,
        "count",
        status,
    )?;
    row(
        &mut file,
        &label,
        &host,
        &trial,
        "stalls",
        stalls as f64,
        "count",
        status,
    )?;
    row(
        &mut file,
        &label,
        &host,
        &trial,
        "disconnects",
        disconnects as f64,
        "count",
        status,
    )?;
    row(
        &mut file,
        &label,
        &host,
        &trial,
        "reconnects",
        0.0,
        "count",
        status,
    )?;
    row(
        &mut file,
        &label,
        &host,
        &trial,
        "runtime_sec",
        elapsed,
        "sec",
        status,
    )?;
    println!(
        "trial={} endpoint={} updates={} stalls={} disconnects={} status={}",
        trial, host, updates, stalls, disconnects, status
    );
    anyhow::ensure!(status == "ok", "stream trial ended with {status}");
    Ok(())
}
