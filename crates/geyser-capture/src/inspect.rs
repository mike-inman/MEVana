use crate::format::{hash_file, read_record, Manifest, Session, SnapshotRecord, MAGIC};
use anyhow::{ensure, Context, Result};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};
use yellowstone_grpc_proto::prelude::{subscribe_update::UpdateOneof, SubscribeUpdate};

pub const MINTS: [&str; 3] = [
    "So11111111111111111111111111111111111111112",
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
];
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Pool {
    pub address: String,
    pub mint_a: String,
    pub mint_b: String,
    pub tick_spacing: u16,
    pub fee_rate: u16,
    pub updates: u64,
    pub slot: u64,
}
#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub files: u64,
    pub bytes_received: u64,
    pub snapshot_frames: u64,
    pub snapshot_present: u64,
    pub snapshot_absent: u64,
    pub snapshot_calls: u64,
    pub snapshot_rpc_bytes: u64,
    pub counts: BTreeMap<String, u64>,
    pub decode: BTreeMap<String, u64>,
    pub unexpected: BTreeMap<String, u64>,
    pub gaps: u64,
    pub stalls: u64,
    pub incomplete: Vec<String>,
    pub pools: BTreeMap<String, Pool>,
}
pub fn inspect(directory: &Path) -> Result<Report> {
    let mut report = Report::default();
    for f in std::fs::read_dir(directory)? {
        let f = f?;
        if f.path().extension().is_some_and(|e| e == "part") {
            report
                .incomplete
                .push(f.file_name().to_string_lossy().into_owned());
        }
    }
    let session: Session = serde_json::from_slice(
        &std::fs::read(directory.join("session.json"))
            .context("missing session manifest (incomplete capture)")?,
    )?;
    let mut previous = 0;
    let mut total = 0;
    for name in &session.files {
        ensure!(
            Path::new(name).components().count() == 1,
            "invalid manifest filename"
        );
        let m: Manifest = serde_json::from_slice(&std::fs::read(directory.join(name))?)?;
        ensure!(
            Path::new(&m.file).components().count() == 1,
            "invalid capture filename"
        );
        let path = directory.join(&m.file);
        ensure!(hash_file(&path)? == m.sha256, "capture hash mismatch");
        ensure!(
            std::fs::metadata(&path)?.len() == m.bytes,
            "capture byte count mismatch"
        );
        ensure!(
            m.metadata.session_id == session.metadata.session_id
                && m.metadata.format_version == session.metadata.format_version
                && matches!(m.metadata.format_version, 1 | 2),
            "manifest identity/version mismatch"
        );
        let mut r = BufReader::new(File::open(path)?);
        let mut magic = [0; 8];
        r.read_exact(&mut magic)?;
        ensure!(&magic == MAGIC, "wrong format magic");
        let mut frames = 0;
        let mut bytes = 0;
        let mut gaps = 0;
        let mut first = None;
        let mut last = None;
        let mut counts = BTreeMap::<String, u64>::new();
        let mut snapshot_frames = 0;
        let mut snapshot_present = 0;
        let mut snapshot_absent = 0;
        let mut snapshot_calls = 0;
        let mut snapshot_rpc_bytes = 0;
        while let Some(rec) = read_record(&mut r)? {
            ensure!(rec.sequence == previous + 1, "non-monotonic sequence");
            previous = rec.sequence;
            frames += 1;
            first.get_or_insert(rec.sequence);
            last = Some(rec.sequence);
            if rec.gap {
                let g: crate::format::Gap = serde_json::from_slice(&rec.payload)?;
                gaps += 1;
                report.gaps += 1;
                if g.reason == "stall" {
                    report.stalls += 1;
                }
                continue;
            }
            if rec.snapshot {
                ensure!(m.metadata.format_version == 2, "kind 2 in v1 capture");
                let record: SnapshotRecord = serde_json::from_slice(&rec.payload)?;
                record.validate()?;
                snapshot_frames += 1;
                match record {
                    SnapshotRecord::Account { present, .. } => {
                        if present {
                            snapshot_present += 1;
                        } else {
                            snapshot_absent += 1;
                        }
                    }
                    SnapshotRecord::Stats {
                        calls, rpc_bytes, ..
                    } => {
                        snapshot_calls += calls;
                        snapshot_rpc_bytes += rpc_bytes;
                    }
                    SnapshotRecord::Failed { .. } => {}
                }
                continue;
            }
            bytes += rec.payload.len() as u64;
            let (kind, _) = crate::format::update_summary(&rec.payload);
            *counts.entry(kind.clone()).or_default() += 1;
            *report.counts.entry(kind).or_default() += 1;
            let u = SubscribeUpdate::decode(rec.payload.as_slice())
                .context("invalid captured protobuf")?;
            if let Some(UpdateOneof::Account(a)) = u.update_oneof {
                let info = a.account.context("account update missing info")?;
                let kind = crate::layout::kind(&info.data);
                let valid = match kind {
                    "pool" => match whirlpool_core::decode::decode_whirlpool(&info.data) {
                        Ok(p) if info.data.len() == crate::layout::POOL_SIZE => {
                            let address = bs58::encode(&info.pubkey).into_string();
                            let row = report.pools.entry(address.clone()).or_default();
                            row.address = address;
                            row.mint_a = p.token_mint_a.to_string();
                            row.mint_b = p.token_mint_b.to_string();
                            row.tick_spacing = p.tick_spacing;
                            row.fee_rate = p.fee_rate;
                            row.updates += 1;
                            row.slot = a.slot;
                            true
                        }
                        _ => false,
                    },
                    "fixed" => whirlpool_core::decode::decode_tick_array(&info.data).is_ok(),
                    "dynamic" => {
                        *report
                            .decode
                            .entry("dynamic_unsupported".into())
                            .or_default() += 1;
                        let len = info.data.len();
                        if !(crate::layout::DYNAMIC_MIN..=crate::layout::DYNAMIC_MIN + 88 * 112)
                            .contains(&len)
                            || !(len - crate::layout::DYNAMIC_MIN).is_multiple_of(112)
                        {
                            *report
                                .unexpected
                                .entry(format!("dynamic_length_{len}"))
                                .or_default() += 1;
                        }
                        continue;
                    }
                    _ => {
                        *report
                            .unexpected
                            .entry(format!(
                                "{kind}_{}_{}",
                                info.data.len(),
                                info.data
                                    .iter()
                                    .take(8)
                                    .map(|b| format!("{b:02x}"))
                                    .collect::<String>()
                            ))
                            .or_default() += 1;
                        false
                    }
                };
                *report
                    .decode
                    .entry(format!(
                        "{kind}_{}",
                        if valid { "success" } else { "failure" }
                    ))
                    .or_default() += 1;
            }
        }
        ensure!(
            frames == m.frames
                && bytes == m.bytes_received
                && gaps == m.gaps.len() as u64
                && first == m.first_sequence
                && last == m.last_sequence
                && counts == m.counts
                && snapshot_frames == m.snapshot_frames
                && snapshot_present == m.snapshot_present
                && snapshot_absent == m.snapshot_absent
                && snapshot_calls == m.snapshot_calls
                && snapshot_rpc_bytes == m.snapshot_rpc_bytes,
            "manifest summary mismatch"
        );
        report.files += 1;
        report.bytes_received += bytes;
        report.snapshot_frames += snapshot_frames;
        report.snapshot_present += snapshot_present;
        report.snapshot_absent += snapshot_absent;
        report.snapshot_calls += snapshot_calls;
        report.snapshot_rpc_bytes += snapshot_rpc_bytes;
        total += bytes;
    }
    ensure!(total == session.bytes_received, "session bytes mismatch");
    let listed: std::collections::BTreeSet<_> = session
        .files
        .iter()
        .map(|n| n.trim_end_matches(".json").to_owned())
        .collect();
    for f in std::fs::read_dir(directory)? {
        let f = f?;
        if f.path().extension().is_some_and(|e| e == "cap") {
            ensure!(
                listed.contains(f.file_name().to_string_lossy().as_ref()),
                "unlisted finalized capture"
            );
        }
    }
    Ok(report)
}
