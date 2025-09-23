use anyhow::Result;
use chrono::{Timelike, Utc};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::orderbook_tracker::ArbitrageEvent;
use crate::property_store::get_property_store;

/// Simple but effective profit filter to eliminate unprofitable trades
/// Goal: $1-2/minute consistently by executing only high-confidence opportunities
#[derive(Debug, Clone)]
pub struct SmartFilter {
    config: SmartFilterConfig,
    stats: Arc<RwLock<FilterStats>>,
    gas_oracle: GasOracle,
    success_tracker: Arc<RwLock<SuccessTracker>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFilterConfig {
    // Minimum net profit after all costs (in SOL)
    pub min_net_profit_sol: f64,

    // Minimum gross profit in basis points
    pub min_profit_bps: i32,

    // Maximum profit bps (avoid "too good to be true" opportunities)
    pub max_profit_bps: i32,

    // Minimum success rate for venue pairs before executing
    pub min_venue_success_rate: f64,

    // Maximum percentage of profit to spend on gas
    pub max_gas_budget_pct: f64,

    // Competition detection thresholds
    pub max_concurrent_opportunities: u32,
    pub competition_lookback_ms: u64,

    // Learning parameters
    pub learning_rate: f64,
    pub min_samples_for_learning: u64,
}

impl Default for SmartFilterConfig {
    fn default() -> Self {
        Self {
            min_net_profit_sol: 0.0001, // ~$0.02 minimum profit - very low for testing
            min_profit_bps: 1,          // 1 bps minimum - very permissive
            max_profit_bps: 500,        // Allow higher opportunities for testing
            min_venue_success_rate: 0.0, // Start with no filtering, learn over time
            max_gas_budget_pct: 80.0,   // Allow higher gas budget for testing
            max_concurrent_opportunities: 20, // Allow more concurrent opportunities
            competition_lookback_ms: 5000, // Look back 5 seconds for competition
            learning_rate: 0.1,         // How fast to adapt to new data
            min_samples_for_learning: 5, // Minimum attempts before trusting success rates
        }
    }
}

impl SmartFilterConfig {
    /// Load configuration from database, fallback to defaults if database unavailable
    pub async fn load_from_database(db_pool: Option<&sqlx::PgPool>) -> Self {
        match db_pool {
            Some(pool) => match Self::fetch_latest_config(pool).await {
                Ok(config) => {
                    log::info!("📊 Loaded SmartFilter config from database: min_profit={}bps, min_net={:.6} SOL", 
                                   config.min_profit_bps, config.min_net_profit_sol);
                    config
                }
                Err(e) => {
                    log::warn!(
                        "Failed to load SmartFilter config from database: {}, using defaults",
                        e
                    );
                    Self::default()
                }
            },
            None => {
                log::warn!("No database pool available, using SmartFilter defaults");
                Self::default()
            }
        }
    }

    /// Fetch the latest configuration values from the database
    async fn fetch_latest_config(pool: &sqlx::PgPool) -> Result<Self, sqlx::Error> {
        let mut config = Self::default();

        // Query to get the latest value for each parameter
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT ON (parameter_name)
                parameter_name,
                new_value
            FROM smart_filter_config_history
            WHERE parameter_name IN (
                'min_net_profit_sol', 'min_profit_bps', 'max_profit_bps',
                'max_gas_budget_pct', 'max_concurrent_opportunities',
                'competition_lookback_ms', 'learning_rate', 'min_venue_success_rate'
            )
            ORDER BY parameter_name, timestamp DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        // Apply database values to config
        for row in rows {
            let parameter_name: String = row.try_get("parameter_name").unwrap_or_default();
            let new_value: Option<String> = row.try_get("new_value").ok();

            match parameter_name.as_str() {
                "min_net_profit_sol" => {
                    if let Some(val) = new_value.as_deref() {
                        config.min_net_profit_sol =
                            val.parse().unwrap_or(config.min_net_profit_sol);
                    }
                }
                "min_profit_bps" => {
                    if let Some(val) = new_value.as_deref() {
                        config.min_profit_bps = val.parse().unwrap_or(config.min_profit_bps);
                    }
                }
                "max_profit_bps" => {
                    if let Some(val) = new_value.as_deref() {
                        config.max_profit_bps = val.parse().unwrap_or(config.max_profit_bps);
                    }
                }
                "max_gas_budget_pct" => {
                    if let Some(val) = new_value.as_deref() {
                        config.max_gas_budget_pct =
                            val.parse().unwrap_or(config.max_gas_budget_pct);
                    }
                }
                "max_concurrent_opportunities" => {
                    if let Some(val) = new_value.as_deref() {
                        config.max_concurrent_opportunities =
                            val.parse().unwrap_or(config.max_concurrent_opportunities);
                    }
                }
                "competition_lookback_ms" => {
                    if let Some(val) = new_value.as_deref() {
                        config.competition_lookback_ms =
                            val.parse().unwrap_or(config.competition_lookback_ms);
                    }
                }
                "learning_rate" => {
                    if let Some(val) = new_value.as_deref() {
                        config.learning_rate = val.parse().unwrap_or(config.learning_rate);
                    }
                }
                "min_venue_success_rate" => {
                    if let Some(val) = new_value.as_deref() {
                        config.min_venue_success_rate =
                            val.parse().unwrap_or(config.min_venue_success_rate);
                    }
                }
                _ => {}
            }
        }

        Ok(config)
    }

