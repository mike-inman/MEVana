use anyhow::Result;
use log::{error, info, warn};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::BorrowedMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message as KafkaMessage;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use crate::account_registry::{report_missing_accounts, AccountRegistry, AccountRegistryConfig};
use crate::bundle_builder::{ArbitrageBundle, BundleBuilder};
use crate::config::ExecutorConfig;
use crate::flash_loan::FlashLoanAggregator;
use crate::hotpath_ws;
use crate::metrics::MetricsServer;
use crate::opportunity_processor::OpportunityProcessor;
use crate::shyft_client::ShyftClient;
use crate::token_registry::{NoopEnricher, ShyftEnricher, TokenRegistry};
use crate::wallet_service::WalletService;

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
    // Cost breakdown
    pub estimated_bundle_fees_lamports: Option<u64>,
    pub estimated_priority_fee_lamports: Option<u64>,
    pub estimated_cu_cost_lamports: Option<u64>,
    pub min_profit_threshold_lamports: Option<u64>,
    // Token balance delta heuristics from simulation (mint -> signed delta)
    pub simulated_token_deltas: Option<std::collections::HashMap<String, i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Received,
    Validated,
    FlashLoanQuoted,
    BundleBuilt,
    Simulated,
    Submitted,
    LandedPendingProfit,
    Confirmed,
    Failed,
    Rejected,
    Unlanded,
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
    metrics: Arc<MetricsServer>,
    account_registry: Arc<AccountRegistry>,
    cu_price_micro_lamports: u64, // configured compute unit price (micro lamports per CU)
    pub(crate) token_registry: Arc<TokenRegistry>,
    // Cumulative realized PnL tracking (lamports) for daily loss guard
    daily_realized_pnl_lamports: std::sync::atomic::AtomicI64,
    daily_reset_unix: std::sync::atomic::AtomicU64,
    db: PgPool,
    registry_hash_cache: parking_lot::RwLock<Option<String>>, // cached last computed token registry hash
    live_trading_enabled: std::sync::atomic::AtomicBool,
    circuit_consec_failures: std::sync::atomic::AtomicU64,
    circuit_consec_unlanded: std::sync::atomic::AtomicU64,
    circuit_open: std::sync::atomic::AtomicBool,
}

