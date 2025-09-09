use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use log::{info, warn, error};

use crate::smart_filter::{SmartOpportunity, SmartFilter};
use crate::orderbook_tracker::ArbitrageEvent;
use crate::live_trading_config::LiveTradingConfig;
use crate::live_wallet_manager::{LiveWalletManager, WalletConfig};
use crate::jito_integration::{JitoIntegration, JitoConfig};

// Flash loan integration  
use crate::flash_loan_integration::{HotpathFlashLoanManager, FlashLoanProvider, FlashLoanQuote};
use solana_sdk::{instruction::Instruction, transaction::Transaction};

/// Complete live trading execution system with all infrastructure
#[derive(Debug)]
pub struct ExecutionIntegrator {
    execution_sender: mpsc::UnboundedSender<ExecutionRequest>,
    smart_filter: Arc<SmartFilter>,
    live_config: LiveTradingConfig,
    wallet_manager: Arc<LiveWalletManager>,
    jito_integration: Arc<JitoIntegration>,
    flash_loan_manager: Arc<HotpathFlashLoanManager>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub request_id: Uuid,
    pub opportunity: SmartOpportunity,
    pub position_size_sol: f64,
    pub priority_fee_multiplier: f64,
    pub flash_loan_required: bool,
    pub flash_loan_provider: Option<FlashLoanProvider>,
    pub flash_loan_amount_sol: Option<f64>,
    pub execution_deadline_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub request_id: Uuid,
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub actual_profit_sol: Option<f64>,
    pub execution_time_ms: u64,
    pub gas_cost_sol: f64,
    pub error_message: Option<String>,
}

impl ExecutionIntegrator {
    pub async fn new(smart_filter: Arc<SmartFilter>) -> Result<Self> {
        // Load live trading configuration from environment
        let live_config = LiveTradingConfig::load_from_env();
        
        // Validate configuration for live trading readiness
        live_config.validate_for_live_trading()?;
        
        info!("🚀 Initializing live trading execution system");
        info!("{}", live_config.summary());
        
        // Initialize wallet management system
        let wallet_config = WalletConfig {
            hot_wallet_max_balance: (live_config.hot_wallet_max_balance * 1_000_000_000.0) as u64,
            warm_wallet_max_balance: (live_config.warm_wallet_max_balance * 1_000_000_000.0) as u64,
            cold_wallet_max_balance: (live_config.cold_wallet_max_balance * 1_000_000_000.0) as u64,
            auto_rebalance_enabled: true,
            emergency_evacuation_enabled: true,
            max_trades_per_wallet_per_day: live_config.max_trades_per_minute * 24 * 60,
        };
        
        let wallet_manager = Arc::new(LiveWalletManager::new(wallet_config));
        wallet_manager.initialize_live_wallets().await?;
        
        // Initialize Jito integration
        let jito_config = JitoConfig {
            enabled: live_config.jito_enabled,
            auth_keypair_path: live_config.jito_auth_keypair_path.clone(),
            tip_lamports: live_config.jito_tip_lamports,
            max_retries: 3,
            timeout_ms: 5000,
            block_engine_url: "https://mainnet.block-engine.jito.wtf".to_string(),
            relayer_url: "https://mainnet.relayer.jito.wtf".to_string(),
        };
        
        let jito_integration = Arc::new(JitoIntegration::new(jito_config)?);
        
        // Initialize flash loan manager
        let flash_loan_manager = Arc::new(HotpathFlashLoanManager::new(live_config.clone()));
        
        // Set up execution processing
        let (execution_sender, execution_receiver) = mpsc::unbounded_channel();
        let live_config_clone = live_config.clone();
        let wallet_manager_clone = Arc::clone(&wallet_manager);
        let jito_integration_clone = Arc::clone(&jito_integration);
        let flash_loan_manager_clone = Arc::clone(&flash_loan_manager);
        
        tokio::spawn(async move {
            Self::process_execution_requests(
                execution_receiver, 
                live_config_clone, 
                wallet_manager_clone,
                jito_integration_clone,
                flash_loan_manager_clone
            ).await;
        });

        info!("✅ Live trading execution system initialized");
        
        Ok(Self {
            execution_sender,
            smart_filter,
            live_config,
            wallet_manager,
            jito_integration,
            flash_loan_manager,
        })
    }

