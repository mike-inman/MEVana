//! Source-derived layouts; see docs/2026-10-29-2200-CDT-geyser-account-layouts.md.
use std::collections::HashMap;
use whirlpool_core::decode::anchor_discriminator;
use yellowstone_grpc_proto::prelude::*;

pub const DISCRIMINATOR: usize = 8;
pub const PUBKEY: usize = 32;
pub const TICK: usize = 1 + 4 * 16 + 3 * 16;
pub const TICKS: usize = 88;
pub const FIXED_POOL_OFFSET: usize = DISCRIMINATOR + 4 + TICKS * TICK;
pub const FIXED_SIZE: usize = FIXED_POOL_OFFSET + PUBKEY;
pub const DYNAMIC_POOL_OFFSET: usize = DISCRIMINATOR + 4;
pub const DYNAMIC_MIN: usize = DISCRIMINATOR + 4 + PUBKEY + 16 + TICKS;
pub const DYNAMIC_DATA: usize = TICK - 1;
pub const POOL_SIZE: usize = DISCRIMINATOR
    + 32
    + 1
    + 4 * 2
    + 2 * 16
    + 4
    + 2 * 8
    + 4 * PUBKEY
    + 2 * 16
    + 8
    + 3 * (3 * PUBKEY + 2 * 16);
pub const POSITION_SIZE: usize =
    DISCRIMINATOR + 2 * PUBKEY + 16 + 2 * 4 + 2 * (16 + 8) + 3 * (16 + 8);
pub const ORACLE_SIZE: usize =
    DISCRIMINATOR + PUBKEY + 8 + (5 * 2 + 2 * 4 + 16) + (2 * 8 + 3 * 4 + 16) + 128;

pub fn disc(name: &str) -> [u8; 8] {
    anchor_discriminator(&format!("account:{name}"))
}
pub fn kind(data: &[u8]) -> &'static str {
    for (name, label) in [
        ("Whirlpool", "pool"),
        ("TickArray", "fixed"),
        ("DynamicTickArray", "dynamic"),
        ("Position", "position"),
        ("Oracle", "oracle"),
    ] {
        if data.get(..8) == Some(disc(name).as_slice()) {
            return label;
        }
    }
    "unknown"
}
fn memcmp(offset: usize, bytes: Vec<u8>) -> SubscribeRequestFilterAccountsFilter {
    SubscribeRequestFilterAccountsFilter {
        filter: Some(subscribe_request_filter_accounts_filter::Filter::Memcmp(
            SubscribeRequestFilterAccountsFilterMemcmp {
                offset: offset as u64,
                data: Some(subscribe_request_filter_accounts_filter_memcmp::Data::Bytes(bytes)),
            },
        )),
    }
}
fn size(n: usize) -> SubscribeRequestFilterAccountsFilter {
    SubscribeRequestFilterAccountsFilter {
        filter: Some(subscribe_request_filter_accounts_filter::Filter::Datasize(
            n as u64,
        )),
    }
}
fn account(filters: Vec<SubscribeRequestFilterAccountsFilter>) -> SubscribeRequestFilterAccounts {
    SubscribeRequestFilterAccounts {
        owner: vec![shared_core::programs::ORCA_WHIRLPOOL.into()],
        filters,
        ..Default::default()
    }
}
pub fn request(allowlist: &[String], transactions: bool, commitment: i32) -> SubscribeRequest {
    let mut r = SubscribeRequest {
        commitment: Some(commitment),
        ..Default::default()
    };
    if transactions {
        if !allowlist.is_empty() {
            r.transactions.insert(
                "pool_transactions".into(),
                SubscribeRequestFilterTransactions {
                    vote: Some(false),
                    failed: None,
                    account_include: allowlist.to_vec(),
                    ..Default::default()
                },
            );
        }
        return r;
    }
    r.slots = HashMap::from([(
        "slots".into(),
        SubscribeRequestFilterSlots {
            filter_by_commitment: Some(false),
            interslot_updates: Some(false),
        },
    )]);
    r.accounts.insert(
        "pools".into(),
        account(vec![size(POOL_SIZE), memcmp(0, disc("Whirlpool").to_vec())]),
    );
    for (i, pool) in allowlist.iter().enumerate() {
        // Keys are validated before this function; Base58 preserves exact public key bytes.
        for (name, offset) in [
            ("TickArray", FIXED_POOL_OFFSET),
            ("DynamicTickArray", DYNAMIC_POOL_OFFSET),
        ] {
            let mut filters = vec![
                memcmp(0, disc(name).to_vec()),
                SubscribeRequestFilterAccountsFilter {
                    filter: Some(subscribe_request_filter_accounts_filter::Filter::Memcmp(
                        SubscribeRequestFilterAccountsFilterMemcmp {
                            offset: offset as u64,
                            data: Some(
                                subscribe_request_filter_accounts_filter_memcmp::Data::Base58(
                                    pool.clone(),
                                ),
                            ),
                        },
                    )),
                },
            ];
            if name == "TickArray" {
                filters.push(size(FIXED_SIZE));
            }
            r.accounts.insert(format!("{name}_{i}"), account(filters));
        }
    }
    r
}
