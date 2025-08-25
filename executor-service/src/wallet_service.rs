use anyhow::Result;
use chrono::Datelike;
use dashmap::DashMap;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

use crate::config::ExecutorConfig;
use crate::shyft_client::ShyftClient;

/// Comprehensive wallet service for secure, fault-tolerant trading operations
pub struct WalletService {
    config: Arc<ExecutorConfig>,
    shyft_client: Arc<ShyftClient>,

    // Wallet state tracking
    primary_wallet: String,
    backup_wallets: Vec<String>,
    current_wallet_index: AtomicU64,

    // Balance tracking (in lamports)
    sol_balance: AtomicI64,
    token_balances: Arc<DashMap<String, u64>>, // mint -> balance
    reserved_balances: Arc<DashMap<String, u64>>, // mint -> reserved amount

    // Risk management
    daily_pnl: AtomicI64, // in lamports
    daily_trade_count: AtomicU64,
    last_reset_day: AtomicU64,
    active_trades: Arc<DashMap<String, ActiveTrade>>,

    // Fault tolerance
    last_balance_update: AtomicU64,
    consecutive_failures: AtomicU64,
    wallet_health_status: Arc<DashMap<String, WalletHealth>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTrade {
    pub trade_id: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: u64,
    pub reserved_amount: u64,
    pub start_time: u64,
    pub max_execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletHealth {
    pub address: String,
    pub last_successful_transaction: u64,
    pub consecutive_failures: u64,
    pub is_healthy: bool,
    pub last_balance_check: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletStatus {
    pub primary_wallet: String,
    pub active_wallet: String,
    pub sol_balance_sol: f64,
    pub token_balances: HashMap<String, u64>,
    pub daily_pnl_sol: f64,
    pub daily_trades: u64,
    pub active_trades: u64,
    pub wallet_health: Vec<WalletHealth>,
    pub is_trading_enabled: bool,
}

impl WalletService {
    pub fn new(config: Arc<ExecutorConfig>, shyft_client: Arc<ShyftClient>) -> Result<Self> {
        let primary_wallet = Self::load_wallet_address(&config.wallet_keypair_path)?;
        let backup_wallets: Vec<String> = config
            .wallet_backup_paths
            .iter()
            .filter_map(|path| Self::load_wallet_address(path).ok())
            .collect();

        info!("Wallet service initialized:");
        info!("  Primary wallet: {}", primary_wallet);
        info!("  Backup wallets: {}", backup_wallets.len());

        let service = Self {
            config,
            shyft_client,
            primary_wallet: primary_wallet.clone(),
            backup_wallets,
            current_wallet_index: AtomicU64::new(0),
            sol_balance: AtomicI64::new(0),
            token_balances: Arc::new(DashMap::new()),
            reserved_balances: Arc::new(DashMap::new()),
            daily_pnl: AtomicI64::new(0),
            daily_trade_count: AtomicU64::new(0),
            last_reset_day: AtomicU64::new(Self::current_day()),
            active_trades: Arc::new(DashMap::new()),
            last_balance_update: AtomicU64::new(0),
            consecutive_failures: AtomicU64::new(0),
            wallet_health_status: Arc::new(DashMap::new()),
        };

        // Initialize wallet health status
        service.wallet_health_status.insert(
            primary_wallet,
            WalletHealth {
                address: service.primary_wallet.clone(),
                last_successful_transaction: 0,
                consecutive_failures: 0,
                is_healthy: true,
                last_balance_check: 0,
            },
        );

        Ok(service)
    }

    /// Get current active wallet address
    pub fn get_active_wallet(&self) -> String {
        let index = self.current_wallet_index.load(Ordering::Relaxed) as usize;
        if index == 0 {
            self.primary_wallet.clone()
        } else if index - 1 < self.backup_wallets.len() {
            self.backup_wallets[index - 1].clone()
        } else {
            // Fallback to primary
            self.primary_wallet.clone()
        }
    }

    /// Check if we have sufficient balance for a trade
    pub fn can_execute_trade(&self, token_mint: &str, required_amount: u64) -> Result<bool> {
        // Check daily limits first
        if !self.is_within_daily_limits() {
            return Ok(false);
        }

        // Check if we have too many active trades
        if self.active_trades.len() >= self.config.max_concurrent_trades as usize {
            return Ok(false);
        }

        // Check token balance
        let available_balance = self.get_available_balance(token_mint);
        if available_balance < required_amount {
            return Ok(false);
        }

        // Check SOL balance for transaction fees (minimum 0.01 SOL)
        let sol_balance = self.sol_balance.load(Ordering::Relaxed);
        if sol_balance < 10_000_000 {
            // 0.01 SOL in lamports
            warn!(
                "Insufficient SOL balance for transaction fees: {}",
                sol_balance
            );
            return Ok(false);
        }

        Ok(true)
    }

    /// Reserve balance for a trade
    pub fn reserve_balance(&self, trade_id: String, token_mint: &str, amount: u64) -> Result<()> {
        let available = self.get_available_balance(token_mint);
        if available < amount {
            anyhow::bail!(
                "Insufficient balance to reserve {} of {}",
                amount,
                token_mint
            );
        }

        // Add to reserved balances
        let mut reserved = self
            .reserved_balances
            .entry(token_mint.to_string())
            .or_insert(0);
        *reserved += amount;

        // Track active trade
        let active_trade = ActiveTrade {
            trade_id: trade_id.clone(),
            token_in: token_mint.to_string(),
            token_out: "".to_string(), // Will be updated during execution
            amount_in: amount,
            reserved_amount: amount,
            start_time: chrono::Utc::now().timestamp_millis() as u64,
            max_execution_time_ms: self.config.max_execution_time_ms,
        };

        self.active_trades.insert(trade_id, active_trade);

        info!("Reserved {} of {} for trade", amount, token_mint);
        Ok(())
    }

    /// Release reserved balance (on trade completion or failure)
    pub fn release_balance(&self, trade_id: &str) -> Result<()> {
        if let Some((_, trade)) = self.active_trades.remove(trade_id) {
            let mut reserved = self
                .reserved_balances
                .entry(trade.token_in.clone())
                .or_insert(0);
            *reserved = reserved.saturating_sub(trade.reserved_amount);

            info!(
                "Released {} of {} for trade {}",
                trade.reserved_amount, trade.token_in, trade_id
            );
        }
        Ok(())
    }

    /// Update PnL tracking
    pub fn record_trade_result(&self, profit_loss_lamports: i64) {
        // Reset daily counters if needed
        self.reset_daily_counters_if_needed();

        // Update daily PnL
        self.daily_pnl
            .fetch_add(profit_loss_lamports, Ordering::Relaxed);
        self.daily_trade_count.fetch_add(1, Ordering::Relaxed);

        let daily_pnl_sol = profit_loss_lamports as f64 / 1_000_000_000.0;
        if profit_loss_lamports > 0 {
            info!("Trade profit: {:.6} SOL", daily_pnl_sol);
        } else {
            warn!("Trade loss: {:.6} SOL", daily_pnl_sol);
        }

        // Check if we've hit daily loss limit
        let current_daily_pnl = self.daily_pnl.load(Ordering::Relaxed);
        let daily_loss_limit_lamports = (self.config.daily_loss_limit_sol * 1_000_000_000.0) as i64;

        if current_daily_pnl < -daily_loss_limit_lamports {
            error!(
                "DAILY LOSS LIMIT EXCEEDED: {:.6} SOL",
                current_daily_pnl as f64 / 1_000_000_000.0
            );
        }
    }

    /// Refresh wallet balances from chain
    pub async fn refresh_balances(&self) -> Result<()> {
        let active_wallet = self.get_active_wallet();

        match self.shyft_client.get_wallet_balances(&active_wallet).await {
            Ok(balances) => {
                // Update SOL balance
                self.sol_balance
                    .store(balances.sol_balance as i64, Ordering::Relaxed);

                // Update token balances
                for (mint, balance) in balances.token_balances {
                    self.token_balances.insert(mint, balance);
                }

                self.last_balance_update.store(
                    chrono::Utc::now().timestamp_millis() as u64,
                    Ordering::Relaxed,
                );
                self.consecutive_failures.store(0, Ordering::Relaxed);

                // Update wallet health
                if let Some(mut health) = self.wallet_health_status.get_mut(&active_wallet) {
                    health.is_healthy = true;
                    health.consecutive_failures = 0;
                    health.last_balance_check = chrono::Utc::now().timestamp_millis() as u64;
                }

                Ok(())
            }
            Err(e) => {
                error!("Failed to refresh balances: {}", e);
                self.consecutive_failures.fetch_add(1, Ordering::Relaxed);

                // Mark wallet as unhealthy if too many failures
                if self.consecutive_failures.load(Ordering::Relaxed) > 3 {
                    self.mark_wallet_unhealthy(&active_wallet);
                    self.try_failover_wallet().await?;
                }

                Err(e)
            }
        }
    }

    /// Get comprehensive wallet status
    pub fn get_status(&self) -> WalletStatus {
        WalletStatus {
            primary_wallet: self.primary_wallet.clone(),
            active_wallet: self.get_active_wallet(),
            sol_balance_sol: self.sol_balance.load(Ordering::Relaxed) as f64 / 1_000_000_000.0,
            token_balances: self
                .token_balances
                .iter()
                .map(|entry| (entry.key().clone(), *entry.value()))
                .collect(),
            daily_pnl_sol: self.daily_pnl.load(Ordering::Relaxed) as f64 / 1_000_000_000.0,
            daily_trades: self.daily_trade_count.load(Ordering::Relaxed),
            active_trades: self.active_trades.len() as u64,
            wallet_health: self
                .wallet_health_status
                .iter()
                .map(|entry| entry.value().clone())
                .collect(),
            is_trading_enabled: self.is_trading_enabled(),
        }
    }

    /// Check if trading should be enabled based on risk management
    pub fn is_trading_enabled(&self) -> bool {
        // Check daily loss limit
        let daily_pnl = self.daily_pnl.load(Ordering::Relaxed);
        let daily_loss_limit = (self.config.daily_loss_limit_sol * 1_000_000_000.0) as i64;

        if daily_pnl < -daily_loss_limit {
            return false;
        }

        // Check wallet health
        let active_wallet = self.get_active_wallet();
        if let Some(health) = self.wallet_health_status.get(&active_wallet) {
            if !health.is_healthy {
                return false;
            }
        }

        // Check minimum balance requirements
        let sol_balance = self.sol_balance.load(Ordering::Relaxed);
        let min_balance_lamports = (self.config.min_wallet_balance_sol * 1_000_000_000.0) as i64;

        if sol_balance < min_balance_lamports {
            return false;
        }

        true
    }

    // Private helper methods

    fn load_wallet_address(keypair_path: &str) -> Result<String> {
        let expanded_path = shellexpand::tilde(keypair_path);
        // In real implementation, would load keypair and extract public key
        // For now, return a placeholder
        Ok(format!(
            "wallet_from_{}",
            expanded_path.chars().take(8).collect::<String>()
        ))
    }

    fn get_available_balance(&self, token_mint: &str) -> u64 {
        let total_balance = if token_mint == "SOL" {
            self.sol_balance.load(Ordering::Relaxed) as u64
        } else {
            self.token_balances.get(token_mint).map(|v| *v).unwrap_or(0)
        };

        let reserved = self
            .reserved_balances
            .get(token_mint)
            .map(|v| *v)
            .unwrap_or(0);
        total_balance.saturating_sub(reserved)
    }

    fn is_within_daily_limits(&self) -> bool {
        self.reset_daily_counters_if_needed();

        let daily_pnl = self.daily_pnl.load(Ordering::Relaxed);
        let daily_loss_limit = (self.config.daily_loss_limit_sol * 1_000_000_000.0) as i64;

        daily_pnl > -daily_loss_limit
    }

    fn reset_daily_counters_if_needed(&self) {
        let current_day = Self::current_day();
        let last_reset = self.last_reset_day.load(Ordering::Relaxed);

        if current_day > last_reset {
            self.daily_pnl.store(0, Ordering::Relaxed);
            self.daily_trade_count.store(0, Ordering::Relaxed);
            self.last_reset_day.store(current_day, Ordering::Relaxed);
            info!("Daily counters reset for new day");
        }
    }

    fn current_day() -> u64 {
        chrono::Utc::now().date_naive().ordinal() as u64
    }

    fn mark_wallet_unhealthy(&self, wallet_address: &str) {
        if let Some(mut health) = self.wallet_health_status.get_mut(wallet_address) {
            health.is_healthy = false;
            health.consecutive_failures += 1;
            warn!("Marked wallet {} as unhealthy", wallet_address);
        }
    }

    async fn try_failover_wallet(&self) -> Result<()> {
        let current_index = self.current_wallet_index.load(Ordering::Relaxed);
        let total_wallets = 1 + self.backup_wallets.len() as u64; // primary + backups

        for i in 1..total_wallets {
            let new_index = (current_index + i) % total_wallets;
            let new_wallet = if new_index == 0 {
                &self.primary_wallet
            } else {
                &self.backup_wallets[new_index as usize - 1]
            };

            // Test wallet health
            match self.shyft_client.get_wallet_balances(new_wallet).await {
                Ok(_) => {
                    self.current_wallet_index
                        .store(new_index, Ordering::Relaxed);
                    info!("Failed over to wallet: {}", new_wallet);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failover wallet {} also unhealthy: {}", new_wallet, e);
                    self.mark_wallet_unhealthy(new_wallet);
                }
            }
        }

        anyhow::bail!("All wallets are unhealthy - cannot execute trades")
    }
}
