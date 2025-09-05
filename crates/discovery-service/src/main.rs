// Fixed version of discovery-service main.rs
// This resolves the borrow checker issues in the process_unknown_program method

use anyhow::Result;
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use log::{error, info};
use prometheus::Encoder; // encode trait
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message; // for payload_view
use rdkafka::producer::{FutureProducer, FutureRecord};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

// AI-ready data structures for program discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnknownProgram {
    program_id: String,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    encounter_count: u32,
    account_types: Vec<String>,
    data_patterns: Vec<DataPattern>,
    potential_dex: bool,
    confidence_score: f64,
    ai_analysis_status: AIAnalysisStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataPattern {
    discriminator: String,
    data_length: usize,
    byte_patterns: Vec<u8>,
    frequency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum AIAnalysisStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveryMetrics {
    programs_discovered: u64,
    programs_analyzed: u64,
    ai_analysis_time_ms: f64,
}

struct DiscoveryService {
    kafka_consumer: StreamConsumer,
    kafka_producer: FutureProducer,
    redis_client: redis::Client,
    unknown_programs: HashMap<String, UnknownProgram>,
    metrics: DiscoveryMetrics,
    // Prometheus metrics
    m_prog_discovered: prometheus::IntCounter,
    m_prog_analyzed: prometheus::IntCounter,
    m_ai_latency_ms: prometheus::Histogram,
    m_queue_candidates: prometheus::IntCounter,
}

impl DiscoveryService {
    async fn new() -> Result<Self> {
        dotenv().ok();

        // Kafka consumer for unknown program IDs
        let consumer: StreamConsumer = rdkafka::ClientConfig::new()
            .set("group.id", "discovery-service")
            .set(
                "bootstrap.servers",
                std::env::var("KAFKA_BOOTSTRAP_SERVERS")
                    .unwrap_or_else(|_| "localhost:9092".to_string()),
            )
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .create()?;

        // Kafka producer for AI analysis queue
        let producer: FutureProducer = rdkafka::ClientConfig::new()
            .set(
                "bootstrap.servers",
                std::env::var("KAFKA_BOOTSTRAP_SERVERS")
                    .unwrap_or_else(|_| "localhost:9092".to_string()),
            )
            .set("message.timeout.ms", "5000")
            .create()?;

        // Redis client for caching and metrics
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379/0".to_string());
        let redis_client = redis::Client::open(redis_url)?;

        // Register Prometheus metrics
        let registry = prometheus::default_registry();
        let m_prog_discovered = prometheus::IntCounter::new(
            "discovery_programs_discovered_total",
            "Total unknown programs observed",
        )
        .unwrap();
        let m_prog_analyzed = prometheus::IntCounter::new(
            "discovery_programs_queued_total",
            "Total programs queued for AI analysis",
        )
        .unwrap();
        let m_ai_latency_ms = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "discovery_ai_analysis_time_ms",
                "Last analysis processing time (ms)",
            )
            .buckets(vec![
                1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
            ]),
        )
        .unwrap();
        let m_queue_candidates = prometheus::IntCounter::new(
            "discovery_queue_candidates_total",
            "Programs meeting queue criteria (may be queued once)",
        )
        .unwrap();
        let _ = registry.register(Box::new(m_prog_discovered.clone()));
        let _ = registry.register(Box::new(m_prog_analyzed.clone()));
        let _ = registry.register(Box::new(m_ai_latency_ms.clone()));
        let _ = registry.register(Box::new(m_queue_candidates.clone()));

        Ok(DiscoveryService {
            kafka_consumer: consumer,
            kafka_producer: producer,
            redis_client,
            unknown_programs: HashMap::new(),
            metrics: DiscoveryMetrics {
                programs_discovered: 0,
                programs_analyzed: 0,
                ai_analysis_time_ms: 0.0,
            },
            m_prog_discovered,
            m_prog_analyzed,
            m_ai_latency_ms,
            m_queue_candidates,
        })
    }

    async fn run(&mut self) -> Result<()> {
        info!("discovery-service: starting program discovery and AI preparation");

        // Subscribe to unknown program ID topic
        self.kafka_consumer
            .subscribe(&[&std::env::var("UNKNOWN_PROGRAMS_TOPIC")
                .unwrap_or_else(|_| "unknown-programs".to_string())])?;

        loop {
            let msg_res = self.kafka_consumer.recv().await; // end immutable borrow before processing mutably
            match msg_res {
                Ok(msg) => {
                    if let Some(bytes) = msg.payload() {
                        if let Ok(s) = std::str::from_utf8(bytes) {
                            let payload_owned = s.to_owned(); // owned copy breaks borrow chain
                            if let Err(e) = self.process_unknown_program(&payload_owned).await {
                                error!("Error processing unknown program: {}", e);
                            }
                        } else {
                            error!("Invalid UTF-8 payload");
                        }
                    }
                }
                Err(e) => error!("Error receiving message: {}", e),
            }

            // Report metrics every 100 programs
            if self.metrics.programs_discovered % 100 == 0 {
                self.report_metrics().await?;
            }
        }
    }

    async fn process_unknown_program(&mut self, payload: &str) -> Result<()> {
        let start = Instant::now();

        // Parse the unknown program data
        let program_data: serde_json::Value = serde_json::from_str(payload)?;

        // Extract program ID and analyze patterns
        if let Some(program_id) = program_data.get("program_id").and_then(|v| v.as_str()) {
            let program_id = program_id.to_string();

            // Step 1: Get or create the program entry and update basic fields
            {
                let entry = self
                    .unknown_programs
                    .entry(program_id.clone())
                    .or_insert_with(|| UnknownProgram {
                        program_id: program_id.clone(),
                        first_seen: Utc::now(),
                        last_seen: Utc::now(),
                        encounter_count: 0,
                        account_types: Vec::new(),
                        data_patterns: Vec::new(),
                        potential_dex: false,
                        confidence_score: 0.0,
                        ai_analysis_status: AIAnalysisStatus::Pending,
                    });

                entry.last_seen = Utc::now();
                entry.encounter_count += 1;
                if entry.encounter_count == 1 {
                    // first time seen
                    self.metrics.programs_discovered += 1;
                    self.m_prog_discovered.inc();
                }
            }

            // Step 2: Analyze data patterns for AI learning
            // Analyze patterns (separate helper without borrowing self mutably)
            if let Some(entry) = self.unknown_programs.get_mut(&program_id) {
                analyze_data_patterns(entry, &program_data)?;
            }

            // Step 3: Check if should queue for AI analysis
            let should_queue = if let Some(entry) = self.unknown_programs.get(&program_id) {
                self.should_queue_for_ai_analysis(entry)
            } else {
                false
            };
            if should_queue {
                self.m_queue_candidates.inc();
            }

            // Step 4: Queue for AI analysis if meets criteria
            if should_queue {
                self.queue_for_ai_analysis(&program_id).await?;
            }

            // Step 5: Cache in Redis for persistence
            if let Some(entry) = self.unknown_programs.get(&program_id) {
                self.cache_program_data(entry).await?;
            }
        }

        let analysis_time = start.elapsed();
        let ms = analysis_time.as_millis() as f64;
        self.metrics.ai_analysis_time_ms = ms;
        self.m_ai_latency_ms.observe(ms);

        Ok(())
    }

    // queue method refactored to avoid simultaneous &mut self and &mut entry borrows
    async fn queue_for_ai_analysis(&mut self, program_id: &str) -> Result<()> {
        if let Some(program) = self.unknown_programs.get_mut(program_id) {
            program.ai_analysis_status = AIAnalysisStatus::InProgress;
            let ai_payload = serde_json::to_string(&program)?;
            let record = FutureRecord::to("ai-analysis-queue")
                .payload(ai_payload.as_bytes())
                .key(&program.program_id);
            match self
                .kafka_producer
                .send(record, std::time::Duration::from_secs(5))
                .await
            {
                Ok(_) => {
                    info!("Queued program {} for AI analysis", program.program_id);
                    self.metrics.programs_analyzed += 1;
                    self.m_prog_analyzed.inc();
                }
                Err((e, _)) => {
                    error!("Failed to queue program for AI analysis: {}", e);
                    program.ai_analysis_status = AIAnalysisStatus::Failed;
                }
            }
        }
        Ok(())
    }
}

