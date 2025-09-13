use anyhow::Result;
use log::{info, warn, debug, error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::ai_strategy_engine::{MarketCondition, AIStrategyEngine, ExecutionStrategy};
use crate::orderbook_tracker::ArbitrageEvent;

/// Real-time market intelligence system that feeds the AI
#[derive(Debug)]
pub struct MarketIntelligence {
    // Data sources
    price_feeds: Arc<RwLock<HashMap<String, PriceFeed>>>, // token -> price data
    volume_tracker: Arc<RwLock<VolumeTracker>>,
    whale_detector: Arc<RwLock<WhaleDetector>>,
    news_analyzer: Arc<RwLock<NewsAnalyzer>>,
    defi_monitor: Arc<RwLock<DeFiProtocolMonitor>>,
    
    // Analysis engines
    volatility_analyzer: Arc<VolatilityAnalyzer>,
    trend_detector: Arc<TrendDetector>,
    liquidity_monitor: Arc<LiquidityMonitor>,
    
    // Communication with AI
    ai_engine: Arc<AIStrategyEngine>,
    market_updates: mpsc::UnboundedSender<MarketUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceFeed {
    pub token: String,
    pub price_usd: f64,
    pub price_history: Vec<(u64, f64)>, // (timestamp, price)
    pub volume_24h: f64,
    pub liquidity_depth: f64,
    pub last_update: u64,
}

#[derive(Debug)]
pub struct VolumeTracker {
    venue_volumes: HashMap<String, VenueVolumeData>,
    token_volumes: HashMap<String, TokenVolumeData>,
    total_market_volume: f64,
    volume_spikes: Vec<VolumeSpike>,
}

#[derive(Debug, Clone)]
pub struct VenueVolumeData {
    pub venue: String,
    pub volume_1m: f64,
    pub volume_5m: f64,
    pub volume_1h: f64,
    pub volume_24h: f64,
    pub volume_trend: VolumeTrend,
}

#[derive(Debug, Clone)]
pub struct TokenVolumeData {
    pub token: String,
    pub volume_usd: f64,
    pub trade_count: u64,
    pub unique_traders: u64,
    pub avg_trade_size: f64,
}

#[derive(Debug, Clone)]
pub enum VolumeTrend {
    Increasing(f64), // percentage increase
    Decreasing(f64), // percentage decrease
    Stable,
    Spiking(f64),   // sudden spike percentage
}

#[derive(Debug, Clone)]
pub struct VolumeSpike {
    pub venue: String,
    pub token: String,
    pub spike_percentage: f64,
    pub spike_volume: f64,
    pub timestamp: u64,
    pub duration_ms: u64,
}

#[derive(Debug)]
pub struct WhaleDetector {
    whale_addresses: HashMap<String, WhaleProfile>,
    recent_movements: Vec<WhaleMovement>,
    whale_thresholds: HashMap<String, f64>, // token -> threshold
    tracking_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct WhaleProfile {
    pub address: String,
    pub estimated_net_worth: f64,
    pub trading_pattern: TradingPattern,
    pub impact_score: f64, // How much their trades move markets
    pub confidence_level: f64,
}

#[derive(Debug, Clone)]
pub enum TradingPattern {
    Accumulator,     // Steadily buying
    Distributor,     // Steadily selling
    Arbitrageur,     // Fast buy/sell cycles
    Hodler,         // Rarely trades
    MarketMaker,    // Provides liquidity
    Liquidated,     // Forced selling
}

#[derive(Debug, Clone)]
pub struct WhaleMovement {
    pub whale_address: String,
    pub token: String,
    pub amount: f64,
    pub direction: TradeDirection,
    pub estimated_impact: f64,
    pub venues_affected: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum TradeDirection {
    Buy,
    Sell,
    Transfer,
}

#[derive(Debug)]
pub struct NewsAnalyzer {
    news_sources: Vec<String>,
    sentiment_scores: HashMap<String, f64>, // token -> sentiment
    news_events: Vec<NewsEvent>,
    ai_sentiment_model: Option<String>, // Path to sentiment model
}

#[derive(Debug, Clone)]
pub struct NewsEvent {
    pub event_id: String,
    pub title: String,
    pub content: String,
    pub source: String,
    pub tokens_mentioned: Vec<String>,
    pub sentiment_score: f64, // -1.0 (very negative) to 1.0 (very positive)
    pub impact_prediction: f64, // Predicted price impact
    pub confidence: f64,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct DeFiProtocolMonitor {
    protocols: HashMap<String, ProtocolHealth>,
    governance_events: Vec<GovernanceEvent>,
    liquidation_events: Vec<LiquidationEvent>,
    protocol_updates: Vec<ProtocolUpdate>,
}

#[derive(Debug, Clone)]
pub struct ProtocolHealth {
    pub protocol_name: String,
    pub tvl_usd: f64,
    pub tvl_change_24h: f64,
    pub utilization_rate: f64,
    pub health_score: f64, // 0.0-1.0
    pub risk_level: RiskLevel,
    pub recent_issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct GovernanceEvent {
    pub protocol: String,
    pub proposal_id: String,
    pub event_type: GovernanceEventType,
    pub description: String,
    pub voting_ends: u64,
    pub current_support: f64,
    pub estimated_impact: MarketImpact,
}

#[derive(Debug, Clone)]
pub enum GovernanceEventType {
    ParameterChange,
    ProtocolUpgrade,
    TreasuryAction,
    EmergencyAction,
}

#[derive(Debug, Clone)]
pub struct MarketImpact {
    pub impact_type: ImpactType,
    pub estimated_magnitude: f64,
    pub affected_tokens: Vec<String>,
    pub time_horizon: TimeHorizon,
}

#[derive(Debug, Clone)]
pub enum ImpactType {
    Bullish,
    Bearish,
    Neutral,
    Volatile,
}

#[derive(Debug, Clone)]
pub enum TimeHorizon {
    Immediate,     // <1 hour
    ShortTerm,     // 1-24 hours
    MediumTerm,    // 1-7 days
    LongTerm,      // >7 days
}

#[derive(Debug, Clone)]
pub struct LiquidationEvent {
    pub protocol: String,
    pub liquidated_amount: f64,
    pub tokens_involved: Vec<String>,
    pub liquidation_cascade_risk: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct ProtocolUpdate {
    pub protocol: String,
    pub update_type: UpdateType,
    pub details: String,
    pub tokens_affected: Vec<String>,
    pub go_live_timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum UpdateType {
    NewMarket,
    ParameterChange,
    SecurityFix,
    FeatureAddition,
    Integration,
}

#[derive(Debug)]
pub struct VolatilityAnalyzer {
    volatility_data: HashMap<String, VolatilityMetrics>,
    volatility_thresholds: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct VolatilityMetrics {
    pub token: String,
    pub current_volatility: f64,
    pub volatility_1h: f64,
    pub volatility_24h: f64,
    pub volatility_trend: VolatilityTrend,
    pub volatility_percentile: f64, // Where current volatility ranks historically
}

#[derive(Debug, Clone)]
pub enum VolatilityTrend {
    Increasing,
    Decreasing,
    Stable,
    Explosive, // Sudden volatility spike
}

#[derive(Debug)]
pub struct TrendDetector {
    trend_data: HashMap<String, TrendAnalysis>,
    momentum_indicators: HashMap<String, MomentumIndicators>,
}

#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    pub token: String,
    pub short_term_trend: Trend, // 5-15 minutes
    pub medium_term_trend: Trend, // 1-4 hours
    pub long_term_trend: Trend,  // 24+ hours
    pub trend_strength: f64,     // 0.0-1.0
    pub trend_confidence: f64,   // 0.0-1.0
}

#[derive(Debug, Clone)]
pub enum Trend {
    StrongUp(f64),
    WeakUp(f64),
    Sideways,
    WeakDown(f64),
    StrongDown(f64),
}

#[derive(Debug, Clone)]
pub struct MomentumIndicators {
    pub rsi: f64,          // Relative Strength Index
    pub macd: f64,         // Moving Average Convergence Divergence
    pub momentum: f64,     // Price momentum
    pub volume_momentum: f64, // Volume momentum
}

#[derive(Debug)]
pub struct LiquidityMonitor {
    liquidity_data: HashMap<String, LiquidityMetrics>,
    liquidity_alerts: Vec<LiquidityAlert>,
}

#[derive(Debug, Clone)]
pub struct LiquidityMetrics {
    pub venue: String,
    pub token_pair: String,
    pub bid_depth: f64,
    pub ask_depth: f64,
    pub spread_bps: u32,
    pub liquidity_score: f64, // 0.0-1.0
    pub slippage_impact: HashMap<f64, f64>, // trade_size -> slippage
}

#[derive(Debug, Clone)]
pub struct LiquidityAlert {
    pub venue: String,
    pub token_pair: String,
    pub alert_type: LiquidityAlertType,
    pub severity: AlertSeverity,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum LiquidityAlertType {
    LowLiquidity,
    WideSpread,
    HighSlippage,
    LiquidityDrained,
    LiquidityRestored,
}

#[derive(Debug, Clone)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct MarketUpdate {
    pub update_type: MarketUpdateType,
    pub data: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum MarketUpdateType {
    VolatilitySpike(String),      // token
    WhaleMovement(WhaleMovement),
    LiquidityChange(String),      // venue_token
    NewsEvent(NewsEvent),
    ProtocolEvent(String),        // protocol
    TrendChange(String),          // token
    VolumeSpike(VolumeSpike),
}

impl MarketIntelligence {
    pub async fn new(ai_engine: Arc<AIStrategyEngine>) -> Result<Self> {
        let (market_sender, mut market_receiver) = mpsc::unbounded_channel();
        
        let intelligence = Self {
            price_feeds: Arc::new(RwLock::new(HashMap::new())),
            volume_tracker: Arc::new(RwLock::new(VolumeTracker {
                venue_volumes: HashMap::new(),
                token_volumes: HashMap::new(),
                total_market_volume: 0.0,
                volume_spikes: Vec::new(),
            })),
            whale_detector: Arc::new(RwLock::new(WhaleDetector {
                whale_addresses: HashMap::new(),
                recent_movements: Vec::new(),
                whale_thresholds: HashMap::new(),
                tracking_enabled: true,
            })),
            news_analyzer: Arc::new(RwLock::new(NewsAnalyzer {
                news_sources: vec![
                    "https://coindesk.com".to_string(),
                    "https://cointelegraph.com".to_string(),
                    "https://theblock.co".to_string(),
                ],
                sentiment_scores: HashMap::new(),
                news_events: Vec::new(),
                ai_sentiment_model: None,
            })),
            defi_monitor: Arc::new(RwLock::new(DeFiProtocolMonitor {
                protocols: HashMap::new(),
                governance_events: Vec::new(),
                liquidation_events: Vec::new(),
                protocol_updates: Vec::new(),
            })),
            volatility_analyzer: Arc::new(VolatilityAnalyzer {
                volatility_data: HashMap::new(),
                volatility_thresholds: HashMap::new(),
            }),
            trend_detector: Arc::new(TrendDetector {
                trend_data: HashMap::new(),
                momentum_indicators: HashMap::new(),
            }),
            liquidity_monitor: Arc::new(LiquidityMonitor {
                liquidity_data: HashMap::new(),
                liquidity_alerts: Vec::new(),
            }),
            ai_engine,
            market_updates: market_sender,
        };
        
        // Start background analysis
        let intelligence_clone = intelligence.clone();
        tokio::spawn(async move {
            intelligence_clone.process_market_updates(market_receiver).await;
        });
        
        // Start real-time monitoring
        let intelligence_clone = intelligence.clone();
        tokio::spawn(async move {
            intelligence_clone.monitor_market_conditions().await;
        });
        
        info!("🧠 Market Intelligence system initialized");
        Ok(intelligence)
    }
    
    /// Analyze arbitrage event and determine market conditions
    pub async fn analyze_arbitrage_event(&self, event: &ArbitrageEvent) -> Result<Vec<MarketCondition>> {
        let mut conditions = Vec::new();
        
        // 1. Check volatility
        if let Some(volatility) = self.calculate_volatility(&event.token_pair, &event.token_pair).await? {
            if volatility > 0.05 { // 5% volatility threshold
                conditions.push(MarketCondition::HighVolatility { threshold: volatility });
                info!("🌪️ High volatility detected: {:.2}%", volatility * 100.0);
            }
        }
        
        // 2. Check liquidity
        if let Some(liquidity_depth) = self.check_liquidity_depth(&event.venue_sell, &event.venue_buy).await? {
            if liquidity_depth < 10000.0 { // $10k threshold
                conditions.push(MarketCondition::LowLiquidity { depth_threshold: liquidity_depth });
                warn!("💧 Low liquidity detected: ${:.0}", liquidity_depth);
            }
        }
        
        // 3. Check for whale movements
        if let Some(whale_movement) = self.detect_recent_whale_activity(&event.token_pair).await? {
            conditions.push(MarketCondition::WhaleMovement { 
                size_threshold: whale_movement.amount 
            });
            info!("🐋 Whale movement detected: {:.2} {} tokens", 
                  whale_movement.amount, whale_movement.token);
        }
        
        // 4. Check trends
        if let Some(trend) = self.analyze_trend(&event.token_pair).await? {
            match trend {
                Trend::StrongUp(momentum) | Trend::WeakUp(momentum) => {
                    conditions.push(MarketCondition::TrendingUp { momentum });
                }
                Trend::StrongDown(momentum) | Trend::WeakDown(momentum) => {
                    conditions.push(MarketCondition::TrendingDown { momentum });
                }
                _ => {}
            }
        }
        
        // 5. Check volume spikes
        if let Some(volume_spike) = self.check_volume_spike(&event.venue_sell, &event.token_pair).await? {
            info!("📈 Volume spike detected: {:.0}% increase", volume_spike.spike_percentage);
        }
        
        // 6. Check DeFi protocol events
        if let Some(protocol_event) = self.check_protocol_events(&event.token_pair).await? {
            conditions.push(MarketCondition::DeFiProtocolEvent { 
                protocol: protocol_event.protocol, 
                event_type: format!("{:?}", protocol_event.update_type)
            });
        }
        
        // 7. News sentiment analysis
        if let Some(news_impact) = self.analyze_news_impact(&event.token_pair).await? {
            conditions.push(MarketCondition::NewsEvent { impact_score: news_impact });
        }
        
        // Update AI with new conditions
        if !conditions.is_empty() {
            self.ai_engine.update_market_conditions(conditions.clone()).await;
        }
        
        Ok(conditions)
    }
    
    /// Start real-time monitoring of all market conditions
    async fn monitor_market_conditions(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10)); // Every 10 seconds
        
        loop {
            interval.tick().await;
            
            // Monitor key metrics
            if let Err(e) = self.update_price_feeds().await {
                error!("Failed to update price feeds: {}", e);
            }
            
            if let Err(e) = self.monitor_whale_activity().await {
                error!("Failed to monitor whale activity: {}", e);
            }
            
            if let Err(e) = self.analyze_volume_patterns().await {
                error!("Failed to analyze volume patterns: {}", e);
            }
            
            if let Err(e) = self.check_protocol_health().await {
                error!("Failed to check protocol health: {}", e);
            }
            
            if let Err(e) = self.update_liquidity_metrics().await {
                error!("Failed to update liquidity metrics: {}", e);
            }
        }
    }
    
    // Implementation methods...
    
    async fn calculate_volatility(&self, token_a: &str, token_b: &str) -> Result<Option<f64>> {
        // Calculate volatility based on recent price movements
        let analyzer = &self.volatility_analyzer;
        
        if let Some(metrics) = analyzer.volatility_data.get(token_a) {
            Ok(Some(metrics.current_volatility))
        } else {
            // Calculate volatility from price history
            Ok(Some(0.03)) // Placeholder
        }
    }
    
    async fn check_liquidity_depth(&self, venue_a: &str, venue_b: &str) -> Result<Option<f64>> {
        let monitor = &self.liquidity_monitor;
        let liquidity_key = format!("{}_{}", venue_a, "SOL/USDC"); // Simplified
        
        if let Some(metrics) = monitor.liquidity_data.get(&liquidity_key) {
            Ok(Some(metrics.bid_depth + metrics.ask_depth))
        } else {
            Ok(Some(50000.0)) // Placeholder
        }
    }
    
    async fn detect_recent_whale_activity(&self, token: &str) -> Result<Option<WhaleMovement>> {
        let detector = self.whale_detector.read().await;
        
        // Check for recent whale movements in the last 5 minutes
        let cutoff_time = chrono::Utc::now().timestamp() as u64 - 300; // 5 minutes ago
        
        for movement in &detector.recent_movements {
            if movement.token == token && movement.timestamp > cutoff_time {
                return Ok(Some(movement.clone()));
            }
        }
        
        Ok(None)
    }
    
    async fn analyze_trend(&self, token: &str) -> Result<Option<Trend>> {
        let detector = &self.trend_detector;
        
        if let Some(analysis) = detector.trend_data.get(token) {
            Ok(Some(analysis.short_term_trend.clone()))
        } else {
            Ok(None)
        }
    }
    
    async fn check_volume_spike(&self, venue: &str, token: &str) -> Result<Option<VolumeSpike>> {
        let tracker = self.volume_tracker.read().await;
        
        // Check for recent volume spikes
        let cutoff_time = chrono::Utc::now().timestamp() as u64 - 60; // Last minute
        
        for spike in &tracker.volume_spikes {
            if spike.venue == venue && spike.token == token && spike.timestamp > cutoff_time {
                return Ok(Some(spike.clone()));
            }
        }
        
        Ok(None)
    }
    
    async fn check_protocol_events(&self, token: &str) -> Result<Option<ProtocolUpdate>> {
        let monitor = self.defi_monitor.read().await;
        
        // Check for recent protocol updates affecting this token
        let cutoff_time = chrono::Utc::now().timestamp() as u64 - 3600; // Last hour
        
        for update in &monitor.protocol_updates {
            if update.tokens_affected.contains(&token.to_string()) && 
               update.go_live_timestamp > cutoff_time {
                return Ok(Some(update.clone()));
            }
        }
        
        Ok(None)
    }
    
    async fn analyze_news_impact(&self, token: &str) -> Result<Option<f64>> {
        let analyzer = self.news_analyzer.read().await;
        
        // Check recent news events mentioning this token
        let cutoff_time = chrono::Utc::now().timestamp() as u64 - 3600; // Last hour
        
        for event in &analyzer.news_events {
            if event.tokens_mentioned.contains(&token.to_string()) && 
               event.timestamp > cutoff_time {
                return Ok(Some(event.impact_prediction));
            }
        }
        
        Ok(None)
    }
    
    async fn update_price_feeds(&self) -> Result<()> {
        // Update price feeds from various sources
        // This would integrate with real price APIs
        debug!("📊 Updating price feeds");
        Ok(())
    }
    
    async fn monitor_whale_activity(&self) -> Result<()> {
        // Monitor known whale addresses for large transactions
        debug!("🐋 Monitoring whale activity");
        Ok(())
    }
    
    async fn analyze_volume_patterns(&self) -> Result<()> {
        // Analyze volume patterns across venues
        debug!("📈 Analyzing volume patterns");
        Ok(())
    }
    
    async fn check_protocol_health(&self) -> Result<()> {
        // Check health of DeFi protocols
        debug!("🏥 Checking protocol health");
        Ok(())
    }
    
    async fn update_liquidity_metrics(&self) -> Result<()> {
        // Update liquidity metrics across venues
        debug!("💧 Updating liquidity metrics");
        Ok(())
    }
    
    async fn process_market_updates(&self, mut receiver: mpsc::UnboundedReceiver<MarketUpdate>) {
        while let Some(update) = receiver.recv().await {
            match update.update_type {
                MarketUpdateType::VolatilitySpike(token) => {
                    info!("🌪️ Volatility spike detected for {}", token);
                }
                MarketUpdateType::WhaleMovement(movement) => {
                    info!("🐋 Whale movement: {} {} {}", 
                          movement.amount, movement.token, 
                          if matches!(movement.direction, TradeDirection::Buy) { "bought" } else { "sold" });
                }
                MarketUpdateType::VolumeSpike(spike) => {
                    info!("📈 Volume spike: {} on {} (+{:.1}%)", 
                          spike.token, spike.venue, spike.spike_percentage);
                }
                _ => {}
            }
        }
    }
}

impl Clone for MarketIntelligence {
    fn clone(&self) -> Self {
        Self {
            price_feeds: Arc::clone(&self.price_feeds),
            volume_tracker: Arc::clone(&self.volume_tracker),
            whale_detector: Arc::clone(&self.whale_detector),
            news_analyzer: Arc::clone(&self.news_analyzer),
            defi_monitor: Arc::clone(&self.defi_monitor),
            volatility_analyzer: Arc::clone(&self.volatility_analyzer),
            trend_detector: Arc::clone(&self.trend_detector),
            liquidity_monitor: Arc::clone(&self.liquidity_monitor),
            ai_engine: Arc::clone(&self.ai_engine),
            market_updates: self.market_updates.clone(),
        }
    }
}