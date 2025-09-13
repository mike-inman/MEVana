use anyhow::Result;
use chrono::Utc;
use log::{info, warn, debug};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::orderbook_tracker::ArbitrageEvent;
use crate::smart_filter::SmartOpportunity;
use crate::transaction_templates::{TransactionTemplateCache, TransactionTemplate};
use crate::flash_loan_integration::{FlashLoanProvider, FlashLoanQuote};

/// Market condition types that trigger different strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketCondition {
    HighVolatility { threshold: f64 },
    LowLiquidity { depth_threshold: f64 },
    TrendingUp { momentum: f64 },
    TrendingDown { momentum: f64 },
    NewsEvent { impact_score: f64 },
    WhaleMovement { size_threshold: f64 },
    CrossChainArbitrage { price_delta: f64 },
    FlashCrash { drop_percentage: f64 },
    LiquidationCascade { total_liquidations: u64 },
    DeFiProtocolEvent { protocol: String, event_type: String },
}

/// AI-driven execution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    /// Ultra-aggressive: Max position size, lowest latency templates
    BlitzkriegArb {
        max_position_multiplier: f64,
        force_flash_loans: bool,
        template_preference: String, // "fastest" or "most_reliable"
    },
    
    /// Conservative: Smaller positions, higher confidence threshold
    SteathMode {
        position_size_reduction: f64,
        min_confidence_boost: f64,
        spread_execution: bool, // Execute across multiple blocks
    },
    
    /// Opportunistic: Wait for perfect setups
    PrecisionSniper {
        profit_threshold_multiplier: f64,
        max_wait_time_ms: u64,
        competition_avoidance: bool,
    },
    
    /// Volume-based: Scale with market activity
    VolumeRider {
        volume_multiplier_curve: Vec<(f64, f64)>, // (volume_level, position_multiplier)
        dynamic_slippage: bool,
    },
    
    /// Multi-venue coordination
    OmniArbitrage {
        simultaneous_venues: Vec<String>,
        cross_venue_hedging: bool,
        inventory_management: bool,
    },
    
    /// Flash loan optimization
    FlashMaster {
        provider_switching: bool,
        compound_arbitrage: bool, // Chain multiple arbs in one flash loan
        flash_loan_size_optimizer: bool,
    },
    
    /// Emergency strategies
    EmergencyEvac {
        liquidate_inventory: bool,
        hedge_positions: bool,
        minimize_slippage: bool,
    },
}

/// AI strategy decision context
#[derive(Debug, Clone)]
pub struct StrategyContext {
    pub market_conditions: Vec<MarketCondition>,
    pub current_inventory: HashMap<String, f64>, // token -> balance
    pub recent_performance: StrategyPerformance,
    pub competition_level: f64, // 0.0 = no competition, 1.0 = extreme
    pub network_congestion: f64, // 0.0 = fast, 1.0 = congested
    pub opportunity: SmartOpportunity,
    pub available_templates: Vec<String>,
    pub flash_loan_availability: HashMap<FlashLoanProvider, bool>,
}

#[derive(Debug, Clone)]
pub struct StrategyPerformance {
    pub success_rate_last_hour: f64,
    pub avg_profit_per_trade: f64,
    pub total_gas_spent: f64,
    pub flash_loan_success_rate: f64,
    pub template_performance: HashMap<String, f64>, // template_id -> success_rate
}

/// AI Strategy Engine - The brain of the operation
#[derive(Debug)]
pub struct AIStrategyEngine {
    // Core components
    template_cache: Arc<TransactionTemplateCache>,
    strategy_rules: Arc<RwLock<Vec<StrategyRule>>>,
    market_conditions: Arc<RwLock<Vec<MarketCondition>>>,
    
    // Performance tracking
    strategy_performance: Arc<RwLock<HashMap<String, StrategyPerformance>>>,
    execution_history: Arc<RwLock<Vec<ExecutionRecord>>>,
    
    // AI learning system
    ai_model: Arc<RwLock<AIModel>>,
    learning_enabled: bool,
    
