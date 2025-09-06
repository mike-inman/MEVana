// crates/hotpath_arb_engine/src/wallet_manager.rs
//! Wallet management utilities and CLI for adding wallets to the security system

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use solana_sdk::signature::{Keypair, Signer};
use std::fs;
use std::path::Path;

use crate::wallet_security::{WalletSecurityManager, WalletSecurityTier, SecurityPolicy};

/// CLI tool for managing wallets in the MEV trading system
#[derive(Parser)]
#[command(name = "wallet-manager")]
#[command(about = "Manage wallets for MEV trading platform")]
pub struct WalletManagerCli {
    #[command(subcommand)]
    pub command: WalletCommand,
}

#[derive(Subcommand)]
pub enum WalletCommand {
    /// Generate a new wallet keypair
    Generate {
        /// Security tier (hot/warm/cold)
        #[arg(short, long, default_value = "hot")]
        tier: String,
        
        /// Maximum balance in lamports
        #[arg(short, long, default_value = "100000000")]
        max_balance: u64,
        
        /// Output file for private key (optional)
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// Add existing wallet from private key
    Add {
        /// Path to private key file (JSON format)
        #[arg(short, long)]
        key_file: String,
        
        /// Security tier (hot/warm/cold)
        #[arg(short, long, default_value = "hot")]
        tier: String,
        
        /// Maximum balance in lamports
        #[arg(short, long, default_value = "100000000")]
        max_balance: u64,
    },
    
    /// List all registered wallets
    List,
    
    /// Show wallet security status
    Status,
}

pub struct WalletManager {
    security_manager: WalletSecurityManager,
}

impl WalletManager {
    pub fn new() -> Self {
        let security_policy = SecurityPolicy::default();
        let security_manager = WalletSecurityManager::new(security_policy);
        
        Self { security_manager }
    }

    /// Generate a new wallet and add it to the security system
    pub async fn generate_wallet(
        &self,
        tier: WalletSecurityTier,
        max_balance: u64,
        output_file: Option<&str>,
    ) -> Result<String> {
        // Generate new keypair
        let keypair = Keypair::new();
        let pubkey_str = keypair.pubkey().to_string();
        
        // Save private key if output file specified
        if let Some(output_path) = output_file {
            let key_bytes = keypair.to_bytes();
            let key_json = serde_json::json!(key_bytes.to_vec());
            fs::write(output_path, serde_json::to_string_pretty(&key_json)?)?;
            info!("💾 Private key saved to: {}", output_path);
        }
        
        // Register with security manager
        self.security_manager
            .register_wallet(keypair, tier, max_balance)
            .await?;
        
        info!(
            "✅ Generated and registered {:?} wallet: {} (max: {} lamports)",
            tier, pubkey_str, max_balance
        );
        
        Ok(pubkey_str)
    }

    /// Load wallet from private key file and add to security system
    pub async fn add_wallet_from_file(
        &self,
        key_file: &str,
        tier: WalletSecurityTier,
        max_balance: u64,
    ) -> Result<String> {
        // Read and parse private key file
        let key_data = fs::read_to_string(key_file)?;
        let key_bytes: Vec<u8> = if key_data.trim().starts_with('[') {
            // JSON array format
            serde_json::from_str(&key_data)?
        } else if key_data.trim().starts_with('{') {
            // JSON object format
            let json: serde_json::Value = serde_json::from_str(&key_data)?;
            json.as_array()
                .ok_or_else(|| anyhow::anyhow!("Invalid key file format"))?
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect()
        } else {
            // Base58 format
            bs58::decode(key_data.trim()).into_vec()?
        };

        if key_bytes.len() != 64 {
            return Err(anyhow::anyhow!("Invalid private key length: expected 64 bytes, got {}", key_bytes.len()));
        }

        // Create keypair from bytes
        let keypair = Keypair::from_bytes(&key_bytes)?;
        let pubkey_str = keypair.pubkey().to_string();
        
        // Register with security manager
        self.security_manager
            .register_wallet(keypair, tier, max_balance)
            .await?;
        
        info!(
            "✅ Added {:?} wallet from {}: {} (max: {} lamports)",
            tier, key_file, pubkey_str, max_balance
        );
        
        Ok(pubkey_str)
    }

