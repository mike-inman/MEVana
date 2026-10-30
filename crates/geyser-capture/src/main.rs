use anyhow::{ensure, Context, Result};
use clap::Parser;
use geyser_capture::{
    config::{self, Args},
    format::{self, Metadata, Writer},
    layout, snapshot,
    source::{self, Event, GrpcSource},
};
use prost::Message;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, watch},
};
use yellowstone_grpc_proto::prelude::{subscribe_update::UpdateOneof, SubscribeUpdate};

fn launch_snapshot(
    tx: mpsc::Sender<snapshot::SnapshotRun>,
    allowlist: Vec<String>,
    key: Arc<String>,
    id: u8,
    remaining_bytes: u64,
) {
    tokio::spawn(async move {
        let result = snapshot::run(&allowlist, &key, id, remaining_bytes).await;
        let _ = tx.send(result).await;
    });
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("capture failed: {e}");
        std::process::exit(1);
    }
}
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        tokio::select! {_=tokio::signal::ctrl_c()=>{},_=term.recv()=>{}}
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
async fn run() -> Result<()> {
    let a = Args::parse();
    ensure!(
        a.duration > 0
            && a.duration <= 600
            && a.stall_seconds > 0
            && a.max_reconnect_failures > 0
            && a.rotation_seconds > 0
            && a.rotation_bytes > 1024
            && a.max_bytes > 0
            && a.max_bytes <= 450_000_000,
        "invalid bounded run configuration"
    );
    ensure!(
        !a.filter_check || a.duration <= 60,
        "filter check limited to 60 seconds"
    );
    ensure!(a.metrics.ip().is_loopback(), "metrics must bind loopback");
    ensure!(
        a.connection_tier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "invalid tier"
    );
    let commitment = a.commitment()?;
    let snapshot_enabled = a.snapshot_rpc.is_some();
    ensure!(
        !snapshot_enabled || (!a.synthetic_source && a.provider == config::Provider::Sodae),
        "snapshot requires public Sodae stream"
    );
    let snapshot_key = if snapshot_enabled {
        Some(Arc::new(
            config::credential(config::Provider::Tatum, &a.credentials, |k| {
                std::env::var(k).ok()
            })?
            .expose()
            .to_owned(),
        ))
    } else {
        None
    };
    let mut allowlist = a.allowlist()?;
    // A known public pool key tests targeted filters; this check does not use an owner-wide stream.
    if a.filter_check && allowlist.is_empty() {
        allowlist.push("HJPjoWUrhoZzkGQ93amVjpRKcTWnAB8GH64wkHBEcHvt".into());
    }
    let endpoint = a.endpoint.clone().unwrap_or_else(|| a.provider.endpoint());
    let url = config::validate_endpoint(a.provider, &endpoint, a.synthetic_source)?;
    let host = url.host_str().context("missing endpoint host")?.to_owned();
    let ips: Vec<_> = tokio::net::lookup_host((
        host.as_str(),
        url.port_or_known_default().context("missing port")?,
    ))
    .await
    .map_err(|_| anyhow::anyhow!("endpoint DNS failed"))?
    .collect();
    ensure!(
        !ips.is_empty() && (a.synthetic_source || ips.iter().all(|v| !config::private_ip(v.ip()))),
        "private or unresolved endpoint refused"
    );
    let secret = Arc::new(config::credential(a.provider, &a.credentials, |k| {
        std::env::var(k).ok()
    })?);
    let listener = tokio::net::TcpListener::bind(a.metrics)
        .await
        .context("metrics port unavailable; no fallback")?;
    let out = a.output.clone().unwrap_or_else(|| {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
            .join("MEVanaData/captures")
    });
    std::fs::create_dir_all(&out)?;
    // Count earlier finalized runs and conservatively include incomplete file bytes.
    // The lock prevents two simultaneous sessions spending the same budget.
    let budget_lock = out.join("handoff09-budget.lock");
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&budget_lock)
        .context("capture budget locked; inspect prior session before removing stale lock")?;
    struct LockGuard(std::path::PathBuf, std::fs::File);
    impl Drop for LockGuard {
        fn drop(&mut self) {
            let _ = self.1.sync_all();
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = LockGuard(budget_lock, lock);
    let prior = prior_bytes(&out)?;
    ensure!(prior < 450_000_000, "slice byte budget exhausted");
    let max_bytes = a.max_bytes.min(450_000_000 - prior);
    let budget = source::Budget::new(max_bytes);
    let session_id = format!("capture-{}", format::now_ns());
    let dir = out.join(&session_id);
    let accounts = layout::request(&allowlist, false, commitment);
    let tx_request = layout::request(&allowlist, true, commitment);
    let tx_enabled = a.transactions && !allowlist.is_empty();
    let metadata = Metadata {
        format_version: if snapshot_enabled { 2 } else { 1 },
        session_id: session_id.clone(),
        provider: a.provider,
        endpoint_host: host.clone(),
        connection_tier: a.connection_tier.clone(),
        source: if a.synthetic_source {
            "Synthetic"
        } else {
            "MarketData"
        }
        .into(),
        observed_cluster: "unknown".into(),
        commitment: a.commitment.clone(),
        allowlist: allowlist.clone(),
        filters: BTreeMap::from([
            ("accounts".into(), format!("{accounts:?}")),
            (
                "transactions".into(),
                if tx_enabled {
                    format!("{tx_request:?}")
                } else {
                    "disabled".into()
                },
            ),
        ]),
        crate_version: env!("CARGO_PKG_VERSION").into(),
        git_commit: std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_else(|| "unknown".into()),
    };
    let mut writer = Writer::new(dir.clone(), metadata, a.rotation_seconds, a.rotation_bytes)?;
    println!(
        "session={} provider={:?} host={} ips={:?} duration={} transactions={} tier={} metrics={}",
        dir.display(),
        a.provider,
        host,
        ips,
        a.duration,
        tx_enabled,
        a.connection_tier,
        a.metrics
    );
    let (tx, mut rx) = mpsc::channel(128);
    let (snapshot_tx, mut snapshot_rx) =
        mpsc::channel::<snapshot::SnapshotRun>(snapshot::MAX_SNAPSHOTS as usize);
    let (stop, stop_rx) = watch::channel(false);
    let metrics = Arc::new(Mutex::new(String::new()));
    let metrics_clone = metrics.clone();
    let mut metrics_stop = stop_rx.clone();
    let metrics_task = tokio::spawn(async move {
        loop {
            tokio::select! {biased;_=metrics_stop.changed()=>break,result=listener.accept()=>{if let Ok((mut socket,_))=result {let body=metrics_clone.lock().expect("metrics lock").clone();let mut request=[0;1024];let _=tokio::time::timeout(Duration::from_secs(1),socket.read(&mut request)).await;let _=tokio::time::timeout(Duration::from_secs(1),socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes())).await;}}}
        }
    });
    let mut handles = vec![tokio::spawn(source::drive(
        GrpcSource::new(
            endpoint.clone(),
            a.provider,
            secret.clone(),
            a.synthetic_source,
            accounts,
        ),
        0,
        tx.clone(),
        stop_rx.clone(),
        Duration::from_secs(a.stall_seconds),
        a.max_reconnect_failures,
        budget.clone(),
    ))];
    if tx_enabled {
        handles.push(tokio::spawn(source::drive(
            GrpcSource::new(endpoint, a.provider, secret, a.synthetic_source, tx_request),
            1,
            tx.clone(),
            stop_rx,
            Duration::from_secs(a.stall_seconds),
            a.max_reconnect_failures,
            budget.clone(),
        )));
    }
    drop(tx);
    let started = Instant::now();
    let deadline = tokio::time::sleep(Duration::from_secs(a.duration));
    tokio::pin!(deadline);
    let signal = shutdown();
    tokio::pin!(signal);
    let mut stopping = false;
    let mut status = "duration";
    let mut fatal = None;
    let mut counts = BTreeMap::<String, u64>::new();
    let mut gaps = 0;
    let mut stalls = 0;
    let mut bytes_written = 0;
    let mut last_slot = 0;
    let mut last_message = Instant::now();
    let mut first_confirmed = false;
    let mut snapshot_active = false;
    let mut snapshot_pending = false;
    let mut snapshots_started = 0u8;
    let mut snapshot_failures = 0u8;
    let mut snapshot_calls = 0u64;
    let mut snapshot_rpc_bytes = 0u64;
    let mut source_closed = false;
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {biased;
            _=&mut deadline,if !stopping=>{stopping=true;let _=stop.send(true);},
            _=&mut signal,if !stopping=>{status="signal";stopping=true;let _=stop.send(true);},
            e=rx.recv(),if !source_closed=>{match e {
                Some(Event::Raw{connection,receive_ns,bytes,appended})=>{let summary_bytes=bytes.clone();bytes_written+=bytes.len() as u64+22;writer.write(connection,receive_ns,bytes,false)?;let _=appended.send(());let(kind,slot)=format::update_summary(&summary_bytes);*counts.entry(kind).or_default()+=1;if let Some(s)=slot{last_slot=s;}last_message=Instant::now();if snapshot_enabled && connection==0 && !first_confirmed && !stopping {
                    if let Ok(update)=SubscribeUpdate::decode(summary_bytes.as_slice()) {
                        if matches!(update.update_oneof,Some(UpdateOneof::Slot(s)) if s.status==1) {
                            first_confirmed=true;
                            if snapshots_started < snapshot::MAX_SNAPSHOTS {
                                snapshots_started+=1;snapshot_active=true;
                                launch_snapshot(snapshot_tx.clone(),allowlist.clone(),snapshot_key.as_ref().expect("key exists").clone(),snapshots_started,snapshot::MAX_RPC_BYTES-snapshot_rpc_bytes);
                            }
                        }
                    }
                }
                if writer.session.bytes_received>=max_bytes && !stopping{status="byte_budget";fatal=Some("byte_budget".to_owned());stopping=true;let _=stop.send(true);}},
                Some(Event::Gap{connection,receive_ns,gap})=>{gaps+=1;if gap.reason=="stall"{stalls+=1;}let reconnect=gap.reason=="reconnect" && connection==0;writer.write(connection,receive_ns,serde_json::to_vec(&gap)?,true)?;if snapshot_enabled && reconnect && first_confirmed && !stopping {if snapshot_active {snapshot_pending=true;} else if snapshots_started<snapshot::MAX_SNAPSHOTS {snapshots_started+=1;snapshot_active=true;launch_snapshot(snapshot_tx.clone(),allowlist.clone(),snapshot_key.as_ref().expect("key exists").clone(),snapshots_started,snapshot::MAX_RPC_BYTES-snapshot_rpc_bytes);}}},
                Some(Event::Fatal(e))=>{status="failed";fatal=Some(format!("{e:?}"));stopping=true;let _=stop.send(true);},
                None=>source_closed=true,
            }},
            result=snapshot_rx.recv(),if snapshot_active=>{
                let run=result.context("snapshot task disappeared")?;
                snapshot_calls+=run.calls;
                snapshot_rpc_bytes+=run.bytes;
                ensure!(snapshot_rpc_bytes<=snapshot::MAX_RPC_BYTES,"snapshot byte budget exceeded");
                for record in &run.records {writer.write_snapshot(format::now_ns(),record)?;}
                snapshot_active=false;
                if run.failed {snapshot_failures+=1;if snapshot_failures>=2 {status="snapshot_failed";fatal=Some("two snapshot failures".into());stopping=true;let _=stop.send(true);}}
                if snapshot_pending && !stopping && snapshots_started<snapshot::MAX_SNAPSHOTS {snapshot_pending=false;snapshots_started+=1;snapshot_active=true;launch_snapshot(snapshot_tx.clone(),allowlist.clone(),snapshot_key.as_ref().expect("key exists").clone(),snapshots_started,snapshot::MAX_RPC_BYTES-snapshot_rpc_bytes);}
            },
            _=ticker.tick()=>{let mut body=format!("geyser_capture_bytes_written {bytes_written}\ngeyser_capture_last_slot {last_slot}\ngeyser_capture_seconds_since_last_message {}\ngeyser_capture_gaps_total {gaps}\ngeyser_capture_stalls_total {stalls}\ngeyser_capture_current_file{{file=\"{}\"}} 1\n",last_message.elapsed().as_secs(),writer.manifest.file);for(k,v)in &counts{body.push_str(&format!("geyser_capture_updates_total{{type=\"{k}\"}} {v}\ngeyser_capture_updates_per_second{{type=\"{k}\"}} {}\n",*v as f64/started.elapsed().as_secs_f64()));}*metrics.lock().expect("metrics lock")=body;}
        }
        if source_closed && !snapshot_active {
            break;
        }
    }
    let _ = stop.send(true);
    for h in handles {
        h.await?;
    }
    metrics_task.await?;
    let session = writer.finish(status)?;
    println!("finished session={} bytes_received={} elapsed_seconds={:.3} status={} counts={} gaps={} stalls={} snapshots={} tatum_calls={} tatum_bytes={}",session_id,session.bytes_received,started.elapsed().as_secs_f64(),status,serde_json::to_string(&counts)?,gaps,stalls,snapshots_started,snapshot_calls,snapshot_rpc_bytes);
    if let Some(f) = fatal {
        anyhow::bail!("live source stopped: {f}; operator review required");
    }
    Ok(())
}

fn prior_bytes(out: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;
    for entry in std::fs::read_dir(out)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let dir = entry.path();
        if !entry.file_name().to_string_lossy().starts_with("capture-") {
            continue;
        }
        let session = dir.join("session.json");
        if session.exists() {
            let s: format::Session = serde_json::from_slice(&std::fs::read(session)?)?;
            total = total
                .checked_add(s.bytes_received)
                .context("budget overflow")?;
        } else {
            anyhow::bail!("incomplete previous capture: inspect before any new stream");
        }
        for file in std::fs::read_dir(&dir)? {
            let file = file?;
            ensure!(
                file.path().extension().is_none_or(|e| e != "part"),
                "incomplete previous capture blocks new streams"
            );
        }
    }
    Ok(total)
}