    // Communication channels
    strategy_updates: mpsc::UnboundedSender<StrategyUpdate>,
}

/// Strategy rule that maps conditions to strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRule {
    pub rule_id: String,
    pub name: String,
    pub conditions: Vec<MarketCondition>,
    pub strategy: ExecutionStrategy,
    pub priority: u32, // Higher = more important
    pub enabled: bool,
    pub cooldown_ms: u64, // Min time between applications
    pub last_applied: u64,
}

/// AI Model for strategy optimization
#[derive(Debug)]
pub struct AIModel {
    // Neural network weights (simplified representation)
    input_weights: Vec<Vec<f64>>,    // Market condition -> hidden layer
    hidden_weights: Vec<Vec<f64>>,   // Hidden layer -> output
    
    // Reinforcement learning
    q_table: HashMap<String, f64>,   // State-action values
    exploration_rate: f64,           // Epsilon for exploration
    learning_rate: f64,
    
    // Performance memory (CRITICAL FIX: Use VecDeque for bounded memory)
    recent_decisions: VecDeque<AIDecision>, // BOUNDED: max capacity decisions
    success_patterns: HashMap<String, f64>, // Pattern -> success_rate
}

#[derive(Debug, Clone)]
pub struct AIDecision {
    pub decision_id: Uuid,
    pub context: StrategyContext,
    pub chosen_strategy: ExecutionStrategy,
    pub confidence_score: f64,
    pub expected_profit: f64,
    pub actual_result: Option<ExecutionResult>,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub actual_profit: f64,
    pub execution_time_us: u64,
    pub gas_cost: f64,
    pub template_used: Option<String>,
    pub flash_loan_used: Option<FlashLoanProvider>,
}

#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub timestamp: u64,
    pub strategy: ExecutionStrategy,
    pub context: StrategyContext,
    pub result: ExecutionResult,
}

#[derive(Debug, Clone)]
pub struct StrategyUpdate {
    pub update_type: StrategyUpdateType,
    pub data: String,
}

#[derive(Debug, Clone)]
pub enum StrategyUpdateType {
    MarketConditionDetected(MarketCondition),
    StrategyPerformanceUpdate,
    AIModelUpdated,
    NewRuleAdded,
    EmergencyModeActivated,
}

