use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::hotpath::SignalTx;
use crate::models::{ArbSignal, Pair};
use crate::stats::record_profit;

// Re-export from executor-service
pub use crate::executor_integration::{
    JupiterIntegration, MultiHopOpportunity, HopStep, COMMON_TOKENS
};

#[derive(Debug, Clone)]
pub struct JupiterArbConfig {
    pub min_profit_bps: i32,
    pub max_price_impact: f64,
    pub max_slippage_bps: u16,
    pub scan_interval_ms: u64,
    pub base_amounts: Vec<u64>, // Different amounts to test (1 SOL, 5 SOL, 10 SOL, etc.)
    pub max_hops: u8,
    pub enable_paper_trading: bool,
}

impl Default for JupiterArbConfig {
    fn default() -> Self {
        Self {
            min_profit_bps: 50, // Higher threshold for multi-hop due to complexity
            max_price_impact: 3.0, // 3% max price impact
            max_slippage_bps: 200, // 2% max slippage
            scan_interval_ms: 2000, // Scan every 2 seconds
            base_amounts: vec![
                1_000_000_000,   // 1 SOL
                5_000_000_000,   // 5 SOL  
                10_000_000_000,  // 10 SOL
                25_000_000_000,  // 25 SOL
            ],
            max_hops: 3,
            enable_paper_trading: true, // Start in paper trading mode
        }
    }
}

#[derive(Debug)]
struct PaperTradingResults {
    total_opportunities: u64,
    profitable_opportunities: u64,
    total_paper_profit: i64,
    best_opportunity: Option<MultiHopOpportunity>,
    worst_opportunity: Option<MultiHopOpportunity>,
    average_profit_bps: f64,
    venue_performance: std::collections::HashMap<String, (u64, i64)>, // (count, total_profit)
}

