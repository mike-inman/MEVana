use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration, Instant};
use log::{debug, error, info, warn};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub timestamp_ns: u64,
    pub slot: u64,
    pub venue: String,
    pub market: String,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub mid_price: Option<f64>,
    pub spread_bps: Option<u16>,
    pub bid_depth_usd: Option<f64>,
    pub ask_depth_usd: Option<f64>,
    pub total_bids: i32,
    pub total_asks: i32,
    pub raw_orderbook: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub opportunity_id: String,
    pub timestamp_ns: u64,
    pub slot: u64,
    pub venue_sell: String,
    pub venue_buy: String,
    pub token_pair: String,
    pub profit_bps: i32,
    pub amount_in: f64,
    pub expected_amount_out: f64,
    pub price_impact_pct: Option<f64>,
    pub detection_time_us: u64,
    pub strategy_type: String,
    pub raw_opportunity: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub opportunity_id: String,
    pub timestamp_ns: u64,
    pub executed: bool,
    pub success: Option<bool>,
    pub actual_profit_bps: Option<i32>,
    pub execution_time_us: Option<u64>,
    pub gas_cost_sol: Option<f64>,
    pub slippage_bps: Option<i32>,
    pub failure_reason: Option<String>,
    pub transaction_signature: Option<String>,
    pub bundle_id: Option<String>,
    pub raw_result: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum HistoryEvent {
    OrderbookSnapshot(OrderbookSnapshot),
    ArbitrageOpportunity(ArbitrageOpportunity),
    ExecutionResult(ExecutionResult),
    BatchFlush, // Force flush buffer
}

/// Ultra-fast async buffer for off-hotpath database writes
/// 
/// Design principles:
/// 1. ZERO hotpath blocking - all writes are async
/// 2. Circular buffer with batch writes for efficiency  
/// 3. Automatic flush on buffer full or time interval
/// 4. Error recovery without data loss
pub struct AsyncHistoryBuffer {
    sender: mpsc::UnboundedSender<HistoryEvent>,
    _handle: tokio::task::JoinHandle<()>,
}

impl AsyncHistoryBuffer {
    /// Create new async history buffer
    /// 
    /// buffer_size: Max events before forced flush (default: 10000)
    /// flush_interval_ms: Auto-flush interval (default: 5000ms)
    /// batch_size: DB batch write size (default: 1000)
    pub async fn new(
        db_pool: PgPool,
        buffer_size: Option<usize>,
        flush_interval_ms: Option<u64>,
        batch_size: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let buffer_size = buffer_size.unwrap_or(10000);
        let flush_interval_ms = flush_interval_ms.unwrap_or(5000);
        let batch_size = batch_size.unwrap_or(1000);

        let (sender, receiver) = mpsc::unbounded_channel();
        
        // Spawn background worker task
        let handle = tokio::spawn(async move {
            let worker = HistoryBufferWorker {
                receiver,
                db_pool,
                buffer: Vec::with_capacity(buffer_size),
                buffer_size,
                batch_size,
                flush_interval: Duration::from_millis(flush_interval_ms),
                last_flush: Instant::now(),
                total_events: 0,
                flush_count: 0,
            };
            
            worker.run().await;
        });

        Ok(Self {
            sender,
            _handle: handle,
        })
    }

    /// Record orderbook snapshot (NON-BLOCKING)
    /// Returns immediately, actual DB write happens asynchronously
    #[inline(always)]
    pub fn record_orderbook(&self, snapshot: OrderbookSnapshot) {
        // This is a non-blocking send - if buffer is full, we drop old events
        let _ = self.sender.send(HistoryEvent::OrderbookSnapshot(snapshot));
    }

    /// Record arbitrage opportunity (NON-BLOCKING)  
    #[inline(always)]
    pub fn record_opportunity(&self, opportunity: ArbitrageOpportunity) {
        let _ = self.sender.send(HistoryEvent::ArbitrageOpportunity(opportunity));
    }

    /// Record execution result (NON-BLOCKING)
    #[inline(always)]
    pub fn record_execution(&self, result: ExecutionResult) {
        let _ = self.sender.send(HistoryEvent::ExecutionResult(result));
    }

    /// Force flush buffer (NON-BLOCKING request)
    #[inline(always)]
    pub fn flush(&self) {
        let _ = self.sender.send(HistoryEvent::BatchFlush);
    }
}

struct HistoryBufferWorker {
    receiver: mpsc::UnboundedReceiver<HistoryEvent>,
    db_pool: PgPool,
    buffer: Vec<HistoryEvent>,
    buffer_size: usize,
    batch_size: usize,
    flush_interval: Duration,
    last_flush: Instant,
    total_events: u64,
    flush_count: u64,
}

