//! 2026-10-29: offline extraction and evaluation of public Whirlpool swaps.
use anyhow::{bail, ensure, Context, Result};
use geyser_capture::{format, layout};
use prost::Message;
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
};
use whirlpool_core::{decode::decode_whirlpool, exact};
use yellowstone_grpc_proto::prelude::{subscribe_update::UpdateOneof, SubscribeUpdate};

#[derive(Clone)]
struct Account {
    session: String,
    key: Pubkey,
    slot: u64,
    write_version: u64,
    sequence: u64,
    signature: Option<Vec<u8>>,
    data: Vec<u8>,
}

impl Account {
    fn order(&self) -> (u64, u64) {
        (self.slot, self.write_version)
    }
}

struct Transaction {
    keys: Vec<Pubkey>,
    pre: BTreeMap<Pubkey, u64>,
    post: BTreeMap<Pubkey, u64>,
    successful: bool,
    slot: u64,
}

struct Collected {
    accounts: Vec<Account>,
    transactions: BTreeMap<Vec<u8>, Transaction>,
    gaps: Vec<(String, u64)>,
}

#[derive(Serialize)]
struct Case {
    version: u32,
    source: &'static str,
    signature: String,
    slot: u64,
    pool: String,
    tick_spacing: u16,
    a_to_b: bool,
    observed_input: u64,
    observed_output: u64,
    pre_pool: Vec<u8>,
    post_pool: Vec<u8>,
    arrays: BTreeMap<String, Vec<u8>>,
    result: String,
    exact_in: Option<(u64, u64, u64)>,
    exact_out: Option<(u64, u64, u64)>,
    crossed_tick: bool,
}

#[derive(Serialize, Default)]
struct Report {
    version: u32,
    source: &'static str,
    pool_updates: usize,
    successful_single_pool_posts: usize,
    cases_total: usize,
    evaluable: usize,
    exact_matches: usize,
    current_array_available: usize,
    exact_in_missing_array: usize,
    exact_out_missing_array: usize,
    arrays_by_case: BTreeMap<usize, usize>,
    array_pda_le_matches: usize,
    array_pda_decimal_matches: usize,
    tick_crossings: usize,
    a_to_b: usize,
    b_to_a: usize,
    classes: BTreeMap<String, usize>,
    per_pool: BTreeMap<String, BTreeMap<String, usize>>,
    unexplained_examples: Vec<String>,
}

fn key(bytes: &[u8]) -> Result<Pubkey> {
    ensure!(bytes.len() == 32, "invalid account key length");
    Ok(Pubkey::new_from_array(bytes.try_into()?))
}

fn tx_from_update(
    update: yellowstone_grpc_proto::prelude::SubscribeUpdateTransaction,
) -> Result<Option<(Vec<u8>, Transaction)>> {
    let info = update.transaction.context("transaction info absent")?;
    let meta = info.meta.context("transaction meta absent")?;
    let message = info
        .transaction
        .context("transaction absent")?
        .message
        .context("message absent")?;
    let mut keys: Vec<_> = message
        .account_keys
        .iter()
        .map(|bytes| key(bytes))
        .collect::<Result<_>>()?;
    keys.extend(
        meta.loaded_writable_addresses
            .iter()
            .map(|bytes| key(bytes))
            .collect::<Result<Vec<_>>>()?,
    );
    keys.extend(
        meta.loaded_readonly_addresses
            .iter()
            .map(|bytes| key(bytes))
            .collect::<Result<Vec<_>>>()?,
    );
    let balances =
        |rows: &[yellowstone_grpc_proto::prelude::TokenBalance]| -> Result<BTreeMap<Pubkey, u64>> {
            let mut result = BTreeMap::new();
            for row in rows {
                let account = keys
                    .get(row.account_index as usize)
                    .context("token balance index outside keys")?;
                let amount = row
                    .ui_token_amount
                    .as_ref()
                    .context("token amount absent")?
                    .amount
                    .parse()?;
                result.insert(*account, amount);
            }
            Ok(result)
        };
    let pre = balances(&meta.pre_token_balances)?;
    let post = balances(&meta.post_token_balances)?;
    Ok(Some((
        info.signature,
        Transaction {
            keys,
            pre,
            post,
            successful: meta.err.is_none(),
            slot: update.slot,
        },
    )))
}

fn files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    for session in fs::read_dir(root)? {
        let session = session?;
        if !session.file_type()?.is_dir()
            || !session
                .file_name()
                .to_string_lossy()
                .starts_with("capture-")
        {
            continue;
        }
        for file in fs::read_dir(session.path())? {
            let file = file?;
            if file.path().extension().is_some_and(|ext| ext == "cap") {
                result.push(file.path());
            }
        }
    }
    result.sort();
    ensure!(result.len() == 6, "expected six finalized capture files");
    Ok(result)
}

