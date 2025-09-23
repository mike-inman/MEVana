use anyhow::Result;
use log::{error, info, warn};
use prometheus::{Counter, Encoder, TextEncoder};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{Message, TopicPartitionList};
use serde_json::Value;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use whirlpool_core::quoter::{ClmmQuoter, Direction};
use whirlpool_core::types::{TickArray, WhirlpoolMinimal};

mod edge;
mod pool_manager;
mod venue_handlers;

use edge::{ClmmState, EdgeDelta};
use pool_manager::PoolManager;
use venue_handlers::VenueId;

// Metrics
lazy_static::lazy_static! {
    static ref EDGES_PROCESSED: Counter = Counter::new(
        "edge_assembler_edges_processed",
        "Total edges processed by edge assembler"
    ).unwrap();
    static ref POOLS_UPDATED: Counter = Counter::new(
        "edge_assembler_pools_updated",
        "Total pool updates processed"
    ).unwrap();
}

pub struct EdgeAssembler {
    consumer: StreamConsumer,
    producer: FutureProducer,
    pool_manager: Arc<PoolManager>,
    input_topics: Vec<String>,
    output_topic_delta: String,
    output_topic_snapshot: String,
    snapshot_interval_ms: u64,
}

impl EdgeAssembler {
    pub fn new() -> Result<Self> {
        let kafka_broker =
            env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());

        // Input topics from various normalizer services
        let input_topics = vec![
            env::var("ORCA_POOLS_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed.v2".to_string()),
            env::var("ORCA_TICKARRAYS_TOPIC")
                .unwrap_or_else(|_| "orca-tickarrays.parsed.v2".to_string()),
            // Add more venue topics as they come online
        ];

        let output_topic_delta =
            env::var("EDGE_DELTA_TOPIC").unwrap_or_else(|_| "edge.delta.v1".to_string());
        let output_topic_snapshot =
            env::var("EDGE_SNAPSHOT_TOPIC").unwrap_or_else(|_| "edge.snapshot.v1".to_string());

