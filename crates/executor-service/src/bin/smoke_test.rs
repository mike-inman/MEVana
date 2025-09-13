use anyhow::Result;
use executor_service::account_registry::{AccountRegistry, AccountRegistryConfig};
use executor_service::token_registry::TokenRegistry; // assuming pub mod exported
use std::path::Path;
use std::time::Duration;

// Simple smoke test:
// 1. Load primary account registry file (first arg) in strict mode.
// 2. Initialize token registry & load snapshot (env TOKEN_REGISTRY_SNAPSHOT_PATH).
// 3. Note a few test mints (optional args) and run short enrichment loop.
// 4. Exit 0 if no errors.

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("Usage: smoke-test <registry.json> [mint1 mint2 ...]");
        std::process::exit(1);
    }
    let registry_path = args.remove(0);
    if !Path::new(&registry_path).exists() {
        anyhow::bail!("Registry file not found: {registry_path}");
    }
    let cfg = AccountRegistryConfig {
        strict: true,
        path: Some(registry_path.clone()),
    };
    let reg = AccountRegistry::load(&cfg)?;
    println!(
        "Loaded registry OK flash_loans={} clmm_pools={} orderbooks={} oracles={}",
        reg.data.flash_loans.len(),
        reg.data.clmm_pools.len(),
        reg.data.orderbooks.len(),
        reg.data.oracles.len()
    );

    let token_reg = TokenRegistry::new();
    let snapshot_path = std::env::var("TOKEN_REGISTRY_SNAPSHOT_PATH")
        .unwrap_or_else(|_| "token_registry_snapshot.json".into());
    let _ = token_reg.load_snapshot_file(&snapshot_path)?;
    for mint in &args {
        token_reg.note_seen_mint(mint);
    }
    // Run a short wait to allow enrichment (noop currently)
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("Smoke test complete.");
    Ok(())
}
