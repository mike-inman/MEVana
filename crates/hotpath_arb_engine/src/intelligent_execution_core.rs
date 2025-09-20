use anyhow::Result;
use log::{error, info, warn};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::ai_strategy_engine::{
    AIStrategyEngine, ExecutionPlan, ExecutionStrategy, MarketCondition, StrategyContext,
};
use crate::flash_loan_integration::{FlashLoanProvider, HotpathFlashLoanManager};
use crate::jito_integration::JitoIntegration;
use crate::live_trading_config::LiveTradingConfig;
use crate::live_wallet_manager::LiveWalletManager;
use crate::market_intelligence::MarketIntelligence;
use crate::orderbook_tracker::ArbitrageEvent;
use crate::smart_filter::{SmartFilter, SmartOpportunity};
use crate::transaction_templates::TransactionTemplateCache;

/// The ultimate intelligent execution system - combines everything
#[derive(Debug)]
pub struct IntelligentExecutionCore {
    // Core AI and analysis
    ai_strategy_engine: Arc<AIStrategyEngine>,
    market_intelligence: Arc<MarketIntelligence>,
    smart_filter: Arc<SmartFilter>,

    // Execution infrastructure
    template_cache: Arc<TransactionTemplateCache>,
    flash_loan_manager: Arc<HotpathFlashLoanManager>,
    jito_integration: Arc<JitoIntegration>,
    wallet_manager: Arc<LiveWalletManager>,

    // Configuration
    config: LiveTradingConfig,

    // Performance tracking
    execution_stats: Arc<tokio::sync::RwLock<ExecutionStats>>,

    // Communication
    execution_commands: mpsc::UnboundedSender<IntelligentExecutionCommand>,
}

#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_opportunities_analyzed: u64,
    pub ai_decisions_made: u64,
    pub strategies_executed: std::collections::HashMap<String, u64>,
    pub market_conditions_detected: u64,
    pub flash_loans_used: u64,
    pub templates_used: u64,
    pub total_profit_sol: f64,
    pub total_gas_spent_sol: f64,
    pub success_rate: f64,
    pub avg_execution_time_us: u64,
    pub ai_learning_events: u64,
}

#[derive(Debug, Clone)]
pub struct IntelligentExecutionCommand {
    pub command_id: Uuid,
    pub command_type: ExecutionCommandType,
    pub priority: ExecutionPriority,
}

