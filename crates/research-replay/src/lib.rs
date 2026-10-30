//! Offline, integer-only Whirlpool research replay. No provider or trading path.
use anyhow::{bail, ensure, Context, Result};
use geyser_capture::format::SnapshotRecord;
use geyser_capture::{format, layout};
use prost::Message;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufReader, Read},
    path::Path,
    str::FromStr,
};
use whirlpool_core::{decode::decode_whirlpool, exact, types::WhirlpoolMinimal};
use yellowstone_grpc_proto::prelude::{subscribe_update::UpdateOneof, SubscribeUpdate};
pub mod engine;
pub mod policy;
pub mod slot;

pub const SOL: &str = "So11111111111111111111111111111111111111112";
pub const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
pub const PROGRAM: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const DATASET_ID: &str = "orca-wp-allowlist-20261029";
pub const STRATEGY_VERSION: &str = "ranking-v0-heuristic";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pair {
    A,
    B,
    C,
}
impl Pair {
    pub fn tokens(self) -> (&'static str, &'static str) {
        match self {
            Self::A => (SOL, USDC),
            Self::B => (SOL, USDT),
            Self::C => (USDC, USDT),
        }
    }
    pub fn expected_pools(self) -> usize {
        match self {
            Self::A => 6,
            Self::B => 5,
            Self::C => 2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

#[derive(Clone)]
pub struct AccountWrite {
    pub slot: u64,
    pub receive_ns: u64,
    pub write_version: u64,
    pub sequence: u64,
    pub key: Pubkey,
    pub data: Vec<u8>,
}
#[derive(Clone)]
pub struct GapEvent {
    pub reason: String,
    pub previous_slot: Option<u64>,
    pub next_slot: Option<u64>,
    pub receive_ns: u64,
    pub sequence: u64,
}
#[derive(Clone)]
pub struct SnapshotEvent {
    pub receive_ns: u64,
    pub sequence: u64,
    pub record: SnapshotRecord,
}
#[derive(Clone, Default)]
pub struct PoolState {
    pub pool: Option<WhirlpoolMinimal>,
    pub arrays: BTreeMap<Pubkey, exact::DecodedTickArray>,
    pub absent_arrays: BTreeSet<Pubkey>,
    pub stale: bool,
    pub generation: u64,
}
#[derive(Clone, Default)]
pub struct MarketState {
    pub pools: BTreeMap<Pubkey, PoolState>,
    pub snapshot_context: BTreeMap<Pubkey, u64>,
    pub latest_write_slot: BTreeMap<Pubkey, u64>,
}
impl MarketState {
    pub fn gap(&mut self) {
        for state in self.pools.values_mut() {
            state.stale = true;
            state.generation += 1;
        }
    }
    pub fn generation(&self, id: &Pubkey) -> u64 {
        self.pools.get(id).map_or(0, |state| state.generation)
    }
    pub fn apply(&mut self, write: &AccountWrite, allowlist: &BTreeSet<Pubkey>) -> Result<()> {
        if self
            .snapshot_context
            .get(&write.key)
            .is_some_and(|&c| write.slot <= c)
        {
            return Ok(());
        }
        if self
            .latest_write_slot
            .get(&write.key)
            .is_some_and(|&s| write.slot < s)
        {
            return Ok(());
        }
        match layout::kind(&write.data) {
            "pool" if allowlist.contains(&write.key) => {
                let pool = decode_whirlpool(&write.data)?;
                let state = self.pools.entry(write.key).or_default();
                state.pool = Some(pool);
                state.stale = false;
                state.generation += 1;
                self.latest_write_slot.insert(write.key, write.slot);
            }
            "fixed" | "dynamic" => {
                let array = exact::decode_array(&write.data)?;
                let owner = array.whirlpool;
                if allowlist.contains(&owner) {
                    let state = self.pools.entry(owner).or_default();
                    state.arrays.insert(write.key, array);
                    state.absent_arrays.remove(&write.key);
                    state.generation += 1;
                    self.latest_write_slot.insert(write.key, write.slot);
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn apply_snapshot(
        &mut self,
        record: &SnapshotRecord,
        allowlist: &BTreeSet<Pubkey>,
    ) -> Result<()> {
        let SnapshotRecord::Account {
            account_kind,
            pool_id,
            key,
            context_slot,
            present,
            data_base64,
            ..
        } = record
        else {
            return Ok(());
        };
        let pool_id = Pubkey::from_str(pool_id)?;
        let key = Pubkey::from_str(key)?;
        if !allowlist.contains(&pool_id) {
            return Ok(());
        }
        let previous_context = self.snapshot_context.get(&key).copied().unwrap_or(0);
        if *context_slot < previous_context {
            return Ok(());
        }
        self.snapshot_context.insert(key, *context_slot);
        if self
            .latest_write_slot
            .get(&key)
            .is_some_and(|&slot| slot > *context_slot)
        {
            return Ok(());
        }
        let state = self.pools.entry(pool_id).or_default();
        match (account_kind.as_str(), present) {
            ("pool", true) => {
                let data = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    data_base64.as_deref().context("missing snapshot pool")?,
                )?;
                state.pool = Some(decode_whirlpool(&data)?);
                state.stale = false;
            }
            ("pool", false) => {
                state.pool = None;
                state.stale = false;
            }
            ("tick_array", true) => {
                let data = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    data_base64.as_deref().context("missing snapshot array")?,
                )?;
                state.arrays.insert(key, exact::decode_array(&data)?);
                state.absent_arrays.remove(&key);
            }
            ("tick_array", false) => {
                state.arrays.remove(&key);
                state.absent_arrays.insert(key);
            }
            _ => anyhow::bail!("invalid snapshot account kind"),
        }
        state.generation += 1;
        self.latest_write_slot.insert(key, *context_slot);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Capture {
    pub format_version: u32,
    pub session_id: String,
    pub writes: Vec<AccountWrite>,
    pub gaps: Vec<GapEvent>,
    pub slot_nodes: BTreeMap<u64, slot::SlotNode>,
    pub snapshots: Vec<SnapshotEvent>,
    pub slots: BTreeSet<u64>,
    pub allowlist: BTreeSet<Pubkey>,
    pub hashes: BTreeMap<String, String>,
    pub first_receive_ns: u64,
    pub last_receive_ns: u64,
    pub first_slot: u64,
    pub last_slot: u64,
}

pub fn load_capture(root: &Path) -> Result<Capture> {
    let session: format::Session =
        serde_json::from_slice(&std::fs::read(root.join("session.json"))?)?;
    ensure!(
        matches!(session.metadata.format_version, 1 | 2)
            && session.metadata.commitment == "processed",
        "unsupported capture"
    );
    ensure!(
        session.metadata.source == "MarketData" && session.metadata.connection_tier == "dev-test",
        "wrong dataset"
    );
    ensure!(
        (!session.files.is_empty())
            && (session.metadata.format_version == 2 || session.files.len() == 4)
            && session.metadata.allowlist.len() == 18,
        "wrong allowlist session"
    );
    let allowlist: BTreeSet<Pubkey> = session
        .metadata
        .allowlist
        .iter()
        .map(|s| Pubkey::from_str(s))
        .collect::<std::result::Result<_, _>>()?;
    let mut writes = Vec::new();
    let mut gaps = Vec::new();
    let mut slot_nodes = BTreeMap::<u64, slot::SlotNode>::new();
    let mut snapshots = Vec::<SnapshotEvent>::new();
    let mut slots = BTreeSet::new();
    let mut slot_receive_ns = BTreeMap::<u64, u64>::new();
    let mut hashes = BTreeMap::new();
    let mut previous = 0;
    let mut first_receive_ns = u64::MAX;
    let mut last_receive_ns = 0;
    let mut first_slot = u64::MAX;
    let mut last_slot = 0;
    for name in &session.files {
        ensure!(
            Path::new(name).components().count() == 1,
            "invalid manifest path"
        );
        let manifest: format::Manifest = serde_json::from_slice(&std::fs::read(root.join(name))?)?;
        ensure!(
            manifest.metadata.session_id == session.metadata.session_id
                && manifest.metadata.format_version == session.metadata.format_version,
            "manifest identity mismatch"
        );
        ensure!(
            Path::new(&manifest.file).components().count() == 1,
            "invalid capture path"
        );
        let path = root.join(&manifest.file);
        ensure!(
            format::hash_file(&path)? == manifest.sha256,
            "capture SHA-256 mismatch"
        );
        ensure!(
            std::fs::metadata(&path)?.len() == manifest.bytes,
            "capture length mismatch"
        );
        hashes.insert(manifest.file.clone(), manifest.sha256);
        first_receive_ns =
            first_receive_ns.min(manifest.first_receive_ns.context("missing receive start")?);
        last_receive_ns =
            last_receive_ns.max(manifest.last_receive_ns.context("missing receive end")?);
        first_slot = first_slot.min(manifest.first_slot.context("missing slot start")?);
        last_slot = last_slot.max(manifest.last_slot.context("missing slot end")?);
        let mut reader = BufReader::new(File::open(path)?);
        let mut magic = [0; 8];
        reader.read_exact(&mut magic)?;
        ensure!(&magic == format::MAGIC, "wrong capture magic");
        while let Some(record) = format::read_record(&mut reader)? {
            ensure!(record.sequence == previous + 1, "capture sequence gap");
            previous = record.sequence;
            if record.gap {
                if record.connection == 0 {
                    let gap: format::Gap = serde_json::from_slice(&record.payload)?;
                    gaps.push(GapEvent {
                        reason: gap.reason,
                        previous_slot: gap.previous_slot,
                        next_slot: gap.next_slot,
                        receive_ns: record.receive_ns,
                        sequence: record.sequence,
                    });
                }
                continue;
            }
            if record.snapshot {
                ensure!(
                    session.metadata.format_version == 2,
                    "snapshot in v1 capture"
                );
                let payload: SnapshotRecord = serde_json::from_slice(&record.payload)?;
                payload.validate()?;
                snapshots.push(SnapshotEvent {
                    receive_ns: record.receive_ns,
                    sequence: record.sequence,
                    record: payload,
                });
                continue;
            }
            let update = SubscribeUpdate::decode(record.payload.as_slice())?;
            match update.update_oneof {
                Some(UpdateOneof::Account(account)) if record.connection == 0 => {
                    let info = account.account.context("missing account info")?;
                    if matches!(layout::kind(&info.data), "pool" | "fixed" | "dynamic") {
                        writes.push(AccountWrite {
                            slot: account.slot,
                            receive_ns: record.receive_ns,
                            write_version: info.write_version,
                            sequence: record.sequence,
                            key: Pubkey::try_from(info.pubkey.as_slice())?,
                            data: info.data,
                        });
                    }
                }
                Some(UpdateOneof::Slot(slot)) if record.connection == 0 => {
                    let node = slot_nodes.entry(slot.slot).or_default();
                    if let Some(parent) = slot.parent {
                        if let Some(existing) = node.parent {
                            ensure!(existing == parent, "slot parent changed");
                        }
                        node.parent = Some(parent);
                    }
                    let event = slot::SeenStatus {
                        receive_ns: record.receive_ns,
                        sequence: record.sequence,
                    };
                    node.statuses
                        .entry(slot.status)
                        .and_modify(|old| {
                            if (event.receive_ns, event.sequence) < (old.receive_ns, old.sequence) {
                                *old = event;
                            }
                        })
                        .or_insert(event);
                    if slot.status == 0 {
                        slots.insert(slot.slot);
                        slot_receive_ns
                            .entry(slot.slot)
                            .and_modify(|t| *t = (*t).min(record.receive_ns))
                            .or_insert(record.receive_ns);
                    }
                }
                _ => {}
            }
        }
    }
    ensure!(
        first_slot < last_slot && first_receive_ns < last_receive_ns,
        "empty capture window"
    );
    writes.sort_by_key(|w| (w.slot, w.write_version, w.sequence));
    gaps.sort_by_key(|g| (g.receive_ns, g.sequence));
    snapshots.sort_by_key(|s| (s.receive_ns, s.sequence));
    slots.extend(writes.iter().map(|w| w.slot));
    // Window endpoints are observed Processed slot notifications, not arbitrary first/last frames.
    if let (Some((&start_slot, &start_ns)), Some((&end_slot, &end_ns))) = (
        slot_receive_ns.first_key_value(),
        slot_receive_ns.last_key_value(),
    ) {
        ensure!(
            start_slot >= first_slot && end_slot <= last_slot,
            "slot manifest mismatch"
        );
        first_slot = start_slot;
        last_slot = end_slot;
        first_receive_ns = start_ns;
        last_receive_ns = end_ns;
    }
    Ok(Capture {
        format_version: session.metadata.format_version,
        session_id: session.metadata.session_id,
        writes,
        gaps,
        slot_nodes,
        snapshots,
        slots,
        allowlist,
        hashes,
        first_receive_ns,
        last_receive_ns,
        first_slot,
        last_slot,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub first: Pubkey,
    pub second: Pubkey,
    pub base: Pubkey,
    pub quote: Pubkey,
}
pub fn routes(pair: Pair, pools: &[Pubkey]) -> Result<Vec<Route>> {
    ensure!(
        pools.len() == pair.expected_pools(),
        "wrong standard-fee pool count"
    );
    let (base, quote) = pair.tokens();
    let base = Pubkey::from_str(base)?;
    let quote = Pubkey::from_str(quote)?;
    let mut result = Vec::new();
    for &first in pools {
        for &second in pools {
            if first != second {
                result.push(Route {
                    first,
                    second,
                    base,
                    quote,
                });
            }
        }
    }
    Ok(result)
}
pub fn direction(pool: &WhirlpoolMinimal, input: Pubkey, output: Pubkey) -> Result<bool> {
    if pool.token_mint_a == input && pool.token_mint_b == output {
        Ok(true)
    } else if pool.token_mint_b == input && pool.token_mint_a == output {
        Ok(false)
    } else {
        bail!("token alignment failure")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuoteFailure {
    InsufficientState,
    TickArrayAbsent,
    StateGap,
    AdaptiveFeeUnsupported,
    InvalidQuote,
}
impl QuoteFailure {
    pub fn code(self) -> &'static str {
        match self {
            Self::InsufficientState => "insufficient_state",
            Self::TickArrayAbsent => "tick_array_absent",
            Self::StateGap => "state_gap",
            Self::AdaptiveFeeUnsupported => "adaptive_fee_unsupported",
            Self::InvalidQuote => "invalid_quote",
        }
    }
}
fn exact_failure(error: exact::ExactError) -> QuoteFailure {
    match error {
        exact::ExactError::AdaptiveFeeUnsupported => QuoteFailure::AdaptiveFeeUnsupported,
        exact::ExactError::MissingTickArray { .. } => QuoteFailure::InsufficientState,
        _ => QuoteFailure::InvalidQuote,
    }
}
pub fn quote_leg(
    state: &MarketState,
    program: &Pubkey,
    pool_id: &Pubkey,
    input: Pubkey,
    output: Pubkey,
    amount: u64,
) -> std::result::Result<u64, QuoteFailure> {
    let entry = state
        .pools
        .get(pool_id)
        .ok_or(QuoteFailure::InsufficientState)?;
    if entry.stale {
        return Err(QuoteFailure::StateGap);
    }
    let pool = entry.pool.as_ref().ok_or(QuoteFailure::InsufficientState)?;
    let a_to_b = direction(pool, input, output).map_err(|_| QuoteFailure::InvalidQuote)?;
    match exact::quote_exact_in(program, pool_id, pool, &entry.arrays, amount, a_to_b) {
        Ok(quote) => Ok(quote.output_amount),
        Err(exact::ExactError::MissingTickArray { start_tick_index }) => {
            let key = exact::derive_tick_array_address(program, pool_id, start_tick_index).0;
            Err(if entry.absent_arrays.contains(&key) {
                QuoteFailure::TickArrayAbsent
            } else {
                QuoteFailure::InsufficientState
            })
        }
        Err(error) => Err(exact_failure(error)),
    }
}
pub fn quote_route(
    state: &MarketState,
    program: &Pubkey,
    route: &Route,
    input: u64,
) -> std::result::Result<u64, QuoteFailure> {
    let base = quote_leg(state, program, &route.first, route.quote, route.base, input)?;
    quote_leg(state, program, &route.second, route.base, route.quote, base)
}
pub fn cost_lamports(
    base_fee: u64,
    priority_micro_lamports: u64,
    compute_units: u64,
    tip: u64,
) -> Result<u64> {
    let priority = u128::from(priority_micro_lamports)
        .checked_mul(u128::from(compute_units))
        .context("priority overflow")?
        .div_ceil(1_000_000);
    Ok(u64::try_from(
        u128::from(base_fee) + priority + u128::from(tip),
    )?)
}
pub fn net(output: u64, input: u64, cost: u64) -> Result<i64> {
    Ok(i64::try_from(
        i128::from(output) - i128::from(input) - i128::from(cost),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    #[test]
    fn synthetic_routes_and_direction() {
        let ids: Vec<_> = (1..=6).map(|i| Pubkey::new_from_array([i; 32])).collect();
        let all = routes(Pair::A, &ids).unwrap();
        assert_eq!(all.len(), 30);
        assert!(all.iter().all(|r| r.first != r.second && r.base != r.quote));
    }
    #[test]
    fn synthetic_fee_units_and_costs() {
        let pool_bytes = zstd::decode_all(
            &include_bytes!("../../whirlpool-core/tests/fixtures/usdc_usdt_pool.bin.zst")[..],
        )
        .unwrap();
        let array_bytes = zstd::decode_all(
            &include_bytes!("../../whirlpool-core/tests/fixtures/usdc_usdt_tick_array.bin.zst")[..],
        )
        .unwrap();
        let mut pool = decode_whirlpool(&pool_bytes).unwrap();
        pool.fee_rate = 400; // Synthetic mutation of a public captured account.
        let pool_id = Pubkey::from_str("4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4").unwrap();
        let program = Pubkey::from_str(PROGRAM).unwrap();
        let array = exact::decode_array(&array_bytes).unwrap();
        let key =
            exact::derive_tick_array_address(&program, &pool_id, array.facade.start_tick_index).0;
        let arrays = BTreeMap::from([(key, array)]);
        let quote =
            exact::quote_exact_in(&program, &pool_id, &pool, &arrays, 1_000_000, true).unwrap();
        assert_eq!(quote.fee_amount, 400); // 400 parts per million = 0.04% of input.
        assert!(direction(&pool, pool.token_mint_a, pool.token_mint_b).unwrap());
        assert!(!direction(&pool, pool.token_mint_b, pool.token_mint_a).unwrap());
        assert!(direction(&pool, pool.token_mint_a, Pubkey::new_unique()).is_err());
        assert_eq!(
            cost_lamports(5_000, 1_000, 400_000, 10_000).unwrap(),
            15_400
        );
        assert_eq!(net(10_000_000, 10_000_000, 15_400).unwrap(), -15_400);
    }
    #[test]
    fn synthetic_gap_marks_all_prior_pools_stale() {
        let mut state = MarketState::default();
        state
            .pools
            .insert(Pubkey::new_unique(), PoolState::default());
        state.gap();
        assert!(state.pools.values().all(|p| p.stale));
    }
    #[test]
    fn synthetic_ordering() {
        let key = Pubkey::new_unique();
        let mut writes = [
            AccountWrite {
                slot: 2,
                receive_ns: 3,
                write_version: 1,
                sequence: 3,
                key,
                data: vec![],
            },
            AccountWrite {
                slot: 1,
                receive_ns: 2,
                write_version: 2,
                sequence: 2,
                key,
                data: vec![],
            },
            AccountWrite {
                slot: 1,
                receive_ns: 1,
                write_version: 1,
                sequence: 1,
                key,
                data: vec![],
            },
        ];
        writes.sort_by_key(|w| (w.slot, w.write_version, w.sequence));
        assert_eq!(
            writes.iter().map(|w| w.sequence).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }
    #[test]
    fn synthetic_snapshot_context_and_absent_array() {
        let raw = zstd::decode_all(
            &include_bytes!("../../whirlpool-core/tests/fixtures/usdc_usdt_pool.bin.zst")[..],
        )
        .unwrap();
        let pool = decode_whirlpool(&raw).unwrap();
        let pool_id = Pubkey::from_str("4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4").unwrap();
        let program = Pubkey::from_str(PROGRAM).unwrap();
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &raw);
        let record = SnapshotRecord::Account {
            version: 1,
            snapshot_id: 1,
            account_kind: "pool".into(),
            pool_id: pool_id.to_string(),
            key: pool_id.to_string(),
            context_slot: 100,
            present: true,
            owner: Some(PROGRAM.into()),
            lamports: Some(1),
            data_base64: Some(encoded),
            sha256: Some(format!("{:x}", sha2::Sha256::digest(&raw))),
        };
        let mut state = MarketState::default();
        let allowlist = BTreeSet::from([pool_id]);
        state.apply_snapshot(&record, &allowlist).unwrap();
        let generation = state.generation(&pool_id);
        state
            .apply(
                &AccountWrite {
                    slot: 99,
                    receive_ns: 99,
                    write_version: 1,
                    sequence: 1,
                    key: pool_id,
                    data: raw.clone(),
                },
                &allowlist,
            )
            .unwrap();
        assert_eq!(state.generation(&pool_id), generation);
        let missing =
            exact::quote_exact_in(&program, &pool_id, &pool, &BTreeMap::new(), 1_000_000, true)
                .unwrap_err();
        let exact::ExactError::MissingTickArray { start_tick_index } = missing else {
            panic!("expected missing array")
        };
        let key = exact::derive_tick_array_address(&program, &pool_id, start_tick_index).0;
        let absent = SnapshotRecord::Account {
            version: 1,
            snapshot_id: 1,
            account_kind: "tick_array".into(),
            pool_id: pool_id.to_string(),
            key: key.to_string(),
            context_slot: 100,
            present: false,
            owner: None,
            lamports: None,
            data_base64: None,
            sha256: None,
        };
        state.apply_snapshot(&absent, &allowlist).unwrap();
        assert_eq!(
            quote_leg(
                &state,
                &program,
                &pool_id,
                pool.token_mint_a,
                pool.token_mint_b,
                1_000_000
            ),
            Err(QuoteFailure::TickArrayAbsent)
        );
        state
            .apply(
                &AccountWrite {
                    slot: 101,
                    receive_ns: 101,
                    write_version: 1,
                    sequence: 2,
                    key: pool_id,
                    data: raw,
                },
                &allowlist,
            )
            .unwrap();
        assert!(state.generation(&pool_id) > generation);
    }
}
