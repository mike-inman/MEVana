//! One-time, read-only public account verification. This never logs credentials.
use anyhow::{ensure, Context, Result};
use clap::Parser;
use geyser_capture::config::{credential, Provider};
use serde_json::{json, Value};
use std::path::PathBuf;

const RPC: &str = "https://solana-mainnet.gateway.tatum.io/";

#[derive(Parser)]
struct Args {
    #[arg(long, required = true, num_args = 1..=100)]
    keys: Vec<String>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "/Users/michaelinman/Repos/MEVana/.env")]
    credentials: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(!args.output.exists(), "output already exists");
    ensure!(
        args.keys
            .iter()
            .all(|key| bs58::decode(key).into_vec().is_ok_and(|v| v.len() == 32)),
        "invalid public key"
    );
    let key = credential(Provider::Tatum, &args.credentials, |name| {
        std::env::var(name).ok()
    })?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let body = json!({"jsonrpc":"2.0","id":1,"method":"getMultipleAccounts","params":[args.keys,{"encoding":"base64","commitment":"confirmed"}]});
    let response = client
        .post(RPC)
        .header("x-api-key", key.expose())
        .json(&body)
        .send()
        .await
        .context("account request failed")?;
    ensure!(response.status().is_success(), "account HTTP failure");
    let bytes = response.bytes().await?;
    ensure!(bytes.len() <= 10_000_000, "account response too large");
    let value: Value = serde_json::from_slice(&bytes).context("invalid account JSON")?;
    ensure!(
        value.get("error").is_none_or(Value::is_null),
        "account RPC error"
    );
    let values = value["result"]["value"]
        .as_array()
        .context("missing account values")?;
    ensure!(
        values.len() == body["params"][0].as_array().unwrap().len(),
        "account count mismatch"
    );
    let output = json!({"contextSlot":value["result"]["context"]["slot"], "accounts":args.keys.iter().zip(values).map(|(address,account)| json!({"address":address,"account":account})).collect::<Vec<_>>()});
    std::fs::write(&args.output, serde_json::to_vec_pretty(&output)?)?;
    println!("Wrote {} public account records", values.len());
    Ok(())
}
