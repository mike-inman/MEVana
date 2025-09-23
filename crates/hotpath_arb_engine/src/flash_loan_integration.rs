use anyhow::Result;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::live_trading_config::LiveTradingConfig;
use crate::marginfi_instructions::{
    calculate_marginfi_flash_loan_fee, create_marginfi_flash_loan_instruction,
    create_marginfi_flash_loan_repay_instruction, validate_marginfi_flash_loan_preconditions,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FlashLoanProvider {
    Marginfi,
    Solend,
    PortFinance,
    Kamino,
}

impl FlashLoanProvider {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "marginfi" => Some(Self::Marginfi),
            "solend" => Some(Self::Solend),
            "portfinance" | "port" => Some(Self::PortFinance),
            "kamino" | "kamino lend" | "k-lend" => Some(Self::Kamino),
            _ => None,
        }
    }

    pub fn program_id(&self) -> Pubkey {
        match self {
            Self::Marginfi => "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA"
                .parse()
                .unwrap(),
            Self::Solend => "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo"
                .parse()
                .unwrap(),
            Self::PortFinance => "Port7uDYB3wk6GJAw4KT1WpTeMtSu9bTcChBHkX2LfR"
                .parse()
                .unwrap(),
            Self::Kamino => "KMino8LendZzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".parse().unwrap(),
        }
    }

    pub fn typical_fee_bps(&self) -> u16 {
        match self {
            Self::Marginfi => 5,     // 0.05%
            Self::Solend => 10,      // 0.10%
            Self::PortFinance => 12, // 0.12%
            Self::Kamino => 1,       // 0.001% (docs indicate low fee)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanQuote {
    pub provider: FlashLoanProvider,
    pub token_mint: Pubkey,
    pub available_amount: u64,
    pub fee_bps: u16,
    pub estimated_cost_lamports: u64,
    pub is_available: bool,
}

#[derive(Debug, Clone)]
pub struct FlashLoanBundle {
    pub flash_loan_instructions: Vec<Instruction>,
    pub arbitrage_instructions: Vec<Instruction>,
    pub repay_instructions: Vec<Instruction>,
    pub provider: FlashLoanProvider,
    pub loan_amount: u64,
    pub token_mint: Pubkey,
}

/// Simplified flash loan system for hotpath arbitrage
#[derive(Debug)]
pub struct HotpathFlashLoanManager {
    config: LiveTradingConfig,
    provider_performance: Arc<RwLock<HashMap<FlashLoanProvider, f64>>>, // success_rate
}

impl HotpathFlashLoanManager {
    pub fn new(config: LiveTradingConfig) -> Self {
        Self {
            config,
            provider_performance: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get best flash loan provider for given token and amount
    pub async fn get_best_provider(
        &self,
        token_mint: &Pubkey,
        amount_sol: f64,
    ) -> Result<Option<FlashLoanQuote>> {
        if !self.config.flash_loan_enabled {
            return Ok(None);
        }

        let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;
        if amount_lamports > (self.config.max_flash_loan_amount_sol * 1_000_000_000.0) as u64 {
            warn!(
                "Flash loan amount {:.6} SOL exceeds max {:.6} SOL",
                amount_sol, self.config.max_flash_loan_amount_sol
            );
            return Ok(None);
        }

        // Get preferred provider from config
        let preferred_provider =
            FlashLoanProvider::from_string(&self.config.preferred_flash_loan_provider);

        if let Some(provider) = preferred_provider {
            let quote = self
                .simulate_flash_loan_quote(provider, *token_mint, amount_lamports)
                .await;
            if quote.is_available {
                info!(
                    "Flash loan available: {} via {:?}, fee: {} bps",
                    amount_sol, provider, quote.fee_bps
                );
                return Ok(Some(quote));
            }
        }

        // Fallback to other providers
        for provider in [
            FlashLoanProvider::Marginfi,
            FlashLoanProvider::Solend,
            FlashLoanProvider::PortFinance,
            FlashLoanProvider::Kamino,
        ] {
            if Some(provider) == preferred_provider {
                continue; // Already tried
            }

            let quote = self
                .simulate_flash_loan_quote(provider, *token_mint, amount_lamports)
                .await;
            if quote.is_available {
                info!(
                    "Flash loan fallback: {} via {:?}, fee: {} bps",
                    amount_sol, provider, quote.fee_bps
                );
                return Ok(Some(quote));
            }
        }

        warn!(
            "No flash loan providers available for {:.6} SOL",
            amount_sol
        );
        Ok(None)
    }

    /// Create flash loan + arbitrage + repay instruction bundle
    pub async fn create_flash_loan_bundle(
        &self,
        quote: &FlashLoanQuote,
        arbitrage_instructions: Vec<Instruction>,
        payer: &Keypair,
    ) -> Result<FlashLoanBundle> {
        let flash_loan_instructions = self.create_flash_loan_instructions(quote, payer).await?;
        let repay_instructions = self.create_repay_instructions(quote, payer).await?;

        Ok(FlashLoanBundle {
            flash_loan_instructions,
            arbitrage_instructions,
            repay_instructions,
            provider: quote.provider,
            loan_amount: quote.available_amount,
            token_mint: quote.token_mint,
        })
    }

    /// Simulate flash loan quote (in production would query real protocols)
    async fn simulate_flash_loan_quote(
        &self,
        provider: FlashLoanProvider,
        token_mint: Pubkey,
        amount_lamports: u64,
    ) -> FlashLoanQuote {
        // Simulate availability (90% success rate)
        let is_available = fastrand::f64() < 0.90;

        let fee_bps = provider.typical_fee_bps();
        let estimated_cost_lamports = match provider {
            FlashLoanProvider::Marginfi => calculate_marginfi_flash_loan_fee(amount_lamports),
            _ => (amount_lamports as f64 * fee_bps as f64 / 10_000.0) as u64,
        };

        FlashLoanQuote {
            provider,
            token_mint,
            available_amount: amount_lamports,
            fee_bps,
            estimated_cost_lamports,
            is_available,
        }
    }

    /// Create real flash loan instructions for the specific protocol
    async fn create_flash_loan_instructions(
        &self,
        quote: &FlashLoanQuote,
        payer: &Keypair,
    ) -> Result<Vec<Instruction>> {
        info!(
            "Creating flash loan instructions for {:?}, amount: {} lamports",
            quote.provider, quote.available_amount
        );

        match quote.provider {
            FlashLoanProvider::Marginfi => {
                // Validate preconditions
                validate_marginfi_flash_loan_preconditions(
                    quote.available_amount,
                    (self.config.max_flash_loan_amount_sol * 1_000_000_000.0) as u64,
                )?;

                // Create real Marginfi flash loan instruction
                let instruction = create_marginfi_flash_loan_instruction(
                    &payer.pubkey(),
                    quote.available_amount,
                )?;

                Ok(vec![instruction])
            }
            _ => {
                // For other providers, implement similar real protocol calls
                warn!(
                    "Provider {:?} not yet implemented with real instructions",
                    quote.provider
                );
                Ok(vec![])
            }
        }
    }

    /// Create real repay instructions for the specific protocol
    async fn create_repay_instructions(
        &self,
        quote: &FlashLoanQuote,
        payer: &Keypair,
    ) -> Result<Vec<Instruction>> {
        let repay_amount = quote.available_amount + quote.estimated_cost_lamports;

        info!(
            "Creating repay instructions for {:?}, repay amount: {} lamports",
            quote.provider, repay_amount
        );

        match quote.provider {
            FlashLoanProvider::Marginfi => {
                // Create real Marginfi flash loan repay instruction
                let instruction =
                    create_marginfi_flash_loan_repay_instruction(&payer.pubkey(), repay_amount)?;

                Ok(vec![instruction])
            }
            _ => {
                // For other providers, implement similar real protocol calls
                warn!(
                    "Provider {:?} repay not yet implemented with real instructions",
                    quote.provider
                );
                Ok(vec![])
            }
        }
    }

    /// Record flash loan execution result for provider performance tracking
    pub async fn record_flash_loan_result(&self, provider: FlashLoanProvider, success: bool) {
        let mut performance = self.provider_performance.write().await;
        let current_rate = performance.get(&provider).copied().unwrap_or(0.5);

        // Simple exponential moving average
        let new_rate = if success {
            current_rate * 0.9 + 0.1
        } else {
            current_rate * 0.9
        };

        performance.insert(provider, new_rate);
        info!(
            "Updated {:?} performance: {:.2}%",
            provider,
            new_rate * 100.0
        );
    }

    /// Get flash loan statistics
    pub async fn get_stats(&self) -> String {
        let performance = self.provider_performance.read().await;
        let mut stats = format!(
            "Flash Loan System:\n\
             ✅ Enabled: {}\n\
             💰 Max Amount: {:.3} SOL\n\
             🎯 Preferred: {}\n\n\
             Provider Performance:\n",
            self.config.flash_loan_enabled,
            self.config.max_flash_loan_amount_sol,
            self.config.preferred_flash_loan_provider
        );

        for provider in [
            FlashLoanProvider::Marginfi,
            FlashLoanProvider::Solend,
            FlashLoanProvider::PortFinance,
            FlashLoanProvider::Kamino,
        ] {
            let rate = performance.get(&provider).copied().unwrap_or(0.0);
            stats.push_str(&format!(
                "  {:?}: {:.1}% success, {} bps fee\n",
                provider,
                rate * 100.0,
                provider.typical_fee_bps()
            ));
        }

        stats
    }
}
