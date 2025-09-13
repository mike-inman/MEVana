use anyhow::Result;
use dashmap::DashMap;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

use crate::config::ExecutorConfig;
use crate::shyft_client::ShyftClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanQuote {
    pub provider: FlashLoanProvider,
    pub token_mint: String,
    pub available_amount: u64,
    pub fee_bps: u16,
    pub estimated_latency_ms: u16,
    pub is_available: bool,
    pub quote_timestamp: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FlashLoanProvider {
    Solend,
    Marginfi,
    Tulip,
    Mango,
    PortFinance,
    JetProtocol,
}

impl FlashLoanProvider {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "solend" => Some(Self::Solend),
            "marginfi" => Some(Self::Marginfi),
            "tulip" => Some(Self::Tulip),
            "mango" => Some(Self::Mango),
            "portfinance" => Some(Self::PortFinance),
            "jetprotocol" => Some(Self::JetProtocol),
            _ => None,
        }
    }

    pub fn program_id(&self) -> &'static str {
        match self {
            Self::Solend => "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo",
            Self::Marginfi => "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA",
            Self::Tulip => "4bcFeLv4nydFrsZqV5CgwCVrPhkQKsXtzfy2KyMz7ozM",
            Self::Mango => "mv3ekLzLbnVPNxjSKvqBpU3ZeZXPQdEC3bp5MDEBG68",
            Self::PortFinance => "Port7uDYB1NMso4t9LZgq4u7GsAmYHj3jdkd8fHRh",
            Self::JetProtocol => "JPLockxtkngHkaQT5AuRYow3HyUv5qWzmhwsCPd653n",
        }
    }

    pub fn base_fee_bps(&self) -> u16 {
        match self {
            Self::Solend => 9,       // 0.09%
            Self::Marginfi => 5,     // 0.05%
            Self::Tulip => 10,       // 0.10%
            Self::Mango => 8,        // 0.08%
            Self::PortFinance => 12, // 0.12%
            Self::JetProtocol => 15, // 0.15%
        }
    }
}

/// Multi-provider flash loan aggregator for optimal rate selection
pub struct FlashLoanAggregator {
    config: Arc<ExecutorConfig>,
    shyft_client: Arc<ShyftClient>,
    providers: Vec<FlashLoanProvider>,

    // Rate caching for performance
    quote_cache: Arc<DashMap<String, FlashLoanQuote>>, // token_mint -> best_quote
    provider_health: Arc<DashMap<FlashLoanProvider, ProviderHealth>>,

    // Performance metrics
    last_quote_refresh: std::sync::atomic::AtomicU64,
    successful_loans: Arc<DashMap<FlashLoanProvider, u64>>,
    failed_loans: Arc<DashMap<FlashLoanProvider, u64>>,
}

#[derive(Debug, Clone)]
struct ProviderHealth {
    is_healthy: bool,
    consecutive_failures: u32,
    last_successful_quote: u64,
    average_latency_ms: u32,
}

impl FlashLoanAggregator {
    pub fn new(config: Arc<ExecutorConfig>, shyft_client: Arc<ShyftClient>) -> Result<Self> {
        let providers: Vec<FlashLoanProvider> = config
            .flash_loan_providers
            .iter()
            .filter_map(|p| FlashLoanProvider::from_string(p))
            .collect();

        if providers.is_empty() {
            anyhow::bail!("No valid flash loan providers configured");
        }

        info!(
            "Flash loan aggregator initialized with {} providers",
            providers.len()
        );
        for provider in &providers {
            info!("  - {:?} ({})", provider, provider.program_id());
        }

        let aggregator = Self {
            config,
            shyft_client,
            providers: providers.clone(),
            quote_cache: Arc::new(DashMap::new()),
            provider_health: Arc::new(DashMap::new()),
            last_quote_refresh: std::sync::atomic::AtomicU64::new(0),
            successful_loans: Arc::new(DashMap::new()),
            failed_loans: Arc::new(DashMap::new()),
        };

        // Initialize provider health
        for provider in providers {
            aggregator.provider_health.insert(
                provider,
                ProviderHealth {
                    is_healthy: true,
                    consecutive_failures: 0,
                    last_successful_quote: 0,
                    average_latency_ms: 100,
                },
            );
        }

        Ok(aggregator)
    }

