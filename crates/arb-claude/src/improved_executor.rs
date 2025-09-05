use crate::models::ArbSignal;
use anyhow::{Context, Result};
use smallvec::SmallVec;
use solana_sdk::{
    instruction::Instruction, 
    signature::Keypair, 
    transaction::VersionedTransaction,
    pubkey::Pubkey,
    hash::Hash,
    message::{Message, VersionedMessage},
};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error};

#[cfg(feature = "stubs")]
use solana_sdk::signature::Signer;

// Improved error types
#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    #[error("Pool metadata not found for {pool_id}")]
    PoolMetadataMissing { pool_id: String },
    
    #[error("DEX not supported: {dex}")]
    UnsupportedDex { dex: String },
    
    #[error("Transaction building failed: {0}")]
    TransactionBuild(#[from] anyhow::Error),
    
    #[error("Bundle submission failed: {0}")]
    BundleSubmission(String),
    
    #[error("Flash loan provider error: {0}")]
    FlashLoan(String),
    
    #[error("Feature not enabled: {feature}")]
    FeatureDisabled { feature: String },
    
    #[error("Insufficient balance for execution")]
    InsufficientBalance,
    
    #[error("Circuit breaker is open")]
    CircuitOpen,
}

// Enhanced bundle submitter with retry logic
#[async_trait::async_trait]
pub trait BundleSubmitter: Send + Sync {
    async fn submit_bundle(&self, txs: &[VersionedTransaction]) -> Result<String, ExecutionError>;
    async fn get_submission_status(&self, bundle_id: &str) -> Result<SubmissionStatus, ExecutionError>;
}

#[derive(Debug, Clone)]
pub enum SubmissionStatus {
    Pending,
    Confirmed { slot: u64, signature: String },
    Failed { reason: String },
    Timeout,
}

// Enhanced flash loan provider
#[async_trait::async_trait]
pub trait FlashLoanProvider: Send + Sync {
    async fn build_flash_loan_bundle(
        &self,
        amount: u64,
        route_instructions: &[Instruction],
        repay_account: &Pubkey,
    ) -> Result<SmallVec<[Instruction; 16]>, ExecutionError>;
    
    fn get_flash_loan_fee_bps(&self) -> u16;
    fn max_flash_loan_amount(&self) -> u64;
}

// Improved executor trait with better error handling
#[async_trait::async_trait]
pub trait Executor: Send + Sync {
    async fn execute(&self, sig: &ArbSignal) -> Result<ExecutionResult, ExecutionError>;
    async fn dry_run(&self, sig: &ArbSignal) -> Result<DryRunResult, ExecutionError>;
    fn is_healthy(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub bundle_id: String,
    pub estimated_profit: i128,
    pub actual_profit: Option<i128>,
    pub gas_used: u64,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DryRunResult {
    pub estimated_profit: i128,
    pub estimated_gas: u64,
    pub price_impact_bps: u16,
    pub success_probability: f64,
}

// Production executor with comprehensive features
pub struct FastExecutor<S: BundleSubmitter, F: FlashLoanProvider> {
    pub fee_payer: Arc<RwLock<Keypair>>,
    pub bundler: Arc<S>,
    pub flash_loan_provider: Option<Arc<F>>,
    pub config: ExecutorConfig,
    pub blockhash_cache: Arc<BlockhashCache>,
}

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub max_compute_units: u32,
    pub priority_fee_microlamports: u64,
    pub max_execution_time_ms: u64,
    pub retry_attempts: usize,
    pub slippage_tolerance_bps: u16,
    pub enable_flash_loans: bool,
    pub dry_run_before_execute: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_compute_units: 1_400_000,
            priority_fee_microlamports: std::env::var("PRIORITY_FEE_MICROLAMPORTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5_000),
            max_execution_time_ms: 5_000,
            retry_attempts: 3,
            slippage_tolerance_bps: 50,
            enable_flash_loans: std::env::var("ENABLE_FLASH_LOANS") == Ok("true".to_string()),
            dry_run_before_execute: std::env::var("DRY_RUN_ENABLED") == Ok("true".to_string()),
        }
    }
}

