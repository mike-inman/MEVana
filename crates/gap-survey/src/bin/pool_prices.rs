//! 2026-11-01: Offline public-account adapter to the verified survey decoder.
use anyhow::{Context, Result};
use base64::Engine;
use gap_survey::decode::{decode_pool, METEORA, ORCA, RAYDIUM};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};
fn raw(a: &Value) -> Result<Vec<u8>> {
    Ok(base64::engine::general_purpose::STANDARD.decode(
        a["account"]["data"][0]
            .as_str()
            .context("account bytes absent")?,
    )?)
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    anyhow::ensure!(args.len() == 3, "usage: pool_prices INPUT OUTPUT");
    let input: BTreeMap<String, Value> =
        serde_json::from_slice(&std::fs::read(PathBuf::from(&args[1]))?)?;
    let mut out = BTreeMap::new();
    for (address, account) in &input {
        let Some(owner) = account["account"]["owner"].as_str() else {
            continue;
        };
        let venue = match owner {
            ORCA => "Orca",
            RAYDIUM => "Raydium",
            METEORA => "Meteora",
            _ => continue,
        };
        let data = raw(account)?;
        if owner == RAYDIUM
            && data.len() == 117
            && data[..8] == [218, 244, 33, 104, 203, 203, 43, 111]
        {
            continue; // Verified AmmConfig, not a pool account.
        }
        let (d0, d1) = if owner == RAYDIUM && data.len() == 1544 {
            (data[233], data[234])
        } else {
            (0, 0)
        };
        let first = decode_pool(venue, owner, &data, d0, d1, None)?;
        let mint0 = raw(input.get(&first.mint0).context("mint0 missing")?)?;
        let mint1 = raw(input.get(&first.mint1).context("mint1 missing")?)?;
        anyhow::ensure!(mint0.len() >= 82 && mint1.len() >= 82, "short mint");
        if let Some(config_account) = first.config.as_ref().and_then(|key| input.get(key)) {
            anyhow::ensure!(
                config_account["account"]["owner"] == RAYDIUM,
                "configuration owner mismatch"
            );
        }
        let config = first
            .config
            .as_ref()
            .and_then(|key| input.get(key))
            .map(raw)
            .transpose()?;
        let decoded = decode_pool(venue, owner, &data, mint0[44], mint1[44], config.as_deref())?;
        let fee = if venue == "Meteora" {
            decoded.base_fee_bps_micro
        } else {
            decoded.fee_bps_micro
        };
        out.insert(address, json!({"venue":venue,"mint0":decoded.mint0,"mint1":decoded.mint1,"price_scaled":decoded.price_scaled.to_string(),"fee_micro_bps":fee,"fee_kind":if venue=="Meteora" {"MinimumBaseFee"} else {"Measured"},"config":decoded.config,"slot":account["slot"],"bin_step_bps":if venue=="Meteora" {Some(u16::from_le_bytes(data[80..82].try_into()?))} else {None}}));
    }
    std::fs::write(PathBuf::from(&args[2]), serde_json::to_vec_pretty(&out)?)?;
    Ok(())
}
