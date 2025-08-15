mod generated {
    // emitted by build.rs (or a stub if codegen fails)
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

use clap::Parser;
use normalizer_runtime as rt;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Parser, Debug)]
#[command(name = "normalizer-service", version)]
enum Cmd {
    /// Normalize a single instruction from a JSON fixture and print NormalizedSwapV1 as JSON
    Fixture { path: PathBuf },
}

#[derive(Deserialize)]
struct Fixture {
    slot: u64,
    block_time_unix: i64,
    signature: String,
    meta_ok: bool,
    instruction: FxInstruction,
    token_balance_changes: Vec<FxBalanceChange>,
}

#[derive(Deserialize)]
struct FxInstruction {
    program_id: String,
    account_keys: Vec<String>, // the tx's message account keys (base58)
    account_indices: Vec<u8>,  // indices into account_keys for this ix
    data_b64: String,          // base64-encoded instruction data
}

#[derive(Deserialize)]
struct FxBalanceChange {
    owner: String,
    mint: String,
    pre: String,  // use string to preserve u128 precision in JSON
    post: String, // same
}

fn parse_u128(s: &str) -> anyhow::Result<u128> {
    Ok(s.parse::<u128>()?)
}

fn main() -> anyhow::Result<()> {
    let cmd = Cmd::parse();

    match cmd {
        Cmd::Fixture { path } => run_fixture(path),
    }
}

fn run_fixture(path: PathBuf) -> anyhow::Result<()> {
    let txt = fs::read_to_string(&path)?;
    let fx: Fixture = serde_json::from_str(&txt)?;

    // Build balance changes in runtime format
    let mut tb: Vec<rt::TokenBalanceChange> = Vec::with_capacity(fx.token_balance_changes.len());
    for c in fx.token_balance_changes {
        tb.push(rt::TokenBalanceChange {
            owner: c.owner,
            mint: c.mint,
            pre: parse_u128(&c.pre)?,
            post: parse_u128(&c.post)?,
        });
    }

    // Decode instruction with generated decoder for this program id
    let ix_data = base64::decode(fx.instruction.data_b64)?;
    let pid = fx.instruction.program_id;
    let decoder = generated::decoder_for(&pid)
        .ok_or_else(|| anyhow::anyhow!("no decoder registered for program_id={}", pid))?;
    let (decoded_name, roles) = decoder(
        &ix_data,
        &fx.instruction.account_indices,
        &fx.instruction.account_keys,
    )
    .ok_or_else(|| anyhow::anyhow!("decoder did not recognize this instruction data"))?;

    // Pick extractor for this program id
    let extractors = generated::registry();
    let mut by_pid: HashMap<String, &Box<dyn rt::SwapExtractor>> = HashMap::new();
    for e in &extractors {
        by_pid.insert(e.program_id().to_string(), e);
    }
    let ex = by_pid
        .get(&pid)
        .ok_or_else(|| anyhow::anyhow!("no extractor registered for program_id={}", pid))?;

    // Build context and extract
    let ctx = rt::TxContext {
        slot: fx.slot,
        block_time_unix: fx.block_time_unix,
        signature: fx.signature,
        ix_index: 0, // set if you test multi-ix
        program_id: pid,
        meta_ok: fx.meta_ok,
        token_balance_changes: &tb,
        account_data_by_pubkey: None,
        decoded_name: Some(decoded_name),
        decoded_args_json: None,
        decoded_accounts_by_role: Some(roles),
    };

    if let Some(n) = ex.try_extract(&ctx) {
        println!("{}", serde_json::to_string_pretty(&n)?);
    } else {
        anyhow::bail!("extractor returned None (not a swap or missing fields)")
    }

    Ok(())
}
