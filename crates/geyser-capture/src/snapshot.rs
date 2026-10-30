//! Opt-in, bounded, read-only confirmed RPC snapshots. No endpoint or secret is configurable.
use crate::format::SnapshotRecord;
use anyhow::{ensure, Context, Result};
use base64::Engine;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;
use std::{str::FromStr, time::Duration};

pub const ENDPOINT: &str = "https://solana-mainnet.gateway.tatum.io/";
pub const WHIRLPOOL_PROGRAM: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const MAX_CALLS: u64 = 6;
pub const MAX_SNAPSHOTS: u8 = 5;
pub const MAX_RPC_BYTES: u64 = 25_000_000;

#[derive(Default)]
struct Spend {
    calls: u64,
    bytes: u64,
}

pub struct SnapshotRun {
    pub records: Vec<SnapshotRecord>,
    pub calls: u64,
    pub bytes: u64,
    pub failed: bool,
}

fn client() -> Result<Client> {
    ensure!(ENDPOINT.starts_with("https://"), "snapshot TLS required");
    Ok(Client::builder()
        .https_only(true)
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()?)
}

async fn rpc(
    client: &Client,
    key: &str,
    keys: &[String],
    min_slot: Option<u64>,
    spend: &mut Spend,
    remaining_bytes: u64,
) -> Result<Value> {
    let mut options = json!({"commitment":"confirmed","encoding":"base64"});
    if let Some(slot) = min_slot {
        options["minContextSlot"] = json!(slot);
    }
    let body =
        json!({"jsonrpc":"2.0","id":1,"method":"getMultipleAccounts","params":[keys,options]});
    for attempt in 0..=2 {
        ensure!(spend.calls < MAX_CALLS, "snapshot call budget reached");
        spend.calls += 1;
        let sent = client
            .post(ENDPOINT)
            .header("x-api-key", key)
            .json(&body)
            .send()
            .await;
        let Ok(mut response) = sent else {
            if attempt < 2 {
                tokio::time::sleep(Duration::from_millis(250 * (1 << attempt))).await;
                continue;
            }
            anyhow::bail!("snapshot request failed");
        };
        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            anyhow::bail!("snapshot authorization rejected");
        }
        if !status.is_success() {
            if attempt < 2 {
                tokio::time::sleep(Duration::from_millis(250 * (1 << attempt))).await;
                continue;
            }
            anyhow::bail!("snapshot HTTP failure");
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| anyhow::anyhow!("snapshot body failed"))?
        {
            spend.bytes = spend
                .bytes
                .checked_add(chunk.len() as u64)
                .context("snapshot byte overflow")?;
            ensure!(
                spend.bytes <= remaining_bytes,
                "snapshot byte budget reached"
            );
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes).context("invalid snapshot JSON")?;
        ensure!(
            value.get("error").is_none_or(Value::is_null),
            "snapshot RPC error"
        );
        return Ok(value);
    }
    unreachable!()
}

fn account_record(
    snapshot_id: u8,
    account_kind: &str,
    pool_id: &str,
    key: &str,
    context_slot: u64,
    value: &Value,
) -> Result<SnapshotRecord> {
    if value.is_null() {
        return Ok(SnapshotRecord::Account {
            version: 1,
            snapshot_id,
            account_kind: account_kind.into(),
            pool_id: pool_id.into(),
            key: key.into(),
            context_slot,
            present: false,
            owner: None,
            lamports: None,
            data_base64: None,
            sha256: None,
        });
    }
    let owner = value["owner"].as_str().context("missing snapshot owner")?;
    ensure!(owner == WHIRLPOOL_PROGRAM, "wrong snapshot account owner");
    let lamports = value["lamports"]
        .as_u64()
        .context("missing snapshot lamports")?;
    let data = value["data"][0]
        .as_str()
        .context("missing snapshot account data")?;
    ensure!(
        value["data"][1].as_str() == Some("base64"),
        "wrong snapshot encoding"
    );
    let raw = base64::engine::general_purpose::STANDARD.decode(data)?;
    let sha256 = format!("{:x}", Sha256::digest(&raw));
    let record = SnapshotRecord::Account {
        version: 1,
        snapshot_id,
        account_kind: account_kind.into(),
        pool_id: pool_id.into(),
        key: key.into(),
        context_slot,
        present: true,
        owner: Some(owner.into()),
        lamports: Some(lamports),
        data_base64: Some(data.into()),
        sha256: Some(sha256),
    };
    record.validate()?;
    Ok(record)
}