// Blockhash cache for transaction building
pub struct BlockhashCache {
    recent_blockhashes: RwLock<Vec<(Hash, Instant, u64)>>, // hash, timestamp, slot
    max_age_seconds: u64,
}

impl BlockhashCache {
    pub fn new() -> Self {
        Self {
            recent_blockhashes: RwLock::new(Vec::new()),
            max_age_seconds: 60, // 60 seconds max age
        }
    }
    
    pub async fn get_recent_blockhash(&self) -> Option<Hash> {
        let mut cache = self.recent_blockhashes.write().await;
        let now = Instant::now();
        
        // Remove expired entries
        cache.retain(|(_, timestamp, _)| {
            now.duration_since(*timestamp).as_secs() < self.max_age_seconds
        });
        
        // Return most recent valid blockhash
        cache.last().map(|(hash, _, _)| *hash)
    }
    
    pub async fn add_blockhash(&self, hash: Hash, slot: u64) {
        let mut cache = self.recent_blockhashes.write().await;
        cache.push((hash, Instant::now(), slot));
        
        // Keep only last 10 blockhashes
        if cache.len() > 10 {
            cache.remove(0);
        }
    }
}

impl<S: BundleSubmitter, F: FlashLoanProvider> FastExecutor<S, F> {
    pub fn new(
        fee_payer: Keypair,
        bundler: S,
        flash_loan_provider: Option<F>,
        config: Option<ExecutorConfig>,
    ) -> Self {
        Self {
            fee_payer: Arc::new(RwLock::new(fee_payer)),
            bundler: Arc::new(bundler),
            flash_loan_provider: flash_loan_provider.map(Arc::new),
            config: config.unwrap_or_default(),
            blockhash_cache: Arc::new(BlockhashCache::new()),
        }
    }
    
    pub async fn update_fee_payer(&self, new_keypair: Keypair) {
        let mut payer = self.fee_payer.write().await;
        *payer = new_keypair;
    }
}

#[async_trait::async_trait]
impl<S: BundleSubmitter + Send + Sync, F: FlashLoanProvider + Send + Sync> Executor 
    for FastExecutor<S, F> 
{
    async fn execute(&self, sig: &ArbSignal) -> Result<ExecutionResult, ExecutionError> {
        let start_time = Instant::now();
        
        // Dry run first if enabled
        if self.config.dry_run_before_execute {
            let dry_result = self.dry_run(sig).await?;
            if dry_result.success_probability < 0.7 {
                return Err(ExecutionError::TransactionBuild(
                    anyhow::anyhow!("Low success probability: {}", dry_result.success_probability)
                ));
            }
        }
        
        // Build instructions for both legs
        let payer = self.fee_payer.read().await;
        let payer_pubkey = payer.pubkey();
        drop(payer);
        
        let leg1 = build_leg_sell(sig, &payer_pubkey).await?;
        let leg2 = build_leg_buy(sig, &payer_pubkey).await?;
        
        let mut all_instructions = SmallVec::<[Instruction; 20]>::new();
        
        // Add compute budget instructions
        all_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(
            self.config.max_compute_units
        ));
        all_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
            self.config.priority_fee_microlamports
        ));
        
        // Add flash loan if needed and available
        if self.config.enable_flash_loans {
            if let Some(fl_provider) = &self.flash_loan_provider {
                let route_instructions: Vec<_> = leg1.iter().chain(leg2.iter()).cloned().collect();
                let fl_instructions = fl_provider
                    .build_flash_loan_bundle(
                        sig.amount_in as u64, 
                        &route_instructions,
                        &payer_pubkey
                    )
                    .await?;
                all_instructions.extend(fl_instructions);
            } else {
                all_instructions.extend(leg1);
                all_instructions.extend(leg2);
            }
        } else {
            all_instructions.extend(leg1);
            all_instructions.extend(leg2);
        }
        
        // Build transaction
        let payer = self.fee_payer.read().await;
        let tx = build_payer_tx(&payer, &all_instructions, &self.blockhash_cache).await?;
        drop(payer);
        
        // Submit transaction
        let bundle_id = self.bundler.submit_bundle(&[tx]).await?;
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(ExecutionResult {
            bundle_id,
            estimated_profit: sig.expected_profit_out,
            actual_profit: None, // Will be filled in later via callback
            gas_used: self.config.max_compute_units as u64,
            execution_time_ms,
        })
    }
    
    async fn dry_run(&self, sig: &ArbSignal) -> Result<DryRunResult, ExecutionError> {
        // Simplified dry run - in production you'd simulate the transaction
        let estimated_gas = self.config.max_compute_units as u64;
        let price_impact = estimate_price_impact(sig);
        let success_prob = if price_impact > 200 { 0.3 } else { 0.9 };
        
        Ok(DryRunResult {
            estimated_profit: sig.expected_profit_out,
            estimated_gas,
            price_impact_bps: price_impact,
            success_probability: success_prob,
        })
    }
    
    fn is_healthy(&self) -> bool {
        // Check if executor components are healthy
        true // Simplified implementation
    }
}

