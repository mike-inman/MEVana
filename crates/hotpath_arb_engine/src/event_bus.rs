use chrono::{DateTime, Utc};
use futures::executor;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Event type for different reporting needs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventType {
    ArbitrageDetected,
    ArbitrageRejected,
    TradeExecution,
    TradeResult,
    SystemHealth,
    AiPrediction,
}

/// Unified event structure for all reporting needs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingEvent {
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
    pub data: serde_json::Value,
    pub processing_time_us: Option<u64>,
}

/// Destinations for event reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterConfig {
    pub enable_file_logging: bool,
    pub file_path: String,
    pub enable_kafka: bool,
    pub kafka_broker: String,
    pub kafka_topic: String,
    pub enable_api_logging: bool,
    pub api_base_url: String,
    pub api_auth_token: Option<String>,
    pub enable_redis: bool,
    pub redis_url: String,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub sampling_rate: f64,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self {
            enable_file_logging: true,
            file_path: "/tmp/mevana_trades.log".to_string(),
            enable_kafka: false,
            kafka_broker: "localhost:29092".to_string(),
            kafka_topic: "alerts.arbitrage.opportunities".to_string(),
            enable_api_logging: true,
            api_base_url: "http://localhost:8092".to_string(),
            api_auth_token: std::env::var("API_AUTH_TOKEN").ok(),
            enable_redis: false,
            redis_url: "redis://localhost:6379".to_string(),
            batch_size: 100,
            flush_interval_ms: 100,
            sampling_rate: 1.0, // Report everything by default
        }
    }
}

/// Reporter service for processing events in batches
pub struct ReporterService {
    config: ReporterConfig,
    event_rx: mpsc::Receiver<TradingEvent>,
    kafka_producer: Option<rdkafka::producer::FutureProducer>,
    redis_client: Option<redis::Client>,
    file_handle: Option<tokio::fs::File>,
    http_client: Option<reqwest::Client>,
    db_pool: Option<PgPool>,
}

