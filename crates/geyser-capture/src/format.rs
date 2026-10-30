use anyhow::{ensure, Context, Result};
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};
use yellowstone_grpc_proto::prelude::{subscribe_update::UpdateOneof, SubscribeUpdate};

pub const MAGIC: &[u8; 8] = b"MEVCAP01";
pub const MAX_FRAME: usize = 16 * 1024 * 1024;
fn is_zero(value: &u64) -> bool {
    *value == 0
}
pub fn now_ns() -> u64 {
    chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("UTC nanosecond range") as u64
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gap {
    pub connection: String,
    pub reason: String,
    pub previous_slot: Option<u64>,
    pub next_slot: Option<u64>,
}
#[derive(Debug, Clone)]
pub struct Record {
    pub sequence: u64,
    pub receive_ns: u64,
    pub connection: u8,
    pub gap: bool,
    pub snapshot: bool,
    pub payload: Vec<u8>,
}
impl Record {
    pub fn encode(&self) -> Result<Vec<u8>> {
        ensure!(self.payload.len() + 18 <= MAX_FRAME, "frame too large");
        let mut v = Vec::with_capacity(self.payload.len() + 22);
        v.extend_from_slice(&((self.payload.len() + 18) as u32).to_le_bytes());
        v.push(if self.snapshot { 2 } else { u8::from(self.gap) });
        v.push(self.connection);
        v.extend_from_slice(&self.sequence.to_le_bytes());
        v.extend_from_slice(&self.receive_ns.to_le_bytes());
        v.extend_from_slice(&self.payload);
        Ok(v)
    }
}
pub fn read_record(r: &mut impl Read) -> Result<Option<Record>> {
    let mut len = [0; 4];
    match r.read(&mut len[..1])? {
        0 => return Ok(None),
        1 => r.read_exact(&mut len[1..])?,
        _ => unreachable!(),
    }
    let n = u32::from_le_bytes(len) as usize;
    ensure!((18..=MAX_FRAME).contains(&n), "invalid frame length");
    let mut b = vec![0; n];
    r.read_exact(&mut b).context("truncated frame")?;
    ensure!(b[0] <= 2 && b[1] <= 1, "invalid frame kind/connection");
    Ok(Some(Record {
        gap: b[0] == 1,
        snapshot: b[0] == 2,
        connection: b[1],
        sequence: u64::from_le_bytes(b[2..10].try_into()?),
        receive_ns: u64::from_le_bytes(b[10..18].try_into()?),
        payload: b[18..].to_vec(),
    }))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub format_version: u32,
    pub session_id: String,
    pub provider: crate::config::Provider,
    pub endpoint_host: String,
    pub connection_tier: String,
    pub source: String,
    pub observed_cluster: String,
    pub commitment: String,
    pub allowlist: Vec<String>,
    pub filters: BTreeMap<String, String>,
    pub crate_version: String,
    pub git_commit: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(flatten)]
    pub metadata: Metadata,
    pub file: String,
    pub sha256: String,
    pub bytes: u64,
    pub bytes_received: u64,
    pub frames: u64,
    pub first_sequence: Option<u64>,
    pub last_sequence: Option<u64>,
    pub first_receive_ns: Option<u64>,
    pub last_receive_ns: Option<u64>,
    pub first_slot: Option<u64>,
    pub last_slot: Option<u64>,
    pub counts: BTreeMap<String, u64>,
    pub gaps: Vec<Gap>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub snapshot_frames: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub snapshot_present: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub snapshot_absent: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub snapshot_calls: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub snapshot_rpc_bytes: u64,
}

