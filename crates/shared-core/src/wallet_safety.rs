use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Wallet states for safe transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalletState {
    /// Normal trading operations
    Active,
    /// Pausing new trades, completing existing ones
    Transitioning,
    /// Emergency stop - no operations allowed
    Locked,
    /// Moving funds to cold storage
    Evacuating,
    /// Minimal operations with reduced funds
    Lifeboat,
}

/// Safety configuration for wallet operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSafetyConfig {
    /// Maximum amount per transaction (lamports)
    pub max_transaction_amount: u64,
    /// Maximum daily trading volume (lamports)
    pub max_daily_volume: u64,
    /// Success rate threshold before evacuation (0.0-1.0)
    pub evacuation_threshold: f64,
    /// Cold wallet for emergency evacuation
    pub cold_wallet: Pubkey,
    /// Warm wallet for intermediate storage
    pub warm_wallet: Pubkey,
    /// Number of chunks to split transfers
    pub transfer_chunks: usize,
    /// Delay between chunk transfers
    pub chunk_delay_ms: u64,
}

impl Default for WalletSafetyConfig {
    fn default() -> Self {
        Self {
            max_transaction_amount: 1_000_000_000, // 1 SOL
            max_daily_volume: 100_000_000_000,     // 100 SOL
            evacuation_threshold: 0.6,             // 60% success rate
            cold_wallet: Pubkey::default(),
            warm_wallet: Pubkey::default(),
            transfer_chunks: 10,
            chunk_delay_ms: 1000,
        }
    }
}

/// Tracks wallet health metrics
#[derive(Debug, Clone)]
pub struct WalletMetrics {
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub daily_volume: u64,
    pub last_reset: Instant,
    pub consecutive_failures: u32,
}

impl Default for WalletMetrics {
    fn default() -> Self {
        Self {
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            daily_volume: 0,
            last_reset: Instant::now(),
            consecutive_failures: 0,
        }
    }
}

impl WalletMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            1.0
        } else {
            self.successful_transactions as f64 / self.total_transactions as f64
        }
    }

    pub fn record_success(&mut self, amount: u64) {
        self.successful_transactions += 1;
        self.total_transactions += 1;
        self.daily_volume += amount;
        self.consecutive_failures = 0;
    }

    pub fn record_failure(&mut self) {
        self.failed_transactions += 1;
        self.total_transactions += 1;
        self.consecutive_failures += 1;
    }

    pub fn should_evacuate(&self, threshold: f64) -> bool {
        self.success_rate() < threshold || self.consecutive_failures > 5
    }

    pub fn reset_daily(&mut self) {
        if self.last_reset.elapsed() > Duration::from_secs(86400) {
            self.daily_volume = 0;
            self.last_reset = Instant::now();
        }
    }
}

/// Manages wallet safety and transitions
pub struct WalletSafetyManager {
    pub state: Arc<RwLock<WalletState>>,
    pub config: WalletSafetyConfig,
    pub metrics: Arc<RwLock<WalletMetrics>>,
    pub pending_trades: Arc<RwLock<Vec<String>>>,
}

