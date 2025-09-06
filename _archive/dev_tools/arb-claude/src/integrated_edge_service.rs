use anyhow::{Context, Result};
use log::{error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use redis::{AsyncCommands, Value};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::edge::EdgeDelta;
use crate::pool_manager::PoolManager;

/// Unified configuration for the edge service
#[derive(Debug, Clone)]
pub struct EdgeServiceConfig {
    // Kafka settings
    pub kafka_broker: String,
    pub input_topics: Vec<String>,
    pub output_topic_delta: String,
    pub output_topic_snapshot: String,
    pub snapshot_interval_ms: u64,
    
    // Redis settings
    pub redis_url: String,
    pub redis_stream_key: String,
    pub redis_start_id: String,
    
    // Service settings
    pub enable_redis_bridge: bool,
    pub enable_depth_enrichment: bool,
    pub max_edge_age_ms: u64,
}

impl EdgeServiceConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            kafka_broker: env::var("KAFKA_BROKER")
                .unwrap_or_else(|_| "localhost:29092".to_string()),
            input_topics: vec![
                env::var("ORCA_POOLS_TOPIC")
                    .unwrap_or_else(|_| "orca-swaps.parsed.v2".to_string()),
                env::var("ORCA_TICKARRAYS_TOPIC")
                    .unwrap_or_else(|_| "orca-tickarrays.parsed.v2".to_string()),
            ],
            output_topic_delta: env::var("EDGE_DELTA_TOPIC")
                .unwrap_or_else(|_| "edge.delta.v1".to_string()),
            output_topic_snapshot: env::var("EDGE_SNAPSHOT_TOPIC")
                .unwrap_or_else(|_| "edge.snapshot.v1".to_string()),
            snapshot_interval_ms: env::var("SNAPSHOT_INTERVAL_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .context("Invalid SNAPSHOT_INTERVAL_MS")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            redis_stream_key: env::var("REDIS_STREAM_KEY")
                .unwrap_or_else(|_| "dex:whirlpool:tickarray_updates".to_string()),
            redis_start_id: env::var("REDIS_START_ID")
                .unwrap_or_else(|_| "$".to_string()),
            enable_redis_bridge: env::var("ENABLE_REDIS_BRIDGE")
                .map(|v| v == "true")
                .unwrap_or(true),
            enable_depth_enrichment: env::var("ENABLE_DEPTH_ENRICHMENT")
                .map(|v| v == "true")
                .unwrap_or(true),
            max_edge_age_ms: env::var("MAX_EDGE_AGE_MS")
                .unwrap_or_else(|_| "10000".to_string())
                .parse()
                .context("Invalid MAX_EDGE_AGE_MS")?,
        })
    }
}

/// Enhanced EdgeAssembler with Redis integration
pub struct EnhancedEdgeAssembler {
    config: EdgeServiceConfig,
    kafka_consumer: StreamConsumer,
    kafka_producer: FutureProducer,
    redis_client: redis::Client,
    pool_manager: Arc<PoolManager>,
    depth_cache: Arc<Mutex<HashMap<String, DepthInfo>>>,
}

#[derive(Debug, Clone)]
struct DepthInfo {
    p0_ui: Option<f64>,
    k_coeff: f64,
    sample_quotes: Option<String>,
    last_updated: u64,
}

impl EnhancedEdgeAssembler {
    pub fn new() -> Result<Self> {
        let config = EdgeServiceConfig::from_env()?;
        
        info!("🚀 Initializing Enhanced Edge Assembler");
        info!("Kafka broker: {}", config.kafka_broker);
        info!("Redis URL: {}", config.redis_url);
        info!("Redis bridge enabled: {}", config.enable_redis_bridge);
        info!("Depth enrichment enabled: {}", config.enable_depth_enrichment);

        // Initialize Kafka consumer
        let kafka_consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "enhanced-edge-assembler")
            .set("bootstrap.servers", &config.kafka_broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set("fetch.wait.max.ms", "1")
            .set("fetch.min.bytes", "1")
            .create()
            .context("Failed to create Kafka consumer")?;

        // Initialize Kafka producer
        let kafka_producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.kafka_broker)
            .set("message.timeout.ms", "1000")
            .set("linger.ms", "0")
            .set("acks", "1")
            .set("compression.type", "lz4")
            .create()
            .context("Failed to create Kafka producer")?;

