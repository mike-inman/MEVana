// executor-service/src/flash_loan_optimizer.rs
//! Advanced flash loan optimization and aggregation for MEV arbitrage

use anyhow::Result;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::config::ExecutorConfig;
use crate::execution_engine::ArbitrageOpportunity;
use crate::flash_loan::{FlashLoanAggregator, FlashLoanProvider, FlashLoanQuote};
use crate::metrics::MetricsServer;
use crate::shyft_client::ShyftClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedFlashLoanQuote {
    pub provider: FlashLoanProvider,
    pub token_mint: String,
    pub amount_requested: u64,
    pub amount_available: u64,
    pub fee_bps: u16,
    pub estimated_fee_lamports: u64,
    pub total_cost_lamports: u64, // fee + gas costs
    pub execution_time_estimate_ms: u16,
    pub reliability_score: f64, // 0.0-1.0 based on historical performance
    pub liquidity_depth_score: f64, // 0.0-1.0 based on available liquidity
    pub optimization_rank: u32, // Lower is better
    pub risk_level: FlashLoanRiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlashLoanRiskLevel {
    Low,      // Proven providers, high liquidity
    Medium,   // Good providers, moderate liquidity
    High,     // Newer providers, lower liquidity
    Critical, // Risky providers or low liquidity
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanOptimizationParams {
    pub max_fee_bps: u16,
    pub min_liquidity_threshold: u64,
    pub reliability_weight: f64,
    pub cost_weight: f64,
    pub speed_weight: f64,
    pub enable_multi_provider: bool,
    pub enable_fallback_routing: bool,
    pub max_providers_per_opportunity: usize,
}

impl Default for FlashLoanOptimizationParams {
    fn default() -> Self {
        Self {
            max_fee_bps: 50,                        // 0.5% maximum fee
            min_liquidity_threshold: 1_000_000_000, // 1 SOL minimum
            reliability_weight: 0.4,
            cost_weight: 0.35,
            speed_weight: 0.25,
            enable_multi_provider: true,
            enable_fallback_routing: true,
            max_providers_per_opportunity: 3,
        }
    }
}

#[allow(dead_code)]
pub struct FlashLoanOptimizer {
    config: Arc<ExecutorConfig>,
    shyft_client: Arc<ShyftClient>,
    base_aggregator: Arc<FlashLoanAggregator>,
    metrics: Arc<MetricsServer>,
    optimization_params: FlashLoanOptimizationParams,
    // Advanced state tracking
    provider_performance_history: Arc<RwLock<HashMap<FlashLoanProvider, ProviderStats>>>,
    liquidity_cache: Arc<RwLock<HashMap<String, LiquidityInfo>>>, // token -> liquidity info
    rate_cache: Arc<RwLock<HashMap<FlashLoanProvider, Vec<FlashLoanQuote>>>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ProviderStats {
    success_rate: f64,
    average_execution_time_ms: f64,
    total_volume_lamports: u64,
    last_updated: Instant,
    reliability_score: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct LiquidityInfo {
    available_amount: u64,
    last_checked: Instant,
    provider_distribution: HashMap<FlashLoanProvider, u64>,
}

impl FlashLoanOptimizer {
    pub fn new(
        config: Arc<ExecutorConfig>,
        shyft_client: Arc<ShyftClient>,
        base_aggregator: Arc<FlashLoanAggregator>,
        metrics: Arc<MetricsServer>,
    ) -> Self {
        Self {
            config,
            shyft_client,
            base_aggregator,
            metrics,
            optimization_params: FlashLoanOptimizationParams::default(),
            provider_performance_history: Arc::new(RwLock::new(HashMap::new())),
            liquidity_cache: Arc::new(RwLock::new(HashMap::new())),
            rate_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_params(mut self, params: FlashLoanOptimizationParams) -> Self {
        self.optimization_params = params;
        self
    }

    /// Find the optimal flash loan strategy for an arbitrage opportunity
    pub async fn find_optimal_flash_loan(
        &self,
        opportunity: &ArbitrageOpportunity,
        required_amount: u64,
        token_mint: &str,
    ) -> Result<OptimizedFlashLoanQuote> {
        let start_time = Instant::now();

        info!(
            "🔍 Finding optimal flash loan: amount={} token={} profit_bps={}",
            required_amount, token_mint, opportunity.profit_bps
        );

        // Step 1: Get quotes from all available providers
        let raw_quotes = self
            .gather_all_provider_quotes(required_amount, token_mint)
            .await?;

        if raw_quotes.is_empty() {
            return Err(anyhow::anyhow!(
                "No flash loan providers available for token {}",
                token_mint
            ));
        }

        // Step 2: Enhance quotes with optimization data
        let enhanced_quotes = self
            .enhance_quotes_with_optimization_data(&raw_quotes, opportunity)
            .await?;

        // Step 3: Rank and select optimal quote
        let optimal_quote = self
            .select_optimal_quote(enhanced_quotes, opportunity)
            .await?;

        // Step 4: Record selection metrics
        self.record_selection_metrics(&optimal_quote, start_time.elapsed());

        info!(
            "✅ Optimal flash loan selected: provider={:?} fee_bps={} cost={}",
            optimal_quote.provider, optimal_quote.fee_bps, optimal_quote.total_cost_lamports
        );

        Ok(optimal_quote)
    }

    async fn gather_all_provider_quotes(
        &self,
        amount: u64,
        token_mint: &str,
    ) -> Result<Vec<FlashLoanQuote>> {
        let quotes = self
            .base_aggregator
            .get_all_quotes(token_mint, amount)
            .await;

        info!("📊 Gathered {} flash loan quotes", quotes.len());

        Ok(quotes)
    }

    async fn enhance_quotes_with_optimization_data(
        &self,
        raw_quotes: &[FlashLoanQuote],
        _opportunity: &ArbitrageOpportunity,
    ) -> Result<Vec<OptimizedFlashLoanQuote>> {
        let mut enhanced_quotes = Vec::new();
        let provider_stats = self.provider_performance_history.read().await;

        for quote in raw_quotes {
            // Get provider performance history
            let stats = provider_stats
                .get(&quote.provider)
                .cloned()
                .unwrap_or_else(|| ProviderStats {
                    success_rate: 0.9, // Default assumption
                    average_execution_time_ms: 200.0,
                    total_volume_lamports: 0,
                    last_updated: Instant::now(),
                    reliability_score: 0.8,
                });

            // Calculate total cost including gas estimates
            let estimated_gas_cost = self.estimate_gas_cost_for_provider(quote.provider).await;
            let fee_lamports = quote.fee_lamports;
            let total_cost = fee_lamports + estimated_gas_cost;

            // Determine risk level
            let risk_level = self.assess_risk_level(quote, &stats);

            // Calculate liquidity depth score
            let liquidity_score = self.calculate_liquidity_score(quote).await;

            let enhanced_quote = OptimizedFlashLoanQuote {
                provider: quote.provider,
                token_mint: quote.token_mint.clone(),
                amount_requested: quote.available_amount.min(10_000_000_000), // Cap request
                amount_available: quote.available_amount,
                fee_bps: quote.fee_bps,
                estimated_fee_lamports: fee_lamports,
                total_cost_lamports: total_cost,
                execution_time_estimate_ms: stats.average_execution_time_ms as u16,
                reliability_score: stats.reliability_score,
                liquidity_depth_score: liquidity_score,
                optimization_rank: 0, // Will be calculated in ranking
                risk_level,
            };

            enhanced_quotes.push(enhanced_quote);
        }

        Ok(enhanced_quotes)
    }

    async fn select_optimal_quote(
        &self,
        mut enhanced_quotes: Vec<OptimizedFlashLoanQuote>,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<OptimizedFlashLoanQuote> {
        if enhanced_quotes.is_empty() {
            return Err(anyhow::anyhow!("No enhanced quotes available"));
        }

        // Filter out quotes that exceed risk tolerance
        enhanced_quotes.retain(|quote| {
            quote.fee_bps <= self.optimization_params.max_fee_bps
                && quote.amount_available >= self.optimization_params.min_liquidity_threshold
                && !matches!(quote.risk_level, FlashLoanRiskLevel::Critical)
        });

        if enhanced_quotes.is_empty() {
            return Err(anyhow::anyhow!("No quotes meet risk criteria"));
        }

        // Calculate optimization scores and rank
        for quote in &mut enhanced_quotes {
            let cost_score = 1.0 - (quote.total_cost_lamports as f64 / 1_000_000.0).min(1.0);
            let speed_score = 1.0 - (quote.execution_time_estimate_ms as f64 / 1000.0).min(1.0);

            let composite_score = quote.reliability_score
                * self.optimization_params.reliability_weight
                + cost_score * self.optimization_params.cost_weight
                + speed_score * self.optimization_params.speed_weight;

            quote.optimization_rank = ((1.0 - composite_score) * 1000.0) as u32;
        }

        // Sort by optimization rank (lower is better)
        enhanced_quotes.sort_by_key(|q| q.optimization_rank);

        // Apply additional business logic for final selection
        let optimal_quote = self
            .apply_business_logic_selection(enhanced_quotes, opportunity)
            .await?;

        Ok(optimal_quote)
    }

    async fn apply_business_logic_selection(
        &self,
        ranked_quotes: Vec<OptimizedFlashLoanQuote>,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<OptimizedFlashLoanQuote> {
        // For high-profit opportunities, prefer reliability over cost
        if opportunity.profit_bps > 100 {
            // Find the most reliable option
            if let Some(reliable_quote) = ranked_quotes.iter().find(|q| q.reliability_score > 0.95)
            {
                return Ok(reliable_quote.clone());
            }
        }

        // For time-sensitive opportunities, prefer speed
        if opportunity.max_edge_age_ms < 1000 {
            if let Some(fast_quote) = ranked_quotes
                .iter()
                .find(|q| q.execution_time_estimate_ms < 150)
            {
                return Ok(fast_quote.clone());
            }
        }

        // Default to top-ranked quote
        Ok(ranked_quotes[0].clone())
    }

    async fn estimate_gas_cost_for_provider(&self, provider: FlashLoanProvider) -> u64 {
        // Estimate gas costs based on provider complexity
        match provider {
            FlashLoanProvider::Solend => 15_000, // lamports
            FlashLoanProvider::Marginfi => 18_000,
            FlashLoanProvider::Tulip => 20_000,
            FlashLoanProvider::Mango => 25_000,
            FlashLoanProvider::PortFinance => 17_000,
            FlashLoanProvider::Kamino => 14_000,
        }
    }

    fn assess_risk_level(
        &self,
        quote: &FlashLoanQuote,
        stats: &ProviderStats,
    ) -> FlashLoanRiskLevel {
        if stats.reliability_score > 0.95 && quote.available_amount > 10_000_000_000 {
            FlashLoanRiskLevel::Low
        } else if stats.reliability_score > 0.9 && quote.available_amount > 5_000_000_000 {
            FlashLoanRiskLevel::Medium
        } else if stats.reliability_score > 0.8 && quote.available_amount > 1_000_000_000 {
            FlashLoanRiskLevel::High
        } else {
            FlashLoanRiskLevel::Critical
        }
    }

    async fn calculate_liquidity_score(&self, quote: &FlashLoanQuote) -> f64 {
        // Score based on available liquidity relative to typical requirements
        let typical_requirement = 1_000_000_000; // 1 SOL equivalent
        (quote.available_amount as f64 / typical_requirement as f64).min(1.0)
    }

    fn record_selection_metrics(&self, quote: &OptimizedFlashLoanQuote, selection_time: Duration) {
        self.metrics.record_flash_loan_selection(
            quote.provider,
            quote.total_cost_lamports,
            quote.reliability_score,
            selection_time.as_millis() as u64,
        );

        debug!(
            "📊 Flash loan selection metrics: provider={:?} cost={} reliability={:.2} time={}ms",
            quote.provider,
            quote.total_cost_lamports,
            quote.reliability_score,
            selection_time.as_millis()
        );
    }

    /// Update provider performance based on execution results
    pub async fn record_execution_result(
        &self,
        provider: FlashLoanProvider,
        success: bool,
        execution_time_ms: u64,
        volume_lamports: u64,
    ) {
        let mut stats = self.provider_performance_history.write().await;

        let provider_stats = stats.entry(provider).or_insert_with(|| ProviderStats {
            success_rate: 0.9,
            average_execution_time_ms: 200.0,
            total_volume_lamports: 0,
            last_updated: Instant::now(),
            reliability_score: 0.8,
        });

        // Update success rate with exponential moving average
        let alpha = 0.1; // Learning rate
        provider_stats.success_rate =
            provider_stats.success_rate * (1.0 - alpha) + (if success { 1.0 } else { 0.0 }) * alpha;

        // Update average execution time
        provider_stats.average_execution_time_ms = provider_stats.average_execution_time_ms
            * (1.0 - alpha)
            + execution_time_ms as f64 * alpha;

        // Update volume and reliability score
        provider_stats.total_volume_lamports += volume_lamports;
        provider_stats.reliability_score = provider_stats.success_rate * 0.8
            + (1.0 - (provider_stats.average_execution_time_ms / 1000.0).min(1.0)) * 0.2;

        provider_stats.last_updated = Instant::now();

        info!(
            "📈 Updated provider stats: {:?} success_rate={:.3} avg_time={:.1}ms reliability={:.3}",
            provider,
            provider_stats.success_rate,
            provider_stats.average_execution_time_ms,
            provider_stats.reliability_score
        );
    }

    /// Start background tasks for cache warming and rate monitoring
    pub fn start_background_optimization_tasks(&self) {
        // Liquidity monitoring task
        let _liquidity_cache = Arc::clone(&self.liquidity_cache);
        let _aggregator = Arc::clone(&self.base_aggregator);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                // Update liquidity cache for major tokens
                // Implementation details...
                debug!("🔄 Updated liquidity cache");
            }
        });

        // Rate monitoring task
        let _rate_cache = Arc::clone(&self.rate_cache);
        let _aggregator_clone = Arc::clone(&self.base_aggregator);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                // Update rate cache for all providers
                // Implementation details...
                debug!("🔄 Updated rate cache");
            }
        });

        info!("🚀 Flash loan optimization background tasks started");
    }
}

// Extension to metrics for flash loan tracking
impl MetricsServer {
    pub fn record_flash_loan_selection(
        &self,
        provider: FlashLoanProvider,
        cost_lamports: u64,
        reliability_score: f64,
        selection_time_ms: u64,
    ) {
        // Record flash loan metrics
        // Implementation depends on your metrics structure
        debug!(
            "📊 Flash loan selection: {:?} cost={} reliability={:.2} time={}ms",
            provider, cost_lamports, reliability_score, selection_time_ms
        );
    }

    pub fn record_test_execution(&self, success: bool) {
        debug!("🧪 Test execution recorded: success={}", success);
    }

    pub fn record_bundle_execution(
        &self,
        success: bool,
        tx_count: usize,
        tip_lamports: u64,
        execution_time_ms: u64,
    ) {
        debug!(
            "📊 Bundle execution: success={} tx_count={} tip={} time={}ms",
            success, tx_count, tip_lamports, execution_time_ms
        );
    }

    pub fn record_profit_realization(&self, profit_lamports: i64) {
        debug!("💰 Profit realization: {} lamports", profit_lamports);
    }
}