impl WalletSafetyManager {
    pub fn new(config: WalletSafetyConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(WalletState::Active)),
            config,
            metrics: Arc::new(RwLock::new(WalletMetrics::default())),
            pending_trades: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if trading is allowed based on current state
    pub async fn can_trade(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, WalletState::Active | WalletState::Lifeboat)
    }

    /// Register a new trade
    pub async fn register_trade(&self, trade_id: String, amount: u64) -> Result<(), String> {
        // Check state
        if !self.can_trade().await {
            return Err("Trading not allowed in current state".to_string());
        }

        // Check limits
        let metrics = self.metrics.read().await;
        if amount > self.config.max_transaction_amount {
            return Err(format!(
                "Amount exceeds max transaction limit: {} > {}",
                amount, self.config.max_transaction_amount
            ));
        }

        if metrics.daily_volume + amount > self.config.max_daily_volume {
            return Err(format!(
                "Would exceed daily volume limit: {} + {} > {}",
                metrics.daily_volume, amount, self.config.max_daily_volume
            ));
        }
        drop(metrics);

        // Register the trade
        let mut pending = self.pending_trades.write().await;
        pending.push(trade_id);

        Ok(())
    }

    /// Complete a trade and update metrics
    pub async fn complete_trade(&self, trade_id: &str, success: bool, amount: u64) {
        // Remove from pending
        let mut pending = self.pending_trades.write().await;
        pending.retain(|id| id != trade_id);
        drop(pending);

        // Update metrics
        let mut metrics = self.metrics.write().await;
        if success {
            metrics.record_success(amount);
        } else {
            metrics.record_failure();
        }

        // Check if we should evacuate
        if metrics.should_evacuate(self.config.evacuation_threshold) {
            drop(metrics);
            self.initiate_evacuation().await;
        }
    }

    /// Wait for all pending trades to complete
    pub async fn await_pending_trades(&self) {
        loop {
            let pending = self.pending_trades.read().await;
            if pending.is_empty() {
                break;
            }
            drop(pending);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Initiate safe wallet transition
    pub async fn initiate_transition(&self) -> Result<(), String> {
        let mut state = self.state.write().await;

        match *state {
            WalletState::Active => {
                *state = WalletState::Transitioning;
                drop(state);

                // Wait for pending trades
                self.await_pending_trades().await;

                Ok(())
            }
            _ => Err(format!("Cannot transition from state: {:?}", *state)),
        }
    }

    /// Emergency evacuation to cold storage
    pub async fn initiate_evacuation(&self) {
        let mut state = self.state.write().await;
        *state = WalletState::Evacuating;
        drop(state);

        log::error!("EMERGENCY EVACUATION INITIATED - Moving funds to cold storage");

        // In production, this would:
        // 1. Stop all new trades immediately
        // 2. Cancel pending orders
        // 3. Transfer funds in chunks to cold wallet
        // 4. Send alerts to admin
    }

    /// Enter lifeboat mode (minimal trading with reduced funds)
    pub async fn enter_lifeboat_mode(&self) {
        let mut state = self.state.write().await;
        *state = WalletState::Lifeboat;

        log::warn!("Entering LIFEBOAT MODE - Trading with reduced capacity");
    }

    /// Resume normal operations
    pub async fn resume_normal_operations(&self) -> Result<(), String> {
        let mut state = self.state.write().await;
        let mut metrics = self.metrics.write().await;

        // Only resume if metrics are healthy
        if metrics.success_rate() < 0.8 {
            return Err("Success rate too low to resume normal operations".to_string());
        }

        *state = WalletState::Active;
        metrics.consecutive_failures = 0;

        log::info!("Resumed normal trading operations");
        Ok(())
    }

    /// Get current safety status
    pub async fn get_status(&self) -> SafetyStatus {
        let state = self.state.read().await;
        let metrics = self.metrics.read().await;
        let pending = self.pending_trades.read().await;

        SafetyStatus {
            state: *state,
            success_rate: metrics.success_rate(),
            daily_volume: metrics.daily_volume,
            pending_trades: pending.len(),
            consecutive_failures: metrics.consecutive_failures,
            is_healthy: metrics.success_rate() > self.config.evacuation_threshold,
        }
    }
}

/// Current safety status for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStatus {
    pub state: WalletState,
    pub success_rate: f64,
    pub daily_volume: u64,
    pub pending_trades: usize,
    pub consecutive_failures: u32,
    pub is_healthy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wallet_state_transitions() {
        let config = WalletSafetyConfig::default();
        let manager = WalletSafetyManager::new(config);

        // Should start active
        assert!(manager.can_trade().await);

        // Register a trade
        manager
            .register_trade("test1".to_string(), 1000)
            .await
            .unwrap();
        manager.complete_trade("test1", true, 1000).await;

        // Initiate transition
        manager.initiate_transition().await.unwrap();

        // Should not allow new trades during transition
        assert!(!manager.can_trade().await);
    }

    #[tokio::test]
    async fn test_evacuation_trigger() {
        let config = WalletSafetyConfig {
            evacuation_threshold: 0.5, // 50% success rate
            ..Default::default()
        };

        let manager = WalletSafetyManager::new(config);

        // Record failures to trigger evacuation
        for i in 0..10 {
            if !manager.can_trade().await {
                break;
            }
            let trade_id = format!("trade_{}", i);
            manager.register_trade(trade_id.clone(), 100).await.unwrap();
            manager.complete_trade(&trade_id, i < 3, 100).await; // Only 30% success
        }

        // Should be evacuating
        let state = manager.state.read().await;
        assert_eq!(*state, WalletState::Evacuating);
    }
}