fn collect(root: &Path, fixture: Option<&Path>) -> Result<Collected> {
    let mut accounts = Vec::new();
    let mut transactions = BTreeMap::new();
    let mut gaps = Vec::new();
    let mut best_dynamic = Vec::new();
    for path in files(root)? {
        let session = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut reader = BufReader::new(File::open(path)?);
        let mut magic = [0; 8];
        reader.read_exact(&mut magic)?;
        ensure!(&magic == format::MAGIC, "wrong capture magic");
        while let Some(record) = format::read_record(&mut reader)? {
            if record.gap {
                gaps.push((session.clone(), record.sequence));
                continue;
            }
            let update = SubscribeUpdate::decode(record.payload.as_slice())?;
            match update.update_oneof {
                Some(UpdateOneof::Account(account)) => {
                    let info = account.account.context("account info absent")?;
                    if layout::kind(&info.data) == "dynamic" && info.data.len() > best_dynamic.len()
                    {
                        best_dynamic = info.data.clone();
                    }
                    if matches!(layout::kind(&info.data), "pool" | "fixed" | "dynamic") {
                        accounts.push(Account {
                            session: session.clone(),
                            key: key(&info.pubkey)?,
                            slot: account.slot,
                            write_version: info.write_version,
                            sequence: record.sequence,
                            signature: info.txn_signature,
                            data: info.data,
                        });
                    }
                }
                Some(UpdateOneof::Transaction(tx)) => {
                    if let Some((sig, tx)) = tx_from_update(tx)? {
                        transactions.insert(sig, tx);
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(target) = fixture {
        ensure!(!best_dynamic.is_empty(), "no dynamic array captured");
        fs::write(target, zstd::encode_all(best_dynamic.as_slice(), 19)?)?;
    }
    Ok(Collected {
        accounts,
        transactions,
        gaps,
    })
}

fn classify(report: &mut Report, pool: &str, class: &str) {
    *report.classes.entry(class.to_string()).or_default() += 1;
    *report
        .per_pool
        .entry(pool.to_string())
        .or_default()
        .entry(class.to_string())
        .or_default() += 1;
}

fn crossed_initialized_tick(
    arrays: &BTreeMap<Pubkey, exact::DecodedTickArray>,
    spacing: u16,
    before: i32,
    after: i32,
    a_to_b: bool,
) -> bool {
    arrays.values().any(|array| {
        array.facade.ticks.iter().enumerate().any(|(i, tick)| {
            let index = array.facade.start_tick_index + i as i32 * i32::from(spacing);
            tick.initialized
                && if a_to_b {
                    after < index && index <= before
                } else {
                    before < index && index <= after
                }
        })
    })
}

fn evaluate(root: &Path, output: &Path, fixture: Option<&Path>) -> Result<Report> {
    let allowlist: BTreeSet<Pubkey> =
        serde_json::from_slice::<Vec<String>>(&fs::read("crates/geyser-capture/allowlist.json")?)?
            .into_iter()
            .map(|s| Pubkey::from_str(&s))
            .collect::<std::result::Result<_, _>>()?;
    let program_id = Pubkey::from_str(shared_core::programs::ORCA_WHIRLPOOL)?;
    let Collected {
        accounts,
        transactions,
        gaps,
    } = collect(root, fixture)?;
    let mut pools: BTreeMap<Pubkey, Vec<&Account>> = BTreeMap::new();
    let mut arrays: BTreeMap<Pubkey, Vec<&Account>> = BTreeMap::new();
    for account in &accounts {
        match layout::kind(&account.data) {
            "pool" if allowlist.contains(&account.key) => {
                pools.entry(account.key).or_default().push(account)
            }
            "fixed" | "dynamic" => arrays.entry(account.key).or_default().push(account),
            _ => continue,
        }
    }
    for history in pools.values_mut().chain(arrays.values_mut()) {
        history.sort_by_key(|a| a.order());
    }
    let mut report = Report {
        version: 1,
        source: "Real public on-chain data; quote correctness only",
        ..Default::default()
    };
    report.pool_updates = pools.values().map(Vec::len).sum();
    for history in arrays.values() {
        let account = history[0];
        let decoded = exact::decode_array(&account.data)?;
        if !allowlist.contains(&decoded.whirlpool) {
            continue;
        }
        let start = decoded.facade.start_tick_index;
        let (le, _) =
            whirlpool_core::pda::derive_tick_array_address(&program_id, &decoded.whirlpool, start);
        let (decimal, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                decoded.whirlpool.as_ref(),
                start.to_string().as_bytes(),
            ],
            &program_id,
        );
        if le == account.key {
            report.array_pda_le_matches += 1;
        }
        if decimal == account.key {
            report.array_pda_decimal_matches += 1;
        }
    }
    fs::create_dir_all(output)?;
    let mut writer = std::io::BufWriter::new(File::create(output.join("cases-v1.jsonl"))?);
    for (pool_id, history) in &pools {
        let pool_name = pool_id.to_string();
        for (i, post) in history.iter().enumerate() {
            let Some(sig) = post.signature.as_ref() else {
                continue;
            };
            let Some(tx) = transactions.get(sig) else {
                continue;
            };
            if !tx.successful
                || tx.slot != post.slot
                || tx.keys.iter().filter(|k| allowlist.contains(k)).count() != 1
            {
                continue;
            }
            let post_pool = decode_whirlpool(&post.data)?;
            if exact::pool_facade(&post_pool).is_err() {
                continue;
            }
            report.successful_single_pool_posts += 1;
            report.cases_total += 1;
            let signature = bs58::encode(sig).into_string();
            let class = if i == 0 {
                Some("missing_pre_state")
            } else if history[i - 1].signature.as_ref() == Some(sig) {
                Some("multiple_swaps_same_pool")
            } else if history[i - 1].session != post.session {
                Some("missing_pre_state")
            } else if gaps.iter().any(|(s, g)| {
                s == &post.session && *g > history[i - 1].sequence && *g < post.sequence
            }) {
                Some("missing_intermediate_write")
            } else {
                None
            };
            if let Some(class) = class {
                classify(&mut report, &pool_name, class);
                continue;
            }
            let pre = history[i - 1];
            let pre_pool = decode_whirlpool(&pre.data)?;
            let amounts = [pre_pool.token_vault_a, pre_pool.token_vault_b]
                .map(|vault| Some((*tx.pre.get(&vault)?, *tx.post.get(&vault)?)));
            let [Some((pre_a, post_a)), Some((pre_b, post_b))] = amounts else {
                classify(&mut report, &pool_name, "non_swap_state_change");
                continue;
            };
            let (a_to_b, input, output) = if post_a > pre_a && pre_b > post_b {
                (true, post_a - pre_a, pre_b - post_b)
            } else if post_b > pre_b && pre_a > post_a {
                (false, post_b - pre_b, pre_a - post_a)
            } else {
                classify(&mut report, &pool_name, "non_swap_state_change");
                continue;
            };
            let mut state = BTreeMap::new();
            let mut bytes = BTreeMap::new();
            let (_, start, _) = whirlpool_core::pda::three_tick_arrays_around(
                pre_pool.tick_current_index,
                i32::from(pre_pool.tick_spacing),
            );
            let span = 88 * i32::from(pre_pool.tick_spacing);
            for step in 0..6 {
                let target_start = if a_to_b {
                    start - step * span
                } else {
                    start + step * span
                };
                let (key, _) = whirlpool_core::exact::derive_tick_array_address(
                    &program_id,
                    pool_id,
                    target_start,
                );
                let Some(history) = arrays.get(&key) else {
                    break;
                };
                let Some(account) = history.iter().rev().find(|a| a.order() <= pre.order()) else {
                    break;
                };
                let decoded = exact::decode_array(&account.data)?;
                state.insert(key, decoded);
                bytes.insert(key.to_string(), account.data.clone());
            }
            let exact_in =
                exact::quote_exact_in(&program_id, pool_id, &pre_pool, &state, input, a_to_b);
            let exact_out =
                exact::quote_exact_out(&program_id, pool_id, &pre_pool, &state, output, a_to_b);
            *report.arrays_by_case.entry(state.len()).or_default() += 1;
            if !state.is_empty() {
                report.current_array_available += 1;
            }
            if matches!(exact_in, Err(exact::ExactError::MissingTickArray { .. })) {
                report.exact_in_missing_array += 1;
            }
            if matches!(exact_out, Err(exact::ExactError::MissingTickArray { .. })) {
                report.exact_out_missing_array += 1;
            }
            let enough = !matches!(exact_in, Err(exact::ExactError::MissingTickArray { .. }))
                && !matches!(exact_out, Err(exact::ExactError::MissingTickArray { .. }));
            if !enough {
                classify(&mut report, &pool_name, "insufficient_tick_arrays");
                continue;
            }
            report.evaluable += 1;
            let exact_in = exact_in.ok();
            let exact_out = exact_out.ok();
            let match_in =
                exact_in.is_some_and(|q| q.input_amount == input && q.output_amount == output);
            let match_out =
                exact_out.is_some_and(|q| q.input_amount == input && q.output_amount == output);
            let matched = match_in || match_out;
            let crossed_tick = crossed_initialized_tick(
                &state,
                pre_pool.tick_spacing,
                pre_pool.tick_current_index,
                post_pool.tick_current_index,
                a_to_b,
            );
            let result = if matched {
                "exact_match"
            } else if pre_pool.liquidity != post_pool.liquidity && !crossed_tick {
                "non_swap_state_change"
            } else {
                "unexplained"
            };
            classify(&mut report, &pool_name, result);
            if matched {
                report.exact_matches += 1;
                if crossed_tick {
                    report.tick_crossings += 1;
                    *report
                        .per_pool
                        .entry(pool_name.clone())
                        .or_default()
                        .entry("tick_crossing".into())
                        .or_default() += 1;
                }
                if a_to_b {
                    report.a_to_b += 1;
                    *report
                        .per_pool
                        .entry(pool_name.clone())
                        .or_default()
                        .entry("a_to_b".into())
                        .or_default() += 1;
                } else {
                    report.b_to_a += 1;
                    *report
                        .per_pool
                        .entry(pool_name.clone())
                        .or_default()
                        .entry("b_to_a".into())
                        .or_default() += 1;
                }
            } else if result == "unexplained" && report.unexplained_examples.len() < 20 {
                report
                    .unexplained_examples
                    .push(format!("{signature}@{}", post.slot));
            }
            let case = Case {
                version: 1,
                source: "Real public on-chain data; quote correctness only",
                signature,
                slot: post.slot,
                pool: pool_name.clone(),
                tick_spacing: pre_pool.tick_spacing,
                a_to_b,
                observed_input: input,
                observed_output: output,
                pre_pool: pre.data.clone(),
                post_pool: post.data.clone(),
                arrays: bytes,
                result: result.to_owned(),
                exact_in: exact_in.map(|q| (q.input_amount, q.output_amount, q.fee_amount)),
                exact_out: exact_out.map(|q| (q.input_amount, q.output_amount, q.fee_amount)),
                crossed_tick,
            };
            serde_json::to_writer(&mut writer, &case)?;
            writer.write_all(b"\n")?;
        }
    }
    writer.flush()?;
    fs::write(
        output.join("report-v1.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let root = args
        .next()
        .context("usage: swap_cases CAPTURE_ROOT OUTPUT_ROOT [DYNAMIC_FIXTURE]")?;
    let output = args.next().context("missing output root")?;
    let fixture = args.next().map(PathBuf::from);
    if args.next().is_some() {
        bail!("too many arguments");
    }
    let root = PathBuf::from(root);
    ensure!(
        root.ends_with("handoff09-20261029"),
        "only handoff 09 capture root allowed"
    );
    let report = evaluate(&root, &PathBuf::from(output), fixture.as_deref())?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use yellowstone_grpc_proto::prelude::{
        Message as ProtoMessage, SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo,
        TokenBalance, Transaction as ProtoTransaction, TransactionStatusMeta, UiTokenAmount,
    };

    #[test]
    fn loaded_lookup_address_indexes_resolve_token_balances() {
        let static_key = Pubkey::new_unique();
        let loaded = Pubkey::new_unique();
        let balance = |amount: &str| TokenBalance {
            account_index: 1,
            ui_token_amount: Some(UiTokenAmount {
                amount: amount.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let update = SubscribeUpdateTransaction {
            slot: 42,
            transaction: Some(SubscribeUpdateTransactionInfo {
                signature: vec![7; 64],
                transaction: Some(ProtoTransaction {
                    message: Some(ProtoMessage {
                        account_keys: vec![static_key.to_bytes().to_vec()],
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                meta: Some(TransactionStatusMeta {
                    loaded_writable_addresses: vec![loaded.to_bytes().to_vec()],
                    pre_token_balances: vec![balance("100")],
                    post_token_balances: vec![balance("120")],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        let (_, tx) = tx_from_update(update).unwrap().unwrap();
        assert_eq!(tx.keys[1], loaded);
        assert_eq!(tx.pre[&loaded], 100);
        assert_eq!(tx.post[&loaded], 120);
    }

    #[test]
    fn crossing_requires_an_initialized_tick() {
        let key = Pubkey::new_unique();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&whirlpool_core::decode::anchor_discriminator(
            "account:DynamicTickArray",
        ));
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes.extend_from_slice(Pubkey::new_unique().as_ref());
        bytes.extend_from_slice(&(1_u128 << 4).to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        bytes.push(1);
        bytes.extend_from_slice(&[0; 112]);
        bytes.extend_from_slice(&[0; 83]);
        let arrays = BTreeMap::from([(key, exact::decode_array(&bytes).unwrap())]);
        assert!(!crossed_initialized_tick(&arrays, 1, 0, 3, false));
        assert!(crossed_initialized_tick(&arrays, 1, 0, 4, false));
        assert!(crossed_initialized_tick(&arrays, 1, 4, 3, true));
    }
}