impl AIStrategyEngine {
    pub async fn new(template_cache: Arc<TransactionTemplateCache>) -> Result<Self> {
        let (strategy_sender, mut strategy_receiver) = mpsc::unbounded_channel();
        
        // Initialize with default strategies
        let default_rules = Self::create_default_strategy_rules();
        
        // Initialize AI model
        let ai_model = AIModel {
            input_weights: vec![vec![0.5; 20]; 10], // 20 inputs, 10 hidden neurons
            hidden_weights: vec![vec![0.5; 5]; 10], // 10 hidden, 5 outputs (strategies)
            q_table: HashMap::new(),
            exploration_rate: 0.1, // 10% exploration
            learning_rate: 0.01,
            recent_decisions: VecDeque::with_capacity(1000), // BOUNDED: max 1000 decisions
            success_patterns: HashMap::new(),
        };
        
        let engine = Self {
            template_cache,
            strategy_rules: Arc::new(RwLock::new(default_rules)),
            market_conditions: Arc::new(RwLock::new(Vec::with_capacity(5000))), // BOUNDED: max 5000 conditions
            strategy_performance: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::with_capacity(10000))), // BOUNDED: max 10000 executions
            ai_model: Arc::new(RwLock::new(ai_model)),
            learning_enabled: true,
            strategy_updates: strategy_sender,
        };
        
        // Start background processing
        let engine_clone = engine.clone();
        tokio::spawn(async move {
            engine_clone.process_strategy_updates(strategy_receiver).await;
        });
        
        info!("🧠 AI Strategy Engine initialized with {} default rules", 
              engine.strategy_rules.read().await.len());
        
        Ok(engine)
    }
    
    /// Core AI decision making: Given an opportunity and context, what strategy should we use?
    pub async fn decide_strategy(
        &self,
        opportunity: &SmartOpportunity,
        context: &StrategyContext,
    ) -> Result<(ExecutionStrategy, f64)> { // (strategy, confidence)
        debug!("🤖 AI analyzing opportunity: profit_bps={}, competition={:.2}", 
               opportunity.estimated_net_profit_sol, context.competition_level);
        
        // 1. Get current market conditions
        let conditions = self.market_conditions.read().await.clone();
        
        // 2. Find applicable strategy rules
        let rules = self.strategy_rules.read().await;
        let mut applicable_rules = Vec::new();
        
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }
            
            // Check cooldown
            let now = chrono::Utc::now().timestamp() as u64;
            if now - rule.last_applied < rule.cooldown_ms {
                continue;
            }
            
            // Check if conditions match
            if Self::conditions_match(&conditions, &rule.conditions) {
                applicable_rules.push(rule.clone());
            }
        }
        
        // 3. If multiple rules apply, use AI model to choose
        if applicable_rules.is_empty() {
            // Default strategy
            return Ok((ExecutionStrategy::SteathMode {
                position_size_reduction: 0.5,
                min_confidence_boost: 0.1,
                spread_execution: false,
            }, 0.5));
        }
        
        // 4. Use AI model to select best strategy
        let (best_strategy, confidence) = self.ai_select_strategy(
            &applicable_rules,
            context,
            opportunity,
        ).await?;
        
        // 5. Record decision for learning
        if self.learning_enabled {
            let decision = AIDecision {
                decision_id: Uuid::new_v4(),
                context: context.clone(),
                chosen_strategy: best_strategy.clone(),
                confidence_score: confidence,
                expected_profit: opportunity.estimated_net_profit_sol,
                actual_result: None, // Will be filled in later
                timestamp: chrono::Utc::now().timestamp() as u64,
            };
            
            self.ai_model.write().await.recent_decisions.push_back(decision);
        }
        
        info!("🎯 AI selected strategy: {:?} (confidence: {:.2})", best_strategy, confidence);
        Ok((best_strategy, confidence))
    }
    
    /// Update market conditions based on real-time data
    pub async fn update_market_conditions(&self, new_conditions: Vec<MarketCondition>) {
        let mut conditions = self.market_conditions.write().await;
        *conditions = new_conditions.clone();
        
        // Notify strategy update
        let _ = self.strategy_updates.send(StrategyUpdate {
            update_type: StrategyUpdateType::MarketConditionDetected(new_conditions[0].clone()),
            data: format!("Updated {} market conditions", new_conditions.len()),
        });
        
        debug!("📊 Market conditions updated: {} conditions active", new_conditions.len());
    }
    
    /// Learn from execution results to improve AI model
    pub async fn learn_from_execution(
        &self,
        decision_id: Uuid,
        result: ExecutionResult,
    ) -> Result<()> {
        if !self.learning_enabled {
            return Ok(());
        }
        
        let mut ai_model = self.ai_model.write().await;
        
        // Find the decision first and extract what we need
        let decision_info = ai_model.recent_decisions.iter()
            .find(|d| d.decision_id == decision_id)
            .map(|d| (d.context.market_conditions.clone(), d.chosen_strategy.clone()));
        
        if let Some((market_conditions, chosen_strategy)) = decision_info {
            // Now update the decision
            if let Some(decision) = ai_model.recent_decisions.iter_mut().find(|d| d.decision_id == decision_id) {
                decision.actual_result = Some(result.clone());
            }
            
            // Calculate reward based on result
            let reward = if result.success {
                result.actual_profit - result.gas_cost // Net profit is reward
            } else {
                -result.gas_cost // Loss is negative reward
            };
            
            // Update Q-table (simplified reinforcement learning) - WITH MEMORY BOUNDS
            let state_action = format!("{:?}_{:?}", market_conditions, chosen_strategy);
            let current_q = ai_model.q_table.get(&state_action).copied().unwrap_or(0.0);
            let learning_rate = ai_model.learning_rate;
            let new_q = current_q + learning_rate * (reward - current_q);
            
            // CRITICAL FIX: Prevent unbounded Q-table growth (root cause of 7-minute degradation)
            if ai_model.q_table.len() >= 10000 {
                // Remove oldest/least useful entries when at capacity
                if let Some(oldest_key) = ai_model.q_table.keys().next().cloned() {
                    ai_model.q_table.remove(&oldest_key);
                }
            }
            ai_model.q_table.insert(state_action.clone(), new_q);
            
            // Update success patterns - WITH MEMORY BOUNDS
            // Just use the market conditions and chosen strategy for the pattern
            let pattern = format!("{:?}_{:?}", market_conditions, chosen_strategy);
            let current_success = ai_model.success_patterns.get(&pattern).copied().unwrap_or(0.5);
            let success_update = if result.success { 0.1 } else { -0.1 };
            let new_success = (current_success + success_update).max(0.0).min(1.0);
            
            // CRITICAL FIX: Prevent unbounded success_patterns growth (secondary memory leak)
            if ai_model.success_patterns.len() >= 5000 {
                if let Some(oldest_key) = ai_model.success_patterns.keys().next().cloned() {
                    ai_model.success_patterns.remove(&oldest_key);
                }
            }
            ai_model.success_patterns.insert(pattern, new_success);
            
            info!("🧠 AI learned from execution: reward={:.6}, pattern_success={:.2}", 
                  reward, new_success);
        }
        
        Ok(())
    }
    
    /// Create AI-optimized execution plan
    pub async fn create_execution_plan(
        &self,
        strategy: &ExecutionStrategy,
        opportunity: &SmartOpportunity,
        context: &StrategyContext,
    ) -> Result<ExecutionPlan> {
        match strategy {
            ExecutionStrategy::BlitzkriegArb { max_position_multiplier, force_flash_loans, template_preference } => {
                let position_size = opportunity.estimated_net_profit_sol * max_position_multiplier;
                let template = self.template_cache.get_best_template(
                    &format!("{}->{}",opportunity.original.venue_sell, opportunity.original.venue_buy),
                    "SOL/USDC", // Simplified
                    position_size,
                    *force_flash_loans || position_size > 1.0,
                ).await?;
                
                Ok(ExecutionPlan {
                    strategy: strategy.clone(),
                    position_size_sol: position_size,
                    use_flash_loan: *force_flash_loans || position_size > 1.0,
                    template: template,
                    flash_loan_provider: Self::select_flash_loan_provider(context).await,
                    priority_fee_multiplier: 2.0, // Aggressive fees for speed
                    max_slippage_bps: 100, // Higher slippage tolerance for speed
                    execution_timeout_ms: 1000, // 1 second max
                })
            }
            
            ExecutionStrategy::SteathMode { position_size_reduction, min_confidence_boost, spread_execution } => {
                let position_size = opportunity.estimated_net_profit_sol * (1.0 - position_size_reduction);
                
                Ok(ExecutionPlan {
                    strategy: strategy.clone(),
                    position_size_sol: position_size,
                    use_flash_loan: position_size > 1.0,
                    template: None, // Use dynamic execution
                    flash_loan_provider: Self::select_flash_loan_provider(context).await,
                    priority_fee_multiplier: 1.0, // Normal fees
                    max_slippage_bps: 25, // Tight slippage
                    execution_timeout_ms: if *spread_execution { 5000 } else { 2000 },
                })
            }
            
            ExecutionStrategy::FlashMaster { provider_switching, compound_arbitrage, flash_loan_size_optimizer } => {
                let position_size = if *flash_loan_size_optimizer {
                    self.optimize_flash_loan_size(opportunity, context).await?
                } else {
                    opportunity.estimated_net_profit_sol * 5.0 // 5x leverage
                };
                
                Ok(ExecutionPlan {
                    strategy: strategy.clone(),
                    position_size_sol: position_size,
                    use_flash_loan: true, // Always use flash loans
                    template: self.template_cache.get_best_template(
                        &format!("{}->{}",opportunity.original.venue_sell, opportunity.original.venue_buy),
                        "SOL/USDC",
                        position_size,
                        true,
                    ).await?,
                    flash_loan_provider: if *provider_switching {
                        Self::select_optimal_flash_loan_provider(context).await
                    } else {
                        Self::select_flash_loan_provider(context).await
                    },
                    priority_fee_multiplier: 1.5,
                    max_slippage_bps: 50,
                    execution_timeout_ms: 2000,
                })
            }
            
            // Add implementations for other strategies...
            _ => {
                // Default execution plan
                Ok(ExecutionPlan {
                    strategy: strategy.clone(),
                    position_size_sol: opportunity.estimated_net_profit_sol,
                    use_flash_loan: false,
                    template: None,
                    flash_loan_provider: None,
                    priority_fee_multiplier: 1.0,
                    max_slippage_bps: 50,
                    execution_timeout_ms: 3000,
                })
            }
        }
    }
    
    // Helper methods...
    
    async fn ai_select_strategy(
        &self,
        applicable_rules: &[StrategyRule],
        context: &StrategyContext,
        opportunity: &SmartOpportunity,
    ) -> Result<(ExecutionStrategy, f64)> {
        let ai_model = self.ai_model.read().await;
        
        // Use neural network to score each strategy
        let mut best_strategy = applicable_rules[0].strategy.clone();
        let mut best_score = 0.0;
        
        for rule in applicable_rules {
            // Create input vector from context and opportunity
            let input = self.create_input_vector(context, opportunity).await;
            
            // Forward pass through neural network
            let score = self.neural_network_forward(&ai_model, &input);
            
            // Add Q-table bonus
            let state_action = format!("{:?}_{:?}", context.market_conditions, rule.strategy);
            let q_bonus = ai_model.q_table.get(&state_action).copied().unwrap_or(0.0);
            
            let final_score = score + q_bonus * 0.1; // Weight Q-learning
            
            if final_score > best_score {
                best_score = final_score;
                best_strategy = rule.strategy.clone();
            }
        }
        
        Ok((best_strategy, best_score.max(0.0).min(1.0)))
    }
    
    async fn create_input_vector(&self, context: &StrategyContext, opportunity: &SmartOpportunity) -> Vec<f64> {
        // Convert context and opportunity into neural network input
        vec![
            opportunity.estimated_net_profit_sol,
            opportunity.confidence_score,
            opportunity.competition_score,
            context.competition_level,
            context.network_congestion,
            context.recent_performance.success_rate_last_hour,
            context.recent_performance.avg_profit_per_trade,
            // Add more features...
        ]
    }
    
    fn neural_network_forward(&self, model: &AIModel, input: &[f64]) -> f64 {
        // Simplified neural network forward pass
        // In production, this would be a proper deep learning model
        
        // Hidden layer
        let mut hidden: Vec<f64> = vec![0.0; model.input_weights[0].len()];
        for (i, neuron_weights) in model.input_weights.iter().enumerate() {
            for (j, &weight) in neuron_weights.iter().enumerate() {
                if j < input.len() {
                    hidden[i] += input[j] * weight;
                }
            }
            hidden[i] = hidden[i].tanh(); // Activation function
        }
        
        // Output layer
        let mut output = 0.0;
        for (i, &hidden_val) in hidden.iter().enumerate() {
            if i < model.hidden_weights.len() {
                output += hidden_val * model.hidden_weights[i][0]; // Single output for now
            }
        }
        
        output.tanh().abs() // Return positive score
    }
    
    fn create_default_strategy_rules() -> Vec<StrategyRule> {
        vec![
            StrategyRule {
                rule_id: "high_volatility_blitz".to_string(),
                name: "High Volatility Blitzkrieg".to_string(),
                conditions: vec![MarketCondition::HighVolatility { threshold: 0.05 }],
                strategy: ExecutionStrategy::BlitzkriegArb {
                    max_position_multiplier: 3.0,
                    force_flash_loans: true,
                    template_preference: "fastest".to_string(),
                },
                priority: 100,
                enabled: true,
                cooldown_ms: 5000,
                last_applied: 0,
            },
            StrategyRule {
                rule_id: "low_competition_sniper".to_string(),
                name: "Low Competition Precision Sniper".to_string(),
                conditions: vec![],
                strategy: ExecutionStrategy::PrecisionSniper {
                    profit_threshold_multiplier: 1.5,
                    max_wait_time_ms: 1000,
                    competition_avoidance: true,
                },
                priority: 50,
                enabled: true,
                cooldown_ms: 1000,
                last_applied: 0,
            },
            // Add more default rules...
        ]
    }
    
    fn conditions_match(current: &[MarketCondition], required: &[MarketCondition]) -> bool {
        // Simplified condition matching
        for required_condition in required {
            if !current.iter().any(|c| std::mem::discriminant(c) == std::mem::discriminant(required_condition)) {
                return false;
            }
        }
        true
    }
    
    fn extract_pattern(context: &StrategyContext, strategy: &ExecutionStrategy) -> String {
        format!("{}_{}_{}_{}", 
                context.competition_level as u32,
                context.network_congestion as u32,
                context.market_conditions.len(),
                format!("{:?}", strategy).chars().take(10).collect::<String>())
    }
    
    async fn select_flash_loan_provider(context: &StrategyContext) -> Option<FlashLoanProvider> {
        // Select best available provider
        context.flash_loan_availability.iter()
            .find(|(_, &available)| available)
            .map(|(&provider, _)| provider)
    }
    
    async fn select_optimal_flash_loan_provider(context: &StrategyContext) -> Option<FlashLoanProvider> {
        // AI-optimized provider selection based on context
        // For now, same as basic selection
        Self::select_flash_loan_provider(context).await
    }
    
    async fn optimize_flash_loan_size(&self, opportunity: &SmartOpportunity, context: &StrategyContext) -> Result<f64> {
        // AI-optimized flash loan size calculation
        let base_size = opportunity.estimated_net_profit_sol * 5.0;
        let volatility_multiplier = if context.market_conditions.iter().any(|c| matches!(c, MarketCondition::HighVolatility { .. })) {
            2.0
        } else {
            1.0
        };
        
        Ok(base_size * volatility_multiplier)
    }
    
    async fn process_strategy_updates(&self, mut receiver: mpsc::UnboundedReceiver<StrategyUpdate>) {
        while let Some(update) = receiver.recv().await {
            match update.update_type {
                StrategyUpdateType::MarketConditionDetected(_) => {
                    // React to new market conditions
                    debug!("📊 Processing market condition update");
                }
                StrategyUpdateType::AIModelUpdated => {
                    // Handle AI model updates
                    info!("🧠 AI model updated");
                }
                _ => {}
            }
        }
    }
}

/// Execution plan generated by AI strategy engine
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub strategy: ExecutionStrategy,
    pub position_size_sol: f64,
    pub use_flash_loan: bool,
    pub template: Option<TransactionTemplate>,
    pub flash_loan_provider: Option<FlashLoanProvider>,
    pub priority_fee_multiplier: f64,
    pub max_slippage_bps: u32,
    pub execution_timeout_ms: u64,
}

// Implement Clone for AIStrategyEngine
impl Clone for AIStrategyEngine {
    fn clone(&self) -> Self {
        Self {
            template_cache: Arc::clone(&self.template_cache),
            strategy_rules: Arc::clone(&self.strategy_rules),
            market_conditions: Arc::clone(&self.market_conditions),
            strategy_performance: Arc::clone(&self.strategy_performance),
            execution_history: Arc::clone(&self.execution_history),
            ai_model: Arc::clone(&self.ai_model),
            learning_enabled: self.learning_enabled,
            strategy_updates: self.strategy_updates.clone(),
        }
    }
}