        // Initialize Redis client
        let redis_client = redis::Client::open(config.redis_url.clone())
            .context("Failed to create Redis client")?;

        let pool_manager = Arc::new(PoolManager::new());
        let depth_cache = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self {
            config,
            kafka_consumer,
            kafka_producer,
            redis_client,
            pool_manager,
            depth_cache,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("🌟 Starting Enhanced Edge Assembler Service");

        // Subscribe to Kafka topics
        let topic_refs: Vec<&str> = self.config.input_topics.iter().map(|s| s.as_str()).collect();
        self.kafka_consumer.subscribe(&topic_refs)
            .context("Failed to subscribe to Kafka topics")?;

        // Test Redis connection
        let mut redis_conn = self.redis_client.get_multiplexed_tokio_connection().await
            .context("Failed to connect to Redis")?;
        let _: String = redis_conn.ping().await
            .context("Redis ping failed")?;
        info!("✅ Redis connection established");

        // Start background tasks
        let tasks = vec![
            self.spawn_kafka_consumer(),
            self.spawn_redis_bridge(),
            self.spawn_snapshot_publisher(),
            self.spawn_depth_enricher(),
            self.spawn_cleanup_task(),
        ];

        // Wait for any task to complete (shouldn't happen in normal operation)
        let (result, _index, _remaining) = futures::future::select_all(tasks).await;
        error!("Background task completed unexpectedly: {:?}", result);
        
        result
    }

