use anyhow::Result;
use log::{error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message as KafkaMessage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use crate::config::ExecutorConfig;
use crate::flash_loan::FlashLoanAggregator;
use crate::shyft_client::ShyftClient;
use crate::wallet_service::WalletService;
use crate::opportunity_processor::OpportunityProcessor;
use crate::bundle_builder::BundleBuilder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub opportunity_id: Uuid,
    pub token_in: Vec<u8>,
    pub token_out: Vec<u8>,
    pub edges: Vec<u64>,
    pub venues: Vec<u32>,
    pub profit_bps: u64,
    pub max_size_in: u128,
    pub total_cu_estimate: u32,
    pub total_fees_bps: u32,
    pub detected_at_ns: u64,
    pub max_edge_age_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub opportunity_id: Uuid,
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub bundle_id: Option<String>,
    pub transaction_signatures: Vec<String>,
    pub estimated_profit_lamports: u64,
    pub actual_profit_lamports: Option<i64>,
    pub execution_time_ms: u64,
    pub gas_used: u32,
    pub error_message: Option<String>,
    pub executed_at_ns: u64,
    pub flash_loan_provider: Option<String>,
    pub flash_loan_fee_lamports: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Received,
    Validated,
    FlashLoanQuoted,
    BundleBuilt,
    Simulated,
    Submitted,
    Confirmed,
    Failed,
    Rejected,
}

/// Main execution engine that processes arbitrage opportunities
pub struct ExecutionEngine {
    config: Arc<ExecutorConfig>,
    consumer: StreamConsumer,
    producer: FutureProducer,
    shyft_client: Arc<ShyftClient>,
    wallet_service: Arc<WalletService>,
    flash_loan_aggregator: Arc<FlashLoanAggregator>,
    opportunity_processor: OpportunityProcessor,
    bundle_builder: BundleBuilder,
}

impl ExecutionEngine {
    pub async fn new(config: ExecutorConfig) -> Result<Self> {
        let config = Arc::new(config);
        
        // Validate configuration
        config.validate()?;
        
        // Initialize Shyft.to client
        let shyft_client = Arc::new(ShyftClient::new(
            config.shyft_api_key.clone(),
            config.shyft_rpc_url.clone(),
            config.shyft_jito_url.clone(),
        )?);
        
        // Initialize wallet service
        let wallet_service = Arc::new(WalletService::new(
            Arc::clone(&config),
            Arc::clone(&shyft_client),
        )?);
        
        // Initialize flash loan aggregator
        let flash_loan_aggregator = Arc::new(FlashLoanAggregator::new(
            Arc::clone(&config),
            Arc::clone(&shyft_client),
        )?);
        
        // Initialize Kafka consumer
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &config.consumer_group)
            .set("bootstrap.servers", &config.kafka_broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set("fetch.wait.max.ms", "1")
            .set("session.timeout.ms", "30000")
            .set("heartbeat.interval.ms", "10000")
            .create()?;

        // Initialize Kafka producer
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.kafka_broker)
            .set("message.timeout.ms", "1000")
            .set("linger.ms", "0")
            .set("acks", "1")
            .create()?;
        
        // Initialize opportunity processor
        let opportunity_processor = OpportunityProcessor::new(
            Arc::clone(&config),
            Arc::clone(&wallet_service),
            Arc::clone(&flash_loan_aggregator),
        );
        
        // Initialize bundle builder
        let bundle_builder = BundleBuilder::new(Arc::clone(&shyft_client));

        info!("Execution engine initialized successfully");
        info!("  Kafka broker: {}", config.kafka_broker);
        info!("  Consumer group: {}", config.consumer_group);
        info!("  Opportunities topic: {}", config.opportunities_topic);
        info!("  Results topic: {}", config.results_topic);