// Standalone helper (no self borrow) to avoid borrow conflicts
fn analyze_data_patterns(program: &mut UnknownProgram, data: &serde_json::Value) -> Result<()> {
    // Extract account type discriminators
    if let Some(account_data) = data.get("account_data") {
        if let Some(discriminator) = account_data.get("discriminator").and_then(|v| v.as_str()) {
            if !program.account_types.contains(&discriminator.to_string()) {
                program.account_types.push(discriminator.to_string());
            }
        }
    }

    // Analyze byte patterns for AI learning
    if let Some(raw_data) = data.get("raw_data").and_then(|v| v.as_str()) {
        if let Ok(bytes) = bs58::decode(raw_data).into_vec() {
            let pattern = DataPattern {
                discriminator: data
                    .get("discriminator")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                data_length: bytes.len(),
                byte_patterns: bytes.iter().take(16).cloned().collect(), // First 16 bytes for pattern recognition
                frequency: 1,
            };

            // Update existing pattern or add new one
            if let Some(existing) = program
                .data_patterns
                .iter_mut()
                .find(|p| p.discriminator == pattern.discriminator)
            {
                existing.frequency += 1;
            } else {
                program.data_patterns.push(pattern);
            }
        }
    }
    Ok(())
}

impl DiscoveryService {
    fn should_queue_for_ai_analysis(&self, program: &UnknownProgram) -> bool {
        program.encounter_count >= 5
            && program.account_types.len() >= 2
            && program.data_patterns.iter().any(|p| p.data_length > 100)
    }

    async fn cache_program_data(&self, program: &UnknownProgram) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_tokio_connection().await?;
        let key = format!("discovery:program:{}", program.program_id);

        let program_json = serde_json::to_string(program)?;
        conn.set_ex::<_, _, ()>(&key, program_json, 86400).await?; // Cache for 24 hours

        Ok(())
    }

    async fn report_metrics(&self) -> Result<()> {
        info!("DISCOVERY METRICS: {:?}", self.metrics);

        // Cache metrics in Redis
        let mut conn = self.redis_client.get_multiplexed_tokio_connection().await?;
        let metrics_json = serde_json::to_string(&self.metrics)?;
        conn.set_ex::<_, _, ()>("discovery:metrics", metrics_json, 3600)
            .await?; // Cache for 1 hour

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    info!("🚀 Starting Discovery Service");
    info!("🔍 AI-ready program discovery and analysis");

    let mut service = DiscoveryService::new().await?;

    // Spawn metrics HTTP endpoint (Prometheus + health)
    tokio::spawn(async move {
        use warp::Filter;
        let metrics_route = warp::path("metrics").and(warp::get()).map(|| {
            let encoder = prometheus::TextEncoder::new();
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
        let port: u16 = std::env::var("DISCOVERY_METRICS_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(9102);
        let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        log::info!("discovery-service metrics listening on {}", addr);
        warp::serve(routes).run(addr).await;
    });

    service.run().await?;

    Ok(())
}
