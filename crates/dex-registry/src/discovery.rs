use anyhow::Result;
use chrono::Utc;
use log::{debug, error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::Message as KafkaMessage;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::cache::RegistryCache;
use crate::config::RegistryConfig;
use crate::database::{Database, DiscoveryQueue, DexProgram, Token};
use crate::plugin::PluginManager;

/// Service for discovering unknown DEX programs and tokens from Geyser stream
pub struct DiscoveryService {
    config: Arc<RegistryConfig>,
    database: Arc<Database>,
    cache: Arc<RegistryCache>,
    plugin_manager: Arc<PluginManager>,
    consumer: Option<StreamConsumer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryEvent {
    pub event_type: String, // "unknown_program", "unknown_token"
    pub program_id: Option<String>,
    pub mint_address: Option<String>,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub transaction_signature: String,
    pub metadata: Option<serde_json::Value>,
}

impl DiscoveryService {
    pub async fn new(
        config: Arc<RegistryConfig>,
        database: Arc<Database>,
        cache: Arc<RegistryCache>,
        plugin_manager: Arc<PluginManager>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            database,
            cache,
            plugin_manager,
            consumer: None,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting discovery service");

        // Start background workers
        let discovery_worker = self.start_discovery_worker();
        let kafka_consumer = self.start_kafka_consumer();
        let idl_fetcher = self.start_idl_fetcher();

        // Run all services concurrently
        tokio::try_join!(discovery_worker, kafka_consumer, idl_fetcher)?;

        Ok(())
    }

    async fn start_kafka_consumer(&self) -> Result<()> {
        info!("Starting Kafka consumer for discovery events");

        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &self.config.consumer_group)
            .set("bootstrap.servers", &self.config.kafka_broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set("fetch.wait.max.ms", "100")
            .create()?;

        consumer.subscribe(&[&self.config.discovery_topic])?;

        loop {
            match consumer.recv().await {
                Ok(message) => {
                    if let Err(e) = self.process_discovery_message(&message).await {
                        error!("Error processing discovery message: {}", e);
                    }

                    if let Err(e) = consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async) {
                        error!("Failed to commit offset: {}", e);
                    }
                }
                Err(e) => {
                    error!("Kafka consumer error: {}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn process_discovery_message(&self, message: &BorrowedMessage<'_>) -> Result<()> {
        let payload = match message.payload() {
            Some(p) => p,
            None => return Ok(()),
        };

        let event: DiscoveryEvent = serde_json::from_slice(payload)?;
        
        match event.event_type.as_str() {
            "unknown_program" => {
                if let Some(program_id) = event.program_id {
                    self.enqueue_program_discovery(&program_id).await?;
                }
            }
            "unknown_token" => {
                if let Some(mint_address) = event.mint_address {
                    self.enqueue_token_discovery(&mint_address).await?;
                }
            }
            _ => {
                debug!("Unknown discovery event type: {}", event.event_type);
            }
        }

        Ok(())
    }

    async fn enqueue_program_discovery(&self, program_id: &str) -> Result<()> {
        // Check if already known
        if RegistryCache::is_known_dex(program_id) {
            return Ok(());
        }

        // Check if already in discovery queue
        // TODO: Add database query to check existing queue entries

        let discovery_item = DiscoveryQueue {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            discovery_type: "dex_program".to_string(),
            priority: 10, // Medium priority
            attempts: 0,
            last_attempt: None,
            created_at: Utc::now(),
            metadata: None,
            status: "pending".to_string(),
        };

        self.database.enqueue_discovery(&discovery_item).await?;
        debug!("Enqueued program discovery: {}", program_id);

        Ok(())
    }

    async fn enqueue_token_discovery(&self, mint_address: &str) -> Result<()> {
        // Check if already known
        if RegistryCache::is_known_token(mint_address) {
            return Ok(());
        }

        let discovery_item = DiscoveryQueue {
            id: Uuid::new_v4().to_string(),
            program_id: mint_address.to_string(),
            discovery_type: "token_mint".to_string(),
            priority: 5, // Lower priority than DEX programs
            attempts: 0,
            last_attempt: None,
            created_at: Utc::now(),
            metadata: None,
            status: "pending".to_string(),
        };

        self.database.enqueue_discovery(&discovery_item).await?;
        debug!("Enqueued token discovery: {}", mint_address);

        Ok(())
    }

    async fn start_discovery_worker(&self) -> Result<()> {
        info!("Starting discovery worker");

        loop {
            // Process discovery queue
            let discoveries = self.database.dequeue_discovery(self.config.discovery_batch_size).await?;
            
            if discoveries.is_empty() {
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            info!("Processing {} discovery items", discoveries.len());

            for discovery in discoveries {
                match discovery.discovery_type.as_str() {
                    "dex_program" => {
                        if let Err(e) = self.process_program_discovery(&discovery).await {
                            error!("Failed to process program discovery {}: {}", discovery.program_id, e);
                            self.database.complete_discovery(&discovery.id, "failed").await?;
                        } else {
                            self.database.complete_discovery(&discovery.id, "completed").await?;
                        }
                    }
                    "token_mint" => {
                        if let Err(e) = self.process_token_discovery(&discovery).await {
                            error!("Failed to process token discovery {}: {}", discovery.program_id, e);
                            self.database.complete_discovery(&discovery.id, "failed").await?;
                        } else {
                            self.database.complete_discovery(&discovery.id, "completed").await?;
                        }
                    }
                    _ => {
                        warn!("Unknown discovery type: {}", discovery.discovery_type);
                        self.database.complete_discovery(&discovery.id, "failed").await?;
                    }
                }
            }
        }
    }

    async fn process_program_discovery(&self, discovery: &DiscoveryQueue) -> Result<()> {
        let program_id = &discovery.program_id;
        info!("Discovering program: {}", program_id);

        // Try to fetch IDL and determine if it's a DEX
        let idl_result = self.fetch_idl(program_id).await;
        
        let (dex_name, protocol_type, quirks) = if let Ok(idl_content) = idl_result {
            // Try to parse with plugins
            if let Some(plugin) = self.plugin_manager.get_plugin_for_program(program_id) {
                let metadata = plugin.metadata();
                (metadata.dex_name, metadata.protocol_type, plugin.get_quirks())
            } else {
                // Generic classification
                self.classify_unknown_program(program_id, &idl_content).await?
            }
        } else {
            // Fallback classification based on transaction patterns
            ("Unknown".to_string(), "Unknown".to_string(), vec![])
        };

        // Create DEX program entry
        let dex_program = DexProgram {
            program_id: program_id.clone(),
            dex_name,
            protocol_type,
            fee_structure: Some(r#"{"swap_fee_bps": 30, "protocol_fee_bps": 1}"#.to_string()),
            liquidity_model: "unknown".to_string(),
            idl_version: None,
            quirks: Some(serde_json::to_string(&quirks)?),
            is_active: true,
            discovered_at: Utc::now(),
            last_seen: Utc::now(),
            transaction_volume_24h: None,
            metadata: None,
        };

        // Save to database and cache
        self.database.insert_dex_program(&dex_program).await?;
        self.cache.set_dex_program(&dex_program).await?;

        info!("Discovered DEX program: {} ({})", dex_program.dex_name, program_id);
        Ok(())
    }

    async fn process_token_discovery(&self, discovery: &DiscoveryQueue) -> Result<()> {
        let mint_address = &discovery.program_id;
        info!("Discovering token: {}", mint_address);

        // Try to fetch token metadata
        let (symbol, name, decimals) = self.fetch_token_metadata(mint_address).await?;

        // Determine if it's a stablecoin
        let is_stable = self.is_stablecoin(&symbol, &name);

        // Create token entry
        let token = Token {
            mint_address: mint_address.clone(),
            symbol: Some(symbol),
            name: Some(name),
            decimals: Some(decimals as i32),
            is_stable_coin: is_stable,
            is_wrapped: false,
            underlying_asset: None,
            coingecko_id: None,
            price_feeds: None,
            market_cap: None,
            daily_volume: None,
            discovered_at: Utc::now(),
            verified: false, // Requires manual verification
            metadata: None,
        };

        // Save to database and cache
        self.database.insert_token(&token).await?;
        self.cache.set_token(&token).await?;

        info!("Discovered token: {} ({})", token.symbol.unwrap_or_default(), mint_address);
        Ok(())
    }

    async fn start_idl_fetcher(&self) -> Result<()> {
        info!("Starting IDL fetcher service");

        // Periodic task to fetch IDLs for known programs
        loop {
            sleep(Duration::from_secs(3600)).await; // Every hour

            let programs = self.database.list_dex_programs(true).await?;
            
            for program in programs {
                if let Ok(idl_content) = self.fetch_idl(&program.program_id).await {
                    // TODO: Update IDL cache
                    debug!("Fetched IDL for: {}", program.program_id);
                }
            }
        }
    }

    async fn fetch_idl(&self, program_id: &str) -> Result<String> {
        // Try Anchor CLI first
        match self.fetch_idl_anchor(program_id).await {
            Ok(idl) => return Ok(idl),
            Err(e) => debug!("Anchor IDL fetch failed for {}: {}", program_id, e),
        }

        // Try Solana CLI as fallback
        match self.fetch_idl_solana(program_id).await {
            Ok(idl) => return Ok(idl),
            Err(e) => debug!("Solana IDL fetch failed for {}: {}", program_id, e),
        }

        anyhow::bail!("Failed to fetch IDL for program: {}", program_id)
    }

    async fn fetch_idl_anchor(&self, program_id: &str) -> Result<String> {
        let output = Command::new(&self.config.anchor_cli_path)
            .args(["idl", "fetch", program_id])
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?)
        } else {
            anyhow::bail!("Anchor CLI failed: {}", String::from_utf8_lossy(&output.stderr))
        }
    }

    async fn fetch_idl_solana(&self, program_id: &str) -> Result<String> {
        let output = Command::new(&self.config.solana_cli_path)
            .args(["program", "show", program_id])
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?)
        } else {
            anyhow::bail!("Solana CLI failed: {}", String::from_utf8_lossy(&output.stderr))
        }
    }

    async fn classify_unknown_program(&self, _program_id: &str, _idl_content: &str) -> Result<(String, String, Vec<String>)> {
        // TODO: Implement heuristic classification
        // For now, return generic classification
        Ok((
            "Unknown DEX".to_string(),
            "AMM".to_string(),
            vec!["unclassified".to_string()],
        ))
    }

    async fn fetch_token_metadata(&self, _mint_address: &str) -> Result<(String, String, u8)> {
        // TODO: Implement token metadata fetching
        // For now, return placeholder data
        Ok((
            "UNKNOWN".to_string(),
            "Unknown Token".to_string(),
            9,
        ))
    }

    fn is_stablecoin(&self, symbol: &str, name: &str) -> bool {
        let stable_indicators = ["USD", "USDC", "USDT", "DAI", "BUSD", "FRAX"];
        
        stable_indicators.iter().any(|&indicator| {
            symbol.contains(indicator) || name.to_uppercase().contains(indicator)
        })
    }
}