    /// Development environment defaults (more aggressive for testing)
    pub fn development_defaults() -> Self {
        Self {
            min_net_profit_sol: 0.0005,      // $0.10 minimum profit
            min_profit_bps: 1,               // 1 bps minimum (very low for testing)
            max_profit_bps: 200,             // Allow high profit for testing
            min_venue_success_rate: 0.0,     // No filtering initially
            max_gas_budget_pct: 40.0,        // More gas budget for testing
            max_concurrent_opportunities: 5, // More concurrent for testing
            competition_lookback_ms: 3000,   // Shorter lookback
            learning_rate: 0.2,              // Faster learning in dev
            min_samples_for_learning: 3,     // Fewer samples needed
        }
    }

    /// Staging environment defaults (conservative testing)
    pub fn staging_defaults() -> Self {
        Self {
            min_net_profit_sol: 0.001,       // $0.20 minimum profit
            min_profit_bps: 10,              // 10 bps minimum
            max_profit_bps: 150,             // Moderate profit cap
            min_venue_success_rate: 0.0,     // No filtering initially
            max_gas_budget_pct: 35.0,        // Moderate gas budget
            max_concurrent_opportunities: 4, // Moderate concurrency
            competition_lookback_ms: 4000,   // Medium lookback
            learning_rate: 0.15,             // Moderate learning rate
            min_samples_for_learning: 4,     // Moderate samples needed
        }
    }

    /// Production environment defaults (conservative and safe)
    pub fn production_defaults() -> Self {
        Self {
            min_net_profit_sol: 0.002,       // $0.40 minimum profit (conservative)
            min_profit_bps: 20,              // 20 bps minimum (conservative)
            max_profit_bps: 100, // Conservative profit cap to avoid suspicious opportunities
            min_venue_success_rate: 0.0, // Start with no filtering, learn over time
            max_gas_budget_pct: 25.0, // Conservative gas budget
            max_concurrent_opportunities: 2, // Conservative concurrency
            competition_lookback_ms: 6000, // Longer lookback for better competition detection
            learning_rate: 0.05, // Slower, more stable learning
            min_samples_for_learning: 10, // More samples for confidence
        }
    }

    /// Safe fallback defaults when database is unavailable
    pub fn safe_fallback_defaults() -> Self {
        Self {
            min_net_profit_sol: 0.005,       // Very conservative - $1.00 minimum
            min_profit_bps: 50,              // Very high minimum (safe mode)
            max_profit_bps: 80,              // Low max to avoid risky opportunities
            min_venue_success_rate: 0.8,     // Only trade on very successful venues
            max_gas_budget_pct: 15.0,        // Very low gas budget
            max_concurrent_opportunities: 1, // Only one opportunity at a time
            competition_lookback_ms: 10000,  // Long lookback for safety
            learning_rate: 0.01,             // Very slow learning
            min_samples_for_learning: 20,    // Many samples needed
        }
    }

