// crates/normalizer-service/src/main.rs

// ---- generated registry from build.rs (normalizer-codegen) ----
mod generated {
    // build.rs writes {OUT_DIR}/registry.rs and program_* modules it references
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

use anyhow::Result;
use clap::Parser;
use normalizer_runtime as rt;
use serde::Deserialize;
use std::{fs, path::PathBuf};

// ---------------- CLI ----------------

#[derive(Parser, Debug)]
#[command(name = "normalizer-service", version)]
enum Cmd {
    /// Normalize a single instruction from a JSON fixture and print NormalizedSwapV1 as JSON
    Fixture { path: PathBuf },
}

fn main() -> Result<()> {
    let cmd = Cmd::parse();
    match cmd {
        Cmd::Fixture { path } => run_fixture(path),
    }
}

// ------------- Fixture format (simple, deterministic) -------------

#[derive(Deserialize)]
struct Fixture {
    slot: u64,
    block_time_unix: i64,
    signature: String,

    /// Whether tx meta indicates success (pass-through to `success` field)
    meta_ok: bool,

    /// One instruction to decode + role-map from generated decoders
    instruction: FxInstruction,

    /// Token balance changes used for amount inference
    token_balance_changes: Vec<FxBalanceChange>,
}

#[derive(Deserialize)]
struct FxInstruction {
    /// Anchor/Program ID that must exist in the generated registry
    program_id: String,

    /// The *transaction* account keys (base58) for index resolution
    account_keys: Vec<String>,

    /// Indices into `account_keys` for this instruction
    account_indices: Vec<u8>,

    /// Base64-encoded instruction data
    data_b64: String,
}

#[derive(Deserialize)]
struct FxBalanceChange {
    owner: String,
    mint: String,
    pre: String,  // decimal string (u128 but keep precision in JSON)
    post: String, // decimal string
}

// ---------------- Helpers ----------------

fn parse_u128(s: &str) -> Result<u128> {
    Ok(s.parse::<u128>()?)
}

// ---------------- Core: run_fixture ----------------

fn run_fixture(path: PathBuf) -> Result<()> {
    // 1) Load fixture
    let txt = fs::read_to_string(&path)?;
    let fx: Fixture = serde_json::from_str(&txt)?;

    // 2) Build TokenBalanceChange vector for runtime inference
    let mut tb: Vec<rt::TokenBalanceChange> = Vec::with_capacity(fx.token_balance_changes.len());
    for c in fx.token_balance_changes {
        tb.push(rt::TokenBalanceChange {
            owner: c.owner,
            mint: c.mint,
            pre: parse_u128(&c.pre)?,
            post: parse_u128(&c.post)?,
        });
    }

    // 3) Decode the instruction using generated decoder for this program id
    use base64::{engine::general_purpose, Engine as _};
    let ix_data = general_purpose::STANDARD.decode(fx.instruction.data_b64)?;
    let pid = fx.instruction.program_id;

    // Ask the generated registry for a decoder function
    let decoder = generated::decoder_for(&pid)
        .ok_or_else(|| anyhow::anyhow!("no decoder registered for program_id={}", pid))?;

    let (decoded_name, roles_by_name) = decoder(
        &ix_data,
        &fx.instruction.account_indices,
        &fx.instruction.account_keys,
    )
    .ok_or_else(|| anyhow::anyhow!("decoder did not recognize this instruction data"))?;

    // 4) Find the program extractor from the generated registry
    let registry = generated::registry();
    let extractor = registry
        .iter()
        .find(|e| e.program_id() == pid)
        .ok_or_else(|| anyhow::anyhow!("no extractor registered for program_id={}", pid))?;

    // 5) Build TxContext and extract a normalized swap (if applicable)
    let ctx = rt::TxContext {
        slot: fx.slot,
        block_time_unix: fx.block_time_unix,
        signature: fx.signature,
        ix_index: 0, // single-instruction fixture; set as needed for multi-ix tests
        program_id: pid,
        meta_ok: fx.meta_ok,
        token_balance_changes: &tb,
        account_data_by_pubkey: None,

        // generated decoder outputs:
        decoded_name: Some(decoded_name),
        decoded_args_json: None,
        decoded_accounts_by_role: Some(roles_by_name),
    };

    if let Some(n) = extractor.try_extract(&ctx) {
        println!("{}", serde_json::to_string_pretty(&n)?);
        Ok(())
    } else {
        anyhow::bail!("extractor returned None (not a swap or missing fields)")
    }
}