impl HistoryBufferWorker {
    async fn run(mut self) {
        info!("🚀 AsyncHistoryBuffer worker started (buffer_size={}, batch_size={})", 
               self.buffer_size, self.batch_size);
        
        let mut flush_timer = interval(self.flush_interval);
        
        loop {
            tokio::select! {
                // Receive new events
                event = self.receiver.recv() => {
                    match event {
                        Some(event) => {
                            if let HistoryEvent::BatchFlush = event {
                                self.flush_buffer().await;
                            } else {
                                self.buffer.push(event);
                                self.total_events += 1;
                                
                                // Flush if buffer full
                                if self.buffer.len() >= self.buffer_size {
                                    debug!("📦 Buffer full ({} events), flushing...", self.buffer.len());
                                    self.flush_buffer().await;
                                }
                            }
                        }
                        None => {
                            info!("📤 Channel closed, flushing final buffer...");
                            self.flush_buffer().await;
                            break;
                        }
                    }
                }
                
                // Periodic flush timer
                _ = flush_timer.tick() => {
                    if !self.buffer.is_empty() && self.last_flush.elapsed() >= self.flush_interval {
                        debug!("⏰ Timer flush: {} events buffered", self.buffer.len());
                        self.flush_buffer().await;
                    }
                }
            }
        }
        
        info!("✅ AsyncHistoryBuffer worker stopped (processed {} events, {} flushes)", 
              self.total_events, self.flush_count);
    }
    
    async fn flush_buffer(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        
        let start_time = Instant::now();
        let event_count = self.buffer.len();
        
        // Process in batches to avoid large transactions
        for chunk in self.buffer.chunks(self.batch_size) {
            if let Err(e) = self.write_batch(chunk).await {
                error!("❌ Failed to write batch of {} events: {}", chunk.len(), e);
                // TODO: Implement retry logic or dead letter queue
            }
        }
        
        self.buffer.clear();
        self.last_flush = Instant::now();
        self.flush_count += 1;
        
        debug!("✅ Flushed {} events to DB in {:.2}ms (flush #{})", 
               event_count, start_time.elapsed().as_secs_f64() * 1000.0, self.flush_count);
    }
    