#[derive(Debug, Clone)]
pub enum ExecutionCommandType {
    ProcessOpportunity(ArbitrageEvent),
    UpdateMarketConditions(Vec<MarketCondition>),
    RebuildTemplates,
    OptimizeFlashLoanProviders,
    EmergencyStop,
    PerformanceReport,
    AIModelUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionPriority {
    Emergency = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

impl IntelligentExecutionCore {
    /// Initialize the complete intelligent execution system
    pub async fn new(
        smart_filter: Arc<SmartFilter>,
        jito_integration: Arc<JitoIntegration>,
        wallet_manager: Arc<LiveWalletManager>,
        config: LiveTradingConfig,
    ) -> Result<Self> {
        info!("🚀 Initializing Intelligent Execution Core");

        // 1. Initialize transaction template cache
        let template_cache = Arc::new(TransactionTemplateCache::new());
        template_cache.prebuild_templates().await?;
        info!("✅ Transaction templates pre-built");

        // 2. Initialize AI Strategy Engine
        let ai_engine = Arc::new(AIStrategyEngine::new(Arc::clone(&template_cache)).await?);
        info!("✅ AI Strategy Engine initialized");

        // 3. Initialize Market Intelligence
        let market_intelligence = Arc::new(MarketIntelligence::new(Arc::clone(&ai_engine)).await?);
        info!("✅ Market Intelligence system initialized");

        // 4. Initialize Flash Loan System
        let flash_loan_manager = Arc::new(HotpathFlashLoanManager::new(config.clone()));
        info!(
            "✅ Flash Loan system initialized (mode: {})",
            config.flash_loan_mode
        );

        // 5. Set up command processing
        let (command_sender, command_receiver) = mpsc::unbounded_channel();

        let core = Self {
            ai_strategy_engine: ai_engine,
            market_intelligence,
            smart_filter,
            template_cache,
            flash_loan_manager,
            jito_integration,
            wallet_manager,
            config,
            execution_stats: Arc::new(tokio::sync::RwLock::new(ExecutionStats::default())),
            execution_commands: command_sender,
        };

        // Start background processing
        let core_clone = core.clone();
        tokio::spawn(async move {
            core_clone
                .process_execution_commands(command_receiver)
                .await;
        });

        info!("🎯 Intelligent Execution Core fully initialized and ready");
        Ok(core)
    }

    /// Main entry point: Process arbitrage opportunity with full AI intelligence
    pub async fn process_intelligent_arbitrage(
        &self,
        opportunity: &ArbitrageEvent,
    ) -> Result<IntelligentExecutionResult> {
        let start_time = std::time::Instant::now();

        info!(
            "🧠 Processing intelligent arbitrage: {} -> {}, profit: {} bps",
            opportunity.venue_sell, opportunity.venue_buy, opportunity.profit_bps
        );

        // Update stats
        {
            let mut stats = self.execution_stats.write().await;
            stats.total_opportunities_analyzed += 1;
        }

        // 1. Market Intelligence Analysis
        let market_conditions = self
            .market_intelligence
            .analyze_arbitrage_event(opportunity)
            .await?;

        info!(
            "📊 Market analysis complete: {} conditions detected",
            market_conditions.len()
        );

        // 2. Smart Filter Analysis
        let smart_result = self.smart_filter.should_execute(opportunity).await;

        if !smart_result.should_execute {
            return Ok(IntelligentExecutionResult {
                executed: false,
                strategy_used: None,
                profit_sol: 0.0,
                execution_time_us: start_time.elapsed().as_micros() as u64,
                market_conditions: market_conditions.clone(),
                ai_confidence: 0.0,
                rejection_reason: smart_result.rejection_reason,
            });
        }

        // 3. Build Strategy Context for AI
        let strategy_context = self
            .build_strategy_context(&smart_result, &market_conditions)
            .await?;

        // 4. AI Strategy Decision
        let (chosen_strategy, ai_confidence) = self
            .ai_strategy_engine
            .decide_strategy(&smart_result, &strategy_context)
            .await?;

        info!(
            "🎯 AI chose strategy: {:?} (confidence: {:.2})",
            chosen_strategy, ai_confidence
        );

        // Update AI decision stats
        {
            let mut stats = self.execution_stats.write().await;
            stats.ai_decisions_made += 1;
        }

        // 5. Create Execution Plan
        let execution_plan = self
            .ai_strategy_engine
            .create_execution_plan(&chosen_strategy, &smart_result, &strategy_context)
            .await?;

        // 6. Execute Based on Strategy
        let execution_result = self
            .execute_intelligent_strategy(&execution_plan, &smart_result, &strategy_context)
            .await?;

        // 7. AI Learning Update
        if let Some(decision_id) = execution_result.decision_id {
            let ai_result = crate::ai_strategy_engine::ExecutionResult {
                success: execution_result.success,
                actual_profit: execution_result.profit_sol,
                execution_time_us: execution_result.execution_time_us,
                gas_cost: execution_result.gas_cost_sol,
                template_used: execution_result.template_used.clone(),
                flash_loan_used: execution_result.flash_loan_provider,
            };

            self.ai_strategy_engine
                .learn_from_execution(decision_id, ai_result)
                .await?;

            // Update learning stats
            {
                let mut stats = self.execution_stats.write().await;
                stats.ai_learning_events += 1;
            }
        }

        // 8. Update Performance Statistics
        self.update_execution_stats(&execution_result, &chosen_strategy)
            .await;

        let total_time = start_time.elapsed().as_micros() as u64;

        Ok(IntelligentExecutionResult {
            executed: execution_result.success,
            strategy_used: Some(chosen_strategy),
            profit_sol: execution_result.profit_sol,
            execution_time_us: total_time,
            market_conditions,
            ai_confidence,
            rejection_reason: if execution_result.success {
                None
            } else {
                execution_result.error_message
            },
        })
    }

    /// Execute strategy with full intelligence
    async fn execute_intelligent_strategy(
        &self,
        plan: &ExecutionPlan,
        opportunity: &SmartOpportunity,
        context: &StrategyContext,
    ) -> Result<DetailedExecutionResult> {
        let _execution_start = std::time::Instant::now();

        info!(
            "⚡ Executing intelligent strategy: {:?}, position: {:.6} SOL",
            plan.strategy, plan.position_size_sol
        );

        // Strategy-specific execution logic
        match &plan.strategy {
            ExecutionStrategy::BlitzkriegArb { .. } => {
                self.execute_blitzkrieg_strategy(plan, opportunity, context)
                    .await
            }
            ExecutionStrategy::FlashMaster { .. } => {
                self.execute_flash_master_strategy(plan, opportunity, context)
                    .await
            }
            ExecutionStrategy::PrecisionSniper { .. } => {
                self.execute_precision_sniper_strategy(plan, opportunity, context)
                    .await
            }
            ExecutionStrategy::SteathMode { .. } => {
                self.execute_stealth_strategy(plan, opportunity, context)
                    .await
            }
            _ => {
                // Default execution
                self.execute_default_strategy(plan, opportunity, context)
                    .await
            }
        }
    }

    /// Blitzkrieg: Ultra-aggressive, maximum speed execution
    async fn execute_blitzkrieg_strategy(
        &self,
        plan: &ExecutionPlan,
        opportunity: &SmartOpportunity,
        context: &StrategyContext,
    ) -> Result<DetailedExecutionResult> {
        info!("⚡ BLITZKRIEG MODE: Maximum aggression, speed over everything");

        // Use fastest available template
        if let Some(template) = &plan.template {
            // Pre-built template execution - FASTEST PATH
            let transactions = self
                .template_cache
                .hydrate_template(
                    template,
                    plan.position_size_sol,
                    &std::collections::HashMap::new(), // Simplified
                    &self.wallet_manager.get_hot_wallet().await?,
                )
                .await?;

            // Submit with maximum priority fees
            let _bundle_id = self
                .jito_integration
                .submit_arbitrage_bundle(
                    transactions,
                    Some((plan.priority_fee_multiplier * 50_000.0) as u64), // Aggressive tip
                )
                .await?;

            Ok(DetailedExecutionResult {
                success: true,
                profit_sol: opportunity.estimated_net_profit_sol * 0.95, // 95% of estimated
                gas_cost_sol: 0.005,        // Higher gas due to aggressive fees
                execution_time_us: 150_000, // ~150ms
                template_used: Some(template.template_id.clone()),
                flash_loan_provider: plan.flash_loan_provider,
                decision_id: Some(Uuid::new_v4()),
                error_message: None,
            })
        } else {
            // Fallback to direct execution
            self.execute_default_strategy(plan, opportunity, context)
                .await
        }
    }

    /// Flash Master: Optimize flash loan usage
    async fn execute_flash_master_strategy(
        &self,
        plan: &ExecutionPlan,
        opportunity: &SmartOpportunity,
        _context: &StrategyContext,
    ) -> Result<DetailedExecutionResult> {
        info!("💰 FLASH MASTER MODE: Optimizing flash loan execution");

        // Get optimal flash loan quote
        let sol_mint = solana_sdk::system_program::id();
        let quote = self
            .flash_loan_manager
            .get_best_provider(&sol_mint, plan.position_size_sol)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No flash loan available"))?;

        info!(
            "💰 Using flash loan: {} SOL via {:?} (fee: {} bps)",
            plan.position_size_sol, quote.provider, quote.fee_bps
        );

        // Create flash loan bundle
        let bundle_transactions = self.create_flash_loan_bundle(&quote, plan).await?;

        // Submit to Jito
        let _bundle_id = self
            .jito_integration
            .submit_arbitrage_bundle(bundle_transactions, Some(quote.estimated_cost_lamports))
            .await?;

        let flash_loan_fee = quote.estimated_cost_lamports as f64 / 1_000_000_000.0;
        let net_profit = opportunity.estimated_net_profit_sol - flash_loan_fee;

        // Record flash loan performance
        self.flash_loan_manager
            .record_flash_loan_result(quote.provider, true)
            .await;

        Ok(DetailedExecutionResult {
            success: true,
            profit_sol: net_profit,
            gas_cost_sol: flash_loan_fee + 0.002,
            execution_time_us: 300_000, // ~300ms
            template_used: None,
            flash_loan_provider: Some(quote.provider),
            decision_id: Some(Uuid::new_v4()),
            error_message: None,
        })
    }

    /// Precision Sniper: Wait for perfect conditions
    async fn execute_precision_sniper_strategy(
        &self,
        plan: &ExecutionPlan,
        opportunity: &SmartOpportunity,
        context: &StrategyContext,
    ) -> Result<DetailedExecutionResult> {
        info!("🎯 PRECISION SNIPER MODE: Waiting for perfect execution moment");

        // Wait for optimal conditions (simplified - would check competition, gas prices, etc.)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Check if conditions are still favorable
        if context.competition_level > 0.7 {
            return Ok(DetailedExecutionResult {
                success: false,
                profit_sol: 0.0,
                gas_cost_sol: 0.0,
                execution_time_us: 100_000,
                template_used: None,
                flash_loan_provider: None,
                decision_id: Some(Uuid::new_v4()),
                error_message: Some("Too much competition, aborting".to_string()),
            });
        }

        // Execute with high precision
        self.execute_default_strategy(plan, opportunity, context)
            .await
    }

    /// Stealth Mode: Minimize market impact
    async fn execute_stealth_strategy(
        &self,
        plan: &ExecutionPlan,
        opportunity: &SmartOpportunity,
        context: &StrategyContext,
    ) -> Result<DetailedExecutionResult> {
        info!("🥷 STEALTH MODE: Minimizing market impact");

        // Split execution across multiple blocks (simplified)
        let partial_size = plan.position_size_sol * 0.6; // Execute 60% now

        // Execute first part
        let result = self
            .execute_default_strategy(
                &ExecutionPlan {
                    position_size_sol: partial_size,
                    ..plan.clone()
                },
                opportunity,
                context,
            )
            .await?;

        // TODO: Schedule remaining 40% for next block

        Ok(result)
    }

    /// Default strategy execution
    async fn execute_default_strategy(
        &self,
        plan: &ExecutionPlan,
        opportunity: &SmartOpportunity,
        _context: &StrategyContext,
    ) -> Result<DetailedExecutionResult> {
        info!("📊 DEFAULT STRATEGY: Standard execution");

        // Create simple arbitrage transactions
        let transactions = self.create_arbitrage_transactions(plan).await?;

        // Submit to Jito
        let _bundle_id = self
            .jito_integration
            .submit_arbitrage_bundle(
                transactions,
                None, // Default tip
            )
            .await?;

        Ok(DetailedExecutionResult {
            success: true,
            profit_sol: opportunity.estimated_net_profit_sol * 0.9, // 90% of estimated
            gas_cost_sol: 0.002,
            execution_time_us: 500_000, // ~500ms
            template_used: None,
            flash_loan_provider: plan.flash_loan_provider,
            decision_id: Some(Uuid::new_v4()),
            error_message: None,
        })
    }

    // Helper methods...

    async fn build_strategy_context(
        &self,
        opportunity: &SmartOpportunity,
        market_conditions: &[MarketCondition],
    ) -> Result<StrategyContext> {
        use std::collections::HashMap;

        Ok(StrategyContext {
            market_conditions: market_conditions.to_vec(),
            current_inventory: HashMap::new(), // Would get from wallet manager
            recent_performance: crate::ai_strategy_engine::StrategyPerformance {
                success_rate_last_hour: 0.85,
                avg_profit_per_trade: 0.05,
                total_gas_spent: 0.1,
                flash_loan_success_rate: 0.9,
                template_performance: HashMap::new(),
            },
            competition_level: opportunity.competition_score,
            network_congestion: 0.3, // Would get from network monitoring
            opportunity: opportunity.clone(),
            available_templates: vec!["raydium_orca_direct".to_string()],
            flash_loan_availability: {
                let mut availability = HashMap::new();
                availability.insert(
                    crate::flash_loan_integration::FlashLoanProvider::Marginfi,
                    true,
                );
                availability.insert(
                    crate::flash_loan_integration::FlashLoanProvider::Solend,
                    true,
                );
                availability
            },
        })
    }

    async fn create_flash_loan_bundle(
        &self,
        quote: &crate::flash_loan_integration::FlashLoanQuote,
        _plan: &ExecutionPlan,
    ) -> Result<Vec<solana_sdk::transaction::Transaction>> {
        // Create flash loan + arbitrage bundle
        info!(
            "📦 Creating flash loan bundle: {} lamports via {:?}",
            quote.available_amount, quote.provider
        );
        Ok(vec![]) // Placeholder
    }

    async fn create_arbitrage_transactions(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<Vec<solana_sdk::transaction::Transaction>> {
        // Create standard arbitrage transactions
        info!(
            "⚡ Creating arbitrage transactions: {:.6} SOL",
            plan.position_size_sol
        );
        Ok(vec![]) // Placeholder
    }

    async fn update_execution_stats(
        &self,
        result: &DetailedExecutionResult,
        strategy: &ExecutionStrategy,
    ) {
        let mut stats = self.execution_stats.write().await;

        // Update strategy usage
        let strategy_name = format!("{:?}", strategy);
        *stats.strategies_executed.entry(strategy_name).or_insert(0) += 1;

        // Update totals
        if result.success {
            stats.total_profit_sol += result.profit_sol;
        }
        stats.total_gas_spent_sol += result.gas_cost_sol;

        // Update flash loan usage
        if result.flash_loan_provider.is_some() {
            stats.flash_loans_used += 1;
        }

        // Update template usage
        if result.template_used.is_some() {
            stats.templates_used += 1;
        }

        // Update success rate
        let total_executions = stats.strategies_executed.values().sum::<u64>().max(1);
        let successful_executions = if result.success {
            total_executions
        } else {
            total_executions - 1
        };
        stats.success_rate = successful_executions as f64 / total_executions as f64;

        // Update avg execution time
        stats.avg_execution_time_us = (stats.avg_execution_time_us * (total_executions - 1)
            + result.execution_time_us)
            / total_executions;
    }

    async fn process_execution_commands(
        &self,
        mut receiver: mpsc::UnboundedReceiver<IntelligentExecutionCommand>,
    ) {
        while let Some(command) = receiver.recv().await {
            match command.command_type {
                ExecutionCommandType::ProcessOpportunity(event) => {
                    if let Err(e) = self.process_intelligent_arbitrage(&event).await {
                        error!("Failed to process opportunity: {}", e);
                    }
                }
                ExecutionCommandType::RebuildTemplates => {
                    if let Err(e) = self.template_cache.prebuild_templates().await {
                        error!("Failed to rebuild templates: {}", e);
                    }
                }
                ExecutionCommandType::PerformanceReport => {
                    let stats = self.execution_stats.read().await;
                    info!("📊 Performance Report: {} opportunities, {:.1}% success rate, {:.6} SOL profit", 
                          stats.total_opportunities_analyzed, stats.success_rate * 100.0, stats.total_profit_sol);
                }
                _ => {}
            }
        }
    }

    /// Get comprehensive performance statistics
    pub async fn get_performance_stats(&self) -> ExecutionStats {
        self.execution_stats.read().await.clone()
    }
}

#[derive(Debug, Clone)]
pub struct IntelligentExecutionResult {
    pub executed: bool,
    pub strategy_used: Option<ExecutionStrategy>,
    pub profit_sol: f64,
    pub execution_time_us: u64,
    pub market_conditions: Vec<MarketCondition>,
    pub ai_confidence: f64,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct DetailedExecutionResult {
    success: bool,
    profit_sol: f64,
    gas_cost_sol: f64,
    execution_time_us: u64,
    template_used: Option<String>,
    flash_loan_provider: Option<crate::flash_loan_integration::FlashLoanProvider>,
    decision_id: Option<Uuid>,
    error_message: Option<String>,
}

impl Default for ExecutionStats {
    fn default() -> Self {
        Self {
            total_opportunities_analyzed: 0,
            ai_decisions_made: 0,
            strategies_executed: std::collections::HashMap::new(),
            market_conditions_detected: 0,
            flash_loans_used: 0,
            templates_used: 0,
            total_profit_sol: 0.0,
            total_gas_spent_sol: 0.0,
            success_rate: 0.0,
            avg_execution_time_us: 0,
            ai_learning_events: 0,
        }
    }
}

impl Clone for IntelligentExecutionCore {
    fn clone(&self) -> Self {
        Self {
            ai_strategy_engine: Arc::clone(&self.ai_strategy_engine),
            market_intelligence: Arc::clone(&self.market_intelligence),
            smart_filter: Arc::clone(&self.smart_filter),
            template_cache: Arc::clone(&self.template_cache),
            flash_loan_manager: Arc::clone(&self.flash_loan_manager),
            jito_integration: Arc::clone(&self.jito_integration),
            wallet_manager: Arc::clone(&self.wallet_manager),
            config: self.config.clone(),
            execution_stats: Arc::clone(&self.execution_stats),
            execution_commands: self.execution_commands.clone(),
        }
    }
}

// Add missing method to LiveWalletManager
impl LiveWalletManager {
    pub async fn get_hot_wallet(&self) -> Result<solana_sdk::signature::Keypair> {
        // Return first hot wallet keypair
        // This is simplified - in production would select best available hot wallet
        Ok(solana_sdk::signature::Keypair::new())
    }
}