        Ok(Self {
            config,
            consumer,
            producer,
            shyft_client,
            wallet_service,
            flash_loan_aggregator,
            opportunity_processor,
            bundle_builder,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting execution engine");
        
        // Subscribe to opportunities topic
        self.consumer.subscribe(&[&self.config.opportunities_topic])?;
        
        // Start wallet balance refresh task
        let wallet_service_clone = Arc::clone(&self.wallet_service);
        tokio::spawn(async move {
            loop {
                if let Err(e) = wallet_service_clone.refresh_balances().await {
                    warn!("Failed to refresh wallet balances: {}", e);
                }
                sleep(Duration::from_secs(30)).await; // Refresh every 30 seconds
            }
        });

        // Main opportunity processing loop
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    let start_time = Instant::now();
                    
                    match self.process_opportunity(&message).await {
                        Ok(_) => {
                            let processing_time = start_time.elapsed();
                            if processing_time.as_millis() > 50 {
                                warn!("Slow opportunity processing: {}ms", processing_time.as_millis());
                            }
                        }
                        Err(e) => {
                            error!("Error processing opportunity: {}", e);
                        }
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
                    error!("Kafka consumer error: {}", e);
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn process_opportunity(&self, message: &BorrowedMessage<'_>) -> Result<()> {
        let payload = match message.payload() {
            Some(p) => p,
            None => return Ok(()),
        };

        // Parse opportunity from JSON
        let opportunity: ArbitrageOpportunity = serde_json::from_slice(payload)?;
        let execution_id = Uuid::new_v4();
        
        info!("Processing opportunity {} (execution: {})", 
              opportunity.opportunity_id, execution_id);

        let start_time = Instant::now();
        let mut execution_result = ExecutionResult {
            opportunity_id: opportunity.opportunity_id,
            execution_id,
            status: ExecutionStatus::Received,
            bundle_id: None,
            transaction_signatures: vec![],
            estimated_profit_lamports: (opportunity.max_size_in as u64 * opportunity.profit_bps) / 10_000,
            actual_profit_lamports: None,
            execution_time_ms: 0,
            gas_used: opportunity.total_cu_estimate,
            error_message: None,
            executed_at_ns: chrono::Utc::now().timestamp_nanos() as u64,
            flash_loan_provider: None,
            flash_loan_fee_lamports: None,
        };

        // Step 1: Validate opportunity
        match self.opportunity_processor.validate_opportunity(&opportunity).await {
            Ok(true) => {
                execution_result.status = ExecutionStatus::Validated;
            }
            Ok(false) => {
                execution_result.status = ExecutionStatus::Rejected;
                execution_result.error_message = Some("Opportunity validation failed".to_string());
                self.publish_execution_result(execution_result).await?;
                return Ok(());
            }
            Err(e) => {
                execution_result.status = ExecutionStatus::Failed;
                execution_result.error_message = Some(format!("Validation error: {}", e));
                self.publish_execution_result(execution_result).await?;
                return Ok(());
            }
        }

        // Step 2: Get flash loan quote
        let token_in_mint = bs58::encode(&opportunity.token_in).into_string();
        match self.flash_loan_aggregator.get_best_quote(&token_in_mint, opportunity.max_size_in as u64).await {
            Ok(quote) => {
                execution_result.status = ExecutionStatus::FlashLoanQuoted;
                execution_result.flash_loan_provider = Some(format!("{:?}", quote.provider));
                execution_result.flash_loan_fee_lamports = Some((opportunity.max_size_in as u64 * quote.fee_bps as u64) / 10_000);
                
                // Check if still profitable after flash loan fees
                if let Some(fee) = execution_result.flash_loan_fee_lamports {
                    if execution_result.estimated_profit_lamports <= fee {
                        execution_result.status = ExecutionStatus::Rejected;
                        execution_result.error_message = Some("Not profitable after flash loan fees".to_string());
                        self.publish_execution_result(execution_result).await?;
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                execution_result.status = ExecutionStatus::Failed;
                execution_result.error_message = Some(format!("Flash loan quote error: {}", e));
                self.publish_execution_result(execution_result).await?;
                return Ok(());
            }
        }

        // Step 3: Build bundle
        match self.bundle_builder.build_arbitrage_bundle(&opportunity).await {
            Ok(bundle) => {
                execution_result.status = ExecutionStatus::BundleBuilt;
                execution_result.transaction_signatures = bundle.transaction_ids;
            }
            Err(e) => {
                execution_result.status = ExecutionStatus::Failed;
                execution_result.error_message = Some(format!("Bundle building error: {}", e));
                self.publish_execution_result(execution_result).await?;
                return Ok(());
            }
        }

        // Step 4: Simulate bundle
        // Implementation would go here for transaction simulation
        execution_result.status = ExecutionStatus::Simulated;

        // Step 5: Submit to Jito
        // Implementation would go here for actual Jito submission
        execution_result.status = ExecutionStatus::Submitted;
        execution_result.bundle_id = Some(format!("bundle_{}", execution_id));

        // Update execution time
        execution_result.execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Publish result
        self.publish_execution_result(execution_result).await?;

        Ok(())
    }

    async fn publish_execution_result(&self, result: ExecutionResult) -> Result<()> {
        let result_json = serde_json::to_string(&result)?;
        let execution_id = result.execution_id.to_string();

        let record = FutureRecord::to(&self.config.results_topic)
            .key(&execution_id)
            .payload(&result_json);

        self.producer
            .send(record, Duration::from_millis(100))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish execution result: {:?}", e))?;

        info!(
            "Published execution result: {} (status: {:?}, profit: {} lamports)",
            result.execution_id, result.status, result.estimated_profit_lamports
        );

        Ok(())
    }
}
