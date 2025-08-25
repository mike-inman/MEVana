use anyhow::Result;
use log::{error, info, warn};
use prost::Message;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message as KafkaMessage;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

mod edge;
mod graph;
mod opportunity;
mod risk;
mod venue;

use edge::EdgeDelta;
use graph::EdgeGraph;
use opportunity::{ArbitrageOpportunity, OpportunityDetector};
use risk::RiskManager;
use venue::VenueId;

pub struct Searcher {
    consumer: StreamConsumer,
    producer: FutureProducer,
    edge_graph: Arc<EdgeGraph>,
    detector: OpportunityDetector,
    risk_manager: RiskManager,
    input_topic: String,
    output_topic: String,
    detection_interval_ms: u64,
    max_edge_age_ms: u64,
    min_profit_bps: u64,
    last_detection: AtomicU64,
}

impl Searcher {
    pub fn new() -> Result<Self> {
        let kafka_broker =
            env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());

        let input_topic =
            env::var("EDGE_DELTA_TOPIC").unwrap_or_else(|_| "edge.delta.v1".to_string());
        let output_topic = env::var("ARBITRAGE_OPPORTUNITIES_TOPIC")
            .unwrap_or_else(|_| "arb.opportunities.v1".to_string());

        let detection_interval_ms = env::var("DETECTION_INTERVAL_MS")
            .unwrap_or_else(|_| "1".to_string()) // 1ms detection loop
            .parse()
            .unwrap_or(1);

        let max_edge_age_ms = env::var("MAX_EDGE_AGE_MS")
            .unwrap_or_else(|_| "100".to_string()) // 100ms max staleness
            .parse()
            .unwrap_or(100);

        let min_profit_bps = env::var("MIN_PROFIT_BPS")
            .unwrap_or_else(|_| "10".to_string()) // 0.1% minimum profit
            .parse()
            .unwrap_or(10);

        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "searcher")
            .set("bootstrap.servers", &kafka_broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set("fetch.wait.max.ms", "0") // No waiting - ultra low latency
            .set("fetch.min.bytes", "1")
            .set("session.timeout.ms", "30000")
            .set("heartbeat.interval.ms", "10000")
            .create()?;

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_broker)
            .set("message.timeout.ms", "500")
            .set("linger.ms", "0") // No batching for latency
            .set("acks", "1")
            .set("compression.type", "none") // No compression for speed
            .create()?;

        let edge_graph = Arc::new(EdgeGraph::new());
        let detector = OpportunityDetector::new(min_profit_bps);
        let risk_manager = RiskManager::new();

        Ok(Self {
            consumer,
            producer,
            edge_graph,
            detector,
            risk_manager,
            input_topic,
            output_topic,
            detection_interval_ms,
            max_edge_age_ms,
            min_profit_bps,
            last_detection: AtomicU64::new(0),
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting Searcher");
        info!("Input topic: {}", self.input_topic);
        info!("Output topic: {}", self.output_topic);
        info!("Detection interval: {}ms", self.detection_interval_ms);
        info!("Max edge age: {}ms", self.max_edge_age_ms);
        info!("Min profit: {} bps", self.min_profit_bps);

        // Subscribe to EdgeDelta topic
        self.consumer.subscribe(&[&self.input_topic])?;

        // Spawn detection loop
        let edge_graph_clone = Arc::clone(&self.edge_graph);
        let detector_clone = self.detector.clone();
        let risk_manager_clone = self.risk_manager.clone();
        let producer_clone = self.producer.clone();
        let output_topic = self.output_topic.clone();
        let detection_interval = self.detection_interval_ms;
        let max_edge_age = self.max_edge_age_ms;
        let last_detection_clone = Arc::new(AtomicU64::new(0));

        tokio::spawn(async move {
            Self::detection_loop(
                edge_graph_clone,
                detector_clone,
                risk_manager_clone,
                producer_clone,
                output_topic,
                detection_interval,
                max_edge_age,
                last_detection_clone,
            )
            .await;
        });

        // Main EdgeDelta consumption loop
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    if let Err(e) = self.process_edge_delta(&message).await {
                        error!("Error processing EdgeDelta: {}", e);
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
                    sleep(Duration::from_millis(1)).await;
                }
            }
        }
    }

    async fn process_edge_delta(&self, message: &BorrowedMessage<'_>) -> Result<()> {
        let payload = match message.payload() {
            Some(p) => p,
            None => return Ok(()),
        };

        // Decode EdgeDelta from protobuf
        let edge_delta = edge::EdgeDelta::decode(payload)
            .map_err(|e| anyhow::anyhow!("Failed to decode EdgeDelta: {}", e))?;

        // Update edge graph with new edge
        self.edge_graph.update_edge(edge_delta).await;

        Ok(())
    }

    async fn detection_loop(
        edge_graph: Arc<EdgeGraph>,
        detector: OpportunityDetector,
        risk_manager: RiskManager,
        producer: FutureProducer,
        output_topic: String,
        interval_ms: u64,
        max_edge_age_ms: u64,
        last_detection: Arc<AtomicU64>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));

        loop {
            interval.tick().await;
            let start_time = Instant::now();

            // Clean stale edges
            edge_graph.cleanup_stale_edges(max_edge_age_ms).await;

            // Detect arbitrage opportunities
            let opportunities = detector.detect_opportunities(&edge_graph).await;

            // Apply risk management
            let filtered_opportunities = risk_manager.filter_opportunities(opportunities);

            // Emit profitable opportunities
            for opportunity in filtered_opportunities {
                if let Err(e) = Self::emit_opportunity(&producer, &output_topic, &opportunity).await
                {
                    error!("Failed to emit opportunity: {}", e);
                } else {
                    info!(
                        "Arbitrage opportunity: {} -> {} via {} edges, profit: {} bps, size: {}",
                        bs58::encode(opportunity.token_in).into_string(),
                        bs58::encode(opportunity.token_out).into_string(),
                        opportunity.edges.len(),
                        opportunity.profit_bps,
                        opportunity.max_size_in
                    );
                }
            }

            let detection_time = start_time.elapsed();
            last_detection.store(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                Ordering::Release,
            );

            // Log performance metrics
            if detection_time.as_millis() > 5 {
                warn!(
                    "Detection loop took {}ms (target: {}ms)",
                    detection_time.as_millis(),
                    interval_ms
                );
            }
        }
    }

    async fn emit_opportunity(
        producer: &FutureProducer,
        topic: &str,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<()> {
        let opportunity_json = serde_json::to_string(opportunity)?;
        let opportunity_id = opportunity.opportunity_id.to_string();

        let record = FutureRecord::to(topic)
            .key(&opportunity_id)
            .payload(&opportunity_json);

        producer
            .send(record, Duration::from_millis(10))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send opportunity: {:?}", e))?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Searcher Service");

    let searcher = Searcher::new()?;
    searcher.start().await?;

    Ok(())
}