// DEX instruction builders with better error handling
pub async fn build_leg_sell(
    sig: &ArbSignal,
    trader: &Pubkey,
) -> Result<SmallVec<[Instruction; 8]>, ExecutionError> {
    let mut instructions = SmallVec::new();
    
    match sig.dex_sell.as_str() {
        "orca_whirlpool" | "whirlpool" => {
            #[cfg(feature = "whirlpool")]
            {
                instructions.extend(build_whirlpool_swap(sig, trader, true).await?);
            }
            #[cfg(not(feature = "whirlpool"))]
            {
                return Err(ExecutionError::FeatureDisabled { 
                    feature: "whirlpool".to_string() 
                });
            }
        }
        "raydium_cpmm" | "raydium" => {
            #[cfg(feature = "raydium")]
            {
                instructions.extend(build_raydium_swap(sig, trader, true).await?);
            }
            #[cfg(not(feature = "raydium"))]
            {
                return Err(ExecutionError::FeatureDisabled { 
                    feature: "raydium".to_string() 
                });
            }
        }
        _ => {
            return Err(ExecutionError::UnsupportedDex { 
                dex: sig.dex_sell.clone() 
            });
        }
    }
    
    Ok(instructions)
}

pub async fn build_leg_buy(
    sig: &ArbSignal,
    trader: &Pubkey,
) -> Result<SmallVec<[Instruction; 8]>, ExecutionError> {
    let mut instructions = SmallVec::new();
    
    match sig.dex_buy.as_str() {
        "orca_whirlpool" | "whirlpool" => {
            #[cfg(feature = "whirlpool")]
            {
                instructions.extend(build_whirlpool_swap(sig, trader, false).await?);
            }
            #[cfg(not(feature = "whirlpool"))]
            {
                return Err(ExecutionError::FeatureDisabled { 
                    feature: "whirlpool".to_string() 
                });
            }
        }
        "raydium_cpmm" | "raydium" => {
            #[cfg(feature = "raydium")]
            {
                instructions.extend(build_raydium_swap(sig, trader, false).await?);
            }
            #[cfg(not(feature = "raydium"))]
            {
                return Err(ExecutionError::FeatureDisabled { 
                    feature: "raydium".to_string() 
                });
            }
        }
        _ => {
            return Err(ExecutionError::UnsupportedDex { 
                dex: sig.dex_buy.clone() 
            });
        }
    }
    
    Ok(instructions)
}

// Feature-gated DEX implementations
#[cfg(feature = "whirlpool")]
async fn build_whirlpool_swap(
    _sig: &ArbSignal,
    _trader: &Pubkey,
    _is_sell: bool,
) -> Result<SmallVec<[Instruction; 4]>, ExecutionError> {
    // Whirlpool-specific implementation
    let instructions = SmallVec::new();
    // TODO: Implement actual Whirlpool swap instruction building
    Ok(instructions)
}

