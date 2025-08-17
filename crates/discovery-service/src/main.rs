use anyhow::Result;
use dotenv::dotenv;
use env_logger;
use log::{info, error};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{ClientConfig, Message as _};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use chrono::{DateTime, Utc};

// AI-ready data structures for program discovery
#[derive(Debug, Serialize, Deserialize)]
struct UnknownProgram {
    program_id: String,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    encounter_count: u64,
    account_types: Vec<String>,
    data_patterns: Vec<DataPattern>,
    potential_dex: bool,
    confidence_score: f64,
    ai_analysis_status: AIAnalysisStatus,
}

#[derive(Debug, Serialize, Deserialize)]
struct DataPattern {
    discriminator: String,
    data_length: usize,
    byte_patterns: Vec<u8>,
    frequency: u64,
}

#[derive(Debug, Serialize, Deserialize)]
enum AIAnalysisStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryMetrics {
    total_unknown_programs: u64,
    programs_analyzed: u64,
    potential_dexes_found: u64,
    ai_analysis_time_ms: f64,
}

struct DiscoveryService {
    kafka_consumer: StreamConsumer,
    kafka_producer: FutureProducer,
    redis_client: redis::Client,
    unknown_programs: HashMap<String, UnknownProgram>,
    metrics: DiscoveryMetrics,
}

impl DiscoveryService {
    async fn new() -> Result<Self> {
        dotenv().ok();
        env_logger::init();

        // Kafka consumer for unknown program IDs
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "discovery-service-group")
            .set("bootstrap.servers", std::env::var("KAFKA_BROKER")?)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .create()?;

        // Kafka producer for AI analysis queue
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", std::env::var("KAFKA_BROKER")?)
            .set("message.timeout.ms", "5000")
            .create()?;

        // Redis client for caching and metrics
        let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379/0".to_string());
        let redis_client = redis::Client::open(redis_url)?;

        Ok(Self {
            kafka_consumer: consumer,
            kafka_producer: producer,
            redis_client,
            unknown_programs: HashMap::new(),
            metrics: DiscoveryMetrics::default(),
        })
    }

    async fn run(&mut self) -> Result<()> {
        info!("discovery-service: starting program discovery and AI preparation");
        
        // Subscribe to unknown program ID topic
        self.kafka_consumer.subscribe(&[&std::env::var("UNKNOWN_PROGRAMS_TOPIC").unwrap_or_else(|_| "unknown-programs".to_string())])?;

        loop {
            match self.kafka_consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload_view::<str>().and_then(|r| r.ok()) {
                        let payload = payload.to_string();
                        match self.process_unknown_program(&payload).await {
                            Ok(_) => {
                                self.metrics.total_unknown_programs += 1;
                                info!("Processed unknown program: {}", payload);
                            }
                            Err(e) => {
                                error!("Failed to process unknown program: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Kafka consumer error: {}", e);
                }
            }

            // Periodic metrics reporting
            if self.metrics.total_unknown_programs % 100 == 0 {
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
                let entry = self.unknown_programs.entry(program_id.clone()).or_insert_with(|| UnknownProgram {
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
            }

            // Step 2: Analyze data patterns for AI learning
            if let Some(entry) = self.unknown_programs.get_mut(&program_id) {
                self.analyze_data_patterns(entry, &program_data).await?;
            }

            // Step 3: Check if should queue for AI analysis
            let should_queue = if let Some(entry) = self.unknown_programs.get(&program_id) {
                self.should_queue_for_ai_analysis(entry)
            } else {
                false
            };

            // Step 4: Queue for AI analysis if meets criteria
            if should_queue {
                if let Some(entry) = self.unknown_programs.get_mut(&program_id) {
                    self.queue_for_ai_analysis(entry).await?;
                }
            }

            // Step 5: Cache in Redis for persistence
            if let Some(entry) = self.unknown_programs.get(&program_id) {
                self.cache_program_data(entry).await?;
            }
        }

        let analysis_time = start.elapsed();
        self.metrics.ai_analysis_time_ms = analysis_time.as_millis() as f64;

        Ok(())
    }

    async fn analyze_data_patterns(&mut self, program: &mut UnknownProgram, data: &serde_json::Value) -> Result<()> {
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
                    discriminator: data.get("discriminator").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                    data_length: bytes.len(),
                    byte_patterns: bytes.iter().take(16).cloned().collect(), // First 16 bytes for pattern recognition
                    frequency: 1,
                };

                // Update existing pattern or add new one
                if let Some(existing) = program.data_patterns.iter_mut().find(|p| p.discriminator == pattern.discriminator) {
                    existing.frequency += 1;
                } else {
                    program.data_patterns.push(pattern);
                }
            }
        }

        Ok(())
    }

    fn should_queue_for_ai_analysis(&self, program: &UnknownProgram) -> bool {
        // Queue programs that show potential DEX characteristics
        program.encounter_count >= 5 && // Seen multiple times
        program.account_types.len() >= 2 && // Multiple account types
        program.data_patterns.iter().any(|p| p.data_length > 100) // Substantial data
    }

    async fn queue_for_ai_analysis(&mut self, program: &mut UnknownProgram) -> Result<()> {
        program.ai_analysis_status = AIAnalysisStatus::InProgress;
        
        // Send to AI analysis queue
        let ai_payload = serde_json::to_string(&program)?;
        let record = FutureRecord::to("ai-analysis-queue")
            .payload(ai_payload.as_bytes())
            .key(&program.program_id);

        match self.kafka_producer.send(record, std::time::Duration::from_secs(5)).await {
            Ok(_) => {
                info!("Queued program {} for AI analysis", program.program_id);
                self.metrics.programs_analyzed += 1;
            }
            Err((e, _)) => {
                error!("Failed to queue program for AI analysis: {}", e);
                program.ai_analysis_status = AIAnalysisStatus::Failed;
            }
        }

        Ok(())
    }

    async fn cache_program_data(&self, program: &UnknownProgram) -> Result<()> {
        let mut conn = self.redis_client.get_multiplexed_tokio_connection().await?;
        let key = format!("discovery:program:{}", program.program_id);
        
        let program_json = serde_json::to_string(program)?;
        conn.set_ex(&key, program_json, 86400).await?; // Cache for 24 hours

        Ok(())
    }

    async fn report_metrics(&self) -> Result<()> {
        info!("DISCOVERY METRICS: {:?}", self.metrics);
        
        // Cache metrics in Redis
        let mut conn = self.redis_client.get_multiplexed_tokio_connection().await?;
        let metrics_json = serde_json::to_string(&self.metrics)?;
        conn.set_ex("discovery:metrics", metrics_json, 3600).await?; // Cache for 1 hour

        Ok(())
    }
}

impl Default for DiscoveryMetrics {
    fn default() -> Self {
        Self {
            total_unknown_programs: 0,
            programs_analyzed: 0,
            potential_dexes_found: 0,
            ai_analysis_time_ms: 0.0,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut service = DiscoveryService::new().await?;
    service.run().await
}