async fn collect(
    client: &Client,
    key: &str,
    allowlist: &[String],
    snapshot_id: u8,
    remaining_bytes: u64,
    spend: &mut Spend,
) -> Result<Vec<SnapshotRecord>> {
    ensure!(allowlist.len() == 18, "snapshot requires 18-pool allowlist");
    let program = Pubkey::from_str(WHIRLPOOL_PROGRAM)?;
    let pools = rpc(client, key, allowlist, None, spend, remaining_bytes).await?;
    let pool_slot = pools["result"]["context"]["slot"]
        .as_u64()
        .context("missing pool context")?;
    let values = pools["result"]["value"]
        .as_array()
        .context("missing pool accounts")?;
    ensure!(
        values.len() == allowlist.len(),
        "pool response count mismatch"
    );
    let mut records = Vec::new();
    let mut array_keys = Vec::<(String, String)>::new();
    for (pool_id, value) in allowlist.iter().zip(values) {
        let record = account_record(snapshot_id, "pool", pool_id, pool_id, pool_slot, value)?;
        if let SnapshotRecord::Account {
            present: true,
            data_base64: Some(data),
            ..
        } = &record
        {
            let raw = base64::engine::general_purpose::STANDARD.decode(data)?;
            let pool = whirlpool_core::decode::decode_whirlpool(&raw)?;
            let span = i32::from(pool.tick_spacing)
                .checked_mul(88)
                .context("tick span overflow")?;
            ensure!(span > 0, "zero tick span");
            let current_start = pool.tick_current_index.div_euclid(span) * span;
            let pool_key = Pubkey::from_str(pool_id)?;
            for offset in -6..=6 {
                let start = current_start
                    .checked_add(offset * span)
                    .context("snapshot tick offset overflow")?;
                let (array_key, _) =
                    whirlpool_core::exact::derive_tick_array_address(&program, &pool_key, start);
                array_keys.push((pool_id.clone(), array_key.to_string()));
            }
        }
        records.push(record);
    }
    for batch in array_keys.chunks(100) {
        let keys: Vec<String> = batch.iter().map(|(_, key)| key.clone()).collect();
        let response = rpc(client, key, &keys, Some(pool_slot), spend, remaining_bytes).await?;
        let context = response["result"]["context"]["slot"]
            .as_u64()
            .context("missing array context")?;
        ensure!(context >= pool_slot, "snapshot context went backwards");
        let values = response["result"]["value"]
            .as_array()
            .context("missing array accounts")?;
        ensure!(values.len() == batch.len(), "array response count mismatch");
        for ((pool_id, key), value) in batch.iter().zip(values) {
            records.push(account_record(
                snapshot_id,
                "tick_array",
                pool_id,
                key,
                context,
                value,
            )?);
        }
    }
    Ok(records)
}

/// `key` is held only in memory and is never formatted into an error or record.
pub async fn run(
    allowlist: &[String],
    key: &str,
    snapshot_id: u8,
    remaining_bytes: u64,
) -> SnapshotRun {
    let mut spend = Spend::default();
    let mut records = Vec::new();
    let result = match client() {
        Ok(client) => {
            collect(
                &client,
                key,
                allowlist,
                snapshot_id,
                remaining_bytes,
                &mut spend,
            )
            .await
        }
        Err(_) => Err(anyhow::anyhow!("snapshot client unavailable")),
    };
    let failed = result.is_err();
    match result {
        Ok(mut accounts) => records.append(&mut accounts),
        Err(_) => records.push(SnapshotRecord::Failed {
            version: 1,
            snapshot_id,
            reason: "snapshot_failed".into(),
        }),
    }
    records.push(SnapshotRecord::Stats {
        version: 1,
        snapshot_id,
        calls: spend.calls,
        rpc_bytes: spend.bytes,
    });
    SnapshotRun {
        records,
        calls: spend.calls,
        bytes: spend.bytes,
        failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_snapshot_budget_and_endpoint_are_fixed() {
        assert!(ENDPOINT.starts_with("https://"));
        assert_eq!(MAX_CALLS, 6);
        assert_eq!(MAX_SNAPSHOTS, 5);
        assert_eq!(MAX_RPC_BYTES, 25_000_000);
        let absent = account_record(
            1,
            "tick_array",
            &Pubkey::new_unique().to_string(),
            &Pubkey::new_unique().to_string(),
            42,
            &Value::Null,
        )
        .unwrap();
        absent.validate().unwrap();
    }
}