/// Kind-2 payload is JSON encoded with this tagged v1 schema. Account data is
/// base64 of raw account bytes; no URL, header or credential field is permitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SnapshotRecord {
    Account {
        version: u8,
        snapshot_id: u8,
        account_kind: String,
        pool_id: String,
        key: String,
        context_slot: u64,
        present: bool,
        owner: Option<String>,
        lamports: Option<u64>,
        data_base64: Option<String>,
        sha256: Option<String>,
    },
    Stats {
        version: u8,
        snapshot_id: u8,
        calls: u64,
        rpc_bytes: u64,
    },
    Failed {
        version: u8,
        snapshot_id: u8,
        reason: String,
    },
}
impl SnapshotRecord {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Account {
                version,
                account_kind,
                pool_id,
                key,
                present,
                owner,
                lamports,
                data_base64,
                sha256,
                ..
            } => {
                ensure!(
                    *version == 1 && matches!(account_kind.as_str(), "pool" | "tick_array"),
                    "invalid snapshot account version/kind"
                );
                ensure!(
                    bs58::decode(pool_id)
                        .into_vec()
                        .is_ok_and(|v| v.len() == 32)
                        && bs58::decode(key).into_vec().is_ok_and(|v| v.len() == 32),
                    "invalid snapshot public key"
                );
                if *present {
                    let raw = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        data_base64.as_deref().context("missing snapshot data")?,
                    )?;
                    ensure!(
                        owner.as_deref() == Some(crate::snapshot::WHIRLPOOL_PROGRAM),
                        "wrong snapshot owner"
                    );
                    ensure!(
                        lamports.is_some()
                            && sha256.as_deref() == Some(&format!("{:x}", Sha256::digest(&raw))),
                        "invalid snapshot account hash"
                    );
                    if account_kind == "pool" {
                        ensure!(key == pool_id, "snapshot pool key mismatch");
                        whirlpool_core::decode::decode_whirlpool(&raw)?;
                    } else {
                        let array = whirlpool_core::exact::decode_array(&raw)?;
                        ensure!(
                            array.whirlpool.to_string() == pool_id.as_str(),
                            "snapshot array pool mismatch"
                        );
                    }
                } else {
                    ensure!(
                        owner.is_none()
                            && lamports.is_none()
                            && data_base64.is_none()
                            && sha256.is_none(),
                        "absent snapshot has account data"
                    );
                }
            }
            Self::Stats { version, .. } | Self::Failed { version, .. } => {
                ensure!(*version == 1, "unknown snapshot version")
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    #[serde(flatten)]
    pub metadata: Metadata,
    pub files: Vec<String>,
    pub bytes_received: u64,
    pub started_ns: u64,
    pub stopped_ns: u64,
    pub status: String,
}
pub fn atomic_json(path: &Path, v: &impl Serialize) -> Result<()> {
    let part = path.with_extension("json.part");
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&part)?;
    serde_json::to_writer_pretty(&mut f, v)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    std::fs::rename(part, path)?;
    File::open(path.parent().context("no parent")?)?.sync_all()?;
    Ok(())
}
pub fn hash_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
pub fn update_summary(raw: &[u8]) -> (String, Option<u64>) {
    let Ok(u) = SubscribeUpdate::decode(raw) else {
        return ("invalid_protobuf".into(), None);
    };
    match u.update_oneof {
        Some(UpdateOneof::Account(a)) => (
            a.account.map_or("account_missing".into(), |a| {
                crate::layout::kind(&a.data).into()
            }),
            Some(a.slot),
        ),
        Some(UpdateOneof::Slot(s)) => ("slot".into(), Some(s.slot)),
        Some(UpdateOneof::Transaction(t)) => ("transaction".into(), Some(t.slot)),
        Some(UpdateOneof::Ping(_)) => ("ping".into(), None),
        Some(UpdateOneof::Pong(_)) => ("pong".into(), None),
        _ => ("other".into(), None),
    }
}
pub struct Writer {
    pub directory: PathBuf,
    pub session: Session,
    pub manifest: Manifest,
    pub sequence: u64,
    file: Option<File>,
    index: u64,
    opened: Instant,
    rotation_seconds: u64,
    rotation_bytes: u64,
    known_mints: std::collections::BTreeSet<String>,
}
impl Writer {
    pub fn new(
        directory: PathBuf,
        metadata: Metadata,
        rotation_seconds: u64,
        rotation_bytes: u64,
    ) -> Result<Self> {
        std::fs::create_dir(&directory).context("session directory must be new")?;
        let manifest = Self::empty(metadata.clone(), 0);
        let mut w = Self {
            directory,
            session: Session {
                metadata,
                files: vec![],
                bytes_received: 0,
                started_ns: now_ns(),
                stopped_ns: 0,
                status: "running".into(),
            },
            manifest,
            sequence: 0,
            file: None,
            index: 0,
            opened: Instant::now(),
            rotation_seconds,
            rotation_bytes,
            known_mints: Default::default(),
        };
        w.open()?;
        Ok(w)
    }
    fn empty(metadata: Metadata, index: u64) -> Manifest {
        Manifest {
            metadata,
            file: format!("{index:06}.cap"),
            sha256: String::new(),
            bytes: 8,
            bytes_received: 0,
            frames: 0,
            first_sequence: None,
            last_sequence: None,
            first_receive_ns: None,
            last_receive_ns: None,
            first_slot: None,
            last_slot: None,
            counts: BTreeMap::new(),
            gaps: vec![],
            snapshot_frames: 0,
            snapshot_present: 0,
            snapshot_absent: 0,
            snapshot_calls: 0,
            snapshot_rpc_bytes: 0,
        }
    }
    fn part(&self) -> PathBuf {
        self.directory.join(format!("{}.part", self.manifest.file))
    }
    fn open(&mut self) -> Result<()> {
        self.manifest = Self::empty(self.session.metadata.clone(), self.index);
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.part())?;
        f.write_all(MAGIC)?;
        self.file = Some(f);
        self.opened = Instant::now();
        Ok(())
    }
    pub fn write(
        &mut self,
        connection: u8,
        receive_ns: u64,
        payload: Vec<u8>,
        gap: bool,
    ) -> Result<()> {
        self.write_inner(connection, receive_ns, payload, gap, false)
    }
    pub fn write_snapshot(&mut self, receive_ns: u64, record: &SnapshotRecord) -> Result<()> {
        ensure!(
            self.session.metadata.format_version == 2,
            "snapshot requires format v2"
        );
        record.validate()?;
        self.write_inner(0, receive_ns, serde_json::to_vec(record)?, false, true)
    }
    fn write_inner(
        &mut self,
        connection: u8,
        receive_ns: u64,
        payload: Vec<u8>,
        gap: bool,
        snapshot: bool,
    ) -> Result<()> {
        if self.manifest.frames > 0
            && (self.manifest.bytes + payload.len() as u64 + 22 > self.rotation_bytes
                || self.opened.elapsed().as_secs() >= self.rotation_seconds)
        {
            self.finalize()?;
            self.index += 1;
            self.open()?;
        }
        self.sequence += 1;
        let rec = Record {
            sequence: self.sequence,
            receive_ns,
            connection,
            gap,
            snapshot,
            payload,
        };
        let encoded = rec.encode()?;
        self.file
            .as_mut()
            .context("writer finalized")?
            .write_all(&encoded)?;
        // Summaries are derived only after the raw payload has been appended, never used to filter ingestion.
        self.manifest.bytes += encoded.len() as u64;
        self.manifest.frames += 1;
        self.manifest.first_sequence.get_or_insert(rec.sequence);
        self.manifest.last_sequence = Some(rec.sequence);
        self.manifest.first_receive_ns.get_or_insert(receive_ns);
        self.manifest.last_receive_ns = Some(receive_ns);
        if gap {
            self.manifest
                .gaps
                .push(serde_json::from_slice(&rec.payload)?);
        } else if snapshot {
            self.manifest.snapshot_frames += 1;
            match serde_json::from_slice::<SnapshotRecord>(&rec.payload)? {
                SnapshotRecord::Account { present, .. } => {
                    if present {
                        self.manifest.snapshot_present += 1;
                    } else {
                        self.manifest.snapshot_absent += 1;
                    }
                }
                SnapshotRecord::Stats {
                    calls, rpc_bytes, ..
                } => {
                    self.manifest.snapshot_calls += calls;
                    self.manifest.snapshot_rpc_bytes += rpc_bytes;
                }
                SnapshotRecord::Failed { .. } => {}
            }
        } else {
            self.manifest.bytes_received += rec.payload.len() as u64;
            self.session.bytes_received += rec.payload.len() as u64;
            let (kind, slot) = update_summary(&rec.payload);
            *self.manifest.counts.entry(kind).or_default() += 1;
            if let Some(slot) = slot {
                self.manifest.first_slot =
                    Some(self.manifest.first_slot.map_or(slot, |s| s.min(slot)));
                self.manifest.last_slot =
                    Some(self.manifest.last_slot.map_or(slot, |s| s.max(slot)));
            }
            if let Ok(u) = SubscribeUpdate::decode(rec.payload.as_slice()) {
                if let Some(UpdateOneof::Account(a)) = u.update_oneof {
                    if let Some(a) = a.account {
                        if let Ok(pool) = whirlpool_core::decode::decode_whirlpool(&a.data) {
                            for mint in
                                [pool.token_mint_a.to_string(), pool.token_mint_b.to_string()]
                            {
                                if crate::inspect::MINTS.contains(&mint.as_str()) {
                                    self.known_mints.insert(mint);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn finalize(&mut self) -> Result<()> {
        if let Some(f) = self.file.take() {
            f.sync_all()?;
            drop(f);
            let path = self.directory.join(&self.manifest.file);
            std::fs::rename(self.part(), &path)?;
            File::open(&self.directory)?.sync_all()?;
            self.manifest.sha256 = hash_file(&path)?;
            let name = format!("{}.json", self.manifest.file);
            atomic_json(&self.directory.join(&name), &self.manifest)?;
            self.session.files.push(name);
        }
        Ok(())
    }
    pub fn finish(mut self, status: &str) -> Result<Session> {
        self.finalize()?;
        if self.session.metadata.source != "Synthetic" && self.known_mints.len() == 3 {
            self.session.metadata.observed_cluster = "mainnet-beta (known mint inventory)".into();
        }
        for f in &self.session.files {
            let path = self.directory.join(f);
            let mut m: Manifest = serde_json::from_slice(&std::fs::read(&path)?)?;
            m.metadata.observed_cluster = self.session.metadata.observed_cluster.clone();
            atomic_json(&path, &m)?;
        }
        self.session.status = status.into();
        self.session.stopped_ns = now_ns();
        atomic_json(&self.directory.join("session.json"), &self.session)?;
        Ok(self.session)
    }
}
