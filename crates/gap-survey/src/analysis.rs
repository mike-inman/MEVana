//! Deterministic, slot-ordered analysis of Indicative mid prices.
use crate::{
    capture::{Allowlist, PoolMeta, RunManifest, UpdateRecord},
    decode::{decode_pool, PoolPrice},
};
use anyhow::{ensure, Context, Result};
use base64::Engine;
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{BufRead, BufReader},
    path::Path,
};
use yellowstone_grpc_proto::prelude::{subscribe_update::UpdateOneof, SubscribeUpdate};

const THRESHOLDS: [i64; 4] = [5, 10, 25, 50];
const MAX_AGE_SLOTS: u64 = 25;

#[derive(Clone)]
struct State {
    slot: u64,
    price: BigUint,
    fee: Option<u64>,
}
#[derive(Clone)]
struct Update {
    slot: u64,
    write_version: u64,
    pubkey: String,
    provider_ns: Option<i64>,
    data: Vec<u8>,
}
#[derive(Default)]
struct MinuteAggregate {
    gross: Vec<u64>,
    net: Vec<i64>,
    stale: u64,
}
#[derive(Clone)]
struct OpenEvent {
    start: u64,
    start_time_unix_ns: Option<i64>,
    last: u64,
    peak: i64,
    volatility: Option<String>,
    after_sharp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinuteStat {
    pub evidence_kind: String,
    pub run_id: String,
    pub minute_unix: i64,
    pub pair: String,
    pub group: String,
    pub pool_a: String,
    pub pool_b: String,
    pub venue_a: String,
    pub venue_b: String,
    pub observation_count: u64,
    pub known_fee_observation_count: u64,
    pub median_gross_bps_micro: Option<u64>,
    pub maximum_gross_bps_micro: Option<u64>,
    pub median_net_bps_micro: Option<i64>,
    pub maximum_net_bps_micro: Option<i64>,
    pub excluded_stale_count: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GapEvent {
    pub evidence_kind: String,
    pub run_id: String,
    pub pair: String,
    pub group: String,
    pub pool_a: String,
    pub pool_b: String,
    pub venue_a: String,
    pub venue_b: String,
    pub threshold_bps: i64,
    pub start_slot: u64,
    pub start_time_unix_ns: Option<i64>,
    pub duration_slots: u64,
    pub peak_net_bps_micro: i64,
    pub volatility_bucket: Option<String>,
    pub after_sharp_move: bool,
    pub open_at_end: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentStat {
    pub evidence_kind: String,
    pub run_id: String,
    pub pair: String,
    pub venue_combination: String,
    pub observation_count: u64,
    pub stale_count: u64,
    pub known_fee_count: u64,
    pub events_10bps: u64,
    pub event_duration_slots: Vec<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketStat {
    pub evidence_kind: String,
    pub run_id: String,
    pub pair: String,
    pub group: String,
    pub venue_combination: String,
    pub volatility_bucket: Option<String>,
    pub after_sharp_move: bool,
    pub observations: u64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub evidence_kind: String,
    pub run_id: String,
    pub provider: String,
    pub window_start_utc: String,
    pub window_end_utc: String,
    pub duration_seconds: u64,
    pub payload_bytes: u64,
    pub updates: u64,
    pub gaps: u64,
    pub stalls: u64,
    pub stop_reason: String,
    pub allowlist_sha256: String,
    pub updates_sha256: String,
    pub updates_by_venue: BTreeMap<String, u64>,
    pub minutes: Vec<MinuteStat>,
    pub events: Vec<GapEvent>,
    pub segments: Vec<SegmentStat>,
    pub buckets: Vec<BucketStat>,
}

pub fn gross_gap_bps_micro(a: &BigUint, b: &BigUint) -> Result<u64> {
    ensure!(
        a > &BigUint::from(0u8) && b > &BigUint::from(0u8),
        "zero price"
    );
    let delta = if a >= b { a - b } else { b - a };
    (delta * BigUint::from(20_000_000_000u64) / (a + b))
        .to_u64()
        .context("gap overflow")
}
pub fn net_gap_bps_micro(
    gross: u64,
    fee_a: Option<u64>,
    fee_b: Option<u64>,
) -> Result<Option<i64>> {
    match (fee_a, fee_b) {
        (Some(a), Some(b)) => Ok(Some(
            i64::try_from(gross)? - i64::try_from(a)? - i64::try_from(b)?,
        )),
        _ => Ok(None),
    }
}
pub fn stale(current: u64, older: u64) -> bool {
    current.saturating_sub(older) > MAX_AGE_SLOTS
}
pub fn volatility_bucket(move_bps_micro: Option<u64>) -> Option<String> {
    move_bps_micro.map(|m| {
        if m < 10_000_000 {
            "below_10_bps"
        } else if m <= 50_000_000 {
            "10_to_50_bps"
        } else {
            "above_50_bps"
        }
        .into()
    })
}
pub fn reaches_threshold(net: Option<i64>, threshold_bps: i64) -> bool {
    net.is_some_and(|n| n >= threshold_bps * 1_000_000)
}
fn median_u64(v: &mut [u64]) -> Option<u64> {
    if v.is_empty() {
        None
    } else {
        v.sort_unstable();
        Some(v[(v.len() - 1) / 2])
    }
}
fn median_i64(v: &mut [i64]) -> Option<i64> {
    if v.is_empty() {
        None
    } else {
        v.sort_unstable();
        Some(v[(v.len() - 1) / 2])
    }
}
fn pair_key(a: &str, b: &str) -> (String, String) {
    if a < b {
        (a.into(), b.into())
    } else {
        (b.into(), a.into())
    }
}
fn venue_combination(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a}/{b}")
    } else {
        format!("{b}/{a}")
    }
}
fn close_event(
    events: &mut Vec<GapEvent>,
    run_id: &str,
    a: &PoolMeta,
    b: &PoolMeta,
    threshold: i64,
    e: OpenEvent,
    open: bool,
) {
    events.push(GapEvent {
        evidence_kind: "Indicative".into(),
        run_id: run_id.into(),
        pair: a.pair.clone(),
        group: a.group.clone(),
        pool_a: a.address.clone(),
        pool_b: b.address.clone(),
        venue_a: a.venue.clone(),
        venue_b: b.venue.clone(),
        threshold_bps: threshold,
        start_slot: e.start,
        start_time_unix_ns: e.start_time_unix_ns,
        duration_slots: e.last.saturating_sub(e.start) + 1,
        peak_net_bps_micro: e.peak,
        volatility_bucket: e.volatility,
        after_sharp_move: e.after_sharp,
        open_at_end: open,
    });
}

pub fn analyze(run_dir: &Path, allowlist_path: &Path) -> Result<Analysis> {
    let manifest: RunManifest = serde_json::from_slice(&fs::read(run_dir.join("manifest.json"))?)?;
    ensure!(
        manifest.evidence_kind == "Indicative",
        "wrong evidence kind"
    );
    ensure!(
        format!("{:x}", Sha256::digest(fs::read(allowlist_path)?)) == manifest.allowlist_sha256,
        "allowlist changed since capture"
    );
    let allowlist = Allowlist::load(allowlist_path)?;
    let index = allowlist.by_address();
    let raw_file = fs::read(run_dir.join("updates.jsonl"))?;
    ensure!(
        format!("{:x}", Sha256::digest(&raw_file)) == manifest.updates_sha256,
        "updates checksum mismatch"
    );
    let mut updates = Vec::new();
    let mut payload_bytes = 0u64;
    for line in BufReader::new(raw_file.as_slice()).lines() {
        let rec: UpdateRecord = serde_json::from_str(&line?)?;
        let raw = base64::engine::general_purpose::STANDARD.decode(&rec.raw_base64)?;
        payload_bytes += raw.len() as u64;
        let proto = SubscribeUpdate::decode(raw.as_slice())?;
        let Some(UpdateOneof::Account(a)) = proto.update_oneof else {
            anyhow::bail!("non-account payload")
        };
        let info = a.account.context("missing account info")?;
        ensure!(
            rec.slot == a.slot
                && rec.write_version == info.write_version
                && rec.pubkey == bs58::encode(&info.pubkey).into_string()
                && rec.owner == bs58::encode(&info.owner).into_string(),
            "record/protobuf mismatch"
        );
        ensure!(
            index.contains_key(&rec.pubkey),
            "unallowlisted account in recording"
        );
        updates.push(Update {
            slot: rec.slot,
            write_version: rec.write_version,
            pubkey: rec.pubkey,
            provider_ns: rec.provider_time_unix_ns,
            data: info.data,
        });
    }
    ensure!(
        payload_bytes == manifest.payload_bytes && updates.len() as u64 == manifest.updates,
        "payload accounting mismatch"
    );
    updates.sort_by(|a, b| {
        (&a.slot, &a.write_version, &a.pubkey).cmp(&(&b.slot, &b.write_version, &b.pubkey))
    });
    let mut states = BTreeMap::<String, State>::new();
    let mut minute = BTreeMap::<(i64, String, String), MinuteAggregate>::new();
    let mut open = BTreeMap::<(String, String, i64), OpenEvent>::new();
    let mut events = Vec::<GapEvent>::new();
    let mut seg = BTreeMap::<(String, String), SegmentStat>::new();
    let mut bucket_counts = BTreeMap::<(String, String, String, Option<String>, bool), u64>::new();
    let mut sharp = BTreeMap::<String, u64>::new();
    let mut sol_history = Vec::<(i64, BigUint)>::new();
    let reference = allowlist
        .pools
        .iter()
        .find(|p| p.pair == "SOL/USDC" && p.venue == "Orca")
        .context("no SOL reference pool")?
        .address
        .clone();
    let mut i = 0;
    while i < updates.len() {
        let slot = updates[i].slot;
        let mut j = i;
        while j < updates.len() && updates[j].slot == slot {
            j += 1
        }
        let mut changed = BTreeSet::<String>::new();
        let mut slot_time = None;
        for u in &updates[i..j] {
            let p = index[&u.pubkey];
            let mut decoded: PoolPrice =
                decode_pool(&p.venue, &p.owner, &u.data, p.decimals0, p.decimals1, None)?;
            ensure!(
                decoded.mint0 == p.mint0 && decoded.mint1 == p.mint1,
                "live mint mismatch"
            );
            if p.venue == "Raydium" && decoded.variable_fee_modeled {
                decoded.fee_bps_micro = p.fee_bps_micro;
            }
            if let Some(old) = states.get(&u.pubkey) {
                if old.price != decoded.price_scaled
                    && gross_gap_bps_micro(&old.price, &decoded.price_scaled)? >= 10_000_000
                {
                    sharp.insert(u.pubkey.clone(), slot);
                }
            }
            if u.pubkey == reference {
                if let Some(t) = u.provider_ns {
                    sol_history.push((t, decoded.price_scaled.clone()));
                }
            }
            states.insert(
                u.pubkey.clone(),
                State {
                    slot,
                    price: decoded.price_scaled,
                    fee: decoded.fee_bps_micro,
                },
            );
            changed.insert(p.pair.clone());
            if let Some(t) = u.provider_ns {
                slot_time = Some(slot_time.map_or(t, |v: i64| v.max(t)));
            }
        }
        let vol = slot_time.and_then(|t| {
            let cutoff = t - 60_000_000_000;
            let first = sol_history
                .iter()
                .find(|(ts, _)| *ts >= cutoff && *ts <= t)?;
            let last = sol_history.iter().rev().find(|(ts, _)| *ts <= t)?;
            gross_gap_bps_micro(&first.1, &last.1).ok()
        });
        let bucket = volatility_bucket(vol);
        for pair in changed {
            let pools: Vec<&PoolMeta> = allowlist.pools.iter().filter(|p| p.pair == pair).collect();
            for ai in 0..pools.len() {
                for bi in ai + 1..pools.len() {
                    let (a, b) = if pools[ai].address < pools[bi].address {
                        (pools[ai], pools[bi])
                    } else {
                        (pools[bi], pools[ai])
                    };
                    let Some(sa) = states.get(&a.address) else {
                        continue;
                    };
                    let Some(sb) = states.get(&b.address) else {
                        continue;
                    };
                    let key = pair_key(&a.address, &b.address);
                    let segment_key = (pair.clone(), venue_combination(&a.venue, &b.venue));
                    let segment = seg
                        .entry(segment_key.clone())
                        .or_insert_with(|| SegmentStat {
                            evidence_kind: "Indicative".into(),
                            run_id: manifest.run_id.clone(),
                            pair: pair.clone(),
                            venue_combination: segment_key.1.clone(),
                            observation_count: 0,
                            stale_count: 0,
                            known_fee_count: 0,
                            events_10bps: 0,
                            event_duration_slots: vec![],
                        });
                    segment.observation_count += 1;
                    let minute_key = slot_time.map(|t| {
                        (
                            t.div_euclid(60_000_000_000) * 60,
                            pair.clone(),
                            format!("{}|{}", a.address, b.address),
                        )
                    });
                    if stale(slot, sa.slot.min(sb.slot)) {
                        segment.stale_count += 1;
                        if let Some(k) = minute_key {
                            minute.entry(k).or_default().stale += 1;
                        }
                        for threshold in THRESHOLDS {
                            if let Some(e) = open.remove(&(key.0.clone(), key.1.clone(), threshold))
                            {
                                close_event(
                                    &mut events,
                                    &manifest.run_id,
                                    a,
                                    b,
                                    threshold,
                                    e,
                                    false,
                                );
                            }
                        }
                        continue;
                    }
                    let gross = gross_gap_bps_micro(&sa.price, &sb.price)?;
                    let net = net_gap_bps_micro(gross, sa.fee, sb.fee)?;
                    if net.is_some() {
                        segment.known_fee_count += 1;
                    }
                    if let Some(k) = minute_key {
                        let agg = minute.entry(k).or_default();
                        agg.gross.push(gross);
                        if let Some(n) = net {
                            agg.net.push(n);
                        }
                    }
                    let after_sharp = [&a.address, &b.address].iter().any(|p| {
                        sharp
                            .get(*p)
                            .is_some_and(|s| slot > *s && slot <= s.saturating_add(10))
                    });
                    if net.is_some() {
                        *bucket_counts
                            .entry((
                                pair.clone(),
                                segment_key.1.clone(),
                                a.group.clone(),
                                bucket.clone(),
                                after_sharp,
                            ))
                            .or_default() += 1;
                    }
                    for threshold in THRESHOLDS {
                        let k = (key.0.clone(), key.1.clone(), threshold);
                        if reaches_threshold(net, threshold) {
                            if let Some(e) = open.get_mut(&k) {
                                e.last = slot;
                                e.peak = e.peak.max(net.unwrap());
                            } else {
                                open.insert(
                                    k,
                                    OpenEvent {
                                        start: slot,
                                        start_time_unix_ns: slot_time,
                                        last: slot,
                                        peak: net.unwrap(),
                                        volatility: bucket.clone(),
                                        after_sharp,
                                    },
                                );
                            }
                        } else if let Some(e) = open.remove(&k) {
                            close_event(&mut events, &manifest.run_id, a, b, threshold, e, false);
                        }
                    }
                }
            }
        }
        i = j;
    }
    for ((ka, kb, threshold), e) in open {
        let a = index[&ka];
        let b = index[&kb];
        close_event(&mut events, &manifest.run_id, a, b, threshold, e, true);
    }
    events.sort_by(|a, b| {
        (&a.pair, &a.pool_a, &a.pool_b, a.threshold_bps, a.start_slot).cmp(&(
            &b.pair,
            &b.pool_a,
            &b.pool_b,
            b.threshold_bps,
            b.start_slot,
        ))
    });
    for e in &events {
        if e.threshold_bps == 10 {
            let k = (e.pair.clone(), venue_combination(&e.venue_a, &e.venue_b));
            if let Some(s) = seg.get_mut(&k) {
                s.events_10bps += 1;
                s.event_duration_slots.push(e.duration_slots);
            }
        }
    }
    let mut minutes = Vec::new();
    for ((minute_unix, pair, ids), mut agg) in minute {
        let (ka, kb) = ids.split_once('|').context("invalid pair id")?;
        let a = index[ka];
        let b = index[kb];
        let maximum_gross_bps_micro = agg.gross.iter().max().copied();
        let maximum_net_bps_micro = agg.net.iter().max().copied();
        let observation_count = agg.gross.len() as u64;
        let known_fee_observation_count = agg.net.len() as u64;
        minutes.push(MinuteStat {
            evidence_kind: "Indicative".into(),
            run_id: manifest.run_id.clone(),
            minute_unix,
            pair,
            group: a.group.clone(),
            pool_a: ka.into(),
            pool_b: kb.into(),
            venue_a: a.venue.clone(),
            venue_b: b.venue.clone(),
            observation_count,
            known_fee_observation_count,
            median_gross_bps_micro: median_u64(&mut agg.gross),
            maximum_gross_bps_micro,
            median_net_bps_micro: median_i64(&mut agg.net),
            maximum_net_bps_micro,
            excluded_stale_count: agg.stale,
        });
    }
    Ok(Analysis {
        evidence_kind: "Indicative".into(),
        run_id: manifest.run_id.clone(),
        provider: manifest.provider,
        window_start_utc: manifest.started_utc,
        window_end_utc: manifest.stopped_utc,
        duration_seconds: manifest.duration_seconds,
        payload_bytes: manifest.payload_bytes,
        updates: manifest.updates,
        gaps: manifest.gaps,
        stalls: manifest.stalls,
        stop_reason: manifest.stop_reason,
        allowlist_sha256: manifest.allowlist_sha256,
        updates_sha256: manifest.updates_sha256,
        updates_by_venue: manifest.updates_by_venue,
        minutes,
        events,
        segments: seg.into_values().collect(),
        buckets: bucket_counts
            .into_iter()
            .map(
                |(
                    (pair, venue_combination, group, volatility_bucket, after_sharp_move),
                    observations,
                )| BucketStat {
                    evidence_kind: "Indicative".into(),
                    run_id: manifest.run_id.clone(),
                    pair,
                    group,
                    venue_combination,
                    volatility_bucket,
                    after_sharp_move,
                    observations,
                },
            )
            .collect(),
    })
}
pub fn write_analysis(run_dir: &Path, allowlist: &Path, output: &Path) -> Result<()> {
    let analysis = analyze(run_dir, allowlist)?;
    let bytes = serde_json::to_vec_pretty(&analysis)?;
    fs::write(output, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hand_computed_gap_and_unknown_fee() {
        let a = BigUint::from(101u8);
        let b = BigUint::from(99u8);
        assert_eq!(gross_gap_bps_micro(&a, &b).unwrap(), 200_000_000);
        assert_eq!(
            net_gap_bps_micro(200_000_000, Some(4_000_000), Some(5_000_000)).unwrap(),
            Some(191_000_000)
        );
        assert_eq!(
            net_gap_bps_micro(200_000_000, Some(4_000_000), None).unwrap(),
            None
        );
    }
    #[test]
    fn stale_and_volatility_boundaries() {
        assert!(!stale(125, 100));
        assert!(stale(126, 100));
        assert_eq!(
            volatility_bucket(Some(9_999_999)).as_deref(),
            Some("below_10_bps")
        );
        assert_eq!(
            volatility_bucket(Some(10_000_000)).as_deref(),
            Some("10_to_50_bps")
        );
        assert_eq!(
            volatility_bucket(Some(50_000_000)).as_deref(),
            Some("10_to_50_bps")
        );
        assert_eq!(
            volatility_bucket(Some(50_000_001)).as_deref(),
            Some("above_50_bps")
        );
        assert!(!reaches_threshold(Some(9_999_999), 10));
        assert!(reaches_threshold(Some(10_000_000), 10));
    }
    #[test]
    fn deterministic_analysis_and_open_event_at_run_end() {
        use crate::capture::{RunManifest, UpdateRecord};
        use yellowstone_grpc_proto::prelude::{SubscribeUpdateAccount, SubscribeUpdateAccountInfo};
        let dir = tempfile::tempdir().unwrap();
        let allow = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/allowlist.json"));
        let orca = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE";
        let ray = "3ucNos4NbumPLZNWztqGHNFFgkHeRMBQAVemeeomsUxv";
        let mut rich = include_bytes!("../tests/fixtures/raydium_sol_usdc.bin").to_vec();
        let sqrt = u128::from_le_bytes(rich[253..269].try_into().unwrap());
        rich[253..269].copy_from_slice(&(sqrt * 1002 / 1000).to_le_bytes());
        let cases = [
            (
                100,
                orca,
                crate::decode::ORCA,
                include_bytes!("../tests/fixtures/orca_sol_usdc.bin").as_slice(),
            ),
            (
                100,
                ray,
                crate::decode::RAYDIUM,
                include_bytes!("../tests/fixtures/raydium_sol_usdc.bin").as_slice(),
            ),
            (101, ray, crate::decode::RAYDIUM, rich.as_slice()),
            (
                102,
                ray,
                crate::decode::RAYDIUM,
                include_bytes!("../tests/fixtures/raydium_sol_usdc.bin").as_slice(),
            ),
            (103, ray, crate::decode::RAYDIUM, rich.as_slice()),
        ];
        let mut lines = Vec::new();
        let mut payload_bytes = 0;
        for (slot, address, owner, data) in cases {
            let proto = SubscribeUpdate {
                filters: vec!["allowlisted_pools".into()],
                created_at: None,
                update_oneof: Some(UpdateOneof::Account(SubscribeUpdateAccount {
                    slot,
                    is_startup: false,
                    account: Some(SubscribeUpdateAccountInfo {
                        pubkey: bs58::decode(address).into_vec().unwrap(),
                        owner: bs58::decode(owner).into_vec().unwrap(),
                        data: data.to_vec(),
                        write_version: 1,
                        ..Default::default()
                    }),
                })),
            };
            let raw = proto.encode_to_vec();
            payload_bytes += raw.len() as u64;
            let rec = UpdateRecord {
                pubkey: address.into(),
                owner: owner.into(),
                slot,
                write_version: 1,
                provider_time_unix_ns: None,
                raw_base64: base64::engine::general_purpose::STANDARD.encode(raw),
            };
            lines.extend(serde_json::to_vec(&rec).unwrap());
            lines.push(b'\n');
        }
        fs::write(dir.path().join("updates.jsonl"), &lines).unwrap();
        let manifest = RunManifest {
            phase: "pilot".into(),
            evidence_kind: "Indicative".into(),
            run_id: "synthetic".into(),
            provider: "mock".into(),
            started_utc: "2026-10-31T00:00:00Z".into(),
            stopped_utc: "2026-10-31T00:01:00Z".into(),
            duration_seconds: 60,
            payload_bytes,
            updates: 5,
            updates_by_venue: Default::default(),
            gaps: 0,
            stalls: 0,
            stop_reason: "duration".into(),
            allowlist_sha256: format!("{:x}", Sha256::digest(fs::read(allow).unwrap())),
            updates_sha256: format!("{:x}", Sha256::digest(&lines)),
        };
        fs::write(
            dir.path().join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let a = serde_json::to_vec(&analyze(dir.path(), allow).unwrap()).unwrap();
        let b = serde_json::to_vec(&analyze(dir.path(), allow).unwrap()).unwrap();
        assert_eq!(a, b);
        let v: Analysis = serde_json::from_slice(&a).unwrap();
        assert!(v
            .events
            .iter()
            .any(|e| e.threshold_bps == 10 && !e.open_at_end && e.start_slot == 101));
        assert!(v
            .events
            .iter()
            .any(|e| e.threshold_bps == 10 && e.open_at_end && e.start_slot == 103));
    }
}