pub struct JupiterArbitrageScanner {
    jupiter: Arc<RwLock<JupiterIntegration>>,
    config: JupiterArbConfig,
    signal_tx: SignalTx,
    paper_results: Arc<RwLock<PaperTradingResults>>,
    tokens: Vec<String>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl JupiterArbitrageScanner {
    pub fn new(signal_tx: SignalTx, config: Option<JupiterArbConfig>) -> Self {
        let config = config.unwrap_or_default();
        
        let jupiter = JupiterIntegration::with_config(
            config.max_slippage_bps,
            config.max_price_impact,
            config.min_profit_bps,
        );

        // Use common high-volume tokens for better liquidity
        let tokens = COMMON_TOKENS.iter()
            .map(|s| s.to_string())
            .collect();

        Self {
            jupiter: Arc::new(RwLock::new(jupiter)),
            config,
            signal_tx,
            paper_results: Arc::new(RwLock::new(PaperTradingResults {
                total_opportunities: 0,
                profitable_opportunities: 0,
                total_paper_profit: 0,
                best_opportunity: None,
                worst_opportunity: None,
                average_profit_bps: 0.0,
                venue_performance: std::collections::HashMap::new(),
            })),
            tokens,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the Jupiter arbitrage scanner
    pub async fn start(&self) -> Result<()> {
        if self.running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            warn!("Jupiter arbitrage scanner is already running");
            return Ok(());
        }

        info!(
            "🚀 Starting Jupiter arbitrage scanner with {} tokens, paper_trading={}",
            self.tokens.len(),
            self.config.enable_paper_trading
        );

        let mut scan_interval = interval(Duration::from_millis(self.config.scan_interval_ms));
        let jupiter = Arc::clone(&self.jupiter);
        let signal_tx = self.signal_tx.clone();
        let paper_results = Arc::clone(&self.paper_results);
        let config = self.config.clone();
        let tokens = self.tokens.clone();
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                scan_interval.tick().await;

                if let Err(e) = Self::scan_opportunities(
                    Arc::clone(&jupiter),
                    &signal_tx,
                    Arc::clone(&paper_results),
                    &config,
                    &tokens,
                ).await {
                    error!("Error scanning Jupiter opportunities: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Stop the Jupiter arbitrage scanner
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        info!("🛑 Stopped Jupiter arbitrage scanner");
    }

    /// Main scanning logic
    async fn scan_opportunities(
        jupiter: Arc<RwLock<JupiterIntegration>>,
        signal_tx: &SignalTx,
        paper_results: Arc<RwLock<PaperTradingResults>>,
        config: &JupiterArbConfig,
        tokens: &[String],
    ) -> Result<()> {
        let start_time = Instant::now();

        // Test different amounts to find optimal size opportunities
        for &amount in &config.base_amounts {
            let mut jupiter_guard = jupiter.write().await;
            
            match jupiter_guard.find_multi_hop_opportunities(tokens, amount).await {
                Ok(opportunities) => {
                    drop(jupiter_guard); // Release lock early
                    
                    for opportunity in opportunities {
                        if opportunity.price_impact_pct <= config.max_price_impact 
                            && opportunity.route_complexity <= config.max_hops {
                            
                            if config.enable_paper_trading {
                                Self::process_paper_opportunity(
                                    Arc::clone(&paper_results),
                                    opportunity.clone(),
                                ).await;
                            } else {
                                // Send real trading signal
                                Self::send_trading_signal(signal_tx, opportunity).await?;
                            }
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed to find opportunities for amount {}: {}", amount, e);
                }
            }

            // Cleanup cache periodically
            if start_time.elapsed() > Duration::from_secs(30) {
                jupiter.write().await.cleanup_cache();
            }
        }

        debug!(
            "Jupiter scan completed in {:.2}s", 
            start_time.elapsed().as_secs_f64()
        );

        Ok(())
    }

    /// Process opportunity in paper trading mode
    async fn process_paper_opportunity(
        paper_results: Arc<RwLock<PaperTradingResults>>,
        opportunity: MultiHopOpportunity,
    ) {
        let mut results = paper_results.write().await;
        
        results.total_opportunities += 1;
        
        if opportunity.profit_bps > 0 {
            results.profitable_opportunities += 1;
            results.total_paper_profit += opportunity.profit_bps as i64;
            
            // Update average
            results.average_profit_bps = results.total_paper_profit as f64 
                / results.profitable_opportunities as f64;
            
            // Track best opportunity
            if results.best_opportunity.is_none() 
                || opportunity.profit_bps > results.best_opportunity.as_ref().unwrap().profit_bps {
                results.best_opportunity = Some(opportunity.clone());
            }
            
            // Track venue performance
            for venue in &opportunity.venues_used {
                let entry = results.venue_performance.entry(venue.clone()).or_insert((0, 0));
                entry.0 += 1; // count
                entry.1 += opportunity.profit_bps as i64; // total profit
            }
            
            info!(
                "📊 Paper Trade: {}bps profit, {} hops, venues: {:?}, amount: {}",
                opportunity.profit_bps,
                opportunity.route_complexity,
                opportunity.venues_used,
                opportunity.input_amount
            );
        } else if results.worst_opportunity.is_none()
            || opportunity.profit_bps < results.worst_opportunity.as_ref().unwrap().profit_bps {
            results.worst_opportunity = Some(opportunity.clone());
        }
    }

    /// Send trading signal for real execution
    async fn send_trading_signal(
        signal_tx: &SignalTx,
        opportunity: MultiHopOpportunity,
    ) -> Result<()> {
        use shared_protocol::{JupiterArbSignalV1, RouteHop, RouteType, ArbSignal, EnhancedArbSignal, ArbSignalType};

        // Convert MultiHopOpportunity to Jupiter signal format
        let route_hops: Vec<RouteHop> = opportunity.route.iter().map(|hop| RouteHop {
            from_token: hop.from_token.clone(),
            to_token: hop.to_token.clone(),
            venue: hop.venue.clone(),
            amount_in: hop.amount_in,
            amount_out: hop.amount_out,
            fee_amount: hop.fee_amount,
            price_impact: hop.price_impact,
        }).collect();

        let route_type = match opportunity.route_complexity {
            2 => RouteType::TwoHopArbitrage,
            3 => RouteType::ThreeHopArbitrage,
            4 => RouteType::FourHopArbitrage,
            _ => RouteType::CrossDexArbitrage,
        };

        let jupiter_signal = JupiterArbSignalV1 {
            opportunity_id: opportunity.opportunity_id.clone(),
            route_type,
            input_token: opportunity.input_token.clone(),
            output_token: opportunity.output_token.clone(),
            amount_in: opportunity.input_amount as u128,
            expected_amount_out: opportunity.expected_output as u128,
            minimum_amount_out: opportunity.minimum_output as u128,
            profit_bps: opportunity.profit_bps,
            route_hops,
            total_price_impact: opportunity.price_impact_pct,
            estimated_fees: opportunity.total_fees,
            max_slippage_bps: 200, // Conservative for multi-hop
            deadline_slot: 0, // Will be set by executor
            flash_loan_required: opportunity.input_amount > 50_000_000_000, // >50 SOL
            priority_fee_lamports: 50_000, // Higher priority for multi-hop
            jupiter_route_plan: "{}".to_string(), // Placeholder for actual Jupiter route plan
            emitted_at_ns: opportunity.detected_at_ns,
            expires_at_ns: opportunity.expires_at_ns,
        };

        let signal = ArbSignal::Jupiter(jupiter_signal);

        // Send signal to hotpath for execution
        if let Err(e) = signal_tx.try_send(signal) {
            warn!("Failed to send Jupiter arbitrage signal: {}", e);
        } else {
            info!(
                "🎯 Sent Jupiter arbitrage signal: {}bps profit via {} hops",
                opportunity.profit_bps,
                opportunity.route_complexity
            );
            
            // Record profit for statistics
            record_profit(opportunity.profit_bps);
        }

        Ok(())
    }

    /// Get paper trading results for analysis
    pub async fn get_paper_results(&self) -> PaperTradingResults {
        self.paper_results.read().await.clone()
    }

    /// Switch from paper trading to live trading
    pub async fn enable_live_trading(&mut self) {
        self.config.enable_paper_trading = false;
        
        let results = self.paper_results.read().await;
        info!(
            "🔥 Switching to LIVE TRADING mode! Paper results: {} opportunities, {}bps avg profit",
            results.profitable_opportunities,
            results.average_profit_bps
        );
    }

    /// Get current Jupiter metrics
    pub async fn get_jupiter_metrics(&self) -> crate::executor_integration::JupiterMetrics {
        self.jupiter.read().await.get_metrics().clone()
    }
}

// Implementation to integrate with existing ArbitrageEngine
impl std::fmt::Debug for JupiterArbitrageScanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JupiterArbitrageScanner")
            .field("config", &self.config)
            .field("tokens", &self.tokens.len())
            .field("running", &self.running.load(std::sync::atomic::Ordering::SeqCst))
            .finish()
    }
}

impl Clone for PaperTradingResults {
    fn clone(&self) -> Self {
        Self {
            total_opportunities: self.total_opportunities,
            profitable_opportunities: self.profitable_opportunities,
            total_paper_profit: self.total_paper_profit,
            best_opportunity: self.best_opportunity.clone(),
            worst_opportunity: self.worst_opportunity.clone(),
            average_profit_bps: self.average_profit_bps,
            venue_performance: self.venue_performance.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_jupiter_scanner_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let scanner = JupiterArbitrageScanner::new(tx, None);
        
        assert!(!scanner.running.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(scanner.tokens.len(), COMMON_TOKENS.len());
    }

    #[tokio::test]
    async fn test_paper_trading_mode() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut config = JupiterArbConfig::default();
        config.enable_paper_trading = true;
        
        let scanner = JupiterArbitrageScanner::new(tx, Some(config));
        let results = scanner.get_paper_results().await;
        
        assert_eq!(results.total_opportunities, 0);
        assert_eq!(results.profitable_opportunities, 0);
    }
}