    /// Get the best flash loan quote for a token
    pub async fn get_best_quote(&self, token_mint: &str, amount: u64) -> Result<FlashLoanQuote> {
        // Check cache first (refresh every 5 seconds)
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let last_refresh = self
            .last_quote_refresh
            .load(std::sync::atomic::Ordering::Relaxed);

        let should_refresh = now - last_refresh > 5000; // 5 seconds

        if should_refresh {
            self.refresh_quotes(token_mint, amount).await?;
        }

        // Get best quote from cache
        self.quote_cache
            .get(token_mint)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| anyhow::anyhow!("No flash loan quotes available for {}", token_mint))
    }

    /// Get quotes from all providers
    pub async fn get_all_quotes(&self, token_mint: &str, amount: u64) -> Vec<FlashLoanQuote> {
        let mut quotes = Vec::new();

        // Query all healthy providers in parallel
        let mut tasks = Vec::new();

        for provider in &self.providers {
            if self.is_provider_healthy(provider) {
                let provider_clone = provider.clone();
                let token_mint = token_mint.to_string();
                let shyft_client = Arc::clone(&self.shyft_client);

                let task = tokio::spawn(async move {
                    Self::get_provider_quote(shyft_client, provider_clone, token_mint, amount).await
                });

                tasks.push(task);
            }
        }

        // Collect results
        for task in tasks {
            if let Ok(Ok(quote)) = task.await {
                quotes.push(quote);
            }
        }

        // Sort by effective cost (fee + latency penalty)
        quotes.sort_by(|a, b| {
            let cost_a = a.fee_bps as u32 + (a.estimated_latency_ms as u32 / 10);
            let cost_b = b.fee_bps as u32 + (b.estimated_latency_ms as u32 / 10);
            cost_a.cmp(&cost_b)
        });

        quotes
    }

    /// Build flash loan instructions for optimal provider
    pub async fn build_flash_loan_instructions(
        &self,
        token_mint: &str,
        amount: u64,
        borrower_pubkey: &str,
    ) -> Result<Vec<String>> {
        let quote = self.get_best_quote(token_mint, amount).await?;

        info!(
            "Building flash loan instructions for {:?} provider",
            quote.provider
        );

        match quote.provider {
            FlashLoanProvider::Solend => {
                self.build_solend_instructions(token_mint, amount, borrower_pubkey)
                    .await
            }
            FlashLoanProvider::Marginfi => {
                self.build_marginfi_instructions(token_mint, amount, borrower_pubkey)
                    .await
            }
            FlashLoanProvider::Tulip => {
                self.build_tulip_instructions(token_mint, amount, borrower_pubkey)
                    .await
            }
            _ => {
                // For now, return placeholder instructions
                // In production, would implement actual instruction building
                warn!(
                    "Flash loan provider {:?} not yet implemented",
                    quote.provider
                );
                Ok(vec![
                    "placeholder_borrow_instruction".to_string(),
                    "placeholder_repay_instruction".to_string(),
                ])
            }
        }
    }

    /// Record flash loan success/failure for provider health tracking
    pub fn record_loan_result(&self, provider: FlashLoanProvider, success: bool) {
        if success {
            self.successful_loans
                .entry(provider)
                .and_modify(|count| *count += 1)
                .or_insert(1);

            // Update provider health
            if let Some(mut health) = self.provider_health.get_mut(&provider) {
                health.is_healthy = true;
                health.consecutive_failures = 0;
                health.last_successful_quote = chrono::Utc::now().timestamp_millis() as u64;
            }
        } else {
            self.failed_loans
                .entry(provider)
                .and_modify(|count| *count += 1)
                .or_insert(1);

            // Update provider health
            if let Some(mut health) = self.provider_health.get_mut(&provider) {
                health.consecutive_failures += 1;

                // Mark as unhealthy after 3 consecutive failures
                if health.consecutive_failures >= 3 {
                    health.is_healthy = false;
                    warn!("Marked flash loan provider {:?} as unhealthy", provider);
                }
            }
        }
    }

    /// Get aggregator status and metrics
    pub fn get_status(&self) -> FlashLoanAggregatorStatus {
        FlashLoanAggregatorStatus {
            total_providers: self.providers.len(),
            healthy_providers: self
                .provider_health
                .iter()
                .filter(|entry| entry.value().is_healthy)
                .count(),
            cached_quotes: self.quote_cache.len(),
            provider_stats: self
                .providers
                .iter()
                .map(|provider| {
                    let successful = self.successful_loans.get(provider).map(|v| *v).unwrap_or(0);
                    let failed = self.failed_loans.get(provider).map(|v| *v).unwrap_or(0);
                    let health = self.provider_health.get(provider).unwrap().clone();

                    ProviderStats {
                        provider: provider.clone(),
                        is_healthy: health.is_healthy,
                        successful_loans: successful,
                        failed_loans: failed,
                        success_rate: if successful + failed > 0 {
                            successful as f64 / (successful + failed) as f64
                        } else {
                            0.0
                        },
                        average_latency_ms: health.average_latency_ms,
                    }
                })
                .collect(),
        }
    }

    // Private helper methods

    async fn refresh_quotes(&self, token_mint: &str, amount: u64) -> Result<()> {
        let quotes = self.get_all_quotes(token_mint, amount).await;

        if let Some(best_quote) = quotes.first() {
            self.quote_cache
                .insert(token_mint.to_string(), best_quote.clone());
            debug!(
                "Cached best quote for {}: {:?} @ {} bps",
                token_mint, best_quote.provider, best_quote.fee_bps
            );
        }

        self.last_quote_refresh.store(
            chrono::Utc::now().timestamp_millis() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );

        Ok(())
    }

    fn is_provider_healthy(&self, provider: &FlashLoanProvider) -> bool {
        self.provider_health
            .get(provider)
            .map(|health| health.is_healthy)
            .unwrap_or(false)
    }

    async fn get_provider_quote(
        shyft_client: Arc<ShyftClient>,
        provider: FlashLoanProvider,
        token_mint: String,
        amount: u64,
    ) -> Result<FlashLoanQuote> {
        let start_time = Instant::now();

        // For now, return simulated quotes based on provider characteristics
        // In production, would query actual lending pool states

        let base_fee = provider.base_fee_bps();
        let utilization_fee = Self::calculate_utilization_fee(&provider, &token_mint, amount).await;
        let total_fee = base_fee + utilization_fee;

        let latency = start_time.elapsed().as_millis() as u16;

        Ok(FlashLoanQuote {
            provider,
            token_mint,
            available_amount: amount, // Simplified - assume always available
            fee_bps: total_fee,
            estimated_latency_ms: latency.max(50), // Minimum 50ms
            is_available: true,
            quote_timestamp: chrono::Utc::now().timestamp_millis() as u64,
        })
    }

    async fn calculate_utilization_fee(
        provider: &FlashLoanProvider,
        _token_mint: &str,
        _amount: u64,
    ) -> u16 {
        // Simplified utilization fee calculation
        // In production, would fetch actual pool utilization
        match provider {
            FlashLoanProvider::Solend => 2,   // +0.02% if high utilization
            FlashLoanProvider::Marginfi => 1, // +0.01% if high utilization
            FlashLoanProvider::Tulip => 3,    // +0.03% if high utilization
            _ => 2,
        }
    }

    async fn build_solend_instructions(
        &self,
        _token_mint: &str,
        _amount: u64,
        _borrower_pubkey: &str,
    ) -> Result<Vec<String>> {
        // Placeholder for Solend flash loan instruction building
        // In production, would:
        // 1. Derive lending market and reserve PDAs
        // 2. Build borrow instruction
        // 3. Build repay instruction with fees

        Ok(vec![
            "solend_flash_borrow_instruction".to_string(),
            "solend_flash_repay_instruction".to_string(),
        ])
    }

    async fn build_marginfi_instructions(
        &self,
        _token_mint: &str,
        _amount: u64,
        _borrower_pubkey: &str,
    ) -> Result<Vec<String>> {
        // Placeholder for Marginfi flash loan instruction building
        Ok(vec![
            "marginfi_flash_borrow_instruction".to_string(),
            "marginfi_flash_repay_instruction".to_string(),
        ])
    }

    async fn build_tulip_instructions(
        &self,
        _token_mint: &str,
        _amount: u64,
        _borrower_pubkey: &str,
    ) -> Result<Vec<String>> {
        // Placeholder for Tulip flash loan instruction building
        Ok(vec![
            "tulip_flash_borrow_instruction".to_string(),
            "tulip_flash_repay_instruction".to_string(),
        ])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanAggregatorStatus {
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub cached_quotes: usize,
    pub provider_stats: Vec<ProviderStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    pub provider: FlashLoanProvider,
    pub is_healthy: bool,
    pub successful_loans: u64,
    pub failed_loans: u64,
    pub success_rate: f64,
    pub average_latency_ms: u32,
}