    /// Validate configuration for safety
    pub fn validate(&self) -> Result<(), String> {
        if self.min_profit_bps > self.max_profit_bps {
            return Err("min_profit_bps cannot be greater than max_profit_bps".to_string());
        }

        if self.max_gas_budget_pct > 50.0 {
            return Err("max_gas_budget_pct cannot exceed 50% (too risky)".to_string());
        }

        if self.min_net_profit_sol <= 0.0 {
            return Err("min_net_profit_sol must be positive".to_string());
        }

        if self.learning_rate <= 0.0 || self.learning_rate > 1.0 {
            return Err("learning_rate must be between 0 and 1".to_string());
        }

        if self.max_concurrent_opportunities > 10 {
            return Err(
                "max_concurrent_opportunities cannot exceed 10 (risk management)".to_string(),
            );
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct FilterStats {
    total_opportunities: u64,
    passed_profit_filter: u64,
    passed_gas_filter: u64,
    passed_competition_filter: u64,
    executed: u64,

    // Track rejections by reason
    rejected_low_profit: u64,
    rejected_high_gas: u64,
    rejected_suspicious: u64,
    rejected_low_success_rate: u64,
    rejected_competition: u64,

    // Real-time learning stats
    adaptive_adjustments: u64,
    profit_predictions: u64,
    prediction_accuracy: f64,
}

/// Simple gas cost estimation
#[derive(Debug, Clone)]
struct GasOracle {
    base_fee_lamports: u64,
    priority_fee_lamports: u64,
    compute_units_estimate: u64,
    last_update: SystemTime,
}

impl Default for GasOracle {
    fn default() -> Self {
        Self {
            base_fee_lamports: 5000,         // Base transaction fee
            priority_fee_lamports: 10000,    // Conservative priority fee
            compute_units_estimate: 400_000, // Typical arbitrage transaction
            last_update: UNIX_EPOCH,
        }
    }
}

/// Track success rates for different patterns
#[derive(Debug, Default)]
struct SuccessTracker {
    venue_pair_stats: HashMap<String, VenueStats>,
    token_pair_stats: HashMap<String, TokenStats>,
    hourly_stats: HashMap<u8, HourlyStats>,
    recent_opportunities: Vec<RecentOpportunity>,
    competition_detector: CompetitionDetector,
}

#[derive(Debug, Clone)]
struct RecentOpportunity {
    venue_pair: String,
    token_pair: String,
    profit_bps: i32,
    timestamp_ms: u64,
    executed: bool,
}

#[derive(Debug, Default)]
struct CompetitionDetector {
    concurrent_count: HashMap<String, u32>, // venue_pair -> count in recent window
    last_cleanup: u64,
}

#[derive(Debug, Default, Clone)]
struct VenueStats {
    attempts: u64,
    successes: u64,
    total_profit: f64,
    avg_execution_time_us: f64,
    recent_profit_estimates: VecDeque<f64>, // CRITICAL FIX: Use VecDeque for O(1) operations
    prediction_errors: VecDeque<f64>,       // CRITICAL FIX: Use VecDeque for O(1) operations
    last_updated: u64,
}

#[derive(Debug, Default, Clone)]
struct TokenStats {
    attempts: u64,
    successes: u64,
    avg_profit_bps: f64,
}

#[derive(Debug, Default, Clone)]
struct HourlyStats {
    attempts: u64,
    successes: u64,
    avg_profit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartOpportunity {
    pub original: ArbitrageEvent,
    pub estimated_gas_cost_sol: f64,
    pub estimated_net_profit_sol: f64,
    pub confidence_score: f64,
    pub should_execute: bool,
    pub rejection_reason: Option<String>,
    pub competition_score: f64,
    pub predicted_success_probability: f64,
}

#[derive(Debug)]
pub struct EnhancedFilterStats {
    pub basic_stats: FilterStats,
    pub active_competitions: usize,
    pub learned_venue_pairs: usize,
    pub avg_competition_score: f64,
    pub prediction_accuracy: f64,
}

impl SmartFilter {
    pub fn new(config: SmartFilterConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(FilterStats::default())),
            gas_oracle: GasOracle::default(),
            success_tracker: Arc::new(RwLock::new(SuccessTracker::default())),
        }
    }

    /// Main filtering function - should we execute this opportunity?
    pub async fn should_execute(&self, opportunity: &ArbitrageEvent) -> SmartOpportunity {
        let mut stats = self.stats.write().await;
        stats.total_opportunities += 1;

        // Step 1: Calculate gas costs
        let gas_cost_sol = self.estimate_gas_cost().await;

        // Step 2: Calculate net profit
        let trade_size_sol = opportunity.amount_in * 1e-9; // Assuming amount_in is in lamports
        let gross_profit_sol = (opportunity.profit_bps as f64 / 10000.0) * trade_size_sol;
        let net_profit_sol = gross_profit_sol - gas_cost_sol;

        // Step 3: Apply filters
        let mut rejection_reason = None;
        let mut should_execute = true;

        // Filter 1: Minimum net profit - Using dynamic PropertyStore values
        let min_net_profit_lamports = get_property_store()
            .get_atomic_u64("min_net_profit_lamports")
            .unwrap_or((self.config.min_net_profit_sol * 1_000_000_000.0) as u64);
        let min_net_profit_sol = min_net_profit_lamports as f64 / 1_000_000_000.0;

        if net_profit_sol < min_net_profit_sol {
            should_execute = false;
            rejection_reason = Some(format!(
                "Net profit {:.6} SOL below minimum {:.6} SOL",
                net_profit_sol, min_net_profit_sol
            ));
            stats.rejected_low_profit += 1;
        }

        // Filter 2: Profit BPS range - Using dynamic PropertyStore values
        let min_profit_bps = get_property_store()
            .get_atomic_i32("min_profit_bps")
            .unwrap_or(self.config.min_profit_bps);
        if opportunity.profit_bps < min_profit_bps {
            should_execute = false;
            rejection_reason = Some(format!(
                "Profit {}bps below minimum {}bps",
                opportunity.profit_bps, min_profit_bps
            ));
            stats.rejected_low_profit += 1;
        }

        let max_profit_bps = get_property_store()
            .get_atomic_i32("max_profit_bps")
            .unwrap_or(self.config.max_profit_bps);
        if opportunity.profit_bps > max_profit_bps {
            should_execute = false;
            rejection_reason = Some(format!(
                "Profit {}bps suspiciously high (max {}bps)",
                opportunity.profit_bps, max_profit_bps
            ));
            stats.rejected_suspicious += 1;
        }

        // Filter 3: Gas budget check - Using dynamic PropertyStore values
        let gas_pct_of_profit = (gas_cost_sol / gross_profit_sol) * 100.0;
        let max_gas_budget_pct = get_property_store()
            .get_atomic_f64("max_gas_budget_pct")
            .unwrap_or(self.config.max_gas_budget_pct);
        if gas_pct_of_profit > max_gas_budget_pct {
            should_execute = false;
            rejection_reason = Some(format!(
                "Gas cost {:.1}% of profit exceeds {:.1}% limit",
                gas_pct_of_profit, max_gas_budget_pct
            ));
            stats.rejected_high_gas += 1;
        }

        // Filter 4: Competition detection
        let venue_pair = format!("{}->{}", opportunity.venue_sell, opportunity.venue_buy);
        let competition_score = self.detect_competition(&venue_pair, opportunity).await;
        if competition_score > 0.95 {
            // Relaxed competition threshold for demo
            should_execute = false;
            rejection_reason = Some(format!(
                "High competition detected: {:.1}% (>95%)",
                competition_score * 100.0
            ));
            stats.rejected_competition += 1;
        }

        // Filter 5: Historical success rate with learning - Using dynamic PropertyStore values
        let success_rate = self.get_venue_success_rate(&venue_pair).await;
        let min_attempts = get_property_store()
            .get_atomic_u64("min_samples_for_learning")
            .unwrap_or(self.config.min_samples_for_learning);
        let min_venue_success_rate = get_property_store()
            .get_atomic_f64("min_venue_success_rate")
            .unwrap_or(self.config.min_venue_success_rate);

        if success_rate < min_venue_success_rate && stats.total_opportunities > min_attempts {
            should_execute = false;
            rejection_reason = Some(format!(
                "Venue pair {} success rate {:.1}% below minimum {:.1}%",
                venue_pair,
                success_rate * 100.0,
                min_venue_success_rate * 100.0
            ));
            stats.rejected_low_success_rate += 1;
        }

        // Calculate enhanced metrics
        let confidence_score = self
            .calculate_confidence(opportunity, net_profit_sol, success_rate, competition_score)
            .await;
        let predicted_success = self
            .predict_success_probability(&venue_pair, opportunity)
            .await;

        // Update stats
        if should_execute {
            stats.passed_profit_filter += 1;
            if gas_pct_of_profit <= self.config.max_gas_budget_pct {
                stats.passed_gas_filter += 1;
            }
            if success_rate >= self.config.min_venue_success_rate || stats.total_opportunities <= 10
            {
                stats.passed_competition_filter += 1;
            }
        }

        // Record this opportunity for competition tracking
        self.record_opportunity(&venue_pair, opportunity, should_execute)
            .await;

        SmartOpportunity {
            original: opportunity.clone(),
            estimated_gas_cost_sol: gas_cost_sol,
            estimated_net_profit_sol: net_profit_sol,
            confidence_score,
            should_execute,
            rejection_reason,
            competition_score,
            predicted_success_probability: predicted_success,
        }
    }

    /// Record the result of an executed trade for learning
    pub async fn record_result(
        &self,
        opportunity: &ArbitrageEvent,
        success: bool,
        actual_profit_sol: f64,
    ) {
        let venue_pair = format!("{}->{}", opportunity.venue_sell, opportunity.venue_buy);
        let hour = Utc::now().hour() as u8;

        // Real-time profit validation and learning
        self.update_profit_prediction_accuracy(opportunity, actual_profit_sol)
            .await;

        // Update all stats in one mutable borrow scope
        let (venue_attempts, venue_successes, venue_total_profit) = {
            let mut tracker = self.success_tracker.write().await;

            // First, update venue pair stats and capture values for logging
            {
                let venue_stats = tracker
                    .venue_pair_stats
                    .entry(venue_pair.clone())
                    .or_default();
                venue_stats.attempts += 1;
                if success {
                    venue_stats.successes += 1;
                    venue_stats.total_profit += actual_profit_sol;
                }
            }

            // Then update token pair stats
            {
                let token_stats = tracker
                    .token_pair_stats
                    .entry(opportunity.token_pair.clone())
                    .or_default();
                token_stats.attempts += 1;
                if success {
                    token_stats.successes += 1;
                    token_stats.avg_profit_bps = (token_stats.avg_profit_bps
                        * (token_stats.successes - 1) as f64
                        + opportunity.profit_bps as f64)
                        / token_stats.successes as f64;
                }
            }

            // Then update hourly stats
            {
                let hourly_stats = tracker.hourly_stats.entry(hour).or_default();
                hourly_stats.attempts += 1;
                if success {
                    hourly_stats.successes += 1;
                    hourly_stats.avg_profit += actual_profit_sol;
                }
            }

            // Finally get venue stats for logging (fresh borrow)
            let venue_stats = &tracker.venue_pair_stats[&venue_pair];
            (
                venue_stats.attempts,
                venue_stats.successes,
                venue_stats.total_profit,
            )
        };

        // Update execution count
        let mut stats = self.stats.write().await;
        stats.executed += 1;

        // Log learning
        let success_rate = venue_successes as f64 / venue_attempts as f64;
        debug!(
            "📊 LEARNING: {} attempts={}, success_rate={:.1}%, avg_profit={:.6} SOL",
            venue_pair,
            venue_attempts,
            success_rate * 100.0,
            venue_total_profit / venue_successes.max(1) as f64
        );
    }

    async fn estimate_gas_cost(&self) -> f64 {
        // Simple gas estimation - in production this could query RPC for current fees
        let total_cost_lamports =
            self.gas_oracle.base_fee_lamports + self.gas_oracle.priority_fee_lamports;
        total_cost_lamports as f64 * 0.000000001 // Convert to SOL
    }

    async fn get_venue_success_rate(&self, venue_pair: &str) -> f64 {
        let tracker = self.success_tracker.read().await;
        if let Some(stats) = tracker.venue_pair_stats.get(venue_pair) {
            if stats.attempts > 0 {
                return stats.successes as f64 / stats.attempts as f64;
            }
        }
        0.5 // Default to neutral 50% if no data
    }

    async fn detect_competition(&self, venue_pair: &str, opportunity: &ArbitrageEvent) -> f64 {
        let tracker = self.success_tracker.read().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let lookback_ms = get_property_store()
            .get_atomic_u64("competition_lookback_ms")
            .unwrap_or(self.config.competition_lookback_ms);
        let recent_similar = tracker
            .recent_opportunities
            .iter()
            .filter(|opp| {
                opp.venue_pair == venue_pair
                    && (now - opp.timestamp_ms) < lookback_ms
                    && (opp.profit_bps - opportunity.profit_bps).abs() < 10 // Similar profit range
            })
            .count();

        let max_concurrent = get_property_store()
            .get_atomic_u32("max_concurrent_opportunities")
            .unwrap_or(self.config.max_concurrent_opportunities);
        if recent_similar >= max_concurrent as usize {
            0.8 // High competition
        } else if recent_similar > 1 {
            0.4 + (recent_similar as f64 * 0.1) // Moderate competition
        } else {
            0.1 // Low competition
        }
    }

    async fn predict_success_probability(
        &self,
        venue_pair: &str,
        opportunity: &ArbitrageEvent,
    ) -> f64 {
        let tracker = self.success_tracker.read().await;

        if let Some(stats) = tracker.venue_pair_stats.get(venue_pair) {
            let min_samples = get_property_store()
                .get_atomic_u64("min_samples_for_learning")
                .unwrap_or(self.config.min_samples_for_learning);
            if stats.attempts >= min_samples {
                let base_success_rate = stats.successes as f64 / stats.attempts as f64;

                // Adjust based on profit level
                let profit_multiplier = if opportunity.profit_bps > 30 {
                    0.9 // Higher profit = more likely to face competition/fail
                } else if opportunity.profit_bps < 15 {
                    0.8 // Lower profit = less attractive
                } else {
                    1.0 // Sweet spot
                };

                // Adjust based on recent performance
                let recent_weight = 0.3;
                let historical_weight = 0.7;

                (base_success_rate * historical_weight + 0.5 * recent_weight) * profit_multiplier
            } else {
                0.5 // Neutral prediction with insufficient data
            }
        } else {
            0.4 // Slightly pessimistic for unknown venue pairs
        }
    }

    async fn record_opportunity(
        &self,
        venue_pair: &str,
        opportunity: &ArbitrageEvent,
        executed: bool,
    ) {
        let mut tracker = self.success_tracker.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Add to recent opportunities
        tracker.recent_opportunities.push(RecentOpportunity {
            venue_pair: venue_pair.to_string(),
            token_pair: opportunity.token_pair.clone(),
            profit_bps: opportunity.profit_bps,
            timestamp_ms: now,
            executed,
        });

        // Keep only recent opportunities (last 30 seconds)
        tracker
            .recent_opportunities
            .retain(|opp| (now - opp.timestamp_ms) < 30000);
    }

    async fn calculate_confidence(
        &self,
        opportunity: &ArbitrageEvent,
        net_profit_sol: f64,
        success_rate: f64,
        competition_score: f64,
    ) -> f64 {
        let mut confidence = 50.0; // Start at 50%

        // Higher profit = higher confidence
        if net_profit_sol > 0.002 {
            confidence += 20.0;
        } // >$0.40 profit
        if net_profit_sol > 0.005 {
            confidence += 15.0;
        } // >$1.00 profit

        // Historical success boosts confidence
        confidence += success_rate * 30.0; // Up to 30% boost for 100% success rate

        // Low competition boosts confidence
        confidence += (1.0 - competition_score) * 20.0; // Up to 20% boost for low competition

        // Moderate profit BPS is good (not too low, not suspiciously high)
        if opportunity.profit_bps >= 20 && opportunity.profit_bps <= 50 {
            confidence += 10.0;
        }

        // Fast execution time is good
        if opportunity.execution_time_us < 1000 {
            confidence += 5.0;
        }

        confidence.min(95.0) / 100.0 // Cap at 95% and convert to 0-1 range
    }

    /// Get current filter statistics
    pub async fn get_stats(&self) -> FilterStats {
        self.stats.read().await.clone()
    }

    /// Get success rates by venue
    pub async fn get_venue_performance(&self) -> HashMap<String, (f64, u64)> {
        let tracker = self.success_tracker.read().await;
        tracker
            .venue_pair_stats
            .iter()
            .map(|(venue, stats)| {
                let success_rate = if stats.attempts > 0 {
                    stats.successes as f64 / stats.attempts as f64
                } else {
                    0.0
                };
                (venue.clone(), (success_rate, stats.attempts))
            })
            .collect()
    }

    async fn update_profit_prediction_accuracy(
        &self,
        opportunity: &ArbitrageEvent,
        actual_profit_sol: f64,
    ) {
        let venue_pair = format!("{}->{}", opportunity.venue_sell, opportunity.venue_buy);
        let trade_size_sol = opportunity.amount_in * 1e-9;
        let predicted_gross_profit = (opportunity.profit_bps as f64 / 10000.0) * trade_size_sol;
        let prediction_error = (predicted_gross_profit - actual_profit_sol).abs();

        let mut tracker = self.success_tracker.write().await;
        let venue_stats = tracker
            .venue_pair_stats
            .entry(venue_pair.clone())
            .or_default();

        // Update profit prediction tracking (CRITICAL FIX: Use O(1) VecDeque operations)
        venue_stats
            .recent_profit_estimates
            .push_back(predicted_gross_profit);
        venue_stats.prediction_errors.push_back(prediction_error);

        // Keep only last 10 predictions for efficiency
        if venue_stats.recent_profit_estimates.len() > 10 {
            venue_stats.recent_profit_estimates.pop_front();
            venue_stats.prediction_errors.pop_front();
        }

        // Update global prediction accuracy
        let mut stats = self.stats.write().await;
        stats.profit_predictions += 1;

        // Calculate rolling accuracy (lower error = higher accuracy)
        let avg_error = venue_stats.prediction_errors.iter().sum::<f64>()
            / venue_stats.prediction_errors.len() as f64;
        let accuracy = (1.0 - (avg_error / predicted_gross_profit).min(1.0)).max(0.0);
        stats.prediction_accuracy = (stats.prediction_accuracy * 0.9) + (accuracy * 0.1); // Exponential smoothing

        // Log learning progress
        if stats.profit_predictions % 10 == 0 {
            info!(
                "🧠 PROFIT LEARNING: {} predictions, {:.1}% accuracy, avg_error={:.6} SOL",
                stats.profit_predictions,
                stats.prediction_accuracy * 100.0,
                avg_error
            );
        }
    }

    /// Get enhanced statistics with competition and learning metrics
    pub async fn get_enhanced_stats(&self) -> EnhancedFilterStats {
        let stats = self.stats.read().await;
        let tracker = self.success_tracker.read().await;

        EnhancedFilterStats {
            basic_stats: stats.clone(),
            active_competitions: tracker.recent_opportunities.len(),
            learned_venue_pairs: tracker.venue_pair_stats.len(),
            avg_competition_score: tracker
                .recent_opportunities
                .iter()
                .map(|opp| if opp.executed { 0.3 } else { 0.7 }) // Rough estimate
                .sum::<f64>()
                / tracker.recent_opportunities.len().max(1) as f64,
            prediction_accuracy: stats.prediction_accuracy,
        }
    }

    /// Update configuration (for runtime tuning)
    pub fn update_config(&mut self, config: SmartFilterConfig) {
        self.config = config;
        info!("🔧 Smart filter config updated: min_profit={:.6} SOL, min_bps={}, max_bps={}, competition_threshold={}",
              self.config.min_net_profit_sol, self.config.min_profit_bps, self.config.max_profit_bps,
              self.config.max_concurrent_opportunities);
    }

    /// Get current configuration (for API access)
    pub fn get_config(&self) -> &SmartFilterConfig {
        &self.config
    }
}