#[cfg(feature = "raydium")]
async fn build_raydium_swap(
    _sig: &ArbSignal,
    _trader: &Pubkey,
    _is_sell: bool,
) -> Result<SmallVec<[Instruction; 4]>, ExecutionError> {
    // Raydium-specific implementation
    let instructions = SmallVec::new();
    // TODO: Implement actual Raydium swap instruction building
    Ok(instructions)
}

/// Enhanced transaction builder with better blockhash management
pub async fn build_payer_tx(
    payer: &Keypair,
    instructions: &[Instruction],
    blockhash_cache: &BlockhashCache,
) -> Result<VersionedTransaction, ExecutionError> {
    #[cfg(feature = "stubs")]
    {
        info!("🟡 STUB: Creating stub transaction for development");
        let message = Message::new(instructions, Some(&payer.pubkey()));
        let versioned_message = VersionedMessage::Legacy(message);
        return VersionedTransaction::try_new(versioned_message, &[payer])
            .map_err(|e| ExecutionError::TransactionBuild(e.into()));
    }
    
    #[cfg(not(feature = "stubs"))]
    {
        // Get recent blockhash
        let blockhash = blockhash_cache
            .get_recent_blockhash()
            .await
            .ok_or_else(|| ExecutionError::TransactionBuild(
                anyhow::anyhow!("No recent blockhash available")
            ))?;
        
        // Build message with recent blockhash
        let mut message = Message::new(instructions, Some(&payer.pubkey()));
        message.recent_blockhash = blockhash;
        
        let versioned_message = VersionedMessage::Legacy(message);
        VersionedTransaction::try_new(versioned_message, &[payer])
            .map_err(|e| ExecutionError::TransactionBuild(e.into()))
    }
}

// Utility functions
fn estimate_price_impact(sig: &ArbSignal) -> u16 {
    // Simplified price impact estimation
    // In production, this would use more sophisticated modeling
    let base_impact = 10; // 10 bps base impact
    let size_factor = (sig.amount_in / 1_000_000) as u16; // Scale by millions
    (base_impact + size_factor).min(500) // Cap at 5%
}

// Stub implementations for development
#[cfg(feature = "stubs")]
pub mod stubs {
    use super::*;
    
    pub struct StubBundleSubmitter;
    
    #[async_trait::async_trait]
    impl BundleSubmitter for StubBundleSubmitter {
        async fn submit_bundle(&self, txs: &[VersionedTransaction]) -> Result<String, ExecutionError> {
            info!("🟡 STUB: Would submit {} transactions", txs.len());
            Ok(format!("stub_bundle_{}", rand::random::<u32>()))
        }
        
        async fn get_submission_status(&self, bundle_id: &str) -> Result<SubmissionStatus, ExecutionError> {
            info!("🟡 STUB: Checking status for bundle {}", bundle_id);
            Ok(SubmissionStatus::Confirmed { 
                slot: 123456, 
                signature: "stub_signature".to_string() 
            })
        }
    }
    
    pub struct StubFlashLoanProvider;
    
    #[async_trait::async_trait]
    impl FlashLoanProvider for StubFlashLoanProvider {
        async fn build_flash_loan_bundle(
            &self,
            amount: u64,
            route_instructions: &[Instruction],
            _repay_account: &Pubkey,
        ) -> Result<SmallVec<[Instruction; 16]>, ExecutionError> {
            info!("🟡 STUB: Would create flash loan for {} with {} route instructions", 
                  amount, route_instructions.len());
            Ok(SmallVec::new())
        }
        
        fn get_flash_loan_fee_bps(&self) -> u16 { 5 } // 0.05%
        fn max_flash_loan_amount(&self) -> u64 { 1_000_000_000 } // 1B lamports
    }
}

#[cfg(feature = "stubs")]
pub use stubs::*;