impl ExecutionEngine {
    #[allow(dead_code)]
    pub fn metrics(&self) -> &Arc<MetricsServer> {
        &self.metrics
    }
    #[allow(dead_code)]
    pub fn db_clone(&self) -> PgPool {
        self.db.clone()
    }
    // Convert simulated token deltas (mint -> amount) into lamports using simple oracle prices in registry.
    // Assumptions: OracleAccount.price field stores a u64 lamports-per-token or scaled price in lamports * 1e9 / token? For now assume it is lamports per whole token with 9 decimals scaling like SOL.
    fn convert_token_deltas_to_lamports(
        &self,
        deltas: &std::collections::HashMap<String, i64>,
    ) -> i64 {
        let mut total: i64 = 0;
        let mut warned_missing: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for (mint, amount) in deltas {
            // Treat wrapped SOL pseudo-mint as native lamports directly
            if mint == "So11111111111111111111111111111111111111112" {
                total += *amount;
                continue;
            }
            if let Some(oracle) = self.account_registry.data.oracles.get(mint) {
                if let Ok(raw_price) = oracle.price.parse::<u128>() {
                    let price_decimals = oracle.price_decimals.unwrap_or(9) as u32; // oracle scaling factor
                    let mint_decimals = self
                        .account_registry
                        .mint_decimals
                        .get(mint)
                        .copied()
                        .unwrap_or(9) as i32;
                    let target_decimals: i32 = 9; // canonical normalization target
                    let normalized_amount_i128: i128 = if mint_decimals < target_decimals {
                        let up = (target_decimals - mint_decimals) as u32;
                        (*amount as i128) * 10i128.pow(up)
                    } else if mint_decimals > target_decimals {
                        let down = (mint_decimals - target_decimals) as u32;
                        (*amount as i128) / 10i128.pow(down)
                    } else {
                        *amount as i128
                    };
                    let scale_div = 10u128.pow(price_decimals);
                    let delta = (normalized_amount_i128 * raw_price as i128) / scale_div as i128;
                    total += delta as i64;
                }
            } else if !warned_missing.contains(mint) {
                warn!("missing_oracle_for_mint mint={}", mint);
                warned_missing.insert(mint.clone());
            }
        }
        total
    }
    pub async fn new(
        config: ExecutorConfig,
        metrics: Arc<MetricsServer>,
        db: PgPool,
    ) -> Result<Self> {
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

        info!("Execution engine initialized successfully");
        info!("  Kafka broker: {}", config.kafka_broker);
        info!("  Consumer group: {}", config.consumer_group);
        info!("  Opportunities topic: {}", config.opportunities_topic);
        info!("  Results topic: {}", config.results_topic);

        // Load account registry (may be strict)
        let strict = std::env::var("ACCOUNT_REGISTRY_STRICT")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let path = std::env::var("ACCOUNT_REGISTRY_PATH").ok().or_else(|| {
            // default example path if present
            let default_path = "profiles/account_registry.example.json";
            if std::path::Path::new(default_path).exists() {
                Some(default_path.to_string())
            } else {
                None
            }
        });
        let account_registry = match AccountRegistry::load(&AccountRegistryConfig { strict, path })
        {
            Ok(reg) => Arc::new(reg),
            Err(e) => {
                error!("account_registry_load_failed strict={} err={}", strict, e);
                if strict {
                    return Err(e);
                } else {
                    Arc::new(AccountRegistry::default())
                }
            }
        };
        // Report missing & update metrics (helper updates registry stats internally)
        report_missing_accounts(&account_registry, Some(&metrics));
        // freshness metric (now)
        let now_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        metrics.update_account_registry_freshness(now_ts);

        // CU price configuration (micro lamports per CU)
        let cu_price_micro_lamports = std::env::var("CU_PRICE_MICRO_LAMPORTS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(200); // default 200 micro-lamports (0.0002 lamport) per CU
                             // Now build bundle builder with registry & metrics
        let builder = BundleBuilder::new(Arc::clone(&shyft_client))
            .with_metrics(Arc::clone(&metrics))
            .with_account_registry(Arc::clone(&account_registry));
        builder.start_background_tasks();
        Ok(Self {
            config,
            consumer,
            producer,
            shyft_client,
            wallet_service,
            flash_loan_aggregator,
            opportunity_processor,
            bundle_builder: builder,
            metrics,
            account_registry,
            cu_price_micro_lamports,
            token_registry: Arc::new(TokenRegistry::new()),
            daily_realized_pnl_lamports: std::sync::atomic::AtomicI64::new(0),
            daily_reset_unix: std::sync::atomic::AtomicU64::new(
                chrono::Utc::now()
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64,
            ),
            db,
            registry_hash_cache: parking_lot::RwLock::new(None),
            live_trading_enabled: std::sync::atomic::AtomicBool::new(false),
            circuit_consec_failures: std::sync::atomic::AtomicU64::new(0),
            circuit_consec_unlanded: std::sync::atomic::AtomicU64::new(0),
            circuit_open: std::sync::atomic::AtomicBool::new(false),
        })
    }

    // Persist current token registry & oracle state (best-effort). Returns new hash.
    pub async fn persist_registry_state(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut entries: Vec<String> = Vec::new();
        // Token metadata from dynamic registry
        for meta in self.token_registry.all_metadata() {
            entries.push(format!(
                "T|{}|{}|{}|{}|{}",
                meta.mint,
                meta.decimals.unwrap_or(0),
                meta.symbol.clone().unwrap_or_default(),
                meta.name.clone().unwrap_or_default(),
                meta.tags.join(";")
            ));
        }
        // Oracle + decimals from account_registry
        for (mint, _dec) in self.account_registry.mint_decimals.iter() {
            let price_part = self
                .account_registry
                .data
                .oracles
                .get(mint)
                .map(|o| {
                    format!(
                        "{}:{}:{}",
                        o.price,
                        o.price_decimals.unwrap_or(0),
                        o.confidence_limit_bps.unwrap_or(0)
                    )
                })
                .unwrap_or_else(|| "-".into());
            entries.push(format!("O|{}|{}|{}", mint, _dec, price_part));
        }
        entries.sort();
        let mut hasher = Sha256::new();
        for e in &entries {
            hasher.update(e.as_bytes());
        }
        let hash = format!("{:x}", hasher.finalize());
        // Cache
        *self.registry_hash_cache.write() = Some(hash.clone());
        // Persist tables (simple upserts)
        let db = self.db.clone();
        // Use a transaction for consistency
        let mut tx = db.begin().await?;
        for meta in self.token_registry.all_metadata() {
            let _ = sqlx::query("INSERT INTO token_registry_tokens (mint, decimals, symbol, name, first_seen, last_refreshed, tags) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (mint) DO UPDATE SET decimals=EXCLUDED.decimals, symbol=EXCLUDED.symbol, name=EXCLUDED.name, last_refreshed=EXCLUDED.last_refreshed, tags=EXCLUDED.tags")
                .bind(&meta.mint)
                .bind(meta.decimals.map(|d| d as i32))
                .bind(meta.symbol)
                .bind(meta.name)
                .bind(meta.first_seen)
                .bind(meta.last_refreshed)
                .bind(if meta.tags.is_empty() { None::<Vec<String>> } else { Some(meta.tags) })
                .execute(&mut *tx).await;
        }
        for (mint, _dec) in self.account_registry.mint_decimals.iter() {
            let (price_lamports, price_decimals) =
                if let Some(o) = self.account_registry.data.oracles.get(mint) {
                    (
                        o.price.parse::<i64>().ok(),
                        o.price_decimals.map(|d| d as i32),
                    )
                } else {
                    (None, None)
                };
            let _ = sqlx::query("INSERT INTO token_oracles (mint, price_lamports, price_decimals, updated_at) VALUES ($1,$2,$3,NOW()) ON CONFLICT (mint) DO UPDATE SET price_lamports=EXCLUDED.price_lamports, price_decimals=EXCLUDED.price_decimals, updated_at=NOW()")
                .bind(mint)
                .bind(price_lamports.map(|v| v as i64))
                .bind(price_decimals)
                .execute(&mut *tx).await;
        }
        tx.commit().await?;
        Ok(hash)
    }

    // Recompute & cache registry hash without persisting (fast path)
    pub fn compute_registry_hash(&self) -> String {
        if let Some(h) = self.registry_hash_cache.read().clone() {
            return h;
        }
        use sha2::{Digest, Sha256};
        let mut entries: Vec<String> = self
            .account_registry
            .mint_decimals
            .iter()
            .map(|(mint, dec)| {
                let price_part = self
                    .account_registry
                    .data
                    .oracles
                    .get(mint)
                    .map(|o| {
                        format!(
                            "{}:{}:{}",
                            o.price,
                            o.price_decimals.unwrap_or(0),
                            o.confidence_limit_bps.unwrap_or(0)
                        )
                    })
                    .unwrap_or_else(|| "-".into());
                format!("{}|{}|{}", mint, dec, price_part)
            })
            .collect();
        entries.sort();
        let mut hasher = Sha256::new();
        for e in &entries {
            hasher.update(e.as_bytes());
        }
        let hash = format!("{:x}", hasher.finalize());
        *self.registry_hash_cache.write() = Some(hash.clone());
        hash
    }

    // Compute input notional lamports used by safety cap (exposed for tests)
    #[allow(dead_code)]
    pub(crate) fn compute_input_notional_lamports(
        &self,
        token_in_mint: &str,
        amount_in_smallest: u128,
    ) -> u128 {
        if token_in_mint == "So11111111111111111111111111111111111111112" {
            return amount_in_smallest;
        }
        if let Some(oracle) = self.account_registry.data.oracles.get(token_in_mint) {
            if let Ok(raw_price) = oracle.price.parse::<u128>() {
                let price_decimals = oracle.price_decimals.unwrap_or(9) as u32;
                let mint_decimals = self
                    .account_registry
                    .mint_decimals
                    .get(token_in_mint)
                    .copied()
                    .unwrap_or(9) as u32;
                let canonical_amount = if mint_decimals < 9 {
                    amount_in_smallest * 10u128.pow((9 - mint_decimals) as u32)
                } else if mint_decimals > 9 {
                    amount_in_smallest / 10u128.pow((mint_decimals - 9) as u32)
                } else {
                    amount_in_smallest
                };
                return (canonical_amount.saturating_mul(raw_price)) / 10u128.pow(price_decimals);
            }
        }
        amount_in_smallest // fallback: treat as lamports equivalence
    }

    // Reset daily PnL if a new UTC day has started.
    fn maybe_reset_daily_pnl(&self) {
        let today_start = chrono::Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp() as u64;
        let prev = self
            .daily_reset_unix
            .load(std::sync::atomic::Ordering::Relaxed);
        if today_start != prev {
            if self
                .daily_reset_unix
                .compare_exchange(
                    prev,
                    today_start,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::Relaxed,
                )
                .is_ok()
            {
                self.daily_realized_pnl_lamports
                    .store(0, std::sync::atomic::Ordering::SeqCst);
                log::info!("daily_pnl_reset new_day_start_unix={}", today_start);
            }
        }
    }

    // Record realized profit/loss (lamports) updating metrics & guard counters.
    fn record_realized_pnl(&self, estimated: u64, realized: i64) {
        self.maybe_reset_daily_pnl();
        self.metrics.record_realized_profit(estimated, realized);
        // Update daily cumulative
        self.daily_realized_pnl_lamports
            .fetch_add(realized, std::sync::atomic::Ordering::SeqCst);
        let cur = self
            .daily_realized_pnl_lamports
            .load(std::sync::atomic::Ordering::Relaxed);
        // Update metrics gauge (SOL)
        self.metrics
            .daily_pnl_sol
            .set(cur as f64 / 1_000_000_000f64);
        // Persist delta to daily_pnl table (best-effort)
        let delta = realized;
        let db = self.db.clone();
        tokio::spawn(async move {
            let day = chrono::Utc::now().date_naive();
            if let Err(e) = sqlx::query("SELECT upsert_daily_pnl($1,$2)")
                .bind(day)
                .bind(delta)
                .execute(&db)
                .await
            {
                log::warn!("daily_pnl_upsert_failed day={} err={}", day, e);
            }
        });
    }

    // Check if daily loss limit breached (returns true if trade should be blocked)
    fn should_block_for_daily_loss(&self) -> bool {
        self.maybe_reset_daily_pnl();
        let limit_sol = self.config.daily_loss_limit_sol;
        let limit_lamports = (limit_sol * 1_000_000_000f64) as i64; // approximate
        let pnl = self
            .daily_realized_pnl_lamports
            .load(std::sync::atomic::Ordering::Relaxed);
        // pnl negative means losses. Block if losses exceed limit.
        if pnl < 0 && -pnl > limit_lamports {
            return true;
        }
        false
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        info!("🚀 Starting execution engine");
        // Start token enrichment if enabled
        let enricher_source =
            std::env::var("TOKEN_ENRICH_SOURCE").unwrap_or_else(|_| "noop".into());
        if let Some(reg) = Arc::get_mut(&mut Arc::clone(&self.token_registry)) {
            let _ = reg;
        }
        let reg_clone = Arc::clone(&self.token_registry);
        match enricher_source.as_str() {
            "shyft" => {
                if let Ok(base_url) = std::env::var("SHYFT_META_BASE_URL") {
                    let api_key = std::env::var("SHYFT_META_API_KEY")
                        .ok()
                        .or_else(|| std::env::var("SHYFT_API_KEY").ok())
                        .unwrap_or_default();
                    let enricher = ShyftEnricher::new(base_url, api_key);
                    tokio::spawn(async move {
                        reg_clone.start_background_enrichment(enricher).await;
                    });
                    info!("token_enrichment_started source=shyft");
                } else {
                    info!("token_enrichment_shyft_disabled missing SHYFT_META_BASE_URL");
                }
            }
            _ => {
                tokio::spawn(async move {
                    reg_clone.start_background_enrichment(NoopEnricher).await;
                });
            }
        }
        if let Ok(ws_url) = std::env::var("ARB_ENGINE_WS") {
            if !ws_url.is_empty() {
                let _ = hotpath_ws::start_hotpath_ws(
                    Arc::clone(&self),
                    Arc::clone(&self.metrics),
                    &ws_url,
                )
                .await;
            }
        }

        // Subscribe to opportunities topic
        self.consumer
            .subscribe(&[&self.config.opportunities_topic])?;

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
                                warn!(
                                    "Slow opportunity processing: {}ms",
                                    processing_time.as_millis()
                                );
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
        // Persist raw opportunity (best-effort)
        {
            let db = self.db.clone();
            let raw_in = bs58::encode(&opportunity.token_in).into_string();
            let raw_out = bs58::encode(&opportunity.token_out).into_string();
            tokio::spawn(async move {
                let ts = chrono::DateTime::<chrono::Utc>::from_timestamp(
                    (opportunity.detected_at_ns / 1_000_000_000) as i64,
                    0,
                )
                .unwrap_or_else(|| chrono::Utc::now());
                let _ = sqlx::query("INSERT INTO opportunities (id, token_in, token_out, profit_bps, max_size_in, total_cu_estimate, total_fees_bps, detected_at, max_edge_age_ms) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (id) DO NOTHING")
                    .bind(opportunity.opportunity_id)
                    .bind(raw_in)
                    .bind(raw_out)
                    .bind(opportunity.profit_bps as i64)
                    .bind(opportunity.max_size_in as i64)
                    .bind(opportunity.total_cu_estimate as i32)
                    .bind(opportunity.total_fees_bps as i32)
                    .bind(ts)
                    .bind(opportunity.max_edge_age_ms as i64)
                    .execute(&db)
                    .await;
            });
        }
        let execution_id = Uuid::new_v4();

        info!(
            "Processing opportunity {} (execution: {})",
            opportunity.opportunity_id, execution_id
        );

        let start_time = Instant::now();
        let mut execution_result = ExecutionResult {
            opportunity_id: opportunity.opportunity_id,
            execution_id,
            status: ExecutionStatus::Received,
            bundle_id: None,
            transaction_signatures: vec![],
            estimated_profit_lamports: (opportunity.max_size_in as u64 * opportunity.profit_bps)
                / 10_000,
            actual_profit_lamports: None,
            execution_time_ms: 0,
            gas_used: opportunity.total_cu_estimate,
            error_message: None,
            executed_at_ns: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            flash_loan_provider: None,
            flash_loan_fee_lamports: None,
            estimated_bundle_fees_lamports: None,
            estimated_priority_fee_lamports: None,
            estimated_cu_cost_lamports: None,
            min_profit_threshold_lamports: None,
            simulated_token_deltas: None,
        };

        // Step 1: Validate opportunity
        match self
            .opportunity_processor
            .validate_opportunity(&opportunity)
            .await
        {
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

        // Global daily loss guard (prevents further risk once breached)
        if self.should_block_for_daily_loss() {
            execution_result.status = ExecutionStatus::Rejected;
            execution_result.error_message = Some("daily_loss_limit_exceeded".into());
            self.publish_execution_result(execution_result).await?;
            return Ok(());
        }

        // Step 2: Get flash loan quote
        let token_in_mint = bs58::encode(&opportunity.token_in).into_string();
        let token_out_mint = bs58::encode(&opportunity.token_out).into_string();
        match self
            .flash_loan_aggregator
            .get_best_quote(&token_in_mint, opportunity.max_size_in as u64)
            .await
        {
            Ok(quote) => {
                execution_result.status = ExecutionStatus::FlashLoanQuoted;
                execution_result.flash_loan_provider = Some(format!("{:?}", quote.provider));
                execution_result.flash_loan_fee_lamports =
                    Some((opportunity.max_size_in as u64 * quote.fee_bps as u64) / 10_000);

                // Check if still profitable after flash loan fees
                if let Some(fee) = execution_result.flash_loan_fee_lamports {
                    if execution_result.estimated_profit_lamports <= fee {
                        execution_result.status = ExecutionStatus::Rejected;
                        execution_result.error_message =
                            Some("Not profitable after flash loan fees".to_string());
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
        let bundle_fee_estimate: u64; // set after build success
        match self
            .bundle_builder
            .build_arbitrage_bundle(
                &opportunity,
                execution_result
                    .flash_loan_provider
                    .as_ref()
                    .and_then(|s| crate::flash_loan::FlashLoanProvider::from_string(s)),
            )
            .await
        {
            Ok(bundle) => {
                execution_result.status = ExecutionStatus::BundleBuilt;
                execution_result.transaction_signatures = bundle.transaction_ids;
                bundle_fee_estimate = bundle.estimated_fees_lamports;
            }
            Err(e) => {
                execution_result.status = ExecutionStatus::Failed;
                execution_result.error_message = Some(format!("Bundle building error: {}", e));
                self.publish_execution_result(execution_result).await?;
                return Ok(());
            }
        }

        // Step 4: Simulate bundle
        if !execution_result.transaction_signatures.is_empty() {
            match self
                .bundle_builder
                .simulate_bundle(&ArbitrageBundle {
                    bundle_id: format!("bundle_{}", execution_id),
                    transaction_ids: execution_result.transaction_signatures.clone(),
                    estimated_cu: opportunity.total_cu_estimate,
                    estimated_fees_lamports: bundle_fee_estimate,
                    instructions_count: 0,
                })
                .await
            {
                Ok(sim) => {
                    if sim.success {
                        execution_result.status = ExecutionStatus::Simulated;
                        execution_result.gas_used = sim.estimated_cu_consumed;
                        // Gross profit from opportunity heuristic
                        let gross =
                            (opportunity.max_size_in as u64 * opportunity.profit_bps) / 10_000;
                        // Priority fee heuristic mirrors compute_dynamic_values logic
                        let priority_fee_lamports = {
                            let base: u64 = 5_000;
                            let extra = (opportunity.profit_bps as u64) * 20;
                            (base + extra).min(50_000)
                        };
                        // CU cost: (price_micro_lamports_per_cu * cu) / 1_000_000
                        let cu_cost_lamports = (sim.estimated_cu_consumed as u128
                            * self.cu_price_micro_lamports as u128)
                            / 1_000_000u128;
                        let total_cost = bundle_fee_estimate
                            .saturating_add(priority_fee_lamports)
                            .saturating_add(cu_cost_lamports as u64);
                        execution_result.estimated_profit_lamports =
                            gross.saturating_sub(total_cost);
                        execution_result.estimated_bundle_fees_lamports = Some(bundle_fee_estimate);
                        execution_result.estimated_priority_fee_lamports =
                            Some(priority_fee_lamports);
                        execution_result.estimated_cu_cost_lamports = Some(cu_cost_lamports as u64);
                        // Min profit guard (env configurable)
                        let min_profit = std::env::var("MIN_PROFIT_LAMPORTS")
                            .ok()
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0);
                        execution_result.min_profit_threshold_lamports = Some(min_profit);
                        if execution_result.estimated_profit_lamports < min_profit {
                            execution_result.status = ExecutionStatus::Rejected;
                            execution_result.error_message = Some("min_profit_not_met".into());
                            if let Some(m) = Arc::get_mut(&mut Arc::clone(&self.metrics)) {
                                let _ = m;
                            }
                            self.metrics.record_min_profit_rejection();
                            self.publish_execution_result(execution_result).await?;
                            return Ok(());
                        }
                        // Safety Caps (Notional & Priority Fee)
                        // MAX_TRADE_LAMPORTS: maximum allowed approximate lamport notional for input side.
                        // If non-SOL token, attempt oracle conversion to lamports.
                        if let Some(max_cap) = std::env::var("MAX_TRADE_LAMPORTS")
                            .ok()
                            .and_then(|v| v.parse::<u128>().ok())
                        {
                            let is_sol =
                                token_in_mint == "So11111111111111111111111111111111111111112";
                            let mut notional_lamports: u128 = opportunity.max_size_in; // assume lamports if SOL
                            if !is_sol {
                                if let Some(oracle) =
                                    self.account_registry.data.oracles.get(&token_in_mint)
                                {
                                    if let Ok(raw_price) = oracle.price.parse::<u128>() {
                                        let price_decimals =
                                            oracle.price_decimals.unwrap_or(9) as u32;
                                        let mint_decimals = self
                                            .account_registry
                                            .mint_decimals
                                            .get(&token_in_mint)
                                            .copied()
                                            .unwrap_or(9)
                                            as u32;
                                        let amount_adj = opportunity.max_size_in; // already in smallest units of token
                                                                                  // Normalize to 9 decimals canonical target
                                        let canonical_amount = if mint_decimals < 9 {
                                            amount_adj * 10u128.pow((9 - mint_decimals) as u32)
                                        } else if mint_decimals > 9 {
                                            amount_adj / 10u128.pow((mint_decimals - 9) as u32)
                                        } else {
                                            amount_adj
                                        };
                                        notional_lamports = (canonical_amount
                                            .saturating_mul(raw_price))
                                            / 10u128.pow(price_decimals);
                                    }
                                }
                            }
                            if notional_lamports > max_cap {
                                info!("safety_cap_reject reason=notional_exceeds_cap notional_lamports={} cap={} opp_id={} exec_id={} token_in_mint={}", notional_lamports, max_cap, opportunity.opportunity_id, execution_id, token_in_mint);
                                execution_result.status = ExecutionStatus::Rejected;
                                execution_result.error_message =
                                    Some("trade_notional_exceeds_cap".into());
                                self.publish_execution_result(execution_result).await?;
                                return Ok(());
                            }
                        }
                        if let Some(pfee) = execution_result.estimated_priority_fee_lamports {
                            if let Some(max_pfee) = std::env::var("MAX_PRIORITY_FEE_LAMPORTS")
                                .ok()
                                .and_then(|v| v.parse::<u64>().ok())
                            {
                                if pfee > max_pfee {
                                    info!("safety_cap_reject reason=priority_fee_exceeds_cap fee_lamports={} cap={} opp_id={} exec_id={}", pfee, max_pfee, opportunity.opportunity_id, execution_id);
                                    execution_result.status = ExecutionStatus::Rejected;
                                    execution_result.error_message =
                                        Some("priority_fee_exceeds_cap".into());
                                    self.publish_execution_result(execution_result).await?;
                                    return Ok(());
                                }
                            }
                        }
                        // Parse token deltas from simulation logs (placeholder heuristic)
                        let mut deltas: std::collections::HashMap<String, i64> =
                            std::collections::HashMap::new();
                        for line in sim.simulation_logs.iter() {
                            // Expected pattern example: "TOKEN_DELTA <mint> <signed_amount>"
                            if let Some(rest) = line.strip_prefix("TOKEN_DELTA ") {
                                let parts: Vec<&str> = rest.split_whitespace().collect();
                                if parts.len() == 2 {
                                    if let Ok(amount) = parts[1].parse::<i64>() {
                                        deltas.insert(parts[0].to_string(), amount);
                                        // Capture unknown mint for enrichment
                                        self.token_registry.note_seen_mint(parts[0]);
                                    }
                                }
                            }
                        }
                        if !deltas.is_empty() {
                            execution_result.simulated_token_deltas = Some(deltas);
                        }
                    } else {
                        execution_result.status = ExecutionStatus::Failed;
                        execution_result.error_message = sim.error_message;
                        self.publish_execution_result(execution_result).await?;
                        return Ok(());
                    }
                }
                Err(e) => {
                    execution_result.status = ExecutionStatus::Failed;
                    execution_result.error_message = Some(format!("Simulation error: {}", e));
                    self.publish_execution_result(execution_result).await?;
                    return Ok(());
                }
            }
        } else {
            execution_result.status = ExecutionStatus::Failed;
            execution_result.error_message = Some("No transactions to simulate".into());
            self.publish_execution_result(execution_result).await?;
            return Ok(());
        }

        // Step 5: Submit to Jito
        // Dry-run gating: by default we DO NOT submit unless explicitly enabled.
        // EXECUTION_DRY_RUN = 1 (default) will skip live submission regardless of allow-live flag.
        // To allow live submission: set EXECUTION_DRY_RUN=0 (or unset) AND EXECUTION_ALLOW_LIVE=1/true.
        let dry_run = std::env::var("EXECUTION_DRY_RUN")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true); // default dry-run ON
        let allow_live = std::env::var("EXECUTION_ALLOW_LIVE")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let live_flag = self
            .live_trading_enabled
            .load(std::sync::atomic::Ordering::Relaxed);
        if dry_run || !allow_live || !live_flag {
            info!(
                "execution_dry_run_skip submit_skipped=true dry_run={} allow_live={} live_flag={} est_profit_lamports={} txs={}",
                dry_run,
                allow_live,
                live_flag,
                execution_result.estimated_profit_lamports,
                execution_result.transaction_signatures.len()
            );
            // Ensure status remains Simulated (already set after simulation)
            execution_result.status = ExecutionStatus::Simulated;
            execution_result.execution_time_ms = start_time.elapsed().as_millis() as u64;
            self.publish_execution_result(execution_result).await?;
            return Ok(());
        }

        let priority_fee = execution_result.estimated_priority_fee_lamports.or(Some(0));

        info!(
            "execution_live_trade_summary exec_id={} opp_id={} profit_bps={} est_profit_lamports={} est_priority_fee_lamports={} tx_count={} first_sig={:?} token_in={} token_out={} max_size_in={} total_cu_estimate={} edges={} venues={} flash_loan_provider={:?}",
            execution_id,
            opportunity.opportunity_id,
            opportunity.profit_bps,
            execution_result.estimated_profit_lamports,
            priority_fee.unwrap_or(0),
            execution_result.transaction_signatures.len(),
            execution_result.transaction_signatures.first(),
            token_in_mint,
            token_out_mint,
            opportunity.max_size_in,
            opportunity.total_cu_estimate,
            opportunity.edges.len(),
            opportunity.venues.len(),
            execution_result.flash_loan_provider
        );
        match self
            .shyft_client
            .submit_jito_bundle(
                execution_result.transaction_signatures.clone(),
                priority_fee,
            )
            .await
        {
            Ok(resp) => {
                execution_result.status = ExecutionStatus::Submitted;
                execution_result.bundle_id = Some(resp.bundle_id.clone());
                self.metrics.record_bundle_submission();
                // Configurable polling parameters
                let max_attempts = std::env::var("BUNDLE_STATUS_MAX_ATTEMPTS")
                    .ok()
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(6);
                let base_delay_ms = std::env::var("BUNDLE_STATUS_BASE_DELAY_MS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(300);
                let backoff_factor = std::env::var("BUNDLE_STATUS_BACKOFF_FACTOR")
                    .ok()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(1.6);
                let jitter_pct = std::env::var("BUNDLE_STATUS_JITTER_PCT")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(20) as f64;
                let mut landed = false;
                for attempt in 0..max_attempts {
                    let ideal_delay =
                        (base_delay_ms as f64 * backoff_factor.powi(attempt as i32)) as u64;
                    let jitter_range = (ideal_delay as f64 * (jitter_pct / 100.0)) as u64;
                    let jitter = if jitter_range > 0 {
                        fastrand::u64(..=jitter_range)
                    } else {
                        0
                    };
                    tokio::time::sleep(Duration::from_millis(ideal_delay + jitter)).await;
                    if let Some(bid) = &execution_result.bundle_id {
                        match self.shyft_client.get_bundle_status(bid).await {
                            Ok(stat) => {
                                if stat.status.eq_ignore_ascii_case("landed") {
                                    execution_result.status = ExecutionStatus::LandedPendingProfit;
                                    if let Some(ref deltas) =
                                        execution_result.simulated_token_deltas
                                    {
                                        let total = self.convert_token_deltas_to_lamports(deltas);
                                        if total != 0 {
                                            execution_result.actual_profit_lamports = Some(total);
                                        } else {
                                            execution_result.actual_profit_lamports = Some(
                                                execution_result.estimated_profit_lamports as i64,
                                            );
                                        }
                                    } else {
                                        execution_result.actual_profit_lamports =
                                            Some(execution_result.estimated_profit_lamports as i64);
                                    }
                                    execution_result.status = ExecutionStatus::Confirmed;
                                    if let Some(realized) = execution_result.actual_profit_lamports
                                    {
                                        self.record_realized_pnl(
                                            execution_result.estimated_profit_lamports,
                                            realized,
                                        );
                                    }
                                    self.metrics.record_bundle_confirmation(true);
                                    landed = true;
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("bundle_status_check_failed attempt={} err={}", attempt, e)
                            }
                        }
                    }
                }
                if !landed {
                    execution_result.status = ExecutionStatus::Unlanded;
                    execution_result
                        .error_message
                        .get_or_insert("bundle_not_landed".into());
                    self.metrics.record_bundle_unlanded();
                }
            }
            Err(e) => {
                execution_result.status = ExecutionStatus::Failed;
                execution_result.error_message = Some(format!("bundle_submission_error: {}", e));
                self.metrics.record_bundle_submit_failure();
            }
        }

        // Update execution time
        execution_result.execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Publish result
        self.publish_execution_result(execution_result).await?;

        Ok(())
    }

    async fn publish_execution_result(&self, result: ExecutionResult) -> Result<()> {
        let result_json = serde_json::to_string(&result)?;
        let execution_id = result.execution_id;

        // Compute hashes
        let config_hash = std::env::var("CONFIG_HASH").ok();
        let token_registry_hash = self.compute_registry_hash();

        // Persist (best-effort) execution result
        let executed_at_ts = (result.executed_at_ns / 1_000_000_000) as i64;
        let executed_at = chrono::DateTime::<chrono::Utc>::from_timestamp(executed_at_ts, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        if let Err(e) = sqlx::query(r#"INSERT INTO execution_results (
                id, opportunity_id, status, bundle_id, transaction_signatures,
                estimated_profit_lamports, actual_profit_lamports, execution_time_ms, gas_used,
                error_message, executed_at, flash_loan_provider, flash_loan_fee_lamports,
                estimated_bundle_fees_lamports, estimated_priority_fee_lamports, estimated_cu_cost_lamports,
                min_profit_threshold_lamports, simulated_token_deltas, config_hash, token_registry_hash
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                actual_profit_lamports = EXCLUDED.actual_profit_lamports,
                execution_time_ms = EXCLUDED.execution_time_ms,
                error_message = EXCLUDED.error_message,
                config_hash = COALESCE(EXCLUDED.config_hash, execution_results.config_hash),
                token_registry_hash = COALESCE(EXCLUDED.token_registry_hash, execution_results.token_registry_hash)"#)
            .bind(result.execution_id)
            .bind(result.opportunity_id)
            .bind(format!("{:?}", result.status))
            .bind(result.bundle_id)
            .bind(&result.transaction_signatures as &Vec<String>)
            .bind(result.estimated_profit_lamports as i64)
            .bind(result.actual_profit_lamports)
            .bind(result.execution_time_ms as i64)
            .bind(result.gas_used as i32)
            .bind(result.error_message.clone())
            .bind(executed_at)
            .bind(result.flash_loan_provider.clone())
            .bind(result.flash_loan_fee_lamports.map(|v| v as i64))
            .bind(result.estimated_bundle_fees_lamports.map(|v| v as i64))
            .bind(result.estimated_priority_fee_lamports.map(|v| v as i64))
            .bind(result.estimated_cu_cost_lamports.map(|v| v as i64))
            .bind(result.min_profit_threshold_lamports.map(|v| v as i64))
            .bind(result
                .simulated_token_deltas
                .as_ref()
                .map(|m| serde_json::to_value(m).ok())
                .flatten())
            .bind(config_hash)
            .bind(token_registry_hash)
            .execute(&self.db).await {
            warn!("execution_result_persist_failed exec_id={} err={}", execution_id, e);
        }

        // Kafka key must implement ToBytes; use string form of UUID
        let key = execution_id.to_string();
        let record = FutureRecord::to(&self.config.results_topic)
            .key(&key)
            .payload(&result_json);

        self.producer
            .send(record, Duration::from_millis(100))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish execution result: {:?}", e))?;

        info!(
            "Published execution result: {} (status: {:?}, profit: {} lamports)",
            result.execution_id, result.status, result.estimated_profit_lamports
        );

        // Circuit breaker update (non-blocking logic) - run after logging but before returning
        self.update_circuit_breaker_status(&result.status);

        // Replace file-based hash-chain with in-memory global logger
        {
            use crate::hash_chain_logger::global_logger;
            let line = format!(
                "exec|{}|{}|{}|{}|{}",
                result.execution_id,
                result.opportunity_id,
                result.estimated_profit_lamports,
                result.actual_profit_lamports.unwrap_or_default(),
                format!("{:?}", result.status)
            );
            global_logger().append(&line);
        }
        Ok(())
    }
}

impl ExecutionEngine {
    pub fn enable_live_trading(&self) {
        self.live_trading_enabled
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.metrics.trading_enable_events_total.inc();
        self.metrics.trading_enabled_gauge.set(1);
        // Reset circuit breaker state on manual enable
        self.circuit_consec_failures
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.circuit_consec_unlanded
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.circuit_open
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.metrics.circuit_breaker_open_gauge.set(0);
        self.metrics.circuit_breaker_consec_failures.set(0);
        self.metrics.circuit_breaker_consec_unlanded.set(0);
        log::info!("trading_enable_event source=api state=enabled");
    }
    pub fn disable_live_trading(&self) {
        self.live_trading_enabled
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.metrics.trading_disable_events_total.inc();
        self.metrics.trading_enabled_gauge.set(0);
        log::warn!("trading_disable_event source=api state=disabled");
    }
    pub fn live_trading_status(&self) -> bool {
        self.live_trading_enabled
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    fn update_circuit_breaker_status(&self, status: &ExecutionStatus) {
        use crate::audit_log::log_event;
        let max_fail = std::env::var("CB_MAX_CONSECUTIVE_FAILURES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10);
        let max_unlanded = std::env::var("CB_MAX_CONSECUTIVE_UNLANDED")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);
        let status_str = format!("{:?}", status);
        match status {
            ExecutionStatus::Confirmed => {
                // reset counters
                self.circuit_consec_failures
                    .store(0, std::sync::atomic::Ordering::SeqCst);
                self.circuit_consec_unlanded
                    .store(0, std::sync::atomic::Ordering::SeqCst);
                self.metrics.circuit_breaker_consec_failures.set(0);
                self.metrics.circuit_breaker_consec_unlanded.set(0);
                return;
            }
            ExecutionStatus::Failed | ExecutionStatus::Rejected => {
                let v = self
                    .circuit_consec_failures
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                self.metrics.circuit_breaker_consec_failures.set(v as i64);
            }
            ExecutionStatus::Unlanded => {
                let v = self
                    .circuit_consec_unlanded
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                    + 1;
                self.metrics.circuit_breaker_consec_unlanded.set(v as i64);
            }
            _ => {}
        }
        let f = self
            .circuit_consec_failures
            .load(std::sync::atomic::Ordering::Relaxed);
        let u = self
            .circuit_consec_unlanded
            .load(std::sync::atomic::Ordering::Relaxed);
        if !self.circuit_open.load(std::sync::atomic::Ordering::Relaxed)
            && (f >= max_fail || u >= max_unlanded)
        {
            if self
                .circuit_open
                .compare_exchange(
                    false,
                    true,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::Relaxed,
                )
                .is_ok()
            {
                self.metrics.circuit_breaker_trips_total.inc();
                self.metrics.circuit_breaker_open_gauge.set(1);
                // Auto-disable live trading
                self.live_trading_enabled
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                self.metrics.trading_disable_events_total.inc();
                self.metrics.trading_enabled_gauge.set(0);
                log::error!(
                    "circuit_breaker_trip trading_auto_disabled failures={} unlanded={} max_fail={} max_unlanded={}",
                    f, u, max_fail, max_unlanded
                );
                let extra = serde_json::json!({
                    "failures": f,
                    "unlanded": u,
                    "max_fail": max_fail,
                    "max_unlanded": max_unlanded,
                    "last_status": status_str
                });
                log_event("circuit_breaker_trip", None, None, true, true, Some(&extra));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account_registry::{AccountRegistry, AccountRegistryData, OracleAccount};
    use crate::metrics::MetricsServer;
    use sqlx::PgPool;
    use std::collections::HashMap;

    // Helper to build minimal engine with injected registry
    async fn test_engine_with_oracle(
        price: &str,
        price_decimals: u8,
        mint: &str,
        mint_decimals: u8,
    ) -> ExecutionEngine {
        let cfg = ExecutorConfig {
            kafka_broker: "unused".into(),
            opportunities_topic: "unused".into(),
            results_topic: "unused".into(),
            consumer_group: "unused".into(),
            shyft_api_key: "x".into(),
            shyft_rpc_url: "http://localhost".into(),
            shyft_geyser_url: "http://localhost".into(),
            shyft_jito_url: "http://localhost".into(),
            shyft_geyser_x_token: "".into(),
            wallet_keypair_path: "~/.config/solana/id.json".into(),
            wallet_backup_paths: vec![],
            max_wallet_balance_sol: 100.0,
            min_wallet_balance_sol: 1.0,
            max_position_size_sol: 10.0,
            min_profit_threshold_bps: 10,
            max_slippage_bps: 50,
            max_execution_time_ms: 1000,
            flash_loan_providers: vec!["solend".into()],
            max_flash_loan_amount_sol: 1000.0,
            flash_loan_timeout_ms: 1000,
            daily_loss_limit_sol: 5.0,
            max_concurrent_trades: 1,
            cool_down_period_ms: 1000,
            metrics_port: 0,
            health_check_port: 0,
            log_level: "info".into(),
            shyft_graphql_url: None,
            shyft_meta_api_key: None,
        };
        let metrics = Arc::new(MetricsServer::new(0).unwrap());
        // Use a separate test database (requires TEST_DATABASE_URL or falls back to local) - best effort
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://mev:mevpassword@localhost:5433/mevdb".into());
        let pool = PgPool::connect(&db_url).await.expect("pg connect");
        let mut eng = ExecutionEngine::new(cfg, Arc::clone(&metrics), pool)
            .await
            .unwrap();
        // Inject oracle & decimals
        let mut data = AccountRegistryData::default();
        data.oracles.insert(
            mint.to_string(),
            OracleAccount {
                price: price.into(),
                price_decimals: Some(price_decimals),
                confidence_limit_bps: None,
            },
        );
        let mut reg = AccountRegistry {
            data,
            missing_keys: vec![],
            validation_errors: vec![],
            mint_decimals: HashMap::new(),
        };
        reg.mint_decimals.insert(mint.to_string(), mint_decimals);
        eng.account_registry = Arc::new(reg);
        eng
    }

    #[tokio::test]
    async fn test_notional_cap_compute_sol_passthrough() {
        let engine =
            test_engine_with_oracle("1", 0, "So11111111111111111111111111111111111111112", 9).await;
        let amount = 5_000_000u128; // 0.005 SOL lamports
        let notional = engine
            .compute_input_notional_lamports("So11111111111111111111111111111111111111112", amount);
        assert_eq!(notional, amount);
    }

    #[tokio::test]
    async fn test_notional_cap_compute_token_scaled_up() {
        // token with 6 decimals, price = 2 lamports per canonical 9-dec unit (price_decimals=0 for simplicity)
        let engine = test_engine_with_oracle("2", 0, "TokenMint6Dec", 6).await;
        let amount_smallest = 1_000_000u128; // 1 token (6 decimals)
                                             // canonical amount => multiply by 10^(9-6)=10^3=1000 -> 1_000_000 * 1000 = 1_000_000_000
                                             // notional = canonical_amount * price = 1_000_000_000 * 2 = 2_000_000_000
        let notional = engine.compute_input_notional_lamports("TokenMint6Dec", amount_smallest);
        assert_eq!(notional, 2_000_000_000u128);
    }

    #[tokio::test]
    async fn test_notional_cap_compute_token_scaled_down() {
        // token with 12 decimals (scale down)
        let engine = test_engine_with_oracle("100", 2, "TokenMint12Dec", 12).await; // price=100, price_decimals=2 => effective price=1 lamport per canonical unit
        let amount_smallest = 1_000_000_000_000u128; // 1 token (12 decimals)
                                                     // canonical amount divide by 10^(12-9)=10^3 => 1_000_000_000_000 / 1000 = 1_000_000_000
                                                     // raw_price=100, price_decimals=2 => scale_div=10^2=100 -> (1_000_000_000 * 100)/100 = 1_000_000_000
        let notional = engine.compute_input_notional_lamports("TokenMint12Dec", amount_smallest);
        assert_eq!(notional, 1_000_000_000u128);
    }
}

impl ExecutionEngine {
    pub async fn process_hotpath_opportunity(
        &self,
        opportunity: ArbitrageOpportunity,
    ) -> Result<()> {
        // Duplicate core of process_opportunity sans Kafka decode
        let execution_id = Uuid::new_v4();
        let mut execution_result = ExecutionResult {
            opportunity_id: opportunity.opportunity_id,
            execution_id,
            status: ExecutionStatus::Received,
            bundle_id: None,
            transaction_signatures: vec![],
            estimated_profit_lamports: (opportunity.max_size_in as u64 * opportunity.profit_bps)
                / 10_000,
            actual_profit_lamports: None,
            execution_time_ms: 0,
            gas_used: opportunity.total_cu_estimate,
            error_message: None,
            executed_at_ns: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            flash_loan_provider: None,
            flash_loan_fee_lamports: None,
            estimated_bundle_fees_lamports: None,
            estimated_priority_fee_lamports: None,
            estimated_cu_cost_lamports: None,
            min_profit_threshold_lamports: None,
            simulated_token_deltas: None,
        };
        // Validate
        if !self
            .opportunity_processor
            .validate_opportunity(&opportunity)
            .await
            .unwrap_or(false)
        {
            execution_result.status = ExecutionStatus::Rejected;
            self.publish_execution_result(execution_result).await?;
            return Ok(());
        }
        execution_result.status = ExecutionStatus::Validated;
        // Flash loan quote
        let token_in_mint = bs58::encode(&opportunity.token_in).into_string();
        if let Ok(quote) = self
            .flash_loan_aggregator
            .get_best_quote(&token_in_mint, opportunity.max_size_in as u64)
            .await
        {
            execution_result.status = ExecutionStatus::FlashLoanQuoted;
            execution_result.flash_loan_provider = Some(format!("{:?}", quote.provider));
            execution_result.flash_loan_fee_lamports =
                Some((opportunity.max_size_in as u64 * quote.fee_bps as u64) / 10_000);
        } else {
            execution_result.status = ExecutionStatus::Failed;
            execution_result.error_message = Some("flash_loan_quote_failed".into());
            self.publish_execution_result(execution_result).await?;
            return Ok(());
        }
        // Bundle build
        match self
            .bundle_builder
            .build_arbitrage_bundle(
                &opportunity,
                execution_result
                    .flash_loan_provider
                    .as_ref()
                    .and_then(|s| crate::flash_loan::FlashLoanProvider::from_string(s)),
            )
            .await
        {
            Ok(bundle) => {
                execution_result.status = ExecutionStatus::BundleBuilt;
                execution_result.transaction_signatures = bundle.transaction_ids;
            }
            Err(e) => {
                execution_result.status = ExecutionStatus::Failed;
                execution_result.error_message = Some(format!("bundle_build_error: {}", e));
                self.publish_execution_result(execution_result).await?;
                return Ok(());
            }
        }
        execution_result.status = ExecutionStatus::Simulated; // future simulation
        execution_result.status = ExecutionStatus::Submitted; // future: Jito submission
        execution_result.bundle_id = Some(format!("bundle_{}", execution_id));
        self.publish_execution_result(execution_result).await?;
        Ok(())
    }
}