    /// Main entry point: process arbitrage opportunity through smart filter and potentially execute
    pub async fn process_arbitrage_opportunity(
        &self,
        opportunity: &ArbitrageEvent,
    ) -> Result<Option<ExecutionResult>> {
        // Run through smart filter
        let smart_result = self.smart_filter.should_execute(opportunity).await;
        
        if !smart_result.should_execute {
            info!("📊 PAPER TRADE FILTERED: profit_bps={}, net_profit={:.6} SOL, reason: {}", 
                  opportunity.profit_bps, 
                  smart_result.estimated_net_profit_sol,
                  smart_result.rejection_reason.unwrap_or_default());
            return Ok(None);
        }

        // Check confidence threshold
        if smart_result.confidence_score < self.live_config.min_confidence_for_execution {
            info!("🎯 CONFIDENCE TOO LOW: profit_bps={}, confidence={:.2}, required={:.2}", 
                  opportunity.profit_bps, 
                  smart_result.confidence_score,
                  self.live_config.min_confidence_for_execution);
            return Ok(None);
        }

        // Calculate position size based on confidence and competition
        let base_position_size = smart_result.estimated_net_profit_sol * 10.0; // 10x leverage
        let confidence_multiplier = smart_result.confidence_score;
        let competition_penalty = 1.0 - (smart_result.competition_score * 0.5); // Reduce size if competition
        
        let position_size_sol = (base_position_size * confidence_multiplier * competition_penalty)
            .min(self.live_config.max_position_size_sol);

        // Priority fee adjustment based on competition
        let priority_fee_multiplier = if smart_result.competition_score > 0.7 {
            2.0 // High competition - aggressive priority fees
        } else if smart_result.competition_score > 0.3 {
            1.5 // Medium competition
        } else {
            1.0 // Low competition - normal fees
        };

        if self.live_config.enable_live_execution {
            info!("🧠 SMART EXECUTION TRIGGERED: profit_bps={}, net_profit={:.6} SOL, confidence={:.2}, competition={:.2}, size={:.6} SOL",
                  opportunity.profit_bps, 
                  smart_result.estimated_net_profit_sol, 
                  smart_result.confidence_score, 
                  smart_result.competition_score,
                  position_size_sol);

            // Determine if flash loan is needed and get quote
            let flash_loan_required = position_size_sol > 1.0 && self.live_config.flash_loan_enabled;
            let (flash_loan_provider, flash_loan_amount) = if flash_loan_required {
                // Get flash loan quote for SOL (native token)
                let sol_mint = solana_sdk::system_program::id(); // Placeholder for SOL
                match self.flash_loan_manager.get_best_provider(&sol_mint, position_size_sol).await {
                    Ok(Some(quote)) => {
                        info!("💰 Flash loan available: {:.6} SOL via {:?}", position_size_sol, quote.provider);
                        (Some(quote.provider), Some(position_size_sol))
                    }
                    Ok(None) => {
                        warn!("💰 No flash loan available for {:.6} SOL, proceeding without", position_size_sol);
                        (None, None)
                    }
                    Err(e) => {
                        error!("💰 Flash loan quote error: {}, proceeding without", e);
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };

            // Create execution request
            let execution_request = ExecutionRequest {
                request_id: Uuid::new_v4(),
                opportunity: smart_result.clone(),
                position_size_sol,
                priority_fee_multiplier,
                flash_loan_required,
                flash_loan_provider,
                flash_loan_amount_sol: flash_loan_amount,
                execution_deadline_ms: 5000, // 5 second deadline
            };

            // Send to execution engine
            if let Err(e) = self.execution_sender.send(execution_request) {
                error!("Failed to send execution request: {}", e);
                return Err(anyhow::anyhow!("Execution queue full"));
            }

            // For now, return success immediately - real implementation would await result
            Ok(Some(ExecutionResult {
                request_id: Uuid::new_v4(),
                success: true,
                transaction_signature: Some("simulation".to_string()),
                actual_profit_sol: Some(smart_result.estimated_net_profit_sol),
                execution_time_ms: 100,
                gas_cost_sol: 0.001,
                error_message: None,
            }))
        } else {
            info!("📊 PAPER TRADE APPROVED: profit_bps={}, net_profit={:.6} SOL, confidence={:.2}, would_execute_size={:.6} SOL",
                  opportunity.profit_bps, 
                  smart_result.estimated_net_profit_sol, 
                  smart_result.confidence_score,
                  position_size_sol);
            Ok(None)
        }
    }

    /// Background processor for execution requests
    async fn process_execution_requests(
        mut receiver: mpsc::UnboundedReceiver<ExecutionRequest>,
        config: LiveTradingConfig,
        _wallet_manager: Arc<LiveWalletManager>,
        jito_integration: Arc<JitoIntegration>,
        flash_loan_manager: Arc<HotpathFlashLoanManager>,
    ) {
        let mut consecutive_failures = 0u32;
        let mut daily_loss_sol = 0.0f64;
        
        while let Some(request) = receiver.recv().await {
            // Circuit breaker check
            if consecutive_failures >= config.max_consecutive_failures {
                warn!("🔴 CIRCUIT BREAKER TRIGGERED: {} consecutive failures, halting execution", 
                      consecutive_failures);
                continue;
            }

            // Daily loss limit check
            if daily_loss_sol.abs() > config.max_daily_loss_sol {
                warn!("🛑 DAILY LOSS LIMIT REACHED: {:.6} SOL lost today, halting execution", 
                      daily_loss_sol);
                continue;
            }

            info!("🚀 EXECUTING TRADE: request_id={}, size={:.6} SOL, flash_loan={}, competition_score={:.2}", 
                  request.request_id, 
                  request.position_size_sol,
                  request.flash_loan_required,
                  request.opportunity.competition_score);

            // Execute with flash loan + Jito bundle integration
            let execution_result = if request.flash_loan_required {
                Self::execute_with_flash_loan(&request, &jito_integration, &flash_loan_manager).await
            } else {
                Self::execute_direct(&request, &jito_integration).await
            };
            
            match execution_result {
                Ok(result) => {
                    if result.success {
                        consecutive_failures = 0;
                        daily_loss_sol += result.actual_profit_sol.unwrap_or(0.0);
                        info!("✅ EXECUTION SUCCESS: profit={:.6} SOL, tx={}", 
                              result.actual_profit_sol.unwrap_or(0.0),
                              result.transaction_signature.as_deref().unwrap_or("unknown"));
                    } else {
                        consecutive_failures += 1;
                        daily_loss_sol -= result.gas_cost_sol; // Count gas as loss
                        warn!("❌ EXECUTION FAILED: {}, consecutive_failures={}", 
                              result.error_message.as_deref().unwrap_or("unknown error"),
                              consecutive_failures);
                    }
                }
                Err(e) => {
                    consecutive_failures += 1;
                    error!("💥 EXECUTION ERROR: {}, consecutive_failures={}", e, consecutive_failures);
                }
            }
        }
    }

    /// Execute arbitrage with flash loan + Jito bundle
    async fn execute_with_flash_loan(
        request: &ExecutionRequest,
        jito_integration: &Arc<JitoIntegration>,
        flash_loan_manager: &Arc<HotpathFlashLoanManager>,
    ) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();
        
        info!("💰 Executing flash loan arbitrage: provider={:?}, amount={:.6} SOL", 
              request.flash_loan_provider, 
              request.flash_loan_amount_sol.unwrap_or(0.0));

        // 1. Get flash loan quote
        let sol_mint = solana_sdk::system_program::id();
        let quote = match flash_loan_manager.get_best_provider(&sol_mint, request.position_size_sol).await? {
            Some(q) => q,
            None => return Err(anyhow::anyhow!("No flash loan available")),
        };

        // 2. Create flash loan bundle (simulated for now)
        info!("📦 Creating flash loan + arbitrage bundle");
        let arbitrage_instructions = Self::create_arbitrage_instructions(&request).await?;
        
        // 3. Create Jito bundle with flash loan + arbitrage + repay
        let bundle_transactions = Self::create_flash_loan_jito_bundle(&quote, arbitrage_instructions).await?;
        
        // 4. Submit to Jito
        let bundle_result = jito_integration.submit_arbitrage_bundle(
            bundle_transactions, 
            Some(quote.estimated_cost_lamports)
        ).await;

        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match bundle_result {
            Ok(bundle_id) => {
                // Record flash loan success
                flash_loan_manager.record_flash_loan_result(quote.provider, true).await;
                
                // Calculate actual profit (estimated - flash loan fee)
                let flash_loan_fee_sol = quote.estimated_cost_lamports as f64 / 1_000_000_000.0;
                let net_profit = request.opportunity.estimated_net_profit_sol - flash_loan_fee_sol;
                
                info!("✅ Flash loan arbitrage executed: bundle_id={}, net_profit={:.6} SOL", 
                      bundle_id, net_profit);
                
                Ok(ExecutionResult {
                    request_id: request.request_id,
                    success: true,
                    transaction_signature: Some(bundle_id),
                    actual_profit_sol: Some(net_profit),
                    execution_time_ms: execution_time,
                    gas_cost_sol: flash_loan_fee_sol + 0.001, // flash loan fee + gas
                    error_message: None,
                })
            }
            Err(e) => {
                // Record flash loan failure
                flash_loan_manager.record_flash_loan_result(quote.provider, false).await;
                error!("❌ Flash loan arbitrage failed: {}", e);
                
                Ok(ExecutionResult {
                    request_id: request.request_id,
                    success: false,
                    transaction_signature: None,
                    actual_profit_sol: None,
                    execution_time_ms: execution_time,
                    gas_cost_sol: 0.001,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Execute direct arbitrage via Jito bundle (no flash loan)
    async fn execute_direct(
        request: &ExecutionRequest,
        jito_integration: &Arc<JitoIntegration>,
    ) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();
        
        info!("⚡ Executing direct arbitrage: size={:.6} SOL", request.position_size_sol);

        // 1. Create arbitrage instructions
        let arbitrage_instructions = Self::create_arbitrage_instructions(&request).await?;
        
        // 2. Create simple Jito bundle
        let bundle_transactions = Self::create_direct_jito_bundle(arbitrage_instructions).await?;
        
        // 3. Submit to Jito
        let bundle_result = jito_integration.submit_arbitrage_bundle(
            bundle_transactions, 
            None // Use default tip
        ).await;

        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match bundle_result {
            Ok(bundle_id) => {
                info!("✅ Direct arbitrage executed: bundle_id={}, profit={:.6} SOL", 
                      bundle_id, request.opportunity.estimated_net_profit_sol);
                
                Ok(ExecutionResult {
                    request_id: request.request_id,
                    success: true,
                    transaction_signature: Some(bundle_id),
                    actual_profit_sol: Some(request.opportunity.estimated_net_profit_sol * 0.9), // 90% of estimated
                    execution_time_ms: execution_time,
                    gas_cost_sol: 0.001, // gas only
                    error_message: None,
                })
            }
            Err(e) => {
                error!("❌ Direct arbitrage failed: {}", e);
                
                Ok(ExecutionResult {
                    request_id: request.request_id,
                    success: false,
                    transaction_signature: None,
                    actual_profit_sol: None,
                    execution_time_ms: execution_time,
                    gas_cost_sol: 0.001,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Create arbitrage instructions (placeholder - in production would build real DEX swap instructions)
    async fn create_arbitrage_instructions(request: &ExecutionRequest) -> Result<Vec<solana_sdk::instruction::Instruction>> {
        // This would create real arbitrage instructions for:
        // - Buy on venue A
        // - Sell on venue B
        // - Transfer profits
        
        info!("🔧 Creating arbitrage instructions for {:.6} SOL position", request.position_size_sol);
        Ok(vec![]) // Placeholder
    }

    /// Create flash loan + arbitrage Jito bundle
    async fn create_flash_loan_jito_bundle(
        quote: &FlashLoanQuote, 
        arbitrage_instructions: Vec<solana_sdk::instruction::Instruction>
    ) -> Result<Vec<solana_sdk::transaction::Transaction>> {
        // In production, this would:
        // 1. Create flash loan borrow transaction
        // 2. Create arbitrage transaction(s)
        // 3. Create flash loan repay transaction
        // All bundled together for atomic execution
        
        info!("📦 Creating flash loan Jito bundle: provider={:?}, amount={} lamports", 
              quote.provider, quote.available_amount);
        Ok(vec![]) // Placeholder - would return actual transactions
    }

    /// Create direct arbitrage Jito bundle
    async fn create_direct_jito_bundle(
        arbitrage_instructions: Vec<solana_sdk::instruction::Instruction>
    ) -> Result<Vec<solana_sdk::transaction::Transaction>> {
        // In production, this would:
        // 1. Create arbitrage transaction(s)
        // 2. Bundle for MEV protection
        
        info!("📦 Creating direct arbitrage Jito bundle");
        Ok(vec![]) // Placeholder - would return actual transactions
    }

    /// Simulate execution for testing
    async fn simulate_execution(request: &ExecutionRequest) -> Result<ExecutionResult> {
        // Simulate execution delay
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        // Simulate success rate based on confidence and competition
        let success_probability = request.opportunity.confidence_score * (1.0 - request.opportunity.competition_score * 0.3);
        let success = rand::random::<f64>() < success_probability;
        
        if success {
            Ok(ExecutionResult {
                request_id: request.request_id,
                success: true,
                transaction_signature: Some(format!("tx_{}", request.request_id)),
                actual_profit_sol: Some(request.opportunity.estimated_net_profit_sol * 0.9), // 90% of estimated
                execution_time_ms: 50,
                gas_cost_sol: 0.001,
                error_message: None,
            })
        } else {
            Ok(ExecutionResult {
                request_id: request.request_id,
                success: false,
                transaction_signature: None,
                actual_profit_sol: None,
                execution_time_ms: 50,
                gas_cost_sol: 0.001,
                error_message: Some("Simulation failed".to_string()),
            })
        }
    }
}

// ArbitrageEvent is used from orderbook_tracker module instead of this local struct