    /// Get security status
    pub async fn get_status(&self) -> Result<String> {
        let status = self.security_manager.get_security_status().await;
        
        Ok(format!(
            r#"🔐 WALLET SECURITY STATUS
========================

Total Wallets: {}
Emergency Shutdown: {}
Successful Trades: {}
Failed Trades: {}
Security Incidents: {}
Total Volume: {} lamports

Last Security Audit: {} seconds ago
"#,
            status.total_wallets,
            if status.emergency_shutdown_active { "🚨 ACTIVE" } else { "❌ DISABLED" },
            status.successful_trades,
            status.failed_trades,
            status.security_incidents,
            status.total_volume_lamports,
            status.last_security_audit
        ))
    }
}

/// Parse security tier from string
impl std::str::FromStr for WalletSecurityTier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hot" => Ok(WalletSecurityTier::Hot),
            "warm" => Ok(WalletSecurityTier::Warm),
            "cold" => Ok(WalletSecurityTier::Cold),
            _ => Err(anyhow::anyhow!(
                "Invalid wallet tier: {}. Use 'hot', 'warm', or 'cold'",
                s
            )),
        }
    }
}

/// CLI entry point
pub async fn run_wallet_cli() -> Result<()> {
    let args = WalletManagerCli::parse();
    let manager = WalletManager::new();

    match args.command {
        WalletCommand::Generate { tier, max_balance, output } => {
            let tier = tier.parse::<WalletSecurityTier>()?;
            let pubkey = manager.generate_wallet(tier, max_balance, output.as_deref()).await?;
            println!("Generated wallet: {}", pubkey);
        }
        
        WalletCommand::Add { key_file, tier, max_balance } => {
            let tier = tier.parse::<WalletSecurityTier>()?;
            let pubkey = manager.add_wallet_from_file(&key_file, tier, max_balance).await?;
            println!("Added wallet: {}", pubkey);
        }
        
        WalletCommand::List => {
            println!("📋 Wallet listing not yet implemented - check /wallet/security endpoint");
        }
        
        WalletCommand::Status => {
            let status = manager.get_status().await?;
            println!("{}", status);
        }
    }

    Ok(())
}

/// Quick wallet generation for testing
pub async fn generate_demo_wallets(security_manager: &WalletSecurityManager) -> Result<Vec<String>> {
    let mut wallets = Vec::new();
    
    // Generate 2 hot wallets
    for i in 1..=2 {
        let keypair = Keypair::new();
        let pubkey = keypair.pubkey();
        security_manager.register_wallet(
            keypair,
            WalletSecurityTier::Hot,
            100_000_000, // 0.1 SOL
        ).await?;
        
        wallets.push(pubkey.to_string());
        info!("✅ Generated demo hot wallet {}: {}", i, pubkey);
    }
    
    // Generate 1 warm wallet
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    security_manager.register_wallet(
        keypair,
        WalletSecurityTier::Warm,
        10_000_000_000, // 10 SOL
    ).await?;
    
    wallets.push(pubkey.to_string());
    info!("✅ Generated demo warm wallet: {}", pubkey);
    
    // Generate 1 cold wallet
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    security_manager.register_wallet(
        keypair,
        WalletSecurityTier::Cold,
        u64::MAX, // No limit
    ).await?;
    
    wallets.push(pubkey.to_string());
    info!("✅ Generated demo cold wallet: {}", pubkey);
    
    Ok(wallets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wallet_generation() {
        let manager = WalletManager::new();
        
        let pubkey = manager
            .generate_wallet(WalletSecurityTier::Hot, 100_000_000, None)
            .await
            .unwrap();
        
        assert!(!pubkey.is_empty());
        assert_eq!(pubkey.len(), 44); // Base58 pubkey length
    }

    #[test]
    fn test_tier_parsing() {
        assert!(matches!("hot".parse::<WalletSecurityTier>().unwrap(), WalletSecurityTier::Hot));
        assert!(matches!("warm".parse::<WalletSecurityTier>().unwrap(), WalletSecurityTier::Warm));
        assert!(matches!("cold".parse::<WalletSecurityTier>().unwrap(), WalletSecurityTier::Cold));
        assert!("invalid".parse::<WalletSecurityTier>().is_err());
    }
}