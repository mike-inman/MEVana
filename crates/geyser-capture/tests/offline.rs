//! Offline and synthetic tests only. No provider connections or real decoder fixtures.
use geyser_capture::{
    config::{self, Provider},
    format::*,
    layout,
    source::*,
};
use prost::Message;
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
    sync::Arc,
    time::Duration,
};
use yellowstone_grpc_proto::prelude::*;
fn metadata() -> Metadata {
    Metadata {
        format_version: 1,
        session_id: "Synthetic-test".into(),
        provider: Provider::Sodae,
        endpoint_host: "localhost".into(),
        connection_tier: "dev-test".into(),
        source: "Synthetic".into(),
        observed_cluster: "unknown".into(),
        commitment: "processed".into(),
        allowlist: vec![],
        filters: BTreeMap::new(),
        crate_version: "test".into(),
        git_commit: "test".into(),
    }
}
fn raw_slot(slot: u64) -> Vec<u8> {
    SubscribeUpdate {
        update_oneof: Some(subscribe_update::UpdateOneof::Slot(SubscribeUpdateSlot {
            slot,
            status: 0,
            ..Default::default()
        })),
        ..Default::default()
    }
    .encode_to_vec()
}
#[test]
fn synthetic_snapshot_v2_round_trip_and_inspection() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("v2");
    let mut metadata = metadata();
    metadata.format_version = 2;
    let mut writer = Writer::new(dir.clone(), metadata, 3600, 1_000_000).unwrap();
    let key = solana_sdk::pubkey::Pubkey::new_unique().to_string();
    writer.write(0, 100, raw_slot(42), false).unwrap();
    writer
        .write_snapshot(
            101,
            &geyser_capture::format::SnapshotRecord::Account {
                version: 1,
                snapshot_id: 1,
                account_kind: "tick_array".into(),
                pool_id: key.clone(),
                key,
                context_slot: 42,
                present: false,
                owner: None,
                lamports: None,
                data_base64: None,
                sha256: None,
            },
        )
        .unwrap();
    writer
        .write_snapshot(
            102,
            &geyser_capture::format::SnapshotRecord::Stats {
                version: 1,
                snapshot_id: 1,
                calls: 4,
                rpc_bytes: 1024,
            },
        )
        .unwrap();
    writer.finish("duration").unwrap();
    let report = geyser_capture::inspect::inspect(&dir).unwrap();
    assert_eq!(report.files, 1);
    let manifest: geyser_capture::format::Manifest =
        serde_json::from_slice(&std::fs::read(dir.join("000000.cap.json")).unwrap()).unwrap();
    assert_eq!(
        (
            manifest.snapshot_frames,
            manifest.snapshot_absent,
            manifest.snapshot_calls,
            manifest.snapshot_rpc_bytes
        ),
        (2, 1, 4, 1024)
    );
    let mut invalid = Record {
        sequence: 1,
        receive_ns: 1,
        connection: 0,
        gap: false,
        snapshot: false,
        payload: raw_slot(1),
    }
    .encode()
    .unwrap();
    invalid[4] = 3;
    assert!(geyser_capture::format::read_record(&mut std::io::Cursor::new(invalid)).is_err());
}
#[test]
fn frame_round_trip_preserves_unknown_protobuf_fields() {
    let mut payload = raw_slot(42);
    payload.extend([0xf8, 0x07, 0x01]);
    let r = Record {
        sequence: 1,
        receive_ns: 123,
        connection: 0,
        gap: false,
        snapshot: false,
        payload: payload.clone(),
    };
    let mut c = Cursor::new(r.encode().unwrap());
    let got = read_record(&mut c).unwrap().unwrap();
    assert_eq!(got.payload, payload);
    assert_eq!(got.sequence, 1);
    assert_eq!(got.receive_ns, 123);
    assert!(read_record(&mut c).unwrap().is_none());
}
#[test]
fn rotation_atomic_rename_hash_and_corruption() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("session");
    let mut w = Writer::new(dir.clone(), metadata(), 3600, 30).unwrap();
    assert!(dir.join("000000.cap.part").exists());
    w.write(0, 1, raw_slot(1), false).unwrap();
    w.write(0, 2, raw_slot(2), false).unwrap();
    assert!(dir.join("000000.cap").exists());
    assert!(dir.join("000001.cap.part").exists());
    let session = w.finish("test").unwrap();
    assert_eq!(session.files.len(), 2);
    assert_eq!(session.metadata.source, "Synthetic");
    let report = geyser_capture::inspect::inspect(&dir).unwrap();
    assert_eq!(report.files, 2);
    let m: Manifest =
        serde_json::from_slice(&std::fs::read(dir.join(&session.files[0])).unwrap()).unwrap();
    assert_eq!(hash_file(&dir.join(&m.file)).unwrap(), m.sha256);
    std::fs::OpenOptions::new()
        .append(true)
        .open(dir.join(&m.file))
        .unwrap()
        .write_all(b"corrupt")
        .unwrap();
    assert!(geyser_capture::inspect::inspect(&dir).is_err());
}
#[test]
fn part_is_never_catalogued_and_truncated_records_fail() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("session");
    let mut w = Writer::new(dir.clone(), metadata(), 3600, 10000).unwrap();
    w.write(0, 1, raw_slot(1), false).unwrap();
    w.finish("test").unwrap();
    std::fs::write(dir.join("crash.cap.part"), b"MEVCAP01").unwrap();
    let r = geyser_capture::inspect::inspect(&dir).unwrap();
    assert_eq!(r.incomplete, vec!["crash.cap.part"]);
    let mut c = Cursor::new(vec![20, 0, 0, 0, 0, 0]);
    assert!(read_record(&mut c).is_err());
}
#[test]
fn mock_gap_record_survives_rotation() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("session");
    let mut w = Writer::new(dir.clone(), metadata(), 3600, 30).unwrap();
    w.write(0, 1, raw_slot(1), false).unwrap();
    let g = Gap {
        connection: "accounts".into(),
        reason: "stall".into(),
        previous_slot: None,
        next_slot: None,
    };
    w.write(0, 2, serde_json::to_vec(&g).unwrap(), true)
        .unwrap();
    w.finish("test").unwrap();
    let r = geyser_capture::inspect::inspect(&dir).unwrap();
    assert_eq!(r.gaps, 1);
    assert_eq!(r.stalls, 1);
}
#[test]
fn credential_redaction_and_environment_precedence() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("credentials.fixture");
    std::fs::write(
        &p,
        "SODAE.IO_API_KEY=synthetic-file\nTATUM_MAINNET=synthetic-other\nUNRELATED=ignored\n",
    )
    .unwrap();
    let s = config::credential(Provider::Sodae, &p, |k| {
        assert_eq!(k, "SODAE.IO_API_KEY");
        Some("synthetic-exported".into())
    })
    .unwrap();
    assert_eq!(s.expose(), "synthetic-exported");
    assert_eq!(format!("{s:?}"), "[REDACTED]");
    assert_eq!(
        config::credential(Provider::Tatum, &p, |_| None)
            .unwrap()
            .expose(),
        "synthetic-other"
    );
}
#[test]
fn tokens_cannot_cross_provider_endpoints() {
    let secret = config::credential(Provider::Sodae, std::path::Path::new("not-read"), |_| {
        Some("synthetic-sodae".into())
    })
    .unwrap();
    assert!(
        config::validate_pair(Provider::Tatum, &Provider::Tatum.endpoint(), &secret, false)
            .is_err()
    );
    assert!(
        config::validate_pair(Provider::Sodae, &Provider::Sodae.endpoint(), &secret, false).is_ok()
    );

    assert!(
        config::validate_endpoint(Provider::Sodae, &Provider::Tatum.endpoint(), false).is_err()
    );
    assert!(
        config::validate_endpoint(Provider::Tatum, &Provider::Sodae.endpoint(), false).is_err()
    );
    assert_eq!(Provider::Sodae.header(), "x-token");
    assert_eq!(Provider::Tatum.header(), "x-api-key");
    assert!(
        config::validate_endpoint(Provider::Sodae, "http://ams.rpc.sodae.io:10201", false).is_err()
    );
    assert!(config::validate_endpoint(Provider::Sodae, "https://localhost:50051", false).is_err());
    assert!(config::validate_endpoint(Provider::Sodae, "https://localhost:50051", true).is_ok());
    assert!(config::validate_endpoint(
        Provider::Sodae,
        "https://ams.tls.sodae.io?token=synthetic",
        false
    )
    .is_err());
    for ip in [
        "127.0.0.1",
        "10.0.0.1",
        "192.168.0.1",
        "169.254.1.1",
        "::1",
        "fc00::1",
        "::ffff:127.0.0.1",
    ] {
        assert!(config::private_ip(ip.parse().unwrap()));
    }
}
fn idl_size(t: &serde_json::Value, defs: &BTreeMap<String, serde_json::Value>) -> usize {
    if let Some(s) = t.as_str() {
        return match s {
            "bool" | "u8" => 1,
            "u16" => 2,
            "u32" | "i32" => 4,
            "u64" => 8,
            "u128" | "i128" => 16,
            "publicKey" => 32,
            _ => panic!("unknown IDL primitive"),
        };
    }
    if let Some(a) = t.get("array") {
        return idl_size(&a[0], defs) * a[1].as_u64().unwrap() as usize;
    }
    defs[t["defined"].as_str().unwrap()]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| idl_size(&f["type"], defs))
        .sum()
}
#[test]
fn idl_derived_sizes_offsets_discriminators_and_filters() {
    let idl: serde_json::Value =
        serde_json::from_str(include_str!("../../../idls/whirlpool_idl.json")).unwrap();
    let defs: BTreeMap<_, _> = idl["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .chain(idl["types"].as_array().unwrap())
        .map(|v| (v["name"].as_str().unwrap().to_owned(), v["type"].clone()))
        .collect();
    for (name, expected) in [
        ("Whirlpool", layout::POOL_SIZE),
        ("TickArray", layout::FIXED_SIZE),
        ("Position", layout::POSITION_SIZE),
    ] {
        assert_eq!(
            8 + idl_size(&serde_json::json!({"defined":name}), &defs),
            expected
        );
    }
    assert_eq!(
        idl_size(&serde_json::json!({"defined":"Tick"}), &defs),
        layout::TICK
    );
    let fields = defs["TickArray"]["fields"].as_array().unwrap();
    let mut offset = 8;
    for f in fields {
        if f["name"] == "whirlpool" {
            assert_eq!(offset, layout::FIXED_POOL_OFFSET);
        }
        offset += idl_size(&f["type"], &defs);
    }
    assert_eq!(layout::DYNAMIC_POOL_OFFSET, 8 + 4);
    assert_eq!(layout::DYNAMIC_MIN, 148);
    assert_eq!(layout::DYNAMIC_MIN + 88 * layout::DYNAMIC_DATA, 10004);
    assert_eq!(layout::ORACLE_SIZE, 254);
    assert_eq!(
        layout::disc("DynamicTickArray"),
        [0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]
    );
    assert_eq!(
        layout::disc("Oracle"),
        [0x8b, 0xc2, 0x83, 0xb3, 0x8c, 0xb3, 0xe5, 0xf4]
    );
    let pools = vec!["So11111111111111111111111111111111111111112".into()];
    let r = layout::request(&pools, false, 0);
    assert_eq!(r.accounts.len(), 3);
    for f in r.accounts.values() {
        assert!(!f.filters.is_empty());
        assert_eq!(f.owner, vec![shared_core::programs::ORCA_WHIRLPOOL]);
    }
    for (name, offset) in [
        ("TickArray_0", layout::FIXED_POOL_OFFSET),
        ("DynamicTickArray_0", layout::DYNAMIC_POOL_OFFSET),
    ] {
        assert!(r.accounts[name].filters.iter().any(|f|matches!(&f.filter,Some(subscribe_request_filter_accounts_filter::Filter::Memcmp(m)) if m.offset==offset as u64)));
    }
    assert_eq!(layout::request(&[], false, 0).accounts.len(), 1);
    assert!(layout::request(&[], true, 0).transactions.is_empty());
    let tx = layout::request(&pools, true, 0);
    let f = &tx.transactions["pool_transactions"];
    assert_eq!(f.failed, None);
    assert_eq!(f.vote, Some(false));
    assert_eq!(f.account_include, pools);
}
#[test]
fn fixed_layout_serializes_to_idl_and_rejects_wrong_lengths() {
    use whirlpool_core::{Tick, TickArray};
    let zero = Tick {
        initialized: false,
        liquidity_net: 0,
        liquidity_gross: 0,
        fee_growth_outside_a: 0,
        fee_growth_outside_b: 0,
        reward_growths_outside: [0; 3],
    };
    let array = TickArray {
        start_tick_index: 0,
        ticks: std::array::from_fn(|_| zero.clone()),
        whirlpool: Default::default(),
    };
    let mut b = layout::disc("TickArray").to_vec();
    b.extend(borsh::to_vec(&array).unwrap());
    assert_eq!(b.len(), layout::FIXED_SIZE);
    assert_eq!(whirlpool_core::decode_tick_array(&b).unwrap(), array);
    assert!(whirlpool_core::decode_tick_array(&b[..b.len() - 1]).is_err());
    b.push(0);
    assert!(whirlpool_core::decode_tick_array(&b).is_err());
}
#[test]
fn port_config_accepts_new_development_key_and_fixed_bind_fails_when_busy() {
    let text = include_str!("../../../config/ports.toml").to_owned();
    let _: shared_core::port_config::PortConfig = toml::from_str(&text).unwrap();
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    assert!(std::net::TcpListener::bind(l.local_addr().unwrap()).is_err());
}
#[test]
fn backoff_is_capped_and_jittered() {
    for attempt in 0..100 {
        for jitter in [0, 1, 250, u64::MAX] {
            let d = backoff(attempt, jitter);
            assert!(d <= Duration::from_secs(30));
            assert!(d >= Duration::from_millis(250));
        }
    }
    assert_ne!(backoff(1, 0), backoff(1, 100));
}
struct Mock {
    items: std::collections::VecDeque<std::result::Result<Option<Vec<u8>>, Failure>>,
    connects: Arc<std::sync::atomic::AtomicUsize>,
    fail_connect: bool,
}
#[async_trait::async_trait]
impl Source for Mock {
    async fn connect(&mut self) -> std::result::Result<(), Failure> {
        self.connects
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail_connect {
            Err(Failure::Transport)
        } else {
            Ok(())
        }
    }
    async fn next(&mut self) -> std::result::Result<Option<Vec<u8>>, Failure> {
        if let Some(v) = self.items.pop_front() {
            v
        } else {
            std::future::pending().await
        }
    }
    async fn ping(&mut self) -> std::result::Result<(), Failure> {
        Ok(())
    }
}
#[tokio::test]
async fn mock_stall_slot_gap_and_disconnect_are_visible() {
    let connects = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mock = Mock {
        items: vec![Ok(Some(raw_slot(10))), Ok(Some(raw_slot(12)))].into(),
        connects,
        fail_connect: false,
    };
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let (stop, s) = tokio::sync::watch::channel(false);
    let h = tokio::spawn(drive(
        mock,
        0,
        tx,
        s,
        Duration::from_millis(5),
        5,
        Budget::new(450_000_000),
    ));
    let mut reasons = vec![];
    while let Some(e) = rx.recv().await {
        if let Event::Raw { appended, .. } = e {
            let _ = appended.send(());
        } else if let Event::Gap { gap, .. } = e {
            reasons.push(gap.reason);
            if reasons.iter().any(|x| x == "disconnect_after_stall") {
                stop.send(true).unwrap();
                break;
            }
        }
    }
    h.await.unwrap();
    assert!(reasons.contains(&"slot_discontinuity".into()));
    assert!(reasons.contains(&"stall".into()));
    assert!(reasons.contains(&"disconnect_after_stall".into()));
}
#[tokio::test]
async fn mock_reconnect_failures_are_finite() {
    let connects = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mock = Mock {
        items: Default::default(),
        connects: connects.clone(),
        fail_connect: true,
    };
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let (_stop, s) = tokio::sync::watch::channel(false);
    let h = tokio::spawn(drive(
        mock,
        0,
        tx,
        s,
        Duration::from_millis(5),
        2,
        Budget::new(450_000_000),
    ));
    let mut fatal = false;
    while let Some(e) = rx.recv().await {
        fatal |= matches!(e, Event::Fatal(_));
    }
    h.await.unwrap();
    assert!(fatal);
    assert_eq!(connects.load(std::sync::atomic::Ordering::SeqCst), 2);
}
#[test]
fn dynamic_accounts_counted_without_fixed_decode() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("session");
    let mut w = Writer::new(dir.clone(), metadata(), 3600, 10000).unwrap();
    let mut data = vec![0; layout::DYNAMIC_MIN + 112];
    data[..8].copy_from_slice(&layout::disc("DynamicTickArray"));
    let raw = SubscribeUpdate {
        update_oneof: Some(subscribe_update::UpdateOneof::Account(
            SubscribeUpdateAccount {
                slot: 1,
                account: Some(SubscribeUpdateAccountInfo {
                    pubkey: vec![1; 32],
                    data,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )),
        ..Default::default()
    }
    .encode_to_vec();
    w.write(0, 1, raw, false).unwrap();
    w.finish("test").unwrap();
    let r = geyser_capture::inspect::inspect(&dir).unwrap();
    assert_eq!(r.decode["dynamic_unsupported"], 1);
    assert!(r.unexpected.is_empty());
}

#[test]
fn both_connections_share_one_byte_budget() {
    let budget = Budget::new(100);
    let other = budget.clone();
    assert!(!budget.account(60));
    assert!(other.account(40));
    assert!(budget.account(1));
}