    /// Spawn Kafka consumer task
    fn spawn_kafka_consumer(&self) -> tokio::task::JoinHandle<Result<()>> {
        let consumer = self.kafka_consumer.clone();
        let pool_manager = Arc::clone(&self.pool_manager);
        let producer = self.kafka_producer.clone();
        let output_topic = self.config.output_topic_delta.clone();
        
        tokio::spawn(async move {
            loop {
                match consumer.recv().await {
                    Ok(message) => {
                        if let Err(e) = Self::process_kafka_message(&message, &pool_manager, &producer, &output_topic).await {
                            error!("Error processing Kafka message: {}", e);
                        }

                        if let Err(e) = consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async) {
                            error!("Failed to commit Kafka offset: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Kafka consumer error: {}", e);
                        sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        })
    }

    /// Spawn Redis bridge task to consume depth updates
    fn spawn_redis_bridge(&self) -> tokio::task::JoinHandle<Result<()>> {
        if !self.config.enable_redis_bridge {
            return tokio::spawn(async move {
                info!("Redis bridge disabled, task sleeping");
                loop { tokio::time::sleep(Duration::from_secs(3600)).await; }
            });
        }

        let redis_client = self.redis_client.clone();
        let stream_key = self.config.redis_stream_key.clone();
        let start_id = self.config.redis_start_id.clone();
        let depth_cache = Arc::clone(&self.depth_cache);
        let pool_manager = Arc::clone(&self.pool_manager);

        tokio::spawn(async move {
            let mut redis_conn = redis_client.get_multiplexed_tokio_connection().await
                .expect("Redis connection failed in bridge task");
            
            let mut last_id = start_id;
            info!("🌉 Redis bridge started, consuming from stream: {}", stream_key);

            loop {
                match Self::consume_redis_stream(&mut redis_conn, &stream_key, &last_id, &depth_cache, &pool_manager).await {
                    Ok(new_last_id) => {
                        if let Some(id) = new_last_id {
                            last_id = id;
                        }
                    }
                    Err(e) => {
                        error!("Redis bridge error: {}", e);
                        sleep(Duration::from_millis(1000)).await;
                    }
                }
            }
        })
    }

    /// Spawn snapshot publisher task
    fn spawn_snapshot_publisher(&self) -> tokio::task::JoinHandle<Result<()>> {
        let pool_manager = Arc::clone(&self.pool_manager);
        let producer = self.kafka_producer.clone();
        let topic = self.config.output_topic_snapshot.clone();
        let interval_ms = self.config.snapshot_interval_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            info!("📸 Snapshot publisher started (interval: {}ms)", interval_ms);

            loop {
                interval.tick().await;

                let snapshot = pool_manager.create_snapshot().await;
                let encoded = prost::Message::encode_to_vec(&snapshot);
                let record = FutureRecord::to(&topic).key("snapshot").payload(&encoded);

                match producer.send(record, Duration::from_millis(100)).await {
                    Ok(_) => {
                        info!("📤 Published snapshot with {} edges", snapshot.edges.len());
                    }
                    Err(e) => {
                        error!("Failed to publish snapshot: {:?}", e);
                    }
                }
            }
        })
    }

    /// Spawn depth enrichment task
    fn spawn_depth_enricher(&self) -> tokio::task::JoinHandle<Result<()>> {
        if !self.config.enable_depth_enrichment {
            return tokio::spawn(async move {
                info!("Depth enrichment disabled, task sleeping");
                loop { tokio::time::sleep(Duration::from_secs(3600)).await; }
            });
        }

        let depth_cache = Arc::clone(&self.depth_cache);
        let max_age_ms = self.config.max_edge_age_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(5000));
            info!("🔬 Depth enricher started");

            loop {
                interval.tick().await;
                
                let mut cache = depth_cache.lock().await;
                let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)
                    .unwrap().as_millis() as u64;
                
                // Remove stale depth info
                cache.retain(|_pool, depth| {
                    now_ms - depth.last_updated < max_age_ms
                });
                
                if cache.len() > 0 {
                    info!("🔍 Depth cache contains {} pools", cache.len());
                }
            }
        })
    }

    /// Spawn cleanup task for stale pools
    fn spawn_cleanup_task(&self) -> tokio::task::JoinHandle<Result<()>> {
        let pool_manager = Arc::clone(&self.pool_manager);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(30000)); // 30 seconds
            info!("🧹 Cleanup task started");

            loop {
                interval.tick().await;
                let initial_count = pool_manager.pool_count();
                pool_manager.cleanup_stale_pools();
                let final_count = pool_manager.pool_count();
                
                if initial_count != final_count {
                    info!("🧹 Cleaned up {} stale pools ({} -> {})", 
                          initial_count - final_count, initial_count, final_count);
                }
            }
        })
    }

    /// Process a single Kafka message
    async fn process_kafka_message(
        message: &rdkafka::message::BorrowedMessage<'_>,
        pool_manager: &Arc<PoolManager>,
        producer: &FutureProducer,
        output_topic: &str,
    ) -> Result<()> {
        let topic = message.topic();
        let payload = match message.payload() {
            Some(p) => p,
            None => return Ok(()),
        };

        let data: serde_json::Value = serde_json::from_slice(payload)
            .context("Failed to parse JSON payload")?;

        match topic {
            t if t.contains("orca") => {
                Self::handle_orca_update(&data, pool_manager, producer, output_topic).await?;
            }
            _ => {
                warn!("Unknown topic: {}", topic);
            }
        }

        Ok(())
    }

    /// Handle Orca-specific updates
    async fn handle_orca_update(
        data: &serde_json::Value,
        pool_manager: &Arc<PoolManager>,
        producer: &FutureProducer,
        output_topic: &str,
    ) -> Result<()> {
        if let Some(account_type) = data.get("account_type").and_then(|v| v.as_str()) {
            match account_type {
                "whirlpool" => {
                    // Handle pool updates (simplified - you'd use your actual pool parsing logic)
                    if let Some(pool_pubkey) = data.get("pubkey").and_then(|v| v.as_str()) {
                        let slot = data.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
                        let write_version = data.get("write_version").and_then(|v| v.as_u64()).unwrap_or(0);
                        
                        // This is where you'd parse the actual pool data and update
                        // For now, we'll just emit a placeholder edge
                        info!("📊 Pool update: {} (slot: {}, wv: {})", pool_pubkey, slot, write_version);
                        
                        // Generate and emit edges would go here
                        // let edges = pool_manager.get_current_edges(pool_pubkey).await;
                        // Self::emit_edges(edges, producer, output_topic).await?;
                    }
                }
                "tick_array" => {
                    // Handle tick array updates using your tick array parser
                    info!("📈 Tick array update received");
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Consume from Redis stream and update depth cache
    async fn consume_redis_stream(
        redis_conn: &mut redis::aio::MultiplexedConnection,
        stream_key: &str,
        last_id: &str,
        depth_cache: &Arc<Mutex<HashMap<String, DepthInfo>>>,
        _pool_manager: &Arc<PoolManager>,
    ) -> Result<Option<String>> {
        let resp: Value = redis::cmd("XREAD")
            .arg("BLOCK")
            .arg(5000) // 5 second timeout
            .arg("STREAMS")
            .arg(stream_key)
            .arg(last_id)
            .query_async(redis_conn)
            .await?;

        let mut new_last_id = None;

        if let Value::Array(streams) = resp {
            if let Some(Value::Array(mut top)) = streams.into_iter().next() {
                if top.len() >= 2 {
                    if let Value::Array(items) = top.remove(1) {
                        for item in items {
                            if let Value::Array(mut pair) = item {
                                if pair.len() == 2 {
                                    let id = match &pair[0] {
                                        Value::BulkString(b) => String::from_utf8_lossy(b).to_string(),
                                        _ => continue,
                                    };
                                    new_last_id = Some(id);

                                    // Parse the depth update
                                    if let Some(depth_info) = Self::parse_depth_update(&pair[1]) {
                                        let mut cache = depth_cache.lock().await;
                                        cache.insert(depth_info.0, depth_info.1);
                                        info!("📊 Updated depth for pool: {}", depth_info.0);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(new_last_id)
    }

    /// Parse depth update from Redis stream entry
    fn parse_depth_update(stream_data: &Value) -> Option<(String, DepthInfo)> {
        if let Value::Array(kvs) = stream_data {
            let mut pool = String::new();
            let mut p0_ui = None;
            let mut k_coeff = 0.0;
            let mut sample_quotes = None;

            let mut it = kvs.iter();
            while let (Some(k), Some(v)) = (it.next(), it.next()) {
                let key = match k {
                    Value::BulkString(b) => String::from_utf8_lossy(b).to_string(),
                    _ => continue,
                };
                
                match key.as_str() {
                    "pool" => {
                        if let Value::BulkString(b) = v {
                            pool = String::from_utf8_lossy(b).to_string();
                        }
                    }
                    "p0_ui" => {
                        if let Value::BulkString(b) = v {
                            p0_ui = String::from_utf8_lossy(b).parse().ok();
                        }
                    }
                    "k_coeff" => {
                        if let Value::BulkString(b) = v {
                            k_coeff = String::from_utf8_lossy(b).parse().unwrap_or(0.0);
                        }
                    }
                    "sample_quotes" => {
                        if let Value::BulkString(b) = v {
                            sample_quotes = Some(String::from_utf8_lossy(b).to_string());
                        }
                    }
                    _ => {}
                }
            }

            if !pool.is_empty() {
                let depth_info = DepthInfo {
                    p0_ui,
                    k_coeff,
                    sample_quotes,
                    last_updated: SystemTime::now().duration_since(UNIX_EPOCH)
                        .unwrap().as_millis() as u64,
                };
                return Some((pool, depth_info));
            }
        }
        None
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🚀 Starting Enhanced Edge Assembler Service");

    let assembler = EnhancedEdgeAssembler::new()
        .context("Failed to initialize Enhanced Edge Assembler")?;
    
    assembler.start().await?;
    
    Ok(())
}
