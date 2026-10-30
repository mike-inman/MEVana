//! Captured slot graph and causal boundary selection. All times come from frames.
use crate::{AccountWrite, GapEvent, MarketState};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommitmentMode {
    Confirmed,
    Processed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SeenStatus {
    pub receive_ns: u64,
    pub sequence: u64,
}

#[derive(Clone, Debug, Default)]
pub struct SlotNode {
    pub parent: Option<u64>,
    pub statuses: BTreeMap<i32, SeenStatus>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GapClass {
    SkippedLeader,
    AbandonedFork,
    TrueGap,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifiedGap {
    pub reason: String,
    pub previous_slot: Option<u64>,
    pub next_slot: Option<u64>,
    pub class: GapClass,
    pub receive_ns: u64,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Boundary {
    pub slot: u64,
    pub event: SeenStatus,
}

pub fn first_status(nodes: &BTreeMap<u64, SlotNode>, slot: u64, status: i32) -> Option<SeenStatus> {
    nodes.get(&slot)?.statuses.get(&status).copied()
}

pub fn boundaries(nodes: &BTreeMap<u64, SlotNode>, mode: CommitmentMode) -> Vec<Boundary> {
    let status = match mode {
        CommitmentMode::Confirmed => 1,
        CommitmentMode::Processed => 0,
    };
    let mut selected: Vec<_> = nodes
        .iter()
        .filter_map(|(&slot, node)| {
            node.statuses
                .get(&status)
                .copied()
                .map(|event| Boundary { slot, event })
        })
        .collect();
    selected.sort_by_key(|b| (b.event.receive_ns, b.event.sequence, b.slot));
    selected
}

pub fn classify(
    nodes: &BTreeMap<u64, SlotNode>,
    gaps: &[GapEvent],
    first_replay_slot: u64,
) -> Vec<ClassifiedGap> {
    let mut result = Vec::new();
    for gap in gaps {
        let class = if gap.reason != "slot_discontinuity" {
            GapClass::TrueGap
        } else if let (Some(previous), Some(next)) = (gap.previous_slot, gap.next_slot) {
            match nodes.get(&next).and_then(|n| n.parent) {
                Some(parent) if parent == previous => GapClass::SkippedLeader,
                Some(parent) if nodes.contains_key(&parent) => GapClass::AbandonedFork,
                _ => GapClass::TrueGap,
            }
        } else {
            GapClass::TrueGap
        };
        result.push(ClassifiedGap {
            reason: gap.reason.clone(),
            previous_slot: gap.previous_slot,
            next_slot: gap.next_slot,
            class,
            receive_ns: gap.receive_ns,
            sequence: gap.sequence,
        });
    }
    for (&slot, node) in nodes {
        if slot >= first_replay_slot
            && node.statuses.contains_key(&1)
            && node.parent.is_some_and(|p| !nodes.contains_key(&p))
        {
            let event = node.statuses[&1];
            result.push(ClassifiedGap {
                reason: "unseen_parent".into(),
                previous_slot: node.parent,
                next_slot: Some(slot),
                class: GapClass::TrueGap,
                receive_ns: event.receive_ns,
                sequence: event.sequence,
            });
        }
    }
    result.sort_by_key(|g| (g.receive_ns, g.sequence, g.next_slot));
    result
}

pub fn dropped_fork_writes(
    nodes: &BTreeMap<u64, SlotNode>,
    writes: &[AccountWrite],
) -> BTreeMap<u64, u64> {
    let confirmed: BTreeSet<_> = nodes
        .iter()
        .filter(|(_, node)| node.statuses.contains_key(&1))
        .map(|(&slot, _)| slot)
        .collect();
    let mut dropped = BTreeMap::new();
    for write in writes {
        if !confirmed.contains(&write.slot) {
            *dropped.entry(write.slot).or_default() += 1;
        }
    }
    dropped
}

pub fn apply_confirmed_writes(
    state: &mut MarketState,
    nodes: &BTreeMap<u64, SlotNode>,
    slot: u64,
    writes: &[&AccountWrite],
    allowlist: &BTreeSet<Pubkey>,
) -> Result<()> {
    ensure!(
        first_status(nodes, slot, 1).is_some(),
        "unconfirmed slot cannot alter confirmed state"
    );
    for write in writes {
        ensure!(write.slot == slot, "write outside confirmed boundary");
        state.apply(write, allowlist)?;
    }
    Ok(())
}

pub fn validate_confirmed_graph(nodes: &BTreeMap<u64, SlotNode>) -> Result<()> {
    ensure!(
        !boundaries(nodes, CommitmentMode::Confirmed).is_empty(),
        "no confirmed slots"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(parent: u64, processed: u64, confirmed: bool) -> SlotNode {
        let mut statuses = BTreeMap::from([(
            0,
            SeenStatus {
                receive_ns: processed,
                sequence: processed,
            },
        )]);
        if confirmed {
            statuses.insert(
                1,
                SeenStatus {
                    receive_ns: processed + 100,
                    sequence: processed + 100,
                },
            );
        }
        SlotNode {
            parent: Some(parent),
            statuses,
        }
    }
    fn gap(previous: u64, next: u64) -> GapEvent {
        GapEvent {
            reason: "slot_discontinuity".into(),
            previous_slot: Some(previous),
            next_slot: Some(next),
            receive_ns: next,
            sequence: next,
        }
    }
    #[test]
    fn synthetic_skipped_fork_orphan_and_reconnect() {
        let nodes = BTreeMap::from([
            (10, node(9, 10, true)),
            (12, node(10, 12, true)),  // skipped leader 11
            (13, node(12, 13, false)), // abandoned fork
            (15, node(12, 15, true)),  // returns to canonical parent
            (16, node(99, 16, true)),  // orphan parent
        ]);
        let mut gaps = vec![gap(10, 12), gap(13, 15)];
        gaps.push(GapEvent {
            reason: "ping_disconnect".into(),
            previous_slot: None,
            next_slot: Some(15),
            receive_ns: 15,
            sequence: 50,
        });
        let classified = classify(&nodes, &gaps, 12);
        assert_eq!(
            classified
                .iter()
                .filter(|g| g.class == GapClass::SkippedLeader)
                .count(),
            1
        );
        assert_eq!(
            classified
                .iter()
                .filter(|g| g.class == GapClass::AbandonedFork)
                .count(),
            1
        );
        assert_eq!(
            classified
                .iter()
                .filter(|g| g.class == GapClass::TrueGap)
                .count(),
            2
        ); // reconnect and the interior orphan parent 99
        assert_eq!(boundaries(&nodes, CommitmentMode::Confirmed).len(), 4);
        assert_eq!(boundaries(&nodes, CommitmentMode::Processed).len(), 5);
    }
    #[test]
    fn synthetic_fork_write_cannot_reach_confirmed_state() {
        let nodes = BTreeMap::from([(12, node(10, 12, true)), (13, node(12, 13, false))]);
        let id = Pubkey::new_unique();
        let data = zstd::decode_all(
            &include_bytes!("../../whirlpool-core/tests/fixtures/usdc_usdt_pool.bin.zst")[..],
        )
        .unwrap();
        let confirmed = AccountWrite {
            slot: 12,
            receive_ns: 12,
            write_version: 1,
            sequence: 12,
            key: id,
            data: data.clone(),
        };
        let fork = AccountWrite {
            slot: 13,
            receive_ns: 13,
            write_version: 1,
            sequence: 13,
            key: id,
            data,
        };
        let mut state = MarketState::default();
        let allowlist = BTreeSet::from([id]);
        assert!(apply_confirmed_writes(&mut state, &nodes, 13, &[&fork], &allowlist).is_err());
        assert!(state.pools.is_empty());
        apply_confirmed_writes(&mut state, &nodes, 12, &[&confirmed], &allowlist).unwrap();
        assert!(state.pools.get(&id).and_then(|s| s.pool.as_ref()).is_some());
    }
}
