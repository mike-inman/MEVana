#![allow(dead_code)]

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_ENGINE;
use base64::Engine;
use chrono::Datelike;
use dashmap::DashMap;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::config::ExecutorConfig;
use crate::shyft_client::ShyftClient;
use bincode::serialize;
use reqwest::Client;
use serde_json::Value;
use solana_sdk::hash::Hash;
use solana_sdk::message::Message;
use solana_sdk::native_token::sol_to_lamports;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;

/// Comprehensive wallet service for secure, fault-tolerant trading operations
pub struct WalletService {
    config: Arc<ExecutorConfig>,
    shyft_client: Arc<ShyftClient>,
    http_client: Client,

    wallet_paths: Arc<HashMap<String, String>>,
    wallet_labels: Arc<HashMap<String, String>>,

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

#[derive(Debug, Clone, Serialize)]
pub struct WalletTransferResult {
    pub signature: String,
    pub lamports: u64,
    pub destination: String,
    pub source: String,
    pub recent_blockhash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WalletAirdropResult {
    pub signature: String,
    pub lamports: u64,
    pub wallet: String,
    pub rpc_url: String,
}

fn normalize_path(path: &str) -> String {
    shellexpand::tilde(path).to_string()
}

fn resolve_wallet_label(
    path: &str,
    overrides: &HashMap<String, String>,
    default_label: &str,
    wallet_address: &str,
    used: &mut HashSet<String>,
) -> String {
    let normalized_path = normalize_path(path);
    let base_label = overrides
        .get(&normalized_path)
        .or_else(|| overrides.get(path))
        .or_else(|| overrides.get(wallet_address))
        .cloned()
        .unwrap_or_else(|| default_label.to_string());

    let base = if base_label.is_empty() {
        if default_label.is_empty() {
            format!(
                "wallet_{}",
                wallet_address.chars().take(8).collect::<String>()
            )
        } else {
            default_label.to_string()
        }
    } else {
        base_label
    };

    if used.insert(base.clone()) {
        return base;
    }

    let short = wallet_address.chars().take(8).collect::<String>();
    let mut counter: u32 = 1;
    loop {
        let candidate = format!("{}_{}_{counter}", base, short);
        if used.insert(candidate.clone()) {
            return candidate;
        }
        counter += 1;
    }
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
pub struct WalletInventory {
    pub address: String,
    pub label: String,
    pub tier: String,
    pub is_active: bool,
    pub sol_balance_lamports: u64,
    pub sol_balance_sol: f64,
    pub token_balances: HashMap<String, u64>,
    pub is_healthy: bool,
    pub consecutive_failures: u64,
    pub last_successful_transaction: u64,
    pub last_balance_check_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletStatus {
    pub primary_wallet: String,
    pub active_wallet: String,
    pub active_wallet_label: String,
    pub sol_balance_sol: f64,
    pub token_balances: HashMap<String, u64>,
    pub daily_pnl_sol: f64,
    pub daily_trades: u64,
    pub active_trades: u64,
    pub wallet_health: Vec<WalletHealth>,
    pub is_trading_enabled: bool,
    pub wallet_labels: HashMap<String, String>,
    #[serde(default)]
    pub wallets: Vec<WalletInventory>,
    #[serde(default)]
    pub refreshed_at_ms: u64,
}

impl WalletService {
    pub fn new(config: Arc<ExecutorConfig>, shyft_client: Arc<ShyftClient>) -> Result<Self> {
        let primary_wallet = Self::load_wallet_address(&config.wallet_keypair_path)?;

        let mut wallet_paths: HashMap<String, String> = HashMap::new();
        wallet_paths.insert(
            primary_wallet.clone(),
            normalize_path(&config.wallet_keypair_path),
        );

        let mut used_labels = HashSet::new();
        let mut wallet_labels: HashMap<String, String> = HashMap::new();
        let primary_label = resolve_wallet_label(
            &config.wallet_keypair_path,
            &config.wallet_label_overrides,
            "primary",
            &primary_wallet,
            &mut used_labels,
        );
        wallet_labels.insert(primary_wallet.clone(), primary_label);

        let mut backup_wallets = Vec::new();
        for (idx, path) in config.wallet_backup_paths.iter().enumerate() {
            match Self::load_wallet_address(path) {
                Ok(address) => {
                    wallet_paths.insert(address.clone(), normalize_path(path));
                    let default_label = format!("backup{}", idx + 1);
                    let label = resolve_wallet_label(
                        path,
                        &config.wallet_label_overrides,
                        &default_label,
                        &address,
                        &mut used_labels,
                    );
                    wallet_labels.insert(address.clone(), label);
                    backup_wallets.push(address);
                }
                Err(err) => {
                    warn!("Failed to load backup wallet from {}: {}", path, err);
                }
            }
        }

        info!("Wallet service initialized:");
        info!("  Primary wallet: {}", primary_wallet);
        info!("  Backup wallets: {}", backup_wallets.len());

        let http_client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("failed to build wallet HTTP client")?;

        let mut service = Self {
            config,
            shyft_client,
            http_client,
            wallet_paths: Arc::new(wallet_paths),
            wallet_labels: Arc::new(wallet_labels),
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

        // Initialize wallet health status for primary and backups
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

        for wallet in &service.backup_wallets {
            service.wallet_health_status.insert(
                wallet.clone(),
                WalletHealth {
                    address: wallet.clone(),
                    last_successful_transaction: 0,
                    consecutive_failures: 0,
                    is_healthy: true,
                    last_balance_check: 0,
                },
            );
        }

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

    /// Resolve the configured label for a wallet address (falls back to prefix of the address)
    pub fn wallet_label(&self, wallet_address: &str) -> String {
        self.wallet_labels
            .get(wallet_address)
            .cloned()
            .unwrap_or_else(|| wallet_address.chars().take(8).collect())
    }

    /// Return a clone of the wallet label mapping (address -> label)
    pub fn wallet_label_map(&self) -> HashMap<String, String> {
        self.wallet_labels.as_ref().clone()
    }

    /// Get current cached SOL balance (lamports)
    pub fn get_sol_balance_lamports(&self) -> i64 {
        self.sol_balance.load(Ordering::Relaxed)
    }

    /// Get a snapshot of current token balances (mint -> balance)
    pub fn get_token_balances_snapshot(&self) -> std::collections::HashMap<String, u64> {
        let mut map = std::collections::HashMap::new();
        for kv in self.token_balances.iter() {
            map.insert(kv.key().clone(), *kv.value());
        }
        map
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

    /// Transfer SOL from the active wallet to the provided destination address
    pub async fn transfer_sol(
        &self,
        destination: &str,
        amount_sol: f64,
        skip_preflight: bool,
    ) -> Result<WalletTransferResult> {
        if amount_sol <= 0.0 {
            anyhow::bail!("transfer amount must be positive");
        }

        let lamports = sol_to_lamports(amount_sol);
        if lamports == 0 {
            anyhow::bail!("transfer amount is below 1 lamport");
        }

        let destination_pubkey =
            Pubkey::from_str(destination).context("invalid destination wallet address")?;

        let active_wallet = self.get_active_wallet();
        let keypair = self.load_keypair_for(&active_wallet)?;
        let source_pubkey = keypair.pubkey();

        let current_balance = self.sol_balance.load(Ordering::Relaxed);
        if lamports as i64 >= current_balance {
            anyhow::bail!(
                "insufficient balance: available {} lamports, requested {}",
                current_balance,
                lamports
            );
        }

        let blockhash_str = self.shyft_client.get_recent_blockhash().await?;
        let recent_blockhash =
            Hash::from_str(&blockhash_str).context("failed to parse recent blockhash")?;

        let instruction =
            system_instruction::transfer(&source_pubkey, &destination_pubkey, lamports);
        let message = Message::new(&[instruction], Some(&source_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction
            .try_sign(&[&keypair], recent_blockhash)
            .context("failed to sign transfer transaction")?;

        let serialized =
            serialize(&transaction).context("failed to serialize transfer transaction")?;
        let encoded = BASE64_ENGINE.encode(serialized);

        let signature = self
            .shyft_client
            .send_transaction(&encoded, skip_preflight)
            .await
            .context("failed to submit transfer transaction")?;

        if let Err(err) = self.refresh_balances().await {
            warn!("post-transfer balance refresh failed: {}", err);
        }

        Ok(WalletTransferResult {
            signature,
            lamports,
            destination: destination_pubkey.to_string(),
            source: source_pubkey.to_string(),
            recent_blockhash: blockhash_str,
        })
    }

    /// Request a Solana airdrop for development/test wallets
    pub async fn request_airdrop(
        &self,
        wallet: &str,
        lamports: u64,
        rpc_url: &str,
    ) -> Result<WalletAirdropResult> {
        if lamports == 0 {
            anyhow::bail!("airdrop amount must be at least 1 lamport");
        }

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "requestAirdrop",
            "params": [wallet, lamports]
        });

        let response = self
            .http_client
            .post(rpc_url)
            .json(&payload)
            .send()
            .await
            .context("failed to submit airdrop request")?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .context("failed to parse airdrop response")?;

        if !status.is_success() {
            anyhow::bail!(
                "airdrop RPC returned {}: {}",
                status,
                body.get("error").cloned().unwrap_or(Value::Null)
            );
        }

        if let Some(error) = body.get("error") {
            anyhow::bail!("airdrop RPC error: {}", error);
        }

        let signature = body
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing signature in airdrop response"))?
            .to_string();

        if let Err(err) = self.refresh_balances().await {
            warn!("post-airdrop balance refresh failed: {}", err);
        }

        Ok(WalletAirdropResult {
            signature,
            lamports,
            wallet: wallet.to_string(),
            rpc_url: rpc_url.to_string(),
        })
    }

    /// Get comprehensive wallet status
    pub fn get_status(&self) -> WalletStatus {
        let active_wallet = self.get_active_wallet();
        WalletStatus {
            primary_wallet: self.primary_wallet.clone(),
            active_wallet: active_wallet.clone(),
            active_wallet_label: self.wallet_label(&active_wallet),
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
            wallet_labels: self.wallet_labels.as_ref().clone(),
            wallets: Vec::new(),
            refreshed_at_ms: self.last_balance_update.load(Ordering::Relaxed),
        }
    }

    /// Build an expanded status response for admin consumers with per-wallet snapshots.
    pub async fn get_admin_status(&self) -> WalletStatus {
        let mut status = self.get_status();
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let active_wallet = status.active_wallet.clone();

        let mut snapshots = Vec::new();

        let mut addresses: Vec<String> = Vec::with_capacity(1 + self.backup_wallets.len());
        addresses.push(self.primary_wallet.clone());
        addresses.extend(self.backup_wallets.clone());

        for (idx, address) in addresses.iter().enumerate() {
            let is_active = address == &active_wallet;
            let mut sol_balance_lamports: u64 = 0;
            let mut token_balances: HashMap<String, u64> = HashMap::new();
            let mut fetch_success = false;

            match self.shyft_client.get_wallet_balances(address).await {
                Ok(balances) => {
                    sol_balance_lamports = balances.sol_balance;
                    token_balances = balances.token_balances;
                    fetch_success = true;
                }
                Err(err) => {
                    warn!("failed to fetch balances for wallet {}: {}", address, err);
                    if is_active {
                        let cached_sol = self.sol_balance.load(Ordering::Relaxed);
                        sol_balance_lamports = cached_sol.max(0) as u64;
                        token_balances = self.get_token_balances_snapshot();
                    }
                }
            }

            if is_active && fetch_success {
                let capped = sol_balance_lamports.min(i64::MAX as u64);
                self.sol_balance.store(capped as i64, Ordering::Relaxed);
                self.token_balances.clear();
                for (mint, balance) in &token_balances {
                    self.token_balances.insert(mint.clone(), *balance);
                }
                self.last_balance_update.store(now_ms, Ordering::Relaxed);
            }

            if let Some(mut health) = self.wallet_health_status.get_mut(address) {
                health.last_balance_check = now_ms;
                if fetch_success {
                    health.is_healthy = true;
                    health.consecutive_failures = 0;
                } else {
                    health.is_healthy = false;
                    health.consecutive_failures = health.consecutive_failures.saturating_add(1);
                }
            }

            let (is_healthy, consecutive_failures, last_successful_transaction, last_balance_check) =
                if let Some(health) = self.wallet_health_status.get(address) {
                    let h = health.value();
                    (
                        h.is_healthy,
                        h.consecutive_failures,
                        h.last_successful_transaction,
                        h.last_balance_check,
                    )
                } else {
                    (true, 0, 0, 0)
                };

            let tier = if idx == 0 { "Hot" } else { "Warm" };
            snapshots.push(WalletInventory {
                address: address.clone(),
                label: self.wallet_label(address),
                tier: tier.to_string(),
                is_active,
                sol_balance_lamports,
                sol_balance_sol: sol_balance_lamports as f64 / 1_000_000_000.0,
                token_balances: token_balances.clone(),
                is_healthy,
                consecutive_failures,
                last_successful_transaction,
                last_balance_check_ms: last_balance_check,
            });
        }

        if let Some(active_snapshot) = snapshots.iter().find(|snapshot| snapshot.is_active) {
            status.sol_balance_sol = active_snapshot.sol_balance_sol;
            status.token_balances = active_snapshot.token_balances.clone();
        }

        status.wallet_health = self
            .wallet_health_status
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        status.wallets = snapshots;
        status.refreshed_at_ms = now_ms;
        status
    }

    /// Manually rotate the active wallet to the next available or specified address.
    pub async fn rotate_wallet(&self, target_wallet: Option<&str>) -> Result<String> {
        let total_wallets = 1 + self.backup_wallets.len() as u64;
        if total_wallets <= 1 {
            anyhow::bail!("no backup wallets configured for rotation");
        }

        let desired_index = if let Some(target) = target_wallet {
            if target == self.primary_wallet {
                0
            } else {
                let position = self
                    .backup_wallets
                    .iter()
                    .position(|addr| addr == target)
                    .ok_or_else(|| {
                        anyhow::anyhow!("wallet {} is not managed by executor", target)
                    })?;
                (position + 1) as u64
            }
        } else {
            let current_index = self.current_wallet_index.load(Ordering::Relaxed);
            (current_index + 1) % total_wallets
        };

        self.current_wallet_index
            .store(desired_index, Ordering::Relaxed);

        let new_active = self.get_active_wallet();
        info!("Rotated active wallet to {}", new_active);

        if let Err(err) = self.refresh_balances().await {
            warn!("failed to refresh balances after rotation: {}", err);
        }

        Ok(new_active)
    }

    /// Deactivate a wallet and fail over if the active wallet is affected.
    pub async fn deactivate_wallet(&self, wallet_address: &str) -> Result<String> {
        if !self.wallet_paths.contains_key(wallet_address) {
            anyhow::bail!("wallet {} is not managed by executor", wallet_address);
        }

        self.mark_wallet_unhealthy(wallet_address);
        info!(
            "Marked wallet {} as inactive per admin request",
            wallet_address
        );

        if self.get_active_wallet() == wallet_address {
            info!(
                "Active wallet {} deactivated – initiating failover",
                wallet_address
            );
            self.try_failover_wallet().await?;
            if let Err(err) = self.refresh_balances().await {
                warn!("failed to refresh balances after failover: {}", err);
            }
        }

        Ok(self.get_active_wallet())
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

        // Load the actual Solana keypair and extract public key
        match solana_sdk::signature::read_keypair_file(&*expanded_path) {
            Ok(keypair) => {
                let pubkey = keypair.pubkey().to_string();
                info!("✅ Loaded wallet from {}: {}", expanded_path, pubkey);
                Ok(pubkey)
            }
            Err(e) => {
                error!("❌ Failed to load wallet from {}: {}", expanded_path, e);
                anyhow::bail!("Failed to load wallet keypair: {}", e)
            }
        }
    }

    fn load_keypair_for(&self, wallet_address: &str) -> Result<Keypair> {
        let path = self
            .wallet_paths
            .get(wallet_address)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("no keypair path registered for wallet {}", wallet_address)
            })?;
        let expanded_path = shellexpand::tilde(&path);
        solana_sdk::signature::read_keypair_file(&*expanded_path).map_err(|err| {
            anyhow::anyhow!("failed to load wallet keypair {}: {}", expanded_path, err)
        })
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
