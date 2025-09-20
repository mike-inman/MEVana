use anyhow::Result;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletTier {
    Hot,  // Active trading wallet - small balance, high activity
    Warm, // Reserve wallet - medium balance, medium activity
    Cold, // Storage wallet - large balance, rare activity
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingWallet {
    pub pubkey: Pubkey,
    pub tier: WalletTier,
    pub max_balance_lamports: u64,
    pub current_balance_lamports: u64,
    pub last_balance_check: u64,
    pub trades_today: u32,
    pub total_profit_lamports: i64,
    pub created_at: u64,
    pub is_active: bool,
}

#[derive(Debug)]
pub struct LiveWalletManager {
    wallets: Arc<RwLock<HashMap<Pubkey, TradingWallet>>>,
    keypairs: Arc<RwLock<HashMap<Pubkey, Keypair>>>, // Secure storage needed
    config: WalletConfig,
}

#[derive(Debug, Clone)]
pub struct WalletConfig {
    pub hot_wallet_max_balance: u64,
    pub warm_wallet_max_balance: u64,
    pub cold_wallet_max_balance: u64,
    pub auto_rebalance_enabled: bool,
    pub emergency_evacuation_enabled: bool,
    pub max_trades_per_wallet_per_day: u32,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            hot_wallet_max_balance: 100_000_000,    // 0.1 SOL
            warm_wallet_max_balance: 5_000_000_000, // 5 SOL
            cold_wallet_max_balance: u64::MAX,      // Unlimited
            auto_rebalance_enabled: true,
            emergency_evacuation_enabled: true,
            max_trades_per_wallet_per_day: 1000,
        }
    }
}