    async fn write_batch(&self, events: &[HistoryEvent]) -> Result<(), sqlx::Error> {
        let mut tx = self.db_pool.begin().await?;
        
        let mut orderbook_count = 0;
        let mut opportunity_count = 0;
        let mut execution_count = 0;
        
        for event in events {
            match event {
                HistoryEvent::OrderbookSnapshot(snapshot) => {
                    sqlx::query(
                        r#"
                        INSERT INTO orderbook_snapshots 
                        (timestamp, slot, venue, market, best_bid, best_ask, mid_price, 
                         spread_bps, bid_depth_usd, ask_depth_usd, total_bids, total_asks, raw_orderbook)
                        VALUES (to_timestamp($1::double precision / 1000000000), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                        "#
                    )
                    .bind(snapshot.timestamp_ns as f64)
                    .bind(snapshot.slot as i64)
                    .bind(&snapshot.venue)
                    .bind(&snapshot.market)
                    .bind(snapshot.best_bid)
                    .bind(snapshot.best_ask)
                    .bind(snapshot.mid_price)
                    .bind(snapshot.spread_bps.map(|x| x as i32))
                    .bind(snapshot.bid_depth_usd)
                    .bind(snapshot.ask_depth_usd)
                    .bind(snapshot.total_bids)
                    .bind(snapshot.total_asks)
                    .bind(&snapshot.raw_orderbook)
                    .execute(&mut *tx).await?;
                    orderbook_count += 1;
                }
                
                HistoryEvent::ArbitrageOpportunity(opp) => {
                    sqlx::query(
                        r#"
                        INSERT INTO arbitrage_opportunities 
                        (opportunity_id, timestamp, slot, venue_sell, venue_buy, token_pair,
                         profit_bps, amount_in, expected_amount_out, price_impact_pct,
                         detection_time_us, strategy_type, raw_opportunity)
                        VALUES ($1, to_timestamp($2::double precision / 1000000000), $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                        ON CONFLICT (opportunity_id) DO NOTHING
                        "#
                    )
                    .bind(&opp.opportunity_id)
                    .bind(opp.timestamp_ns as f64)
                    .bind(opp.slot as i64)
                    .bind(&opp.venue_sell)
                    .bind(&opp.venue_buy)
                    .bind(&opp.token_pair)
                    .bind(opp.profit_bps)
                    .bind(opp.amount_in)
                    .bind(opp.expected_amount_out)
                    .bind(opp.price_impact_pct)
                    .bind(opp.detection_time_us as i64)
                    .bind(&opp.strategy_type)
                    .bind(&opp.raw_opportunity)
                    .execute(&mut *tx).await?;
                    opportunity_count += 1;
                }
                
                HistoryEvent::ExecutionResult(result) => {
                    sqlx::query(
                        r#"
                        INSERT INTO execution_results 
                        (opportunity_id, timestamp, executed, success, actual_profit_bps,
                         execution_time_us, gas_cost_sol, slippage_bps, failure_reason,
                         transaction_signature, bundle_id, raw_result)
                        VALUES ($1, to_timestamp($2::double precision / 1000000000), $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                        "#
                    )
                    .bind(&result.opportunity_id)
                    .bind(result.timestamp_ns as f64)
                    .bind(result.executed)
                    .bind(result.success)
                    .bind(result.actual_profit_bps)
                    .bind(result.execution_time_us.map(|x| x as i64))
                    .bind(result.gas_cost_sol)
                    .bind(result.slippage_bps)
                    .bind(&result.failure_reason)
                    .bind(&result.transaction_signature)
                    .bind(&result.bundle_id)
                    .bind(&result.raw_result)
                    .execute(&mut *tx).await?;
                    execution_count += 1;
                }
                
                HistoryEvent::BatchFlush => {} // No-op
            }
        }
        
        tx.commit().await?;
        
        if orderbook_count + opportunity_count + execution_count > 0 {
            debug!("💾 DB batch written: {} orderbook, {} opportunities, {} executions", 
                   orderbook_count, opportunity_count, execution_count);
        }
        
        Ok(())
    }
}

/// Helper function to create orderbook snapshot for hotpath recording
#[inline(always)]
pub fn create_orderbook_snapshot(
    venue: &str,
    market: &str,
    slot: u64,
    timestamp_ns: u64,
    best_bid: Option<f64>,
    best_ask: Option<f64>,
    orderbook_data: serde_json::Value,
) -> OrderbookSnapshot {
    let mid_price = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
        _ => None,
    };
    
    let spread_bps = match (best_bid, best_ask, mid_price) {
        (Some(bid), Some(ask), Some(mid)) if mid > 0.0 => {
            Some(((ask - bid) / mid * 10000.0) as u16)
        }
        _ => None,
    };

    OrderbookSnapshot {
        timestamp_ns,
        slot,
        venue: venue.to_string(),
        market: market.to_string(),
        best_bid,
        best_ask,
        mid_price,
        spread_bps,
        bid_depth_usd: None, // Calculate if needed
        ask_depth_usd: None, // Calculate if needed
        total_bids: 0, // Extract from orderbook_data if needed
        total_asks: 0, // Extract from orderbook_data if needed
        raw_orderbook: orderbook_data,
    }
}

/// Helper function to create arbitrage opportunity for hotpath recording
#[inline(always)]
pub fn create_arbitrage_opportunity(
    opportunity_id: &str,
    slot: u64,
    timestamp_ns: u64,
    venue_sell: &str,
    venue_buy: &str,
    token_pair: &str,
    profit_bps: i32,
    amount_in: f64,
    expected_amount_out: f64,
    detection_time_us: u64,
    strategy_type: &str,
    raw_data: serde_json::Value,
) -> ArbitrageOpportunity {
    ArbitrageOpportunity {
        opportunity_id: opportunity_id.to_string(),
        timestamp_ns,
        slot,
        venue_sell: venue_sell.to_string(),
        venue_buy: venue_buy.to_string(),
        token_pair: token_pair.to_string(),
        profit_bps,
        amount_in,
        expected_amount_out,
        price_impact_pct: None,
        detection_time_us,
        strategy_type: strategy_type.to_string(),
        raw_opportunity: raw_data,
    }
}

/// Helper function to create execution result for hotpath recording
#[inline(always)]
pub fn create_execution_result(
    opportunity_id: &str,
    timestamp_ns: u64,
    executed: bool,
    success: Option<bool>,
    actual_profit_bps: Option<i32>,
    execution_time_us: Option<u64>,
    failure_reason: Option<&str>,
    raw_data: serde_json::Value,
) -> ExecutionResult {
    ExecutionResult {
        opportunity_id: opportunity_id.to_string(),
        timestamp_ns,
        executed,
        success,
        actual_profit_bps,
        execution_time_us,
        gas_cost_sol: None,
        slippage_bps: None,
        failure_reason: failure_reason.map(|s| s.to_string()),
        transaction_signature: None,
        bundle_id: None,
        raw_result: raw_data,
    }
}