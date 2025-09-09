use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use log::{debug, info, warn};
use crossbeam::queue::SegQueue;
use lazy_static::lazy_static;

// DETECTION QUEUES - Global lock-free queues for qualified arbitrage opportunities
// ARBITRAGE_DETECTED: Profitable opportunities found by detection engine
// EXECUTION_RESULTS: Trade execution outcomes for analysis
lazy_static! {
    pub static ref ARBITRAGE_DETECTED: SegQueue<ArbitrageEvent> = SegQueue::new();
    pub static ref EXECUTION_RESULTS: SegQueue<ExecutionResult> = SegQueue::new();
}

// REDIS KEYS for cross-process communication
pub const REDIS_ARBITRAGE_DETECTED_KEY: &str = "arbitrage:detected";
pub const REDIS_EXECUTION_RESULTS_KEY: &str = "execution:results";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub timestamp: u64,
    pub slot: u64,
    pub venue: String,
    pub market: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub mid_price: Option<f64>,
    pub spread_bps: Option<u16>,
    pub depth_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub size: f64,
    pub cumulative_size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageEvent {
    pub timestamp: u64,
    pub slot: u64,
    pub opportunity_id: String,
    pub venue_sell: String,
    pub venue_buy: String,
    pub token_pair: String,
    pub profit_bps: i32,
    pub amount_in: f64,
    pub amount_out: f64,
    pub execution_time_us: u64,
    pub success: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub timestamp: u64,
    pub opportunity_id: String,
    pub executed: bool,
    pub success: bool,
    pub actual_profit_bps: Option<i32>,
    pub execution_time_us: u64,
    pub gas_cost_sol: Option<f64>,
    pub slippage_bps: Option<u16>,
    pub failure_reason: Option<String>,
    pub transaction_signature: Option<String>,
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStats {
    pub total_opportunities: u64,
    pub profitable_opportunities: u64,
    pub total_profit_bps: i64,
    pub average_profit_bps: f64,
    pub success_rate: f64,
    pub average_execution_time_us: f64,
    pub venue_performance: HashMap<String, VenueStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueStats {
    pub opportunities: u64,
    pub profit_bps: i64,
    pub average_execution_us: f64,
    pub success_rate: f64,
}

pub struct OrderbookTracker {
    // Circular buffers for efficient memory usage
    orderbook_history: Arc<RwLock<VecDeque<OrderbookSnapshot>>>,
    arbitrage_history: Arc<RwLock<VecDeque<ArbitrageEvent>>>,
    market_stats: Arc<RwLock<MarketStats>>,
    
    // Configuration
    max_orderbook_entries: usize,
    max_arbitrage_entries: usize,
}

impl OrderbookTracker {
    pub fn new(max_orderbook_entries: usize, max_arbitrage_entries: usize) -> Self {
        Self {
            orderbook_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_orderbook_entries))),
            arbitrage_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_arbitrage_entries))),
            market_stats: Arc::new(RwLock::new(MarketStats {
                total_opportunities: 0,
                profitable_opportunities: 0,
                total_profit_bps: 0,
                average_profit_bps: 0.0,
                success_rate: 0.0,
                average_execution_time_us: 0.0,
                venue_performance: HashMap::new(),
            })),
            max_orderbook_entries,
            max_arbitrage_entries,
        }
    }

    /// Record orderbook snapshot
    pub async fn record_orderbook(&self, snapshot: OrderbookSnapshot) {
        let mut history = self.orderbook_history.write().await;
        
        // Remove oldest if at capacity
        if history.len() >= self.max_orderbook_entries {
            history.pop_front();
        }
        
        debug!(
            "📊 Recorded orderbook: {} {} @ {} (spread: {}bps)",
            snapshot.venue,
            snapshot.market,
            snapshot.mid_price.unwrap_or(0.0),
            snapshot.spread_bps.unwrap_or(0)
        );
        
        history.push_back(snapshot);
    }

    /// Record arbitrage opportunity/execution
    pub async fn record_arbitrage(&self, event: ArbitrageEvent) {
        let mut arb_history = self.arbitrage_history.write().await;
        let mut stats = self.market_stats.write().await;
        
        // Remove oldest if at capacity - CAPTURE TO DETECTION QUEUE FOR EXECUTION
        if arb_history.len() >= self.max_arbitrage_entries {
            if let Some(detected_event) = arb_history.pop_front() {
                // Fire-and-forget: Push to global detection queue for execution service
                info!(
                    "🎯 QUEUEING FOR EXECUTION: {} {}bps {}->{} (slot: {}) - queue size now: {}",
                    detected_event.opportunity_id,
                    detected_event.profit_bps,
                    detected_event.venue_sell,
                    detected_event.venue_buy,
                    detected_event.slot,
                    ARBITRAGE_DETECTED.len() + 1
                );
                ARBITRAGE_DETECTED.push(detected_event);
            }
        }
        
        // Update statistics
        stats.total_opportunities += 1;
        if event.profit_bps > 0 {
            stats.profitable_opportunities += 1;
            stats.total_profit_bps += event.profit_bps as i64;
        }
        
        // Recalculate averages
        if stats.profitable_opportunities > 0 {
            stats.average_profit_bps = stats.total_profit_bps as f64 / stats.profitable_opportunities as f64;
        }
        
        let success_count = arb_history.iter().filter(|e| e.success).count() + (if event.success { 1 } else { 0 });
        stats.success_rate = success_count as f64 / (arb_history.len() + 1) as f64 * 100.0;
        
        let total_execution_time: u64 = arb_history.iter().map(|e| e.execution_time_us).sum::<u64>() + event.execution_time_us;
        stats.average_execution_time_us = total_execution_time as f64 / (arb_history.len() + 1) as f64;
        
        // Update venue performance
        let venue_key = format!("{}->{}", event.venue_sell, event.venue_buy);
        let venue_stats = stats.venue_performance.entry(venue_key).or_insert(VenueStats {
            opportunities: 0,
            profit_bps: 0,
            average_execution_us: 0.0,
            success_rate: 0.0,
        });
        
        venue_stats.opportunities += 1;
        if event.profit_bps > 0 {
            venue_stats.profit_bps += event.profit_bps as i64;
        }
        
        info!(
            "🎯 RECORDED ARBITRAGE: {} {}bps {}->{} ({}μs) {} | History: {}/{} entries",
            event.opportunity_id,
            event.profit_bps,
            event.venue_sell,
            event.venue_buy,
            event.execution_time_us,
            if event.success { "✅" } else { "❌" },
            arb_history.len() + 1,
            self.max_arbitrage_entries
        );
        
        arb_history.push_back(event);
    }

    /// Get recent orderbook snapshots
    pub async fn get_orderbook_history(&self, limit: Option<usize>) -> Vec<OrderbookSnapshot> {
        let history = self.orderbook_history.read().await;
        let limit = limit.unwrap_or(100).min(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get recent arbitrage events
    pub async fn get_arbitrage_history(&self, limit: Option<usize>) -> Vec<ArbitrageEvent> {
        let history = self.arbitrage_history.read().await;
        let limit = limit.unwrap_or(100).min(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get market statistics
    pub async fn get_market_stats(&self) -> MarketStats {
        self.market_stats.read().await.clone()
    }

    /// Get orderbook for specific venue/market
    pub async fn get_latest_orderbook(&self, venue: &str, market: &str) -> Option<OrderbookSnapshot> {
        let history = self.orderbook_history.read().await;
        history.iter().rev()
            .find(|snapshot| snapshot.venue == venue && snapshot.market == market)
            .cloned()
    }

    /// Get arbitrage opportunities by venue pair
    pub async fn get_arbitrage_by_venues(&self, venue_sell: &str, venue_buy: &str) -> Vec<ArbitrageEvent> {
        let history = self.arbitrage_history.read().await;
        history.iter()
            .filter(|event| event.venue_sell == venue_sell && event.venue_buy == venue_buy)
            .cloned()
            .collect()
    }
}

impl Default for OrderbookTracker {
    fn default() -> Self {
        Self::new(10000, 5000) // 10k orderbook snapshots, 5k arbitrage events
    }
}

/// Helper to create orderbook snapshot from transaction data
pub fn create_orderbook_snapshot(
    venue: &str,
    market: &str,
    slot: u64,
    bids: Vec<(f64, f64)>, // (price, size)
    asks: Vec<(f64, f64)>, // (price, size)
) -> OrderbookSnapshot {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Convert to price levels with cumulative size
    let mut bid_levels = Vec::new();
    let mut cumulative_bid_size = 0.0;
    for (price, size) in bids {
        cumulative_bid_size += size;
        bid_levels.push(PriceLevel {
            price,
            size,
            cumulative_size: cumulative_bid_size,
        });
    }

    let mut ask_levels = Vec::new();
    let mut cumulative_ask_size = 0.0;
    for (price, size) in asks {
        cumulative_ask_size += size;
        ask_levels.push(PriceLevel {
            price,
            size,
            cumulative_size: cumulative_ask_size,
        });
    }

    // Calculate mid price and spread
    let best_bid = bid_levels.first().map(|l| l.price);
    let best_ask = ask_levels.first().map(|l| l.price);
    
    let (mid_price, spread_bps) = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => {
            let mid = (bid + ask) / 2.0;
            let spread = ((ask - bid) / mid * 10000.0) as u16;
            (Some(mid), Some(spread))
        }
        _ => (None, None),
    };

    // Estimate depth in USD (simplified)
    let depth_usd = mid_price.map(|mid| {
        let bid_depth: f64 = bid_levels.iter().take(10).map(|l| l.size * l.price).sum();
        let ask_depth: f64 = ask_levels.iter().take(10).map(|l| l.size * l.price).sum();
        (bid_depth + ask_depth) / 2.0
    });

    OrderbookSnapshot {
        timestamp,
        slot,
        venue: venue.to_string(),
        market: market.to_string(),
        bids: bid_levels,
        asks: ask_levels,
        mid_price,
        spread_bps,
        depth_usd,
    }
}

/// Helper to create arbitrage event
pub fn create_arbitrage_event(
    opportunity_id: &str,
    slot: u64,
    venue_sell: &str,
    venue_buy: &str,
    token_pair: &str,
    profit_bps: i32,
    amount_in: f64,
    amount_out: f64,
    execution_time_us: u64,
    success: bool,
    reason: Option<&str>,
) -> ArbitrageEvent {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    ArbitrageEvent {
        timestamp,
        slot,
        opportunity_id: opportunity_id.to_string(),
        venue_sell: venue_sell.to_string(),
        venue_buy: venue_buy.to_string(),
        token_pair: token_pair.to_string(),
        profit_bps,
        amount_in,
        amount_out,
        execution_time_us,
        success,
        reason: reason.map(|s| s.to_string()),
    }
}

/// Helper to create execution result
pub fn create_execution_result(
    opportunity_id: &str,
    executed: bool,
    success: bool,
    actual_profit_bps: Option<i32>,
    execution_time_us: u64,
    gas_cost_sol: Option<f64>,
    slippage_bps: Option<u16>,
    failure_reason: Option<&str>,
    transaction_signature: Option<&str>,
    bundle_id: Option<&str>,
) -> ExecutionResult {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    ExecutionResult {
        timestamp,
        opportunity_id: opportunity_id.to_string(),
        executed,
        success,
        actual_profit_bps,
        execution_time_us,
        gas_cost_sol,
        slippage_bps,
        failure_reason: failure_reason.map(|s| s.to_string()),
        transaction_signature: transaction_signature.map(|s| s.to_string()),
        bundle_id: bundle_id.map(|s| s.to_string()),
    }
}

/// EXECUTION SERVICE API - Consume detected arbitrage opportunities
/// This is how execution service gets qualified opportunities for trading
/// Zero-impact on hotpath - consuming detected opportunities from lock-free queue
pub fn consume_detected_arbitrage_events(batch_size: usize) -> Vec<ArbitrageEvent> {
    let mut events = Vec::with_capacity(batch_size);
    for _ in 0..batch_size {
        if let Some(event) = ARBITRAGE_DETECTED.pop() {
            events.push(event);
        } else {
            break;
        }
    }
    
    if !events.is_empty() {
        info!(
            "⚡ CONSUMED {} detected arbitrage opportunities (queue had {} remaining)",
            events.len(),
            ARBITRAGE_DETECTED.len()
        );
    }
    
    events
}

/// MONITORING SERVICE API - Consume execution results for analysis
/// Zero-impact on execution path - consuming trade outcomes for performance tracking
pub fn consume_execution_results(batch_size: usize) -> Vec<ExecutionResult> {
    let mut results = Vec::with_capacity(batch_size);
    for _ in 0..batch_size {
        if let Some(result) = EXECUTION_RESULTS.pop() {
            results.push(result);
        } else {
            break;
        }
    }
    
    if !results.is_empty() {
        info!(
            "📊 CONSUMED {} execution results for analysis (queue had {} remaining)",
            results.len(),
            EXECUTION_RESULTS.len()
        );
    }
    
    results
}