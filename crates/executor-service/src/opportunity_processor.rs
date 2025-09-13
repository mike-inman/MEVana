use anyhow::Result;
use log::{debug, info};
use std::sync::Arc;

use crate::config::ExecutorConfig;
use crate::execution_engine::ArbitrageOpportunity;
use crate::flash_loan::FlashLoanAggregator;
use crate::wallet_service::WalletService;

/// Processes and validates arbitrage opportunities before execution
pub struct OpportunityProcessor {
    config: Arc<ExecutorConfig>,
    wallet_service: Arc<WalletService>,
    #[allow(dead_code)]
    flash_loan_aggregator: Arc<FlashLoanAggregator>,
}

impl OpportunityProcessor {
    pub fn new(
        config: Arc<ExecutorConfig>,
        wallet_service: Arc<WalletService>,
        flash_loan_aggregator: Arc<FlashLoanAggregator>,
    ) -> Self {
        Self {
            config,
            wallet_service,
            flash_loan_aggregator,
        }
    }

    /// Validate if an opportunity is worth executing
    pub async fn validate_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<bool> {
        debug!("Validating opportunity {}", opportunity.opportunity_id);

        // Check if trading is enabled
        if !self.wallet_service.is_trading_enabled() {
            info!("Trading disabled - skipping opportunity");
            return Ok(false);
        }

        // Check opportunity age
        // Use non-deprecated chrono API. If nanos not available (extreme ranges), fall back to micros * 1000.
        let now_ns = chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1000)
            as u64;
        let age_ms = (now_ns - opportunity.detected_at_ns) / 1_000_000;

        if age_ms > self.config.max_execution_time_ms {
            info!("Opportunity too old: {}ms", age_ms);
            return Ok(false);
        }

        // Check minimum profit threshold
        if opportunity.profit_bps < self.config.min_profit_threshold_bps {
            info!("Profit too low: {} bps", opportunity.profit_bps);
            return Ok(false);
        }

        // Check position size limits
        let max_position_lamports = (self.config.max_position_size_sol * 1_000_000_000.0) as u128;
        if opportunity.max_size_in > max_position_lamports {
            info!(
                "Position size too large: {} lamports",
                opportunity.max_size_in
            );
            return Ok(false);
        }

        // Check wallet can afford the trade
        let token_in_mint = bs58::encode(&opportunity.token_in).into_string();
        let can_afford = self
            .wallet_service
            .can_execute_trade(&token_in_mint, opportunity.max_size_in as u64)?;

        if !can_afford {
            info!("Insufficient balance for trade");
            return Ok(false);
        }

        // All checks passed
        info!("Opportunity validation successful");
        Ok(true)
    }

    /// Calculate expected profit after all fees
    #[allow(dead_code)]
    pub async fn calculate_net_profit(&self, opportunity: &ArbitrageOpportunity) -> Result<i64> {
        let gross_profit = (opportunity.max_size_in as u64 * opportunity.profit_bps) / 10_000;

        // Get flash loan fees
        let token_in_mint = bs58::encode(&opportunity.token_in).into_string();
        let flash_loan_quote = self
            .flash_loan_aggregator
            .get_best_quote(&token_in_mint, opportunity.max_size_in as u64)
            .await?;

        let flash_loan_fee =
            (opportunity.max_size_in as u64 * flash_loan_quote.fee_bps as u64) / 10_000;

        // Estimate transaction fees (simplified)
        let estimated_tx_fees = 5000; // 0.000005 SOL per transaction
        let total_tx_fees = estimated_tx_fees * opportunity.edges.len() as u64;

        let net_profit = gross_profit as i64 - flash_loan_fee as i64 - total_tx_fees as i64;

        debug!(
            "Profit calculation: gross={}, flash_loan_fee={}, tx_fees={}, net={}",
            gross_profit, flash_loan_fee, total_tx_fees, net_profit
        );

        Ok(net_profit)
    }

    /// Check if opportunity conflicts with any active trades
    #[allow(dead_code)]
    pub fn has_trade_conflict(&self, _opportunity: &ArbitrageOpportunity) -> bool {
        // Implementation would check for:
        // 1. Same token pair being traded
        // 2. Overlapping pools/venues
        // 3. Insufficient remaining balance

        // For now, always return false (no conflicts)
        false
    }

    /// Get processing statistics
    #[allow(dead_code)]
    pub fn get_stats(&self) -> OpportunityProcessorStats {
        OpportunityProcessorStats {
            is_trading_enabled: self.wallet_service.is_trading_enabled(),
            max_position_size_sol: self.config.max_position_size_sol,
            min_profit_threshold_bps: self.config.min_profit_threshold_bps,
            max_execution_time_ms: self.config.max_execution_time_ms,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpportunityProcessorStats {
    pub is_trading_enabled: bool,
    pub max_position_size_sol: f64,
    pub min_profit_threshold_bps: u64,
    pub max_execution_time_ms: u64,
}