        let snapshot_interval_ms = env::var("SNAPSHOT_INTERVAL_MS")
            .unwrap_or_else(|_| "5000".to_string())
            .parse()
            .unwrap_or(5000);

        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "edge-assembler")
            .set("bootstrap.servers", &kafka_broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set("fetch.wait.max.ms", "1")
            .set("fetch.min.bytes", "1")
            .create()?;

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_broker)
            .set("message.timeout.ms", "1000")
            .set("linger.ms", "0")
            .set("acks", "1")
            .set("compression.type", "lz4")
            .create()?;

        let pool_manager = Arc::new(PoolManager::new());

        Ok(Self {
            consumer,
            producer,
            pool_manager,
            input_topics,
            output_topic_delta,
            output_topic_snapshot,
            snapshot_interval_ms,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting Edge Assembler");
        info!("Input topics: {:?}", self.input_topics);
        info!("Output delta topic: {}", self.output_topic_delta);
        info!("Output snapshot topic: {}", self.output_topic_snapshot);

        // Register metrics
        let registry = prometheus::default_registry();
        registry
            .register(Box::new(EDGES_PROCESSED.clone()))
            .unwrap();
        registry.register(Box::new(POOLS_UPDATED.clone())).unwrap();

        // Start metrics server
        tokio::spawn(async move {
            use warp::Filter;
            let metrics_route = warp::path("metrics").and(warp::get()).map(|| {
                let encoder = TextEncoder::new();
                let metric_families = prometheus::gather();
                let mut buffer = Vec::new();
                if encoder.encode(&metric_families, &mut buffer).is_ok() {
                    warp::reply::with_status(
                        String::from_utf8_lossy(&buffer).to_string(),
                        warp::http::StatusCode::OK,
                    )
                } else {
                    warp::reply::with_status(
                        "encode_error".into(),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            });
            let health = warp::path("health").and(warp::get()).map(|| "OK");
            let routes = metrics_route.or(health);
            let port: u16 = std::env::var("METRICS_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(9902);
            let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
            info!("edge-assembler metrics listening on {}", addr);
            warp::serve(routes).run(addr).await;
        });

        // Subscribe to input topics
        let mut topic_partition_list = TopicPartitionList::new();
        for topic in &self.input_topics {
            topic_partition_list.add_partition(topic, 0);
        }
        let topic_refs: Vec<&str> = self.input_topics.iter().map(|s| s.as_str()).collect();
        self.consumer.subscribe(&topic_refs)?;

        // Spawn snapshot publisher
        let pool_manager_clone = Arc::clone(&self.pool_manager);
        let producer_clone = self.producer.clone();
        let snapshot_topic = self.output_topic_snapshot.clone();
        let snapshot_interval = self.snapshot_interval_ms;

        tokio::spawn(async move {
            Self::snapshot_publisher(
                pool_manager_clone,
                producer_clone,
                snapshot_topic,
                snapshot_interval,
            )
            .await;
        });

        // Main message processing loop
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Err(e) = self.process_message(&message).await {
                        error!("Error processing message: {}", e);
                    }

                    // Commit offset after processing
                    if let Err(e) = self
                        .consumer
                        .commit_message(&message, rdkafka::consumer::CommitMode::Async)
                    {
                        error!("Failed to commit offset: {}", e);
                    }
                }
                Err(e) => {
                    error!("Kafka error: {}", e);
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn process_message(&self, message: &BorrowedMessage<'_>) -> Result<()> {
        let topic = message.topic();
        let payload = match message.payload() {
            Some(p) => p,
            None => return Ok(()),
        };

        let data: Value = serde_json::from_slice(payload)?;

        // Route to appropriate venue handler based on topic
        match topic {
            t if t.contains("orca") => {
                self.handle_orca_update(&data).await?;
            }
            t if t.contains("raydium") => {
                self.handle_raydium_update(&data).await?;
            }
            t if t.contains("meteora") => {
                self.handle_meteora_update(&data).await?;
            }
            _ => {
                warn!("Unknown topic: {}", topic);
            }
        }

        Ok(())
    }

    async fn handle_orca_update(&self, data: &Value) -> Result<()> {
        // Determine if this is a pool update or tickarray update
        if let Some(account_type) = data.get("account_type").and_then(|v| v.as_str()) {
            match account_type {
                "whirlpool" => {
                    if let Ok(pool) =
                        serde_json::from_value::<WhirlpoolMinimal>(data["account"].clone())
                    {
                        let pool_pubkey = data
                            .get("pubkey")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| anyhow::anyhow!("Missing pubkey"))?;

                        let slot = data.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
                        let write_version = data
                            .get("write_version")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);

                        self.update_pool_and_emit_edges(pool_pubkey, &pool, slot, write_version)
                            .await?;
                    }
                }
                "tick_array" => {
                    // For now, skip TickArray deserialization since it needs custom handling
                    // TODO: Implement proper TickArray parsing from raw account data
                    if let Some(_account_data) = data.get("account") {
                        info!("Received tick_array update - parsing not yet implemented");
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_raydium_update(&self, _data: &Value) -> Result<()> {
        // TODO: Implement Raydium CPMM handling
        Ok(())
    }

    async fn handle_meteora_update(&self, _data: &Value) -> Result<()> {
        // TODO: Implement Meteora DLMM handling
        Ok(())
    }

    async fn update_pool_and_emit_edges(
        &self,
        pool_pubkey: &str,
        pool: &WhirlpoolMinimal,
        slot: u64,
        write_version: u64,
    ) -> Result<()> {
        POOLS_UPDATED.inc();
        // Update pool in manager
        self.pool_manager
            .update_pool(pool_pubkey, pool.clone(), slot, write_version);

        // Generate EdgeDeltas for both directions
        let edge_a_to_b =
            self.create_clmm_edge_delta(pool_pubkey, pool, Direction::AtoB, slot, write_version)?;

        let edge_b_to_a =
            self.create_clmm_edge_delta(pool_pubkey, pool, Direction::BtoA, slot, write_version)?;

        // Emit both edges
        self.emit_edge_delta(&edge_a_to_b).await?;
        self.emit_edge_delta(&edge_b_to_a).await?;
        EDGES_PROCESSED.inc_by(2.0);

        Ok(())
    }

    #[allow(dead_code)]
    async fn update_tick_array_and_emit_edges(
        &self,
        _ta_pubkey: &str,
        tick_array: &TickArray,
        slot: u64,
        write_version: u64,
    ) -> Result<()> {
        // Update tick array in manager
        let pool_pubkey_str = bs58::encode(tick_array.whirlpool).into_string();
        self.pool_manager.update_tick_array(
            &pool_pubkey_str,
            tick_array.clone(),
            slot,
            write_version,
        );

        // Re-emit edges for this pool since liquidity distribution may have changed
        if let Some(pool_state) = self.pool_manager.get_pool(&pool_pubkey_str) {
            let edge_a_to_b = self.create_clmm_edge_delta(
                &pool_pubkey_str,
                &pool_state.pool,
                Direction::AtoB,
                slot,
                write_version,
            )?;

            let edge_b_to_a = self.create_clmm_edge_delta(
                &pool_pubkey_str,
                &pool_state.pool,
                Direction::BtoA,
                slot,
                write_version,
            )?;

            self.emit_edge_delta(&edge_a_to_b).await?;
            self.emit_edge_delta(&edge_b_to_a).await?;
        }

        Ok(())
    }

    fn create_clmm_edge_delta(
        &self,
        pool_pubkey: &str,
        pool: &WhirlpoolMinimal,
        direction: Direction,
        slot: u64,
        write_version: u64,
    ) -> Result<EdgeDelta> {
        // Get pool state for quoter
        let pool_state = self
            .pool_manager
            .get_pool(pool_pubkey)
            .ok_or_else(|| anyhow::anyhow!("Pool not found in manager"))?;

        // Create quoter with current state
        let quoter = ClmmQuoter {
            sqrt_price_x64: pool.sqrt_price,
            liquidity: pool.liquidity,
            tick_current: pool.tick_current_index,
            tick_spacing: pool.tick_spacing as i32,
            total_fee_bps: pool.fee_rate + pool.protocol_fee_rate,
            window: pool_state.tick_window.clone(),
        };

        // Calculate size to next boundary
        let size_to_next_in = quoter.size_to_next_tick_in(direction).unwrap_or(0);
        let marginal_price = quoter.marginal_price_q64(direction);

        // Generate unique edge ID
        let edge_id = self.generate_edge_id(pool_pubkey, direction)?;

        // Get token addresses
        let (token_in, token_out) = match direction {
            Direction::AtoB => (
                pool.token_mint_a.to_bytes().to_vec(),
                pool.token_mint_b.to_bytes().to_vec(),
            ),
            Direction::BtoA => (
                pool.token_mint_b.to_bytes().to_vec(),
                pool.token_mint_a.to_bytes().to_vec(),
            ),
        };

        // Create CLMM state
        let clmm_state = ClmmState {
            sqrt_price_x64: pool.sqrt_price.to_le_bytes().to_vec(),
            liquidity: pool.liquidity.to_le_bytes().to_vec(),
            tick_current: pool.tick_current_index,
            tick_spacing: pool.tick_spacing as i32,
            size_to_next_in: size_to_next_in.to_le_bytes().to_vec(),
        };

        let edge_delta = EdgeDelta {
            edge_id,
            venue_id: VenueId::OrcaWhirlpool as u32,
            pool_pubkey: bs58::decode(pool_pubkey)
                .into_vec()
                .map_err(|e| anyhow::anyhow!("Invalid base58: {}", e))?,
            token_in,
            token_out,
            direction: direction as u32,
            decimals_in: 9,  // TODO: Get from token registry
            decimals_out: 6, // TODO: Get from token registry
            fee_bps_total: (pool.fee_rate + pool.protocol_fee_rate) as u32,
            state: Some(edge::edge_delta::State::Clmm(clmm_state)),
            slot,
            write_version,
            updated_at_ns: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
            marginal_price_q64: marginal_price.to_le_bytes().to_vec(),
            size_to_next_boundary_in: size_to_next_in.to_le_bytes().to_vec(),
            cu_estimate: 50_000, // Rough estimate for CLMM swap
        };

        Ok(edge_delta)
    }

    fn generate_edge_id(&self, pool_pubkey: &str, direction: Direction) -> Result<u64> {
        // Simple hash-based edge ID: hash(venue_id + pool_pubkey + direction)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        VenueId::OrcaWhirlpool.hash(&mut hasher);
        pool_pubkey.hash(&mut hasher);
        (direction as u32).hash(&mut hasher);
        Ok(hasher.finish())
    }

    async fn emit_edge_delta(&self, edge_delta: &EdgeDelta) -> Result<()> {
        let edge_id = edge_delta.edge_id.to_string();
        let encoded = prost::Message::encode_to_vec(edge_delta);

        let record = FutureRecord::to(&self.output_topic_delta)
            .key(&edge_id)
            .payload(&encoded);

        if let Err(e) = self.producer.send(record, Duration::from_millis(100)).await {
            error!("Failed to send EdgeDelta: {:?}", e);
        } else {
            info!(
                "Emitted EdgeDelta: {} (venue={}, direction={})",
                edge_id, edge_delta.venue_id, edge_delta.direction
            );
        }

        Ok(())
    }

    async fn snapshot_publisher(
        pool_manager: Arc<PoolManager>,
        producer: FutureProducer,
        topic: String,
        interval_ms: u64,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));

        loop {
            interval.tick().await;

            // Create snapshot of all current edges
            let snapshot = pool_manager.create_snapshot();
            let encoded = prost::Message::encode_to_vec(&snapshot);

            let record = FutureRecord::to(&topic).key("snapshot").payload(&encoded);

            if let Err(e) = producer.send(record, Duration::from_millis(100)).await {
                error!("Failed to send EdgeSnapshot: {:?}", e);
            } else {
                info!("Published EdgeSnapshot with {} edges", snapshot.edges.len());
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Edge Assembler Service");

    let assembler = EdgeAssembler::new()?;
    assembler.start().await?;

    Ok(())
}
