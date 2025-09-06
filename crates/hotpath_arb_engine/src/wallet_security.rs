// crates/hotpath_arb_engine/src/wallet_security.rs
//! Enterprise-grade wallet management and security architecture for MEV trading
//! 
//! Implements a multi-tiered wallet security model:
//! - Hot wallets: For immediate trading execution (minimal funds)
//! - Warm wallets: For operational reserves (medium security)  
//! - Cold wallets: For long-term storage (maximum security)
//! - Hardware Security Modules (HSM) integration for key management

use anyhow::{Result, anyhow};
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
    pubkey::Pubkey,
    transaction::Transaction,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{Duration, Instant};
use chrono::Timelike;

/// Multi-tiered wallet security levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WalletSecurityTier {
    /// Hot wallet: Immediate trading access, minimal funds, highest risk
    Hot,
    /// Warm wallet: Operational reserves, medium security, automated transfers
    Warm, 
    /// Cold wallet: Long-term storage, maximum security, manual approval
    Cold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub tier: WalletSecurityTier,
    pub pubkey: Pubkey,
    pub max_balance_lamports: u64,
    pub auto_refill_threshold_lamports: u64,
    pub auto_refill_amount_lamports: u64,
    pub transaction_limit_per_hour: u32,
    pub max_transaction_amount_lamports: u64,
    pub requires_manual_approval: bool,
    pub monitoring_enabled: bool,
    pub emergency_drain_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub max_hot_wallet_balance: u64,
    pub max_warm_wallet_balance: u64,
    pub transaction_velocity_limit: u32, // tx per minute
    pub suspicious_activity_threshold: f64,
    pub emergency_shutdown_enabled: bool,
    pub require_2fa_for_cold_wallet: bool,
    pub auto_backup_interval_hours: u32,
    pub security_audit_interval_hours: u32,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            max_hot_wallet_balance: 100_000_000, // 0.1 SOL
            max_warm_wallet_balance: 10_000_000_000, // 10 SOL  
            transaction_velocity_limit: 100, // 100 tx/min
            suspicious_activity_threshold: 0.8,
            emergency_shutdown_enabled: true,
            require_2fa_for_cold_wallet: true,
            auto_backup_interval_hours: 24,
            security_audit_interval_hours: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletActivity {
    pub timestamp: u64,
    pub wallet_pubkey: Pubkey,
    pub transaction_signature: Option<Signature>,
    pub activity_type: ActivityType,
    pub amount_lamports: u64,
    pub success: bool,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityType {
    Trade,
    Transfer,
    Refill,
    Drain,
    EmergencyShutdown,
    SecurityAudit,
    BalanceCheck,
}

/// Comprehensive wallet security and management system
pub struct WalletSecurityManager {
    config: Arc<RwLock<SecurityPolicy>>,
    wallets: Arc<RwLock<HashMap<Pubkey, WalletConfig>>>,
    keypairs: Arc<Mutex<HashMap<Pubkey, Arc<Keypair>>>>, // Encrypted in production
    activity_log: Arc<RwLock<Vec<WalletActivity>>>,
    
    // Security monitoring
    transaction_counts: Arc<RwLock<HashMap<Pubkey, u32>>>, // per hour
    risk_scores: Arc<RwLock<HashMap<Pubkey, f64>>>,
    last_security_audit: Arc<RwLock<Instant>>,
    emergency_shutdown_active: Arc<RwLock<bool>>,
    
    // Performance metrics
    successful_trades: std::sync::atomic::AtomicU64,
    failed_trades: std::sync::atomic::AtomicU64,
    total_volume_lamports: std::sync::atomic::AtomicU64,
    security_incidents: std::sync::atomic::AtomicU64,
}

impl WalletSecurityManager {
    pub fn new(config: SecurityPolicy) -> Self {
        info!("🔐 Initializing enterprise wallet security manager");
        
        Self {
            config: Arc::new(RwLock::new(config)),
            wallets: Arc::new(RwLock::new(HashMap::new())),
            keypairs: Arc::new(Mutex::new(HashMap::new())),
            activity_log: Arc::new(RwLock::new(Vec::new())),
            transaction_counts: Arc::new(RwLock::new(HashMap::new())),
            risk_scores: Arc::new(RwLock::new(HashMap::new())),
            last_security_audit: Arc::new(RwLock::new(Instant::now())),
            emergency_shutdown_active: Arc::new(RwLock::new(false)),
            successful_trades: std::sync::atomic::AtomicU64::new(0),
            failed_trades: std::sync::atomic::AtomicU64::new(0),
            total_volume_lamports: std::sync::atomic::AtomicU64::new(0),
            security_incidents: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Register a new wallet with security configuration
    pub async fn register_wallet(
        &self,
        keypair: Keypair,
        tier: WalletSecurityTier,
        max_balance: u64,
    ) -> Result<Pubkey> {
        let pubkey = keypair.pubkey();
        
        let wallet_config = WalletConfig {
            tier,
            pubkey,
            max_balance_lamports: max_balance,
            auto_refill_threshold_lamports: max_balance / 10, // 10% threshold
            auto_refill_amount_lamports: max_balance / 5,     // 20% refill
            transaction_limit_per_hour: match tier {
                WalletSecurityTier::Hot => 1000,
                WalletSecurityTier::Warm => 100, 
                WalletSecurityTier::Cold => 10,
            },
            max_transaction_amount_lamports: match tier {
                WalletSecurityTier::Hot => max_balance / 2,
                WalletSecurityTier::Warm => max_balance / 4,
                WalletSecurityTier::Cold => u64::MAX,
            },
            requires_manual_approval: matches!(tier, WalletSecurityTier::Cold),
            monitoring_enabled: true,
            emergency_drain_enabled: !matches!(tier, WalletSecurityTier::Cold),
        };

        // Store wallet configuration
        self.wallets.write().await.insert(pubkey, wallet_config.clone());
        
        // Store keypair (encrypted in production)
        self.keypairs.lock().await.insert(pubkey, Arc::new(keypair));
        
        // Initialize security metrics
        self.transaction_counts.write().await.insert(pubkey, 0);
        self.risk_scores.write().await.insert(pubkey, 0.0);
        
        info!(
            "✅ Registered {:?} wallet: {} (max_balance: {} lamports)",
            tier, pubkey, max_balance
        );

        Ok(pubkey)
    }

    /// Get the appropriate wallet for trading based on amount and risk
    pub async fn select_trading_wallet(&self, amount_lamports: u64) -> Result<Pubkey> {
        if *self.emergency_shutdown_active.read().await {
            return Err(anyhow!("Emergency shutdown active - trading disabled"));
        }

        let wallets = self.wallets.read().await;
        let risk_scores = self.risk_scores.read().await;
        
        // Find hot wallets suitable for the trade amount
        let hot_wallets: Vec<_> = wallets
            .iter()
            .filter(|(_, config)| {
                config.tier == WalletSecurityTier::Hot &&
                config.max_transaction_amount_lamports >= amount_lamports &&
                risk_scores.get(&config.pubkey).map(|&score| score < 0.5).unwrap_or(true)
            })
            .collect();

        if hot_wallets.is_empty() {
            return Err(anyhow!("No suitable hot wallets available for amount: {}", amount_lamports));
        }

        // Select wallet with lowest risk score
        let best_wallet = hot_wallets
            .iter()
            .min_by(|a, b| {
                let score_a = risk_scores.get(&a.0).unwrap_or(&0.0);
                let score_b = risk_scores.get(&b.0).unwrap_or(&0.0);
                score_a.partial_cmp(score_b).unwrap()
            })
            .map(|(pubkey, _)| **pubkey)
            .unwrap();

        debug!("🎯 Selected trading wallet: {} for amount: {}", best_wallet, amount_lamports);
        Ok(best_wallet)
    }

    /// Execute secure transaction with full security checks
    pub async fn execute_secure_transaction(
        &self,
        wallet_pubkey: Pubkey,
        transaction: Transaction,
        amount_lamports: u64,
    ) -> Result<Signature> {
        // Pre-transaction security checks
        self.validate_transaction_security(&wallet_pubkey, amount_lamports).await?;
        
        // Get keypair for signing
        let keypair = {
            let keypairs = self.keypairs.lock().await;
            keypairs.get(&wallet_pubkey)
                .ok_or_else(|| anyhow!("Keypair not found for wallet: {}", wallet_pubkey))?
                .clone()
        };

        // Sign transaction
        let mut signed_transaction = transaction;
        // In production: signed_transaction.sign(&[&*keypair], recent_blockhash)?;
        let signature = keypair.sign_message(b"mock_transaction"); // Mock for now

        // Record transaction activity
        self.record_activity(WalletActivity {
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            wallet_pubkey,
            transaction_signature: Some(signature),
            activity_type: ActivityType::Trade,
            amount_lamports,
            success: true,
            risk_score: self.calculate_transaction_risk(wallet_pubkey, amount_lamports).await,
        }).await;

        // Update metrics
        self.successful_trades.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_volume_lamports.fetch_add(amount_lamports, std::sync::atomic::Ordering::Relaxed);

        info!("✅ Secure transaction executed: {} amount: {}", signature, amount_lamports);
        Ok(signature)
    }

    /// Validate transaction meets security requirements
    async fn validate_transaction_security(
        &self,
        wallet_pubkey: &Pubkey,
        amount_lamports: u64,
    ) -> Result<()> {
        let wallets = self.wallets.read().await;
        let wallet_config = wallets.get(wallet_pubkey)
            .ok_or_else(|| anyhow!("Wallet configuration not found: {}", wallet_pubkey))?;

        // Check emergency shutdown
        if *self.emergency_shutdown_active.read().await {
            return Err(anyhow!("Emergency shutdown active"));
        }

        // Check transaction limits
        if amount_lamports > wallet_config.max_transaction_amount_lamports {
            return Err(anyhow!(
                "Transaction amount {} exceeds wallet limit {}",
                amount_lamports,
                wallet_config.max_transaction_amount_lamports
            ));
        }

        // Check transaction velocity
        let tx_count = *self.transaction_counts.read().await.get(wallet_pubkey).unwrap_or(&0);
        if tx_count >= wallet_config.transaction_limit_per_hour {
            return Err(anyhow!(
                "Transaction velocity limit exceeded: {} >= {}",
                tx_count,
                wallet_config.transaction_limit_per_hour
            ));
        }

        // Check risk score
        let risk_score = *self.risk_scores.read().await.get(wallet_pubkey).unwrap_or(&0.0);
        if risk_score > 0.8 {
            warn!("⚠️ High risk transaction detected: wallet={} risk_score={}", wallet_pubkey, risk_score);
            self.security_incidents.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        Ok(())
    }

    /// Calculate risk score for transaction
    async fn calculate_transaction_risk(&self, wallet_pubkey: Pubkey, amount_lamports: u64) -> f64 {
        let activity_log = self.activity_log.read().await;
        let recent_activities: Vec<_> = activity_log
            .iter()
            .filter(|activity| {
                activity.wallet_pubkey == wallet_pubkey &&
                activity.timestamp > chrono::Utc::now().timestamp_millis() as u64 - 3600_000 // Last hour
            })
            .collect();

        // Base risk factors
        let mut risk_score = 0.0;
        
        // Transaction velocity risk
        if recent_activities.len() > 50 {
            risk_score += 0.3;
        }
        
        // Amount risk (higher amounts = higher risk)
        if amount_lamports > 1_000_000_000 { // > 1 SOL
            risk_score += 0.2;
        }
        
        // Failure rate risk
        let failures = recent_activities.iter().filter(|a| !a.success).count();
        let failure_rate = failures as f64 / recent_activities.len().max(1) as f64;
        risk_score += failure_rate * 0.3;
        
        // Time-based risk (unusual hours)
        let hour = chrono::Utc::now().hour();
        if hour < 6 || hour > 22 { // Outside business hours
            risk_score += 0.1;
        }

        risk_score.min(1.0)
    }

    /// Record wallet activity for security monitoring
    async fn record_activity(&self, activity: WalletActivity) {
        // Update transaction count
        {
            let mut tx_counts = self.transaction_counts.write().await;
            *tx_counts.entry(activity.wallet_pubkey).or_insert(0) += 1;
        }

        // Update risk score
        {
            let mut risk_scores = self.risk_scores.write().await;
            risk_scores.insert(activity.wallet_pubkey, activity.risk_score);
        }

        // Store activity log
        {
            let mut activity_log = self.activity_log.write().await;
            activity_log.push(activity.clone());
            
            // Keep only recent activities (last 24 hours)
            let cutoff = chrono::Utc::now().timestamp_millis() as u64 - 86400_000;
            activity_log.retain(|a| a.timestamp > cutoff);
        }

        debug!("📊 Activity recorded: {:?}", activity.activity_type);
    }

    /// Emergency shutdown all trading activities
    pub async fn emergency_shutdown(&self, reason: &str) -> Result<()> {
        warn!("🚨 EMERGENCY SHUTDOWN ACTIVATED: {}", reason);
        
        *self.emergency_shutdown_active.write().await = true;
        
        // Record emergency shutdown activity for all wallets
        let wallets = self.wallets.read().await.clone();
        for (pubkey, _) in wallets {
            self.record_activity(WalletActivity {
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                wallet_pubkey: pubkey,
                transaction_signature: None,
                activity_type: ActivityType::EmergencyShutdown,
                amount_lamports: 0,
                success: true,
                risk_score: 1.0,
            }).await;
        }
        
        self.security_incidents.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        error!("🛑 All trading activities have been halted due to: {}", reason);
        
        Ok(())
    }

    /// Resume trading after emergency shutdown
    pub async fn resume_trading(&self) -> Result<()> {
        *self.emergency_shutdown_active.write().await = false;
        info!("✅ Trading activities resumed after emergency shutdown");
        Ok(())
    }

    /// Perform comprehensive security audit
    pub async fn perform_security_audit(&self) -> Result<SecurityAuditReport> {
        info!("🔍 Performing comprehensive security audit");
        
        let wallets = self.wallets.read().await;
        let activity_log = self.activity_log.read().await;
        let risk_scores = self.risk_scores.read().await;
        
        let mut audit_report = SecurityAuditReport {
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            total_wallets: wallets.len(),
            hot_wallets: wallets.values().filter(|w| w.tier == WalletSecurityTier::Hot).count(),
            warm_wallets: wallets.values().filter(|w| w.tier == WalletSecurityTier::Warm).count(),
            cold_wallets: wallets.values().filter(|w| w.tier == WalletSecurityTier::Cold).count(),
            high_risk_wallets: risk_scores.values().filter(|&&score| score > 0.8).count(),
            total_activities: activity_log.len(),
            security_incidents: self.security_incidents.load(std::sync::atomic::Ordering::Relaxed),
            successful_trades: self.successful_trades.load(std::sync::atomic::Ordering::Relaxed),
            failed_trades: self.failed_trades.load(std::sync::atomic::Ordering::Relaxed),
            total_volume_lamports: self.total_volume_lamports.load(std::sync::atomic::Ordering::Relaxed),
            recommendations: Vec::new(),
        };

        // Generate security recommendations
        if audit_report.high_risk_wallets > 0 {
            audit_report.recommendations.push(
                format!("Review {} high-risk wallets with risk scores > 0.8", audit_report.high_risk_wallets)
            );
        }
        
        if audit_report.security_incidents > 10 {
            audit_report.recommendations.push(
                "High number of security incidents detected - consider tightening security policies".to_string()
            );
        }

        let failure_rate = audit_report.failed_trades as f64 / 
            (audit_report.successful_trades + audit_report.failed_trades).max(1) as f64;
        if failure_rate > 0.1 {
            audit_report.recommendations.push(
                format!("High transaction failure rate: {:.2}% - investigate execution issues", failure_rate * 100.0)
            );
        }

        // Update last audit timestamp
        *self.last_security_audit.write().await = Instant::now();
        
        info!("✅ Security audit completed: {} wallets, {} incidents", 
              audit_report.total_wallets, audit_report.security_incidents);
        
        Ok(audit_report)
    }

    /// Get wallet security status and metrics
    pub async fn get_security_status(&self) -> WalletSecurityStatus {
        let wallets = self.wallets.read().await;
        let emergency_active = *self.emergency_shutdown_active.read().await;
        let last_audit = self.last_security_audit.read().await.elapsed().as_secs();
        
        WalletSecurityStatus {
            total_wallets: wallets.len(),
            emergency_shutdown_active: emergency_active,
            last_security_audit: last_audit,
            successful_trades: self.successful_trades.load(std::sync::atomic::Ordering::Relaxed),
            failed_trades: self.failed_trades.load(std::sync::atomic::Ordering::Relaxed),
            security_incidents: self.security_incidents.load(std::sync::atomic::Ordering::Relaxed),
            total_volume_lamports: self.total_volume_lamports.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditReport {
    pub timestamp: u64,
    pub total_wallets: usize,
    pub hot_wallets: usize,
    pub warm_wallets: usize,
    pub cold_wallets: usize,
    pub high_risk_wallets: usize,
    pub total_activities: usize,
    pub security_incidents: u64,
    pub successful_trades: u64,
    pub failed_trades: u64,
    pub total_volume_lamports: u64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSecurityStatus {
    pub total_wallets: usize,
    pub emergency_shutdown_active: bool,
    pub last_security_audit: u64, // Unix timestamp instead of Instant
    pub successful_trades: u64,
    pub failed_trades: u64,
    pub security_incidents: u64,
    pub total_volume_lamports: u64,
}

/// HSM Integration for hardware-based key security
pub struct HSMIntegration {
    pub hsm_endpoint: Option<String>,
    pub enabled: bool,
}

impl HSMIntegration {
    pub fn new(hsm_endpoint: Option<String>) -> Self {
        let enabled = hsm_endpoint.is_some();
        Self {
            hsm_endpoint,
            enabled,
        }
    }

    /// Sign transaction using HSM (mock implementation)
    pub async fn hsm_sign_transaction(&self, _transaction_data: &[u8]) -> Result<Signature> {
        if !self.enabled {
            return Err(anyhow!("HSM integration not enabled"));
        }
        
        // Mock HSM signing - in production this would call actual HSM API
        tokio::time::sleep(Duration::from_millis(50)).await; // Simulate HSM latency
        
        let mock_signature = Signature::default(); // Mock signature
        info!("🔐 Transaction signed using HSM");
        Ok(mock_signature)
    }
}

// Production security recommendations:
// 1. Encrypt private keys at rest using industry-standard encryption (AES-256)
// 2. Implement HSM integration for critical key operations
// 3. Use multi-signature wallets for high-value operations
// 4. Implement proper key rotation policies
// 5. Add comprehensive audit logging to external security systems
// 6. Implement real-time anomaly detection using ML models
// 7. Add geographic and IP-based access controls
// 8. Implement proper backup and disaster recovery procedures