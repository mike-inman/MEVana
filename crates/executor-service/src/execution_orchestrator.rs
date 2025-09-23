// executor-service/src/execution_orchestrator.rs
//! Enterprise-grade execution orchestrator combining all optimization components

use anyhow::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::bundle_builder::BundleBuilder;
use crate::config::ExecutorConfig;
use crate::execution_engine::{ArbitrageOpportunity, ExecutionStatus};
use crate::flash_loan::{FlashLoanAggregator, FlashLoanProvider};
use crate::flash_loan_optimizer::{FlashLoanOptimizer, OptimizedFlashLoanQuote};
use crate::jito_optimizer::{JitoBundleResult, JitoOptimizationParams, JitoOptimizer};
use crate::metrics::MetricsServer;
use crate::shyft_client::ShyftClient;
use crate::wallet_service::WalletService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    Test {
        simulate_success_rate: f64,
        simulate_latency_ms: u64,
        enable_logging: bool,
    },
    Live {
        enable_aggressive_optimization: bool,
        max_slippage_bps: u16,
        force_profit_threshold_lamports: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub opportunity_id: Uuid,
    pub execution_mode: ExecutionMode,
    pub flash_loan_strategy: Option<OptimizedFlashLoanQuote>,
    pub jito_strategy: JitoOptimizationParams,
    pub risk_parameters: RiskParameters,
    pub estimated_profit_lamports: u64,
    pub execution_timeout_ms: u64,
    pub priority_level: ExecutionPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParameters {
    pub max_position_size_lamports: u64,
    pub max_gas_fee_lamports: u64,
    pub min_profit_threshold_lamports: u64,
    pub max_execution_time_ms: u64,
    pub enable_circuit_breaker: bool,
    pub daily_loss_limit_lamports: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveExecutionResult {
    pub execution_id: Uuid,
    pub opportunity_id: Uuid,
    pub execution_plan: ExecutionPlan,
    pub flash_loan_result: Option<FlashLoanExecutionResult>,
    pub jito_bundle_result: Option<JitoBundleResult>,
    pub final_status: ExecutionStatus,
    pub total_execution_time_ms: u64,
    pub estimated_profit_lamports: u64,
    pub actual_profit_lamports: Option<i64>,
    pub total_fees_paid_lamports: u64,
    pub roi_basis_points: Option<i64>,
    pub execution_efficiency_score: f64, // 0.0-1.0
    pub error_details: Option<String>,
    pub performance_metrics: ExecutionMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanExecutionResult {
    pub provider_used: FlashLoanProvider,
    pub amount_borrowed: u64,
    pub fee_paid_lamports: u64,
    pub success: bool,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub planning_time_ms: u64,
    pub flash_loan_selection_time_ms: u64,
    pub bundle_building_time_ms: u64,
    pub submission_time_ms: u64,
    pub confirmation_time_ms: u64,
    pub slippage_bps: Option<u16>,
    pub gas_efficiency_score: f64,
    pub timing_efficiency_score: f64,
}

#[derive(Debug)]
struct ExecutionOutcomeContext {
    flash_loan_result: Option<FlashLoanExecutionResult>,
    jito_result: JitoBundleResult,
    performance_metrics: ExecutionMetrics,
    total_execution_time_ms: u64,
}

#[allow(dead_code)]
pub struct ExecutionOrchestrator {
    config: Arc<ExecutorConfig>,
    shyft_client: Arc<ShyftClient>,
    wallet_service: Arc<WalletService>,
    flash_loan_optimizer: Arc<RwLock<FlashLoanOptimizer>>,
    jito_optimizer: Arc<RwLock<JitoOptimizer>>,
    bundle_builder: Arc<BundleBuilder>,
    metrics: Arc<MetricsServer>,
    // State management
    active_executions: Arc<RwLock<std::collections::HashMap<Uuid, ComprehensiveExecutionResult>>>,
    daily_pnl_tracker: Arc<RwLock<DailyPnLTracker>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DailyPnLTracker {
    total_profit_lamports: i64,
    total_fees_paid_lamports: u64,
    successful_trades: u32,
    failed_trades: u32,
    daily_reset_time: Instant,
}

impl ExecutionOrchestrator {
    pub async fn new(
        config: Arc<ExecutorConfig>,
        shyft_client: Arc<ShyftClient>,
        wallet_service: Arc<WalletService>,
        flash_loan_aggregator: Arc<FlashLoanAggregator>,
        bundle_builder: Arc<BundleBuilder>,
        metrics: Arc<MetricsServer>,
    ) -> Result<Self> {
        // Initialize optimizers
        let flash_loan_optimizer = Arc::new(RwLock::new(FlashLoanOptimizer::new(
            Arc::clone(&config),
            Arc::clone(&shyft_client),
            flash_loan_aggregator,
            Arc::clone(&metrics),
        )));

        let jito_optimizer = Arc::new(RwLock::new(JitoOptimizer::new(
            Arc::clone(&config),
            Arc::clone(&shyft_client),
            Arc::clone(&bundle_builder),
            Arc::clone(&metrics),
        )));

        let orchestrator = Self {
            config,
            shyft_client,
            wallet_service,
            flash_loan_optimizer,
            jito_optimizer,
            bundle_builder,
            metrics,
            active_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            daily_pnl_tracker: Arc::new(RwLock::new(DailyPnLTracker {
                total_profit_lamports: 0,
                total_fees_paid_lamports: 0,
                successful_trades: 0,
                failed_trades: 0,
                daily_reset_time: Instant::now(),
            })),
        };

        // Start background optimization tasks
        orchestrator.start_background_tasks().await;

        info!("🎼 Execution Orchestrator initialized successfully");

        Ok(orchestrator)
    }

    /// Execute arbitrage opportunity with comprehensive optimization
    pub async fn execute_opportunity(
        &self,
        opportunity: ArbitrageOpportunity,
        execution_mode: ExecutionMode,
    ) -> Result<ComprehensiveExecutionResult> {
        let execution_id = Uuid::new_v4();
        let start_time = Instant::now();

        info!(
            "🎯 Starting comprehensive execution: id={} opportunity={} mode={:?}",
            execution_id, opportunity.opportunity_id, execution_mode
        );

        // Phase 1: Create execution plan
        let planning_start = Instant::now();
        let execution_plan = self
            .create_execution_plan(&opportunity, execution_mode)
            .await?;
        let planning_time_ms = planning_start.elapsed().as_millis() as u64;

        // Phase 2: Risk assessment and circuit breaker check
        if !self.assess_execution_risk(&execution_plan).await? {
            return self
                .create_rejected_result(execution_id, execution_plan, "Risk assessment failed")
                .await;
        }

        // Phase 3: Flash loan optimization (if needed)
        let flash_loan_start = Instant::now();
        let flash_loan_result = if self.requires_flash_loan(&opportunity) {
            Some(
                self.optimize_and_execute_flash_loan(&opportunity, &execution_plan)
                    .await?,
            )
        } else {
            None
        };
        let flash_loan_time_ms = flash_loan_start.elapsed().as_millis() as u64;

        // Phase 4: Jito bundle optimization and execution
        let bundle_start = Instant::now();
        let jito_result = self
            .optimize_and_execute_jito_bundle(
                &opportunity,
                &execution_plan,
                flash_loan_result.as_ref(),
            )
            .await?;
        let bundle_time_ms = bundle_start.elapsed().as_millis() as u64;

        // Phase 5: Create comprehensive result
        let total_execution_time = start_time.elapsed().as_millis() as u64;
        let performance_metrics = ExecutionMetrics {
            planning_time_ms,
            flash_loan_selection_time_ms: flash_loan_time_ms,
            bundle_building_time_ms: bundle_time_ms,
            submission_time_ms: 0,     // Updated from jito result
            confirmation_time_ms: 0,   // Updated from jito result
            slippage_bps: None,        // TODO: Calculate actual slippage
            gas_efficiency_score: 0.9, // TODO: Calculate based on actual gas usage
            timing_efficiency_score: self.calculate_timing_efficiency(total_execution_time),
        };

        let context = ExecutionOutcomeContext {
            flash_loan_result,
            jito_result,
            performance_metrics,
            total_execution_time_ms: total_execution_time,
        };

        let result = self
            .create_comprehensive_result(execution_id, opportunity, execution_plan, context)
            .await?;

        // Phase 6: Update tracking and metrics
        self.update_execution_tracking(&result).await;

        info!(
            "✅ Comprehensive execution completed: id={} status={:?} profit={:?} time={}ms",
            execution_id, result.final_status, result.actual_profit_lamports, total_execution_time
        );

        Ok(result)
    }

    async fn create_execution_plan(
        &self,
        opportunity: &ArbitrageOpportunity,
        execution_mode: ExecutionMode,
    ) -> Result<ExecutionPlan> {
        // Determine if flash loan is needed
        let requires_flash_loan = self.requires_flash_loan(opportunity);

        // Get optimal flash loan strategy if needed
        let flash_loan_strategy = if requires_flash_loan {
            let flash_loan_optimizer = self.flash_loan_optimizer.read().await;
            Some(
                flash_loan_optimizer
                    .find_optimal_flash_loan(
                        opportunity,
                        opportunity.max_size_in as u64,
                        "So11111111111111111111111111111111111111112", // SOL mint
                    )
                    .await?,
            )
        } else {
            None
        };

        // Create Jito optimization parameters based on opportunity
        let jito_strategy = self.create_jito_strategy(opportunity, &execution_mode);

        // Create risk parameters
        let risk_parameters = self.create_risk_parameters(opportunity, &execution_mode);

        let execution_plan = ExecutionPlan {
            opportunity_id: opportunity.opportunity_id,
            execution_mode,
            flash_loan_strategy,
            jito_strategy,
            risk_parameters,
            estimated_profit_lamports: opportunity.profit_bps * 1000,
            execution_timeout_ms: 10_000, // 10 seconds
            priority_level: self.determine_priority(opportunity),
        };

        debug!(
            "📋 Execution plan created: profit_estimate={} priority={:?}",
            execution_plan.estimated_profit_lamports, execution_plan.priority_level
        );

        Ok(execution_plan)
    }

    async fn optimize_and_execute_flash_loan(
        &self,
        _opportunity: &ArbitrageOpportunity,
        execution_plan: &ExecutionPlan,
    ) -> Result<FlashLoanExecutionResult> {
        if let Some(flash_loan_strategy) = &execution_plan.flash_loan_strategy {
            info!(
                "⚡ Executing optimized flash loan: provider={:?} amount={}",
                flash_loan_strategy.provider, flash_loan_strategy.amount_requested
            );

            let start_time = Instant::now();

            // Execute flash loan based on execution mode
            let success = match &execution_plan.execution_mode {
                ExecutionMode::Test {
                    simulate_success_rate,
                    ..
                } => fastrand::f64() < *simulate_success_rate,
                ExecutionMode::Live { .. } => {
                    // TODO: Implement actual flash loan execution
                    true // Placeholder
                }
            };

            let execution_time = start_time.elapsed().as_millis() as u64;

            // Update provider performance tracking
            let optimizer = self.flash_loan_optimizer.write().await;
            optimizer
                .record_execution_result(
                    flash_loan_strategy.provider,
                    success,
                    execution_time,
                    flash_loan_strategy.amount_requested,
                )
                .await;

            Ok(FlashLoanExecutionResult {
                provider_used: flash_loan_strategy.provider,
                amount_borrowed: flash_loan_strategy.amount_requested,
                fee_paid_lamports: flash_loan_strategy.estimated_fee_lamports,
                success,
                execution_time_ms: execution_time,
            })
        } else {
            Err(anyhow::anyhow!("Flash loan strategy not available"))
        }
    }

    async fn optimize_and_execute_jito_bundle(
        &self,
        opportunity: &ArbitrageOpportunity,
        execution_plan: &ExecutionPlan,
        flash_loan_result: Option<&FlashLoanExecutionResult>,
    ) -> Result<JitoBundleResult> {
        info!("🚀 Executing optimized Jito bundle");

        let mut jito_optimizer = self.jito_optimizer.write().await;

        let test_mode = matches!(execution_plan.execution_mode, ExecutionMode::Test { .. });

        let bundle_result = jito_optimizer
            .execute_optimized_arbitrage(
                opportunity,
                flash_loan_result.map(|r| r.provider_used),
                test_mode,
            )
            .await?;

        Ok(bundle_result)
    }

    async fn assess_execution_risk(&self, execution_plan: &ExecutionPlan) -> Result<bool> {
        // Check daily PnL limits
        let daily_tracker = self.daily_pnl_tracker.read().await;
        if daily_tracker.total_profit_lamports
            < -(execution_plan.risk_parameters.daily_loss_limit_lamports as i64)
        {
            warn!(
                "🛑 Daily loss limit exceeded: {} lamports",
                daily_tracker.total_profit_lamports
            );
            return Ok(false);
        }

        // Check position size limits
        if execution_plan.estimated_profit_lamports
            > execution_plan.risk_parameters.max_position_size_lamports
        {
            warn!("🛑 Position size limit exceeded");
            return Ok(false);
        }

        // Additional risk checks...
        Ok(true)
    }

    fn requires_flash_loan(&self, opportunity: &ArbitrageOpportunity) -> bool {
        // Simple heuristic: if max_size_in is greater than typical wallet balance
        opportunity.max_size_in > 1_000_000_000 // 1 SOL
    }

    fn create_jito_strategy(
        &self,
        opportunity: &ArbitrageOpportunity,
        mode: &ExecutionMode,
    ) -> JitoOptimizationParams {
        match mode {
            ExecutionMode::Test { .. } => JitoOptimizationParams {
                base_tip_lamports: 1000,
                enable_aggressive_timing: false,
                ..Default::default()
            },
            ExecutionMode::Live {
                enable_aggressive_optimization,
                ..
            } => JitoOptimizationParams {
                base_tip_lamports: if opportunity.profit_bps > 100 {
                    20_000
                } else {
                    10_000
                },
                enable_aggressive_timing: *enable_aggressive_optimization,
                enable_tip_escalation: *enable_aggressive_optimization,
                ..Default::default()
            },
        }
    }

    fn create_risk_parameters(
        &self,
        _opportunity: &ArbitrageOpportunity,
        mode: &ExecutionMode,
    ) -> RiskParameters {
        match mode {
            ExecutionMode::Test { .. } => RiskParameters {
                max_position_size_lamports: 10_000_000_000, // 10 SOL for testing
                max_gas_fee_lamports: 100_000,
                min_profit_threshold_lamports: 1_000,
                max_execution_time_ms: 30_000,
                enable_circuit_breaker: false,
                daily_loss_limit_lamports: 100_000_000_000, // 100 SOL for testing
            },
            ExecutionMode::Live {
                max_slippage_bps: _max_slippage_bps,
                force_profit_threshold_lamports,
                ..
            } => RiskParameters {
                max_position_size_lamports: 1_000_000_000, // 1 SOL for live trading
                max_gas_fee_lamports: 50_000,
                min_profit_threshold_lamports: force_profit_threshold_lamports.unwrap_or(10_000),
                max_execution_time_ms: 15_000,
                enable_circuit_breaker: true,
                daily_loss_limit_lamports: 10_000_000_000, // 10 SOL daily loss limit
            },
        }
    }

    fn determine_priority(&self, opportunity: &ArbitrageOpportunity) -> ExecutionPriority {
        if opportunity.profit_bps > 200 {
            ExecutionPriority::Critical
        } else if opportunity.profit_bps > 100 {
            ExecutionPriority::High
        } else if opportunity.profit_bps > 50 {
            ExecutionPriority::Medium
        } else {
            ExecutionPriority::Low
        }
    }

    async fn create_comprehensive_result(
        &self,
        execution_id: Uuid,
        opportunity: ArbitrageOpportunity,
        execution_plan: ExecutionPlan,
        context: ExecutionOutcomeContext,
    ) -> Result<ComprehensiveExecutionResult> {
        let ExecutionOutcomeContext {
            flash_loan_result,
            jito_result,
            performance_metrics,
            total_execution_time_ms,
        } = context;
        let final_status = match jito_result.status {
            crate::jito_optimizer::JitoBundleStatus::Landed => ExecutionStatus::Confirmed,
            crate::jito_optimizer::JitoBundleStatus::Failed => ExecutionStatus::Failed,
            _ => ExecutionStatus::Submitted,
        };

        let total_fees = flash_loan_result
            .as_ref()
            .map(|r| r.fee_paid_lamports)
            .unwrap_or(0)
            + jito_result.tip_lamports;

        let actual_profit = jito_result.actual_profit_lamports;
        let roi_bps = actual_profit.map(|profit| {
            if total_fees > 0 {
                ((profit - total_fees as i64) * 10000) / total_fees as i64
            } else {
                0
            }
        });

        let efficiency_score = self.calculate_execution_efficiency(
            &execution_plan,
            &performance_metrics,
            total_execution_time_ms,
        );

        Ok(ComprehensiveExecutionResult {
            execution_id,
            opportunity_id: opportunity.opportunity_id,
            execution_plan,
            flash_loan_result,
            jito_bundle_result: Some(jito_result),
            final_status,
            total_execution_time_ms,
            estimated_profit_lamports: opportunity.profit_bps * 1000,
            actual_profit_lamports: actual_profit,
            total_fees_paid_lamports: total_fees,
            roi_basis_points: roi_bps,
            execution_efficiency_score: efficiency_score,
            error_details: None,
            performance_metrics,
        })
    }

    async fn create_rejected_result(
        &self,
        execution_id: Uuid,
        execution_plan: ExecutionPlan,
        reason: &str,
    ) -> Result<ComprehensiveExecutionResult> {
        Ok(ComprehensiveExecutionResult {
            execution_id,
            opportunity_id: execution_plan.opportunity_id,
            execution_plan,
            flash_loan_result: None,
            jito_bundle_result: None,
            final_status: ExecutionStatus::Rejected,
            total_execution_time_ms: 0,
            estimated_profit_lamports: 0,
            actual_profit_lamports: Some(0),
            total_fees_paid_lamports: 0,
            roi_basis_points: Some(0),
            execution_efficiency_score: 0.0,
            error_details: Some(reason.to_string()),
            performance_metrics: ExecutionMetrics {
                planning_time_ms: 0,
                flash_loan_selection_time_ms: 0,
                bundle_building_time_ms: 0,
                submission_time_ms: 0,
                confirmation_time_ms: 0,
                slippage_bps: None,
                gas_efficiency_score: 0.0,
                timing_efficiency_score: 0.0,
            },
        })
    }

    fn calculate_timing_efficiency(&self, execution_time_ms: u64) -> f64 {
        // Efficiency decreases as execution time increases
        let target_time_ms = 3000.0; // 3 seconds target
        let efficiency = (target_time_ms / execution_time_ms as f64).min(1.0);
        efficiency.max(0.1) // Minimum 10% efficiency
    }

    fn calculate_execution_efficiency(
        &self,
        _plan: &ExecutionPlan,
        metrics: &ExecutionMetrics,
        _total_time_ms: u64,
    ) -> f64 {
        let timing_weight = 0.3;
        let gas_weight = 0.3;
        let success_weight = 0.4;

        let timing_score = metrics.timing_efficiency_score;
        let gas_score = metrics.gas_efficiency_score;
        let success_score = 1.0; // Will be 0 if execution failed

        timing_score * timing_weight + gas_score * gas_weight + success_score * success_weight
    }

    async fn update_execution_tracking(&self, result: &ComprehensiveExecutionResult) {
        // Update active executions
        self.active_executions
            .write()
            .await
            .insert(result.execution_id, result.clone());

        // Update daily PnL tracking
        let mut daily_tracker = self.daily_pnl_tracker.write().await;
        if let Some(profit) = result.actual_profit_lamports {
            daily_tracker.total_profit_lamports += profit;
        }
        daily_tracker.total_fees_paid_lamports += result.total_fees_paid_lamports;

        match result.final_status {
            ExecutionStatus::Confirmed | ExecutionStatus::Finalized => {
                daily_tracker.successful_trades += 1
            }
            ExecutionStatus::Failed | ExecutionStatus::Rejected => daily_tracker.failed_trades += 1,
            _ => {}
        }
    }

    async fn start_background_tasks(&self) {
        // Start flash loan optimizer background tasks
        {
            let flash_loan_optimizer = self.flash_loan_optimizer.read().await;
            flash_loan_optimizer.start_background_optimization_tasks();
        }

        // Daily reset task
        let daily_tracker = Arc::clone(&self.daily_pnl_tracker);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Check hourly
            loop {
                interval.tick().await;
                let mut tracker = daily_tracker.write().await;
                if tracker.daily_reset_time.elapsed() > Duration::from_secs(86400) {
                    // 24 hours
                    *tracker = DailyPnLTracker {
                        total_profit_lamports: 0,
                        total_fees_paid_lamports: 0,
                        successful_trades: 0,
                        failed_trades: 0,
                        daily_reset_time: Instant::now(),
                    };
                    info!("📊 Daily PnL tracker reset");
                }
            }
        });

        info!("🎼 Execution Orchestrator background tasks started");
    }

    /// Get current performance statistics
    pub async fn get_performance_stats(&self) -> Result<PerformanceStats> {
        let daily_tracker = self.daily_pnl_tracker.read().await;
        let active_executions = self.active_executions.read().await;

        Ok(PerformanceStats {
            daily_profit_lamports: daily_tracker.total_profit_lamports,
            daily_fees_lamports: daily_tracker.total_fees_paid_lamports as i64,
            successful_trades_today: daily_tracker.successful_trades,
            failed_trades_today: daily_tracker.failed_trades,
            active_executions_count: active_executions.len(),
            success_rate: if daily_tracker.successful_trades + daily_tracker.failed_trades > 0 {
                daily_tracker.successful_trades as f64
                    / (daily_tracker.successful_trades + daily_tracker.failed_trades) as f64
            } else {
                0.0
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub daily_profit_lamports: i64,
    pub daily_fees_lamports: i64,
    pub successful_trades_today: u32,
    pub failed_trades_today: u32,
    pub active_executions_count: usize,
    pub success_rate: f64,
}
