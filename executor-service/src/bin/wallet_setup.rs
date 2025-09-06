// executor-service/src/bin/wallet_setup.rs
//! Wallet setup and funding verification tool

use anyhow::Result;
use log::{error, info, warn};
use solana_sdk::signature::{read_keypair_file, Signer};
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    info!("🏦 MEVana Wallet Setup Tool");

    // Check for wallet configuration
    let wallet_path = std::env::var("WALLET_KEYPAIR_PATH")
        .unwrap_or_else(|_| "~/.config/solana/id.json".to_string());

    let expanded_path = shellexpand::tilde(&wallet_path);

    info!("📁 Checking wallet at: {}", expanded_path);

    // Verify wallet file exists
    if !fs::metadata(&*expanded_path).is_ok() {
        error!("❌ Wallet file not found: {}", expanded_path);
        error!(
            "💡 Create a wallet with: solana-keygen new --outfile {}",
            expanded_path
        );
        return Ok(());
    }

    // Load the keypair
    match read_keypair_file(&*expanded_path) {
        Ok(keypair) => {
            let pubkey = keypair.pubkey();
            info!("✅ Wallet loaded successfully!");
            info!("📍 Address: {}", pubkey);

            // TODO: Add balance checking via Shyft.to client
            warn!("⚠️  Balance checking not yet implemented");
            warn!("💰 Please manually verify you have sufficient funds:");
            warn!("   - Test Mode: 5-10 SOL minimum");
            warn!("   - Live Trading: 50-100 SOL minimum");

            info!(
                "🔗 Check balance at: https://explorer.solana.com/address/{}",
                pubkey
            );
        }
        Err(e) => {
            error!("❌ Failed to load wallet: {}", e);
            error!("💡 Ensure the keypair file is valid JSON format");
        }
    }

    // Security warnings
    warn!("🔐 SECURITY REMINDERS:");
    warn!("   - Keep your keypair file secure and backed up");
    warn!("   - Never share your private key");
    warn!("   - Use a dedicated wallet for trading");
    warn!("   - Start with small amounts for testing");

    Ok(())
}