impl ReporterService {
    pub fn new(rx: mpsc::Receiver<TradingEvent>, config: ReporterConfig, db_pool: Option<PgPool>) -> Self {
        // Initialize Kafka producer if enabled
        let kafka_producer = if config.enable_kafka {
            match rdkafka::config::ClientConfig::new()
                .set("bootstrap.servers", &config.kafka_broker)
                .set("acks", "0")                           // Fire-and-forget (no ack wait)
                .set("linger.ms", "1")                     // Batch quickly for throughput
                .set("batch.size", "65536")                // 64KB batches
                .set("buffer.memory", "67108864")          // 64MB buffer
                .set("max.block.ms", "0")                  // Never block producer
                .set("delivery.timeout.ms", "1000")        // Quick timeout
                .set("compression.type", "snappy")         // Fast compression
                .set("retries", "0")                       // No retries for speed
                .set("enable.idempotence", "false")        // Disable for max speed
                .create()
            {
                Ok(producer) => {
                    info!("✅ Initialized Kafka producer for event bus");
                    Some(producer)
                }
                Err(e) => {
                    warn!("⚠️ Failed to initialize Kafka producer: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Initialize Redis client if enabled
        let redis_client = if config.enable_redis {
            match redis::Client::open(config.redis_url.clone()) {
                Ok(client) => {
                    info!("✅ Initialized Redis client for event bus");
                    Some(client)
                }
                Err(e) => {
                    warn!("⚠️ Failed to initialize Redis client: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Initialize HTTP client for API calls if enabled
        let http_client = if config.enable_api_logging {
            Some(reqwest::Client::new())
        } else {
            None
        };

        Self {
            config,
            event_rx: rx,
            kafka_producer,
            redis_client,
            file_handle: None,
            http_client,
            db_pool,
        }
    }

    async fn open_log_file(&mut self) -> Result<(), std::io::Error> {
        if self.config.enable_file_logging && self.file_handle.is_none() {
            self.file_handle = Some(
                tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.config.file_path)
                    .await?,
            );
        }
        Ok(())
    }

    async fn write_to_file(&mut self, events: &[TradingEvent]) -> Result<(), std::io::Error> {
        if !self.config.enable_file_logging {
            return Ok(());
        }

        // Lazily open the file if needed
        if self.file_handle.is_none() {
            self.open_log_file().await?;
        }

        if let Some(file) = &mut self.file_handle {
            use tokio::io::AsyncWriteExt;
            
            for event in events {
                // Only log trade executions to the file
                if matches!(event.event_type, EventType::TradeExecution) {
                    let log_line = match &event.data {
                        serde_json::Value::Object(map) => {
                            if let Some(trade_details) = map.get("trade_details") {
                                format!("{}\n", trade_details.as_str().unwrap_or_default())
                            } else {
                                format!("{}\n", serde_json::to_string(&event.data).unwrap_or_default())
                            }
                        }
                        _ => format!("{}\n", serde_json::to_string(&event.data).unwrap_or_default()),
                    };
                    
                    file.write_all(log_line.as_bytes()).await?;
                }
            }
            
            file.flush().await?;
        }

        Ok(())
    }

    async fn publish_to_kafka(&self, events: &[TradingEvent]) -> Result<(), rdkafka::error::KafkaError> {
        if !self.config.enable_kafka || self.kafka_producer.is_none() {
            return Ok(());
        }

        let producer = self.kafka_producer.as_ref().unwrap();

        for event in events {
            // Only publish arbitrage opportunities to Kafka
            if matches!(event.event_type, EventType::ArbitrageDetected | EventType::ArbitrageRejected) {
                let payload = serde_json::to_string(&event.data).unwrap_or_default();
                let record = rdkafka::producer::FutureRecord::to(&self.config.kafka_topic)
                    .key(&event.signature)
                    .payload(&payload);
                
                match producer.send(record, Duration::ZERO).await {
                    Ok(delivery_result) => {
                        debug!("📤 Published opportunity {} to Kafka: {:?}", 
                               &event.signature[0..8], delivery_result);
                    }
                    Err((kafka_error, _)) => {
                        warn!("⚠️ Failed to publish opportunity {} to Kafka: {}", 
                              &event.signature[0..8], kafka_error);
                        return Err(kafka_error);
                    }
                }
            }
        }

        Ok(())
    }

    async fn publish_to_redis(&self, events: &[TradingEvent]) -> Result<(), redis::RedisError> {
        if !self.config.enable_redis || self.redis_client.is_none() {
            return Ok(());
        }

        // Get a connection from the pool
        let client = self.redis_client.as_ref().unwrap();
        let mut conn = match client.get_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("⚠️ Failed to get Redis connection: {}", e);
                return Err(e);
            }
        };

        use redis::AsyncCommands;
        
        for event in events {
            let json = serde_json::to_string(&event.data).unwrap_or_default();
            let key = match event.event_type {
                EventType::ArbitrageDetected => "arbitrage:detected",
                EventType::ArbitrageRejected => "arbitrage:rejected",
                EventType::TradeExecution => "trades:executions",
                EventType::TradeResult => "trades:results",
                EventType::SystemHealth => "system:health",
                EventType::AiPrediction => "ai:predictions",
            };
            
            let _: Result<(), _> = conn.lpush(key, json).await;
        }

        Ok(())
    }

    async fn write_to_database(&self, events: &[TradingEvent]) -> Result<(), sqlx::Error> {
        let db_pool = match &self.db_pool {
            Some(pool) => pool,
            None => return Ok(()), // Database not available
        };

        // Filter arbitrage events for batching
        let arb_events: Vec<&TradingEvent> = events.iter()
            .filter(|event| matches!(event.event_type, EventType::ArbitrageDetected | EventType::ArbitrageRejected))
            .collect();

        if arb_events.is_empty() {
            return Ok(());
        }

        // Use a transaction for batch writes
        let mut tx = db_pool.begin().await?;
        let mut inserted_count = 0;

        for event in arb_events {
            let data = &event.data;
            
            // Extract data from the event JSON for database insertion
            let opportunity_id = data.get("signature").and_then(|v| v.as_str()).unwrap_or("");
            let slot = data.get("slot").and_then(|v| v.as_u64()).unwrap_or(0) as i64;
            let venue_sell = data.get("venues").and_then(|v| v.as_array()).and_then(|arr| arr.get(0)).and_then(|v| v.as_str()).unwrap_or("");
            let venue_buy = data.get("venues").and_then(|v| v.as_array()).and_then(|arr| arr.get(1)).and_then(|v| v.as_str()).unwrap_or("");
            let token_pair = data.get("token_pair").and_then(|v| v.as_str()).unwrap_or("");
            let profit_bps = data.get("profit_bps").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let amount_in = data.get("amount_in").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let expected_amount_out = data.get("estimated_profit_sol").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let price_impact_pct = data.get("price_impact_pct").and_then(|v| v.as_f64());
            let detection_time_us = event.processing_time_us.unwrap_or(0) as i64;
            let strategy_type = "hotpath";
            
            // Create raw_opportunity JSON with timestamp and status
            let mut raw_opp = data.clone();
            raw_opp["timestamp"] = serde_json::Value::String(event.timestamp.to_rfc3339());
            if matches!(event.event_type, EventType::ArbitrageRejected) {
                raw_opp["status"] = serde_json::Value::String("rejected".to_string());
                raw_opp["rejection_reason"] = data.get("rejection_reason").cloned().unwrap_or(serde_json::Value::Null);
            } else {
                raw_opp["status"] = serde_json::Value::String("detected".to_string());
            }

            // Insert into database using the same schema as async_history_buffer
            let timestamp_ns = event.timestamp.timestamp_nanos_opt().unwrap_or(0) as f64;
            
            match sqlx::query(
                r#"
                INSERT INTO arbitrage_opportunities 
                (opportunity_id, timestamp, slot, venue_sell, venue_buy, token_pair,
                 profit_bps, amount_in, expected_amount_out, price_impact_pct,
                 detection_time_us, strategy_type, raw_opportunity)
                VALUES ($1, to_timestamp($2::double precision / 1000000000), $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ON CONFLICT (opportunity_id) DO NOTHING
                "#
            )
            .bind(opportunity_id)
            .bind(timestamp_ns)
            .bind(slot)
            .bind(venue_sell)
            .bind(venue_buy)
            .bind(token_pair)
            .bind(profit_bps)
            .bind(amount_in)
            .bind(expected_amount_out)
            .bind(price_impact_pct)
            .bind(detection_time_us)
            .bind(strategy_type)
            .bind(&raw_opp)
            .execute(&mut *tx)
            .await
            {
                Ok(_) => {
                    inserted_count += 1;
                }
                Err(e) => {
                    warn!("⚠️ Failed to insert arbitrage opportunity {}: {}", opportunity_id, e);
                }
            }
        }

        // Commit the transaction
        match tx.commit().await {
            Ok(_) => {
                debug!("✅ Successfully batched {} arbitrage opportunities to database", inserted_count);
                Ok(())
            }
            Err(e) => {
                warn!("⚠️ Failed to commit arbitrage opportunities batch: {}", e);
                Err(e)
            }
        }
    }

    pub async fn run(mut self) {
        // Open log file on startup if enabled
        if self.config.enable_file_logging {
            if let Err(e) = self.open_log_file().await {
                warn!("⚠️ Failed to open log file: {}", e);
            }
        }
        
        info!("🔄 Event Bus Reporter service started");

        let mut interval = tokio::time::interval(Duration::from_millis(self.config.flush_interval_ms));
        let mut events = Vec::with_capacity(self.config.batch_size);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Process any collected events on interval tick
                    if !events.is_empty() {
                        self.process_events(&events).await;
                        events.clear();
                    }
                }
                event_opt = self.event_rx.recv() => {
                    match event_opt {
                        Some(event) => {
                            events.push(event);
                            
                            // Process immediately if we hit batch size
                            if events.len() >= self.config.batch_size {
                                self.process_events(&events).await;
                                events.clear();
                            }
                        }
                        None => {
                            // Channel closed, exit
                            warn!("⚠️ Event channel closed, shutting down reporter");
                            // Process any remaining events
                            if !events.is_empty() {
                                self.process_events(&events).await;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn process_events(&mut self, events: &[TradingEvent]) {
        // Record batch size for monitoring
        metrics::histogram!("event_bus_batch_size").record(events.len() as f64);
        let start = std::time::Instant::now();

        // Process events to each destination sequentially due to borrow checker constraints
        let file_result = self.write_to_file(events).await;
        let kafka_result = self.publish_to_kafka(events).await;
        let redis_result = self.publish_to_redis(events).await;
        let db_result = self.write_to_database(events).await;

        // Handle errors
        if let Err(e) = file_result {
            warn!("⚠️ Failed to write events to log file: {}", e);
            metrics::counter!("event_bus_file_errors_total").increment(1);
        }

        if let Err(e) = kafka_result {
            warn!("⚠️ Failed to publish events to Kafka: {}", e);
            metrics::counter!("event_bus_kafka_errors_total").increment(1);
        }

        if let Err(e) = redis_result {
            warn!("⚠️ Failed to publish events to Redis: {}", e);
            metrics::counter!("event_bus_redis_errors_total").increment(1);
        }

        if let Err(e) = db_result {
            warn!("⚠️ Failed to write events to database: {}", e);
            metrics::counter!("event_bus_database_errors_total").increment(1);
        }

        // Record processing time
        let elapsed = start.elapsed();
        metrics::histogram!("event_bus_processing_time_ms").record(elapsed.as_millis() as f64);
    }
}

/// Global event bus for the entire application
pub struct EventBus {
    config: ReporterConfig,
    sender: mpsc::Sender<TradingEvent>,
    _service_handle: Option<JoinHandle<()>>,
}

impl EventBus {
    pub fn new(config: ReporterConfig) -> Self {
        let (tx, rx) = mpsc::channel(10000); // Buffer size for high throughput
        let reporter = ReporterService::new(rx, config.clone(), None);
        
        // Spawn the reporter service
        let handle = tokio::spawn(async move {
            reporter.run().await;
        });

        Self {
            config,
            sender: tx,
            _service_handle: Some(handle),
        }
    }

    pub fn with_database_pool(config: ReporterConfig, db_pool: PgPool) -> Self {
        let (tx, rx) = mpsc::channel(10000); // Buffer size for high throughput
        let reporter = ReporterService::new(rx, config.clone(), Some(db_pool));
        
        // Spawn the reporter service
        let handle = tokio::spawn(async move {
            reporter.run().await;
        });

        Self {
            config,
            sender: tx,
            _service_handle: Some(handle),
        }
    }
    
    /// Set database pool for direct database writes (replaces unreliable API calls)
    /// Note: This method is deprecated. Use EventBus::with_database_pool() constructor instead
    /// for proper database integration during initialization.
    pub async fn set_database_pool(&self, _db_pool: PgPool) {
        warn!("⚠️ EventBus::set_database_pool is deprecated - use EventBus::with_database_pool() constructor instead");
    }

    /// INTERNAL: Set the centralized database pool from main application
    /// This is used by the global EVENT_BUS singleton to receive the shared database pool
    /// from the main application to prevent duplicate database connections.
    pub fn set_shared_database_pool(&self, db_pool: Arc<PgPool>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        warn!("⚠️ EventBus::set_shared_database_pool not yet implemented - EventBus will use API fallback for now");
        warn!("🔧 TODO: Implement runtime database pool injection for existing EventBus instance");
        Ok(())
    }

    pub fn sender(&self) -> mpsc::Sender<TradingEvent> {
        self.sender.clone()
    }
    
    pub async fn send_trade_execution(&self, trade_id: &str, trade_details: String, processing_time_us: u64) {
        // Sample events based on configuration
        if fastrand::f64() > self.config.sampling_rate {
            return;
        }

        let event = TradingEvent {
            event_type: EventType::TradeExecution,
            timestamp: Utc::now(),
            signature: trade_id.to_string(),
            data: serde_json::json!({
                "trade_id": trade_id,
                "trade_details": trade_details,
            }),
            processing_time_us: Some(processing_time_us),
        };

        // Non-blocking send - drop if buffer is full
        debug!("📤 Sending trade execution event: {}", trade_id);
        let _ = self.sender.send(event).await;
    }
    
    pub async fn send_arbitrage_detected(
        &self, 
        signature: &str, 
        venues: &[String], 
        profit_bps: u64,
        token_pair: &str,
        estimated_profit: f64,
        confidence: f64,
        data: serde_json::Value,
        processing_time_us: u64,
    ) {
        // Sample events based on configuration
        if fastrand::f64() > self.config.sampling_rate {
            return;
        }

        let event = TradingEvent {
            event_type: EventType::ArbitrageDetected,
            timestamp: Utc::now(),
            signature: signature.to_string(),
            data: serde_json::json!({
                "signature": signature,
                "venues": venues,
                "profit_bps": profit_bps,
                "token_pair": token_pair,
                "estimated_profit_sol": estimated_profit,
                "confidence_score": confidence,
                "details": data,
            }),
            processing_time_us: Some(processing_time_us),
        };

        // Non-blocking send - drop if buffer is full
        debug!("📤 Sending arbitrage detected event: {}", signature);
        let _ = self.sender.send(event).await;
    }
    
    pub async fn send_arbitrage_rejected(
        &self, 
        signature: &str, 
        venues: &[String], 
        profit_bps: u64,
        token_pair: &str,
        estimated_profit: f64,
        confidence: f64,
        rejection_reason: String,
        data: serde_json::Value,
        processing_time_us: u64,
    ) {
        // Sample events based on configuration
        if fastrand::f64() > self.config.sampling_rate {
            return;
        }

        let event = TradingEvent {
            event_type: EventType::ArbitrageRejected,
            timestamp: Utc::now(),
            signature: signature.to_string(),
            data: serde_json::json!({
                "signature": signature,
                "venues": venues,
                "profit_bps": profit_bps,
                "token_pair": token_pair,
                "estimated_profit_sol": estimated_profit,
                "confidence_score": confidence,
                "rejection_reason": rejection_reason,
                "additional_data": data
            }),
            processing_time_us: Some(processing_time_us),
        };

        // Non-blocking send - drop if buffer is full
        debug!("📤 Sending arbitrage rejected event: {}", signature);
        let _ = self.sender.send(event).await;
    }
}

// Global singleton for the event bus - NO DATABASE CONNECTION
lazy_static::lazy_static! {
    pub static ref EVENT_BUS: Arc<EventBus> = {
        let config = match std::env::var("EVENT_BUS_CONFIG") {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => ReporterConfig::default(),
        };
        
        // Initialize WITHOUT database pool to prevent duplicate connections
        // The main application will provide the centralized database pool via set_shared_database_pool()
        info!("📡 EventBus initialized without database pool (will use centralized pool from main)");
        Arc::new(EventBus::new(config))
    };
}