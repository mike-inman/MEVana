use anyhow::Result;
use log::{error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message as KafkaMessage;
use serde::{Deserialize, Serialize};
// use solana_client::rpc_client::RpcClient;
// use solana_sdk::commitment_config::CommitmentConfig;
// use solana_sdk::pubkey::Pubkey;
// use solana_sdk::signature::{Keypair, Signature};
// use solana_sdk::transaction::Transaction;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use uuid::Uuid;

mod bundle;
mod flash_loan;
mod instruction_builder;
mod jito;
mod position_manager;

use bundle::BundleBuilder;
use flash_loan::FlashLoanProvider;
use instruction_builder::InstructionBuilder;
use jito::JitoClient;
use position_manager::PositionManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub opportunity_id: Uuid,
    pub token_in: Vec<u8>,
    pub token_out: Vec<u8>,
    pub edges: Vec<u64>,
    pub venues: Vec<u32>, // VenueId as u32
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
    pub transaction_signatures: Vec<String>,
    pub bundle_id: Option<String>,
    pub estimated_profit_lamports: u64,
    pub actual_profit_lamports: Option<i64>,
    pub execution_time_ms: u64,
    pub gas_used: u32,
    pub error_message: Option<String>,
    pub executed_at_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Simulated,
    Submitted,
    Confirmed,
    Failed,
    Expired,
}

pub struct Executor {
    consumer: StreamConsumer,
    producer: FutureProducer,
    // rpc_client: Arc<RpcClient>,
    jito_client: JitoClient,
    // keypair: Arc<Keypair>,
    instruction_builder: InstructionBuilder,
    bundle_builder: BundleBuilder,
    flash_loan_provider: FlashLoanProvider,
    position_manager: PositionManager,
    
    // Configuration
    input_topic: String,
    output_topic: String,
    max_execution_time_ms: u64,
    min_profit_threshold_lamports: u64,
    max_position_size_lamports: u64,
    enable_flash_loans: bool,
    dry_run_mode: bool,
}

impl Executor {
    pub fn new() -> Result<Self> {
        let kafka_broker =
            env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());
        
        let input_topic = env::var("ARBITRAGE_OPPORTUNITIES_TOPIC")
            .unwrap_or_else(|_| "arb.opportunities.v1".to_string());
        let output_topic = env::var("EXECUTION_RESULTS_TOPIC")
            .unwrap_or_else(|_| "execution.results.v1".to_string());

        let rpc_url = env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        
        let jito_url = env::var("JITO_URL")
            .unwrap_or_else(|_| "https://mainnet.block-engine.jito.wtf".to_string());

        let max_execution_time_ms = env::var("MAX_EXECUTION_TIME_MS")
            .unwrap_or_else(|_| "5000".to_string()) // 5 second max
            .parse()
            .unwrap_or(5000);

        let min_profit_threshold_lamports = env::var("MIN_PROFIT_THRESHOLD_LAMPORTS")
            .unwrap_or_else(|_| "1000000".to_string()) // 0.001 SOL
            .parse()
            .unwrap_or(1_000_000);

        let max_position_size_lamports = env::var("MAX_POSITION_SIZE_LAMPORTS")
            .unwrap_or_else(|_| "10000000000".to_string()) // 10 SOL
            .parse()
            .unwrap_or(10_000_000_000);

        let enable_flash_loans = env::var("ENABLE_FLASH_LOANS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let dry_run_mode = env::var("DRY_RUN_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        // Load keypair
        let keypair_path = env::var("EXECUTOR_KEYPAIR_PATH")
            .unwrap_or_else(|_| "~/.config/solana/id.json".to_string());
        let keypair_data = std::fs::read_to_string(shellexpand::tilde(&keypair_path).as_ref())?;
        let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)?;
        let keypair = Arc::new(Keypair::from_bytes(&keypair_bytes)?);

