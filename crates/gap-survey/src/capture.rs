//! Exact-account-only Confirmed Geyser recording, bounded by wall time and account payload bytes.
use crate::{validate_budget, MAX_COMBINED_BYTES};
use anyhow::{ensure, Context, Result};
use base64::Engine;
use chrono::Utc;
use geyser_capture::{
    config::{credential, Provider},
    source::{backoff, GrpcSource, Source},
};
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    sync::Arc,
    time::Duration,
};
use yellowstone_grpc_proto::prelude::{
    subscribe_update::UpdateOneof, SubscribeRequest, SubscribeRequestFilterAccounts,
    SubscribeUpdate,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolMeta {
    pub address: String,
    pub venue: String,
    pub pair: String,
    pub group: String,
    pub mint0: String,
    pub mint1: String,
    pub decimals0: u8,
    pub decimals1: u8,
    pub fee_bps_micro: Option<u64>,
    pub base_fee_bps_micro: Option<u64>,
    pub variable_fee_modeled: bool,
    pub owner: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Allowlist {
    pub version: u32,
    pub evidence_kind: String,
    pub pools: Vec<PoolMeta>,
}
impl Allowlist {
    pub fn load(path: &Path) -> Result<Self> {
        let a: Self = serde_json::from_slice(&fs::read(path)?)?;
        ensure!(
            a.version == 1 && a.evidence_kind == "Indicative" && (2..=40).contains(&a.pools.len()),
            "invalid allowlist"
        );
        let mut keys = BTreeSet::new();
        for p in &a.pools {
            ensure!(keys.insert(&p.address), "duplicate pool");
            ensure!(
                bs58::decode(&p.address)
                    .into_vec()
                    .is_ok_and(|v| v.len() == 32),
                "invalid pool key"
            );
            ensure!(
                p.owner == crate::decode::ORCA
                    || p.owner == crate::decode::RAYDIUM
                    || p.owner == crate::decode::METEORA,
                "invalid owner"
            );
            ensure!(p.decimals0 <= 18 && p.decimals1 <= 18, "invalid decimals");
        }
        Ok(a)
    }
    pub fn by_address(&self) -> BTreeMap<String, &PoolMeta> {
        self.pools.iter().map(|p| (p.address.clone(), p)).collect()
    }
}

pub fn request(allowlist: &Allowlist) -> SubscribeRequest {
    let mut r = SubscribeRequest {
        commitment: Some(1),
        ..Default::default()
    };
    r.accounts.insert(
        "allowlisted_pools".into(),
        SubscribeRequestFilterAccounts {
            account: allowlist.pools.iter().map(|p| p.address.clone()).collect(),
            owner: vec![],
            filters: vec![],
            nonempty_txn_signature: None,
        },
    );
    r
}

#[derive(Debug, Clone)]
pub struct ByteBudget {
    remaining: u64,
}
impl ByteBudget {
    pub fn new(max: u64) -> Result<Self> {
        ensure!(max > 0 && max <= MAX_COMBINED_BYTES, "invalid byte budget");
        Ok(Self { remaining: max })
    }
    pub fn admit(&mut self, n: u64) -> bool {
        if n > self.remaining {
            return false;
        }
        self.remaining -= n;
        true
    }
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecord {
    pub pubkey: String,
    pub owner: String,
    pub slot: u64,
    pub write_version: u64,
    pub provider_time_unix_ns: Option<i64>,
    pub raw_base64: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunManifest {
    pub evidence_kind: String,
    pub phase: String,
    pub run_id: String,
    pub provider: String,
    pub started_utc: String,
    pub stopped_utc: String,
    pub duration_seconds: u64,
    pub payload_bytes: u64,
    pub updates: u64,
    pub updates_by_venue: BTreeMap<String, u64>,
    pub gaps: u64,
    pub stalls: u64,
    pub stop_reason: String,
    pub allowlist_sha256: String,
    pub updates_sha256: String,
}

pub fn previous_runs(root: &Path) -> Result<Vec<RunManifest>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut runs = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("manifest.json");
        ensure!(path.exists(), "unclosed prior run directory");
        let m: RunManifest = serde_json::from_slice(&fs::read(path)?)?;
        runs.push(m);
    }
    Ok(runs)
}

pub struct CaptureArgs<'a> {
    pub allowlist: &'a Path,
    pub output_root: &'a Path,
    pub credentials: &'a Path,
    pub provider: Provider,
    pub duration_seconds: u64,
    pub max_bytes: u64,
    pub run_id: &'a str,
    pub phase: &'a str,
}

pub async fn record(args: CaptureArgs<'_>) -> Result<RunManifest> {
    validate_budget(args.duration_seconds, args.max_bytes)?;
    // End of 2026-11-04 in America/Chicago (CST), expressed as UTC epoch seconds.
    ensure!(
        Utc::now().timestamp() + args.duration_seconds as i64 <= 1_793_858_400,
        "live survey would pass the deadline"
    );
    ensure!(
        !args.run_id.is_empty()
            && args
                .run_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "invalid run id"
    );
    let allowlist = Allowlist::load(args.allowlist)?;
    let runs = previous_runs(args.output_root)?;
    let pilots = runs.iter().filter(|r| r.phase == "pilot").count();
    let mains = runs.iter().filter(|r| r.phase == "main").count();
    match args.phase {
        "pilot" => ensure!(
            args.duration_seconds == 300 && pilots == 0 && mains == 0,
            "pilot must be the sole 300-second first run"
        ),
        "main" => ensure!(
            args.duration_seconds <= 1800 && pilots == 1 && mains < 6,
            "main run requires one pilot and permits at most six 30-minute runs"
        ),
        _ => anyhow::bail!("phase must be pilot or main"),
    }
    let used = runs.iter().try_fold(0u64, |sum, r| {
        sum.checked_add(r.payload_bytes)
            .context("total byte overflow")
    })?;
    ensure!(
        used.checked_add(args.max_bytes)
            .is_some_and(|n| n <= MAX_COMBINED_BYTES),
        "combined byte budget would exceed 1.5 GB"
    );
    let run_dir = args.output_root.join(args.run_id);
    fs::create_dir_all(args.output_root)?;
    fs::create_dir(&run_dir).context("run directory already exists")?;
    let result = record_inner(&args, &allowlist, &run_dir).await;
    // A failed partial directory remains as an explicit fail-closed marker.
    result
}

async fn record_inner(
    args: &CaptureArgs<'_>,
    allowlist: &Allowlist,
    run_dir: &Path,
) -> Result<RunManifest> {
    let secret = Arc::new(credential(args.provider, args.credentials, |name| {
        std::env::var(name).ok()
    })?);
    let endpoint = args.provider.endpoint();
    let provider = match args.provider {
        Provider::Sodae => "sodae",
        Provider::Tatum => "tatum",
    };
    let mut source = GrpcSource::new(endpoint, args.provider, secret, false, request(allowlist));
    let file = File::create_new(run_dir.join("updates.jsonl"))?;
    let mut writer = BufWriter::new(file);
    let mut hasher = Sha256::new();
    let start = Utc::now();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(args.duration_seconds);
    let mut budget = ByteBudget::new(args.max_bytes)?;
    let mut updates = 0u64;
    let mut counts = BTreeMap::<String, u64>::new();
    let mut gaps = 0;
    let mut stalls = 0;
    let mut failures = 0u32;
    let index = allowlist.by_address();
    let stop_reason = 'outer: loop {
        let connected = tokio::select! {biased;_ = tokio::time::sleep_until(deadline)=>break 'outer "duration",r=source.connect()=>r};
        if let Err(e) = connected {
            gaps += 1;
            failures += 1;
            if e.fatal() || failures >= 5 {
                break 'outer "connect_failure";
            }
        } else {
            loop {
                let next = tokio::select! {biased;_=tokio::time::sleep_until(deadline)=>break 'outer "duration",r=tokio::time::timeout(Duration::from_secs(30),source.next())=>r};
                match next {
                    Ok(Ok(Some(raw))) => {
                        failures = 0;
                        let update = SubscribeUpdate::decode(raw.as_slice())
                            .context("invalid Geyser protobuf")?;
                        match update.update_oneof {
                            Some(UpdateOneof::Account(a)) => {
                                let info = a.account.context("account update without account")?;
                                let pubkey = bs58::encode(&info.pubkey).into_string();
                                let meta =
                                    index.get(&pubkey).context("unallowlisted pool update")?;
                                ensure!(
                                    bs58::encode(&info.owner).into_string() == meta.owner,
                                    "pool owner changed"
                                );
                                if !budget.admit(raw.len() as u64) {
                                    break 'outer "max_bytes";
                                }
                                let timestamp = update.created_at.and_then(|t| {
                                    t.seconds
                                        .checked_mul(1_000_000_000)?
                                        .checked_add(i64::from(t.nanos))
                                });
                                let rec = UpdateRecord {
                                    pubkey,
                                    owner: meta.owner.clone(),
                                    slot: a.slot,
                                    write_version: info.write_version,
                                    provider_time_unix_ns: timestamp,
                                    raw_base64: base64::engine::general_purpose::STANDARD
                                        .encode(raw),
                                };
                                let mut line = serde_json::to_vec(&rec)?;
                                line.push(b'\n');
                                writer.write_all(&line)?;
                                hasher.update(&line);
                                updates += 1;
                                *counts.entry(meta.venue.clone()).or_default() += 1;
                                if budget.remaining() == 0 {
                                    break 'outer "max_bytes";
                                }
                            }
                            Some(UpdateOneof::Ping(_)) => {
                                if source.ping().await.is_err() {
                                    gaps += 1;
                                    break;
                                }
                            }
                            Some(UpdateOneof::Transaction(_)) => {
                                anyhow::bail!("unexpected transaction update")
                            }
                            Some(UpdateOneof::Slot(_)) => anyhow::bail!("unexpected slot update"),
                            _ => {}
                        }
                    }
                    Ok(Ok(None)) => {
                        gaps += 1;
                        failures += 1;
                        break;
                    }
                    Ok(Err(e)) => {
                        gaps += 1;
                        failures += 1;
                        if e.fatal() {
                            break 'outer "provider_failure";
                        }
                        break;
                    }
                    Err(_) => {
                        stalls += 1;
                        gaps += 1;
                        failures += 1;
                        break;
                    }
                }
            }
            if failures >= 5 {
                break 'outer "reconnect_limit";
            }
        }
        let delay = backoff(failures, 0);
        tokio::select! {biased;_=tokio::time::sleep_until(deadline)=>break 'outer "duration",_ = tokio::time::sleep(delay)=>{}}
    };
    writer.flush()?;
    writer.get_ref().sync_all()?;
    let manifest = RunManifest {
        evidence_kind: "Indicative".into(),
        phase: args.phase.into(),
        run_id: args.run_id.into(),
        provider: provider.into(),
        started_utc: start.to_rfc3339(),
        stopped_utc: Utc::now().to_rfc3339(),
        duration_seconds: args.duration_seconds,
        payload_bytes: args.max_bytes - budget.remaining(),
        updates,
        updates_by_venue: counts,
        gaps,
        stalls,
        stop_reason: stop_reason.into(),
        allowlist_sha256: format!("{:x}", Sha256::digest(fs::read(args.allowlist)?)),
        updates_sha256: format!("{:x}", hasher.finalize()),
    };
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(run_dir.join("manifest.json"))?;
    serde_json::to_writer_pretty(&mut f, &manifest)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_account_request_has_no_other_filters() {
        let a = Allowlist::load(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/allowlist.json"
        )))
        .unwrap();
        let r = request(&a);
        assert_eq!(r.commitment, Some(1));
        assert!(r.transactions.is_empty() && r.slots.is_empty() && r.blocks.is_empty());
        assert_eq!(r.accounts.len(), 1);
        let f = &r.accounts["allowlisted_pools"];
        assert_eq!(f.account.len(), 40);
        assert!(f.owner.is_empty() && f.filters.is_empty());
    }
    #[test]
    fn budget_admits_no_overrun() {
        let mut b = ByteBudget::new(10).unwrap();
        assert!(b.admit(9));
        assert!(!b.admit(2));
        assert_eq!(b.remaining(), 1);
        assert!(b.admit(1));
        assert_eq!(b.remaining(), 0)
    }
    #[test]
    fn mocked_updates_stop_at_byte_and_time_bounds() {
        let updates = [(0u64, 4u64), (1, 4), (2, 4), (3, 4)];
        let mut budget = ByteBudget::new(10).unwrap();
        let mut accepted = 0;
        for (elapsed_seconds, payload_bytes) in updates {
            if elapsed_seconds >= 3 || !budget.admit(payload_bytes) {
                break;
            }
            accepted += 1;
        }
        assert_eq!(accepted, 2);
        assert_eq!(budget.remaining(), 2);

        let mut budget = ByteBudget::new(100).unwrap();
        let mut accepted = 0;
        for (elapsed_seconds, payload_bytes) in updates {
            if elapsed_seconds >= 2 || !budget.admit(payload_bytes) {
                break;
            }
            accepted += 1;
        }
        assert_eq!(accepted, 2);
    }
}