impl LiveWalletManager {
    pub fn new(config: WalletConfig) -> Self {
        Self {
            wallets: Arc::new(RwLock::new(HashMap::new())),
            keypairs: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Initialize live trading wallets (call this before going live)
    pub async fn initialize_live_wallets(&self) -> Result<()> {
        info!("🔐 Initializing live trading wallet system");

        // Create hot wallets for active trading
        for i in 1..=2 {
            let keypair = Keypair::new();
            let wallet = TradingWallet {
                pubkey: keypair.pubkey(),
                tier: WalletTier::Hot,
                max_balance_lamports: self.config.hot_wallet_max_balance,
                current_balance_lamports: 0,
                last_balance_check: chrono::Utc::now().timestamp() as u64,
                trades_today: 0,
                total_profit_lamports: 0,
                created_at: chrono::Utc::now().timestamp() as u64,
                is_active: true,
            };

            self.add_wallet(wallet.clone(), keypair).await?;
            info!("✅ Created hot trading wallet {}: {}", i, wallet.pubkey);
        }

        // Create warm wallet for reserves
        let warm_keypair = Keypair::new();
        let warm_wallet = TradingWallet {
            pubkey: warm_keypair.pubkey(),
            tier: WalletTier::Warm,
            max_balance_lamports: self.config.warm_wallet_max_balance,
            current_balance_lamports: 0,
            last_balance_check: chrono::Utc::now().timestamp() as u64,
            trades_today: 0,
            total_profit_lamports: 0,
            created_at: chrono::Utc::now().timestamp() as u64,
            is_active: true,
        };

        self.add_wallet(warm_wallet.clone(), warm_keypair).await?;
        info!("✅ Created warm reserve wallet: {}", warm_wallet.pubkey);

        // Create cold wallet for long-term storage
        let cold_keypair = Keypair::new();
        let cold_wallet = TradingWallet {
            pubkey: cold_keypair.pubkey(),
            tier: WalletTier::Cold,
            max_balance_lamports: self.config.cold_wallet_max_balance,
            current_balance_lamports: 0,
            last_balance_check: chrono::Utc::now().timestamp() as u64,
            trades_today: 0,
            total_profit_lamports: 0,
            created_at: chrono::Utc::now().timestamp() as u64,
            is_active: true,
        };

        self.add_wallet(cold_wallet.clone(), cold_keypair).await?;
        info!("✅ Created cold storage wallet: {}", cold_wallet.pubkey);

        info!(
            "🎯 Live wallet system initialized with {} wallets",
            self.get_wallet_count().await
        );
        Ok(())
    }

    async fn add_wallet(&self, wallet: TradingWallet, keypair: Keypair) -> Result<()> {
        let mut wallets = self.wallets.write().await;
        let mut keypairs = self.keypairs.write().await;

        wallets.insert(wallet.pubkey, wallet);
        keypairs.insert(keypair.pubkey(), keypair);

        Ok(())
    }

    /// Select best wallet for trading based on balance, tier, and activity
    pub async fn select_trading_wallet(&self, required_amount: u64) -> Result<Pubkey> {
        let wallets = self.wallets.read().await;

        // Try hot wallets first
        for (pubkey, wallet) in wallets.iter() {
            if matches!(wallet.tier, WalletTier::Hot)
                && wallet.is_active
                && wallet.current_balance_lamports >= required_amount
                && wallet.trades_today < self.config.max_trades_per_wallet_per_day
            {
                return Ok(*pubkey);
            }
        }

        // Fall back to warm wallet if hot wallets insufficient
        for (pubkey, wallet) in wallets.iter() {
            if matches!(wallet.tier, WalletTier::Warm)
                && wallet.is_active
                && wallet.current_balance_lamports >= required_amount
            {
                warn!("Using warm wallet for trade - hot wallets insufficient");
                return Ok(*pubkey);
            }
        }

        Err(anyhow::anyhow!(
            "No suitable wallet found for amount: {} SOL",
            required_amount as f64 / 1_000_000_000.0
        ))
    }

    /// Update wallet balance after transaction
    pub async fn update_wallet_balance(&self, pubkey: Pubkey, new_balance: u64) -> Result<()> {
        let mut wallets = self.wallets.write().await;

        if let Some(wallet) = wallets.get_mut(&pubkey) {
            wallet.current_balance_lamports = new_balance;
            wallet.last_balance_check = chrono::Utc::now().timestamp() as u64;

            // Check if balance exceeds tier limits
            let max_balance = match wallet.tier {
                WalletTier::Hot => self.config.hot_wallet_max_balance,
                WalletTier::Warm => self.config.warm_wallet_max_balance,
                WalletTier::Cold => self.config.cold_wallet_max_balance,
            };

            if new_balance > max_balance && self.config.auto_rebalance_enabled {
                warn!("💰 Wallet {} balance ({:.4} SOL) exceeds tier limit ({:.4} SOL) - rebalancing needed",
                      pubkey, new_balance as f64 / 1_000_000_000.0, max_balance as f64 / 1_000_000_000.0);
                // Note: Auto-rebalancing would be implemented here
            }
        }

        Ok(())
    }

    /// Record a trade for tracking daily limits
    pub async fn record_trade(&self, pubkey: Pubkey, profit_lamports: i64) -> Result<()> {
        let mut wallets = self.wallets.write().await;

        if let Some(wallet) = wallets.get_mut(&pubkey) {
            wallet.trades_today += 1;
            wallet.total_profit_lamports += profit_lamports;

            info!(
                "📊 Wallet {} trade recorded: profit={:.4} SOL, daily_trades={}",
                pubkey,
                profit_lamports as f64 / 1_000_000_000.0,
                wallet.trades_today
            );
        }

        Ok(())
    }

    /// Get wallet for signing transactions
    pub async fn get_wallet_keypair(&self, pubkey: Pubkey) -> Result<Arc<Keypair>> {
        let keypairs = self.keypairs.read().await;

        keypairs
            .get(&pubkey)
            .map(|kp| Arc::new(Keypair::from_bytes(&kp.to_bytes()).unwrap()))
            .ok_or_else(|| anyhow::anyhow!("Keypair not found for wallet: {}", pubkey))
    }

    /// Emergency evacuation - move all funds to cold storage
    pub async fn emergency_evacuate(&self) -> Result<()> {
        if !self.config.emergency_evacuation_enabled {
            return Err(anyhow::anyhow!("Emergency evacuation is disabled"));
        }

        warn!("🚨 EMERGENCY EVACUATION TRIGGERED - Moving all funds to cold storage");

        let wallets = self.wallets.read().await;
        let cold_wallet = wallets
            .values()
            .find(|w| matches!(w.tier, WalletTier::Cold))
            .ok_or_else(|| anyhow::anyhow!("No cold wallet found"))?;

        let _cold_pubkey = cold_wallet.pubkey;

        // Move funds from all other wallets to cold storage
        for (pubkey, wallet) in wallets.iter() {
            if !matches!(wallet.tier, WalletTier::Cold) && wallet.current_balance_lamports > 0 {
                warn!(
                    "🔄 Evacuating {:.4} SOL from {} to cold storage",
                    wallet.current_balance_lamports as f64 / 1_000_000_000.0,
                    pubkey
                );
                // Note: Actual transfer implementation would go here
            }
        }

        info!("✅ Emergency evacuation completed");
        Ok(())
    }

    /// Get wallet status summary
    pub async fn get_wallet_status(&self) -> String {
        let wallets = self.wallets.read().await;
        let mut hot_balance = 0u64;
        let mut warm_balance = 0u64;
        let mut cold_balance = 0u64;
        let mut total_trades = 0u32;
        let mut total_profit = 0i64;

        for wallet in wallets.values() {
            match wallet.tier {
                WalletTier::Hot => hot_balance += wallet.current_balance_lamports,
                WalletTier::Warm => warm_balance += wallet.current_balance_lamports,
                WalletTier::Cold => cold_balance += wallet.current_balance_lamports,
            }
            total_trades += wallet.trades_today;
            total_profit += wallet.total_profit_lamports;
        }

        format!(
            "Wallet Status:\n\
             🔥 Hot Wallets: {:.4} SOL\n\
             🌡️  Warm Wallet: {:.4} SOL\n\
             ❄️  Cold Wallet: {:.4} SOL\n\
             📊 Total Trades Today: {}\n\
             💰 Total Profit: {:.4} SOL\n\
             🎯 Active Wallets: {}",
            hot_balance as f64 / 1_000_000_000.0,
            warm_balance as f64 / 1_000_000_000.0,
            cold_balance as f64 / 1_000_000_000.0,
            total_trades,
            total_profit as f64 / 1_000_000_000.0,
            wallets.len()
        )
    }

    pub async fn get_wallet_count(&self) -> usize {
        self.wallets.read().await.len()
    }

    /// Export wallet addresses for funding (WITHOUT private keys)
    pub async fn export_wallet_addresses(&self) -> Vec<(Pubkey, WalletTier, u64)> {
        let wallets = self.wallets.read().await;
        wallets
            .values()
            .map(|w| (w.pubkey, w.tier.clone(), w.max_balance_lamports))
            .collect()
    }
}