        // Initialize clients
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            rpc_url,
            CommitmentConfig::confirmed(),
        ));
        
        let jito_client = JitoClient::new(jito_url)?;

        // Initialize components
        let instruction_builder = InstructionBuilder::new(rpc_client.clone())?;
        let bundle_builder = BundleBuilder::new();
        let flash_loan_provider = FlashLoanProvider::new(rpc_client.clone())?;
        let position_manager = PositionManager::new(keypair.clone(), rpc_client.clone())?;

        // Kafka setup
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "executor")
            .set("bootstrap.servers", &kafka_broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "latest")
            .set("fetch.wait.max.ms", "1")
            .set("session.timeout.ms", "30000")
            .set("heartbeat.interval.ms", "10000")
            .create()?;

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_broker)
            .set("message.timeout.ms", "1000")
            .set("linger.ms", "0")
            .set("acks", "1")
            .create()?;

        Ok(Self {
            consumer,
            producer,
            rpc_client,
            jito_client,
            keypair,
            instruction_builder,
            bundle_builder,
            flash_loan_provider,
            position_manager,
            input_topic,
            output_topic,
            max_execution_time_ms,
            min_profit_threshold_lamports,
            max_position_size_lamports,
            enable_flash_loans,
            dry_run_mode,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting Executor");
        info!("Input topic: {}", self.input_topic);
        info!("Output topic: {}", self.output_topic);
        info!("Dry run mode: {}", self.dry_run_mode);
        info!("Flash loans enabled: {}", self.enable_flash_loans);
        info!("Min profit threshold: {} lamports", self.min_profit_threshold_lamports);

        // Subscribe to opportunities
        self.consumer.subscribe(&[&self.input_topic])?;

        // Main execution loop
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    let start_time = Instant::now();
                    
                    if let Err(e) = self.process_opportunity(&message).await {
                        error!("Error processing opportunity: {}", e);
                    }

                    let processing_time = start_time.elapsed();
                    if processing_time.as_millis() > 100 {
                        warn!(
                            "Slow opportunity processing: {}ms",
                            processing_time.as_millis()
                        );
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

    async fn process_opportunity(&self, message: &BorrowedMessage<'_>) -> Result<()> {
        let payload = match message.payload() {
            Some(p) => p,
            None => return Ok(()),
        };

        // Parse opportunity
        let opportunity: ArbitrageOpportunity = serde_json::from_slice(payload)?;
        
        info!(
            "Processing opportunity {} with profit {} bps",
            opportunity.opportunity_id, opportunity.profit_bps
        );

        // Quick profitability check
        if !self.is_profitable(&opportunity).await? {
            info!("Opportunity {} not profitable, skipping", opportunity.opportunity_id);
            return Ok(());
        }

        // Check if we can still execute (staleness check)
        if self.is_stale(&opportunity) {
            info!("Opportunity {} is stale, skipping", opportunity.opportunity_id);
            return Ok(());
        }

        // Execute the arbitrage
        let execution_result = self.execute_arbitrage(opportunity).await;
        
        // Publish result
        self.publish_execution_result(execution_result).await?;

        Ok(())
    }

    async fn is_profitable(&self, opportunity: &ArbitrageOpportunity) -> Result<bool> {
        // Convert profit from basis points to lamports
        let estimated_profit_lamports = (opportunity.max_size_in as u64 * opportunity.profit_bps) / 10_000;
        
        // Check against minimum threshold
        if estimated_profit_lamports < self.min_profit_threshold_lamports {
            return Ok(false);
        }

        // Check position size limits
        if opportunity.max_size_in > self.max_position_size_lamports as u128 {
            return Ok(false);
        }

        // Additional checks could include:
        // - Current account balances
        // - Flash loan availability
        // - Gas cost estimation
        // - Slippage tolerance

        Ok(true)
    }

    fn is_stale(&self, opportunity: &ArbitrageOpportunity) -> bool {
        let now_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let age_ms = (now_ns - opportunity.detected_at_ns) / 1_000_000;
        
        // Consider stale if older than max execution time
        age_ms > self.max_execution_time_ms
    }

    async fn execute_arbitrage(&self, opportunity: ArbitrageOpportunity) -> ExecutionResult {
        let execution_id = Uuid::new_v4();
        let start_time = Instant::now();
        
        info!("Executing arbitrage {} (execution: {})", 
              opportunity.opportunity_id, execution_id);

        if self.dry_run_mode {
            return self.simulate_execution(opportunity, execution_id, start_time).await;
        }

        // Real execution logic would go here
        // For now, return a simulated result
        ExecutionResult {
            opportunity_id: opportunity.opportunity_id,
            execution_id,
            status: ExecutionStatus::Simulated,
            transaction_signatures: vec![],
            bundle_id: None,
            estimated_profit_lamports: (opportunity.max_size_in as u64 * opportunity.profit_bps) / 10_000,
            actual_profit_lamports: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            gas_used: opportunity.total_cu_estimate,
            error_message: None,
            executed_at_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    async fn simulate_execution(
        &self,
        opportunity: ArbitrageOpportunity,
        execution_id: Uuid,
        start_time: Instant,
    ) -> ExecutionResult {
        info!("Simulating execution for opportunity {}", opportunity.opportunity_id);
        
        // Simulate some processing time
        sleep(Duration::from_millis(10)).await;
        
        ExecutionResult {
            opportunity_id: opportunity.opportunity_id,
            execution_id,
            status: ExecutionStatus::Simulated,
            transaction_signatures: vec!["simulation".to_string()],
            bundle_id: Some("sim-bundle".to_string()),
            estimated_profit_lamports: (opportunity.max_size_in as u64 * opportunity.profit_bps) / 10_000,
            actual_profit_lamports: Some(((opportunity.max_size_in as u64 * opportunity.profit_bps) / 10_000) as i64),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            gas_used: opportunity.total_cu_estimate,
            error_message: None,
            executed_at_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    async fn publish_execution_result(&self, result: ExecutionResult) -> Result<()> {
        let result_json = serde_json::to_string(&result)?;
        let execution_id = result.execution_id.to_string();

        let record = FutureRecord::to(&self.output_topic)
            .key(&execution_id)
            .payload(&result_json);

        self.producer
            .send(record, Duration::from_millis(100))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send execution result: {:?}", e))?;

        info!(
            "Published execution result: {} (status: {:?}, profit: {} lamports)",
            result.execution_id, result.status, result.estimated_profit_lamports
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Executor Service");

    let executor = Executor::new()?;
    executor.start().await?;

    Ok(())
}
