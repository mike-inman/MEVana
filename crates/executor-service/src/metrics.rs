use anyhow::Result;
use prometheus::{
    Counter, Encoder, Gauge, GaugeVec, Histogram, IntCounter, IntGauge, Registry, TextEncoder,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::Filter;

/// Prometheus metrics server for monitoring executor performance
#[allow(dead_code)]
pub struct MetricsServer {
    registry: Arc<Registry>,
    port: u16,

    // Execution metrics
    pub opportunities_received: IntCounter,
    pub opportunities_executed: IntCounter,
    pub opportunities_rejected: IntCounter,
    pub execution_time_seconds: Histogram,
    pub profit_lamports: Counter,
    pub loss_lamports: Counter,

    // Wallet metrics
    pub wallet_balance_sol: GaugeVec,
    pub daily_pnl_sol: Gauge,
    pub active_trades: IntGauge,

    // Flash loan metrics
    pub flash_loan_requests: IntCounter,
    pub flash_loan_successes: IntCounter,
    pub flash_loan_failures: IntCounter,
    pub flash_loan_fees_lamports: Counter,

    // System metrics
    pub bundle_submissions: IntCounter,
    pub bundle_confirmations: IntCounter,
    pub bundle_failures: IntCounter,
    pub bundle_unlanded: IntCounter,
    pub bundle_failed_submit: IntCounter,
    pub system_errors: IntCounter,

    // Hotpath / low-latency feed metrics
    pub hotpath_ws_reconnections: IntCounter,
    pub hotpath_ws_errors: IntCounter,
    pub hotpath_ws_messages: IntCounter,
    pub hotpath_opportunities_processed: IntCounter,
    pub hotpath_processing_time_seconds: Histogram,
    // Pre-bake / template metrics
    pub route_template_hits: IntCounter,
    pub route_template_misses: IntCounter,
    pub blockhash_refreshes: IntCounter,
    // Simulation / build metrics
    pub simulation_requests: IntCounter,
    pub simulation_successes: IntCounter,
    pub simulation_failures: IntCounter,
    pub simulation_latency_seconds: Histogram,
    pub transactions_built: IntCounter,
    pub instructions_instantiated: IntCounter,
    pub route_template_build_time_seconds: Histogram,
    // Account registry
    pub account_sets_missing: IntGauge,
    pub account_sets_loaded: IntGauge,
    pub account_metas_placeholders: IntCounter,
    pub account_metas_real: IntCounter,
    pub account_registry_validation_errors: IntGauge,
    pub account_registry_freshness_unix: IntGauge,
    pub min_profit_rejections: IntCounter,
    pub realized_profit_lamports: Counter,
    pub profit_slippage_lamports: Counter,
    pub profit_overestimation_lamports: Counter,
    pub profit_underestimation_lamports: Counter,
    pub account_sets_required_missing: IntGauge,
    pub profit_slippage_ratio: Histogram,
    // Admin / security
    pub admin_requests_total: IntCounter,
    pub admin_refresh_requests_total: IntCounter,
    pub admin_unauthorized_total: IntCounter,
    pub admin_rate_limited_total: IntCounter,
    pub admin_dual_auth_required_total: IntCounter,
    pub admin_subject_missing_total: IntCounter,
    pub admin_subject_allowed_total: IntCounter,
    pub admin_token_valid_total: IntCounter,
    pub admin_token_invalid_total: IntCounter,
    // Trading control
    pub trading_enable_events_total: IntCounter,
    pub trading_disable_events_total: IntCounter,
    pub trading_enabled_gauge: IntGauge,
    // Circuit breaker
    pub circuit_breaker_trips_total: IntCounter,
    pub circuit_breaker_open_gauge: IntGauge,
    pub circuit_breaker_consec_failures: IntGauge,
    pub circuit_breaker_consec_unlanded: IntGauge,
    // Dedupe
    pub opportunity_dedupe_hits_total: IntCounter,
    pub opportunity_dedupe_inserts_total: IntCounter,
    pub opportunity_dedupe_evictions_total: IntCounter,
    // Reconciliation metrics
    pub reconciliation_started_total: IntCounter,
    pub reconciliation_success_total: IntCounter,
    pub reconciliation_failure_total: IntCounter,
    pub reconciliation_adjustments_total: IntCounter,
    pub reconciliation_adjustment_lamports_total: Counter,
    pub reconciliation_divergence_bps: Histogram,
    pub reconciliation_last_divergence_bps: IntGauge,
}

#[allow(dead_code)]
impl MetricsServer {
    pub fn new(port: u16) -> Result<Self> {
        let registry = Arc::new(Registry::new());

        // Initialize execution metrics
        let opportunities_received = IntCounter::new(
            "mevana_opportunities_received_total",
            "Total number of arbitrage opportunities received",
        )?;
        registry.register(Box::new(opportunities_received.clone()))?;

        let opportunities_executed = IntCounter::new(
            "mevana_opportunities_executed_total",
            "Total number of arbitrage opportunities executed",
        )?;
        registry.register(Box::new(opportunities_executed.clone()))?;

        let opportunities_rejected = IntCounter::new(
            "mevana_opportunities_rejected_total",
            "Total number of arbitrage opportunities rejected",
        )?;
        registry.register(Box::new(opportunities_rejected.clone()))?;

        let execution_time_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "mevana_execution_time_seconds",
                "Time taken to execute arbitrage opportunities",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0]),
        )?;
        registry.register(Box::new(execution_time_seconds.clone()))?;

        let profit_lamports =
            Counter::new("mevana_profit_lamports_total", "Total profit in lamports")?;
        registry.register(Box::new(profit_lamports.clone()))?;

        let loss_lamports = Counter::new("mevana_loss_lamports_total", "Total losses in lamports")?;
        registry.register(Box::new(loss_lamports.clone()))?;

        // Initialize wallet metrics
        let wallet_balance_sol = GaugeVec::new(
            prometheus::Opts::new("mevana_wallet_balance_sol", "Current wallet balance in SOL"),
            &["wallet"],
        )?;
        registry.register(Box::new(wallet_balance_sol.clone()))?;

        let daily_pnl_sol = Gauge::new("mevana_daily_pnl_sol", "Daily profit/loss in SOL")?;
        registry.register(Box::new(daily_pnl_sol.clone()))?;

        let active_trades =
            IntGauge::new("mevana_active_trades", "Number of currently active trades")?;
        registry.register(Box::new(active_trades.clone()))?;

        // Initialize flash loan metrics
        let flash_loan_requests = IntCounter::new(
            "mevana_flash_loan_requests_total",
            "Total number of flash loan requests",
        )?;
        registry.register(Box::new(flash_loan_requests.clone()))?;

        let flash_loan_successes = IntCounter::new(
            "mevana_flash_loan_successes_total",
            "Total number of successful flash loans",
        )?;
        registry.register(Box::new(flash_loan_successes.clone()))?;

        let flash_loan_failures = IntCounter::new(
            "mevana_flash_loan_failures_total",
            "Total number of failed flash loans",
        )?;
        registry.register(Box::new(flash_loan_failures.clone()))?;

        let flash_loan_fees_lamports = Counter::new(
            "mevana_flash_loan_fees_lamports_total",
            "Total flash loan fees paid in lamports",
        )?;
        registry.register(Box::new(flash_loan_fees_lamports.clone()))?;

        // Initialize system metrics
        let bundle_submissions = IntCounter::new(
            "mevana_bundle_submissions_total",
            "Total number of Jito bundle submissions",
        )?;
        registry.register(Box::new(bundle_submissions.clone()))?;

        let bundle_confirmations = IntCounter::new(
            "mevana_bundle_confirmations_total",
            "Total number of confirmed Jito bundles",
        )?;
        registry.register(Box::new(bundle_confirmations.clone()))?;
        let bundle_failures = IntCounter::new(
            "mevana_bundle_failures_total",
            "Total number of failed Jito bundles",
        )?;
        registry.register(Box::new(bundle_failures.clone()))?;
        let bundle_unlanded = IntCounter::new(
            "mevana_bundle_unlanded_total",
            "Bundles that never landed within polling window",
        )?;
        let bundle_failed_submit = IntCounter::new(
            "mevana_bundle_failed_submit_total",
            "Bundles that failed at submission stage",
        )?;
        registry.register(Box::new(bundle_unlanded.clone()))?;
        registry.register(Box::new(bundle_failed_submit.clone()))?;

        let system_errors = IntCounter::new(
            "mevana_system_errors_total",
            "Total number of system errors",
        )?;
        registry.register(Box::new(system_errors.clone()))?;

        // Hotpath metrics
        let hotpath_ws_reconnections = IntCounter::new(
            "mevana_hotpath_ws_reconnections_total",
            "Total number of hotpath websocket reconnections",
        )?;
        registry.register(Box::new(hotpath_ws_reconnections.clone()))?;

        let hotpath_ws_errors = IntCounter::new(
            "mevana_hotpath_ws_errors_total",
            "Total number of hotpath websocket errors",
        )?;
        registry.register(Box::new(hotpath_ws_errors.clone()))?;

        let hotpath_ws_messages = IntCounter::new(
            "mevana_hotpath_ws_messages_total",
            "Total number of raw websocket messages received on hotpath",
        )?;
        registry.register(Box::new(hotpath_ws_messages.clone()))?;

        let hotpath_opportunities_processed = IntCounter::new(
            "mevana_hotpath_opportunities_processed_total",
            "Total number of hotpath arbitrage opportunities processed",
        )?;
        registry.register(Box::new(hotpath_opportunities_processed.clone()))?;

        let hotpath_processing_time_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "mevana_hotpath_processing_time_seconds",
                "End-to-end processing time for hotpath opportunities",
            )
            .buckets(vec![0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1]),
        )?;
        registry.register(Box::new(hotpath_processing_time_seconds.clone()))?;

        // Pre-bake / template metrics
        let route_template_hits = IntCounter::new(
            "mevana_route_template_hits_total",
            "Total number of route template cache hits",
        )?;
        registry.register(Box::new(route_template_hits.clone()))?;

        let route_template_misses = IntCounter::new(
            "mevana_route_template_misses_total",
            "Total number of route template cache misses (materializations)",
        )?;
        registry.register(Box::new(route_template_misses.clone()))?;

        let blockhash_refreshes = IntCounter::new(
            "mevana_blockhash_refreshes_total",
            "Total number of successful recent blockhash refreshes",
        )?;
        registry.register(Box::new(blockhash_refreshes.clone()))?;

        // Simulation / build metrics
        let simulation_requests = IntCounter::new(
            "mevana_simulation_requests_total",
            "Total number of bundle simulation requests",
        )?;
        registry.register(Box::new(simulation_requests.clone()))?;
        let simulation_successes = IntCounter::new(
            "mevana_simulation_successes_total",
            "Total number of successful bundle simulations",
        )?;
        registry.register(Box::new(simulation_successes.clone()))?;
        let simulation_failures = IntCounter::new(
            "mevana_simulation_failures_total",
            "Total number of failed bundle simulations",
        )?;
        registry.register(Box::new(simulation_failures.clone()))?;
        let simulation_latency_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "mevana_simulation_latency_seconds",
                "Latency of bundle simulations",
            )
            .buckets(vec![0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0]),
        )?;
        registry.register(Box::new(simulation_latency_seconds.clone()))?;
        let transactions_built = IntCounter::new(
            "mevana_transactions_built_total",
            "Total number of transactions constructed from templates",
        )?;
        registry.register(Box::new(transactions_built.clone()))?;
        let instructions_instantiated = IntCounter::new(
            "mevana_instructions_instantiated_total",
            "Total number of instructions instantiated from templates",
        )?;
        registry.register(Box::new(instructions_instantiated.clone()))?;
        let route_template_build_time_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "mevana_route_template_build_time_seconds",
                "Time to materialize a new route template",
            )
            .buckets(vec![0.0005, 0.001, 0.002, 0.005, 0.01, 0.02]),
        )?;
        registry.register(Box::new(route_template_build_time_seconds.clone()))?;

        // Account registry gauges
        let account_sets_missing = IntGauge::new(
            "mevana_account_sets_missing",
            "Count of required account sets missing from registry",
        )?;
        registry.register(Box::new(account_sets_missing.clone()))?;
        let account_sets_loaded = IntGauge::new(
            "mevana_account_sets_loaded",
            "Count of distinct account sets loaded",
        )?;
        registry.register(Box::new(account_sets_loaded.clone()))?;

        // Account meta resolution quality counters
        let account_metas_placeholders = IntCounter::new(
            "mevana_account_metas_placeholders_total",
            "Total placeholder account metas used (fallback)",
        )?;
        registry.register(Box::new(account_metas_placeholders.clone()))?;
        let account_metas_real = IntCounter::new(
            "mevana_account_metas_real_total",
            "Total real (registry-derived) account metas used",
        )?;
        registry.register(Box::new(account_metas_real.clone()))?;

        let account_registry_validation_errors = IntGauge::new(
            "mevana_account_registry_validation_errors",
            "Number of validation errors detected in account registry",
        )?;
        registry.register(Box::new(account_registry_validation_errors.clone()))?;

        let account_registry_freshness_unix = IntGauge::new(
            "mevana_account_registry_freshness_unix",
            "Unix timestamp (seconds) when registry file was last loaded",
        )?;
        registry.register(Box::new(account_registry_freshness_unix.clone()))?;

        let min_profit_rejections = IntCounter::new(
            "mevana_min_profit_rejections_total",
            "Total opportunities rejected due to not meeting minimum profit threshold",
        )?;
        registry.register(Box::new(min_profit_rejections.clone()))?;
        let realized_profit_lamports = Counter::new(
            "mevana_realized_profit_lamports_total",
            "Sum of realized (actual) profit across landed bundles in lamports",
        )?;
        registry.register(Box::new(realized_profit_lamports.clone()))?;
        let profit_slippage_lamports = Counter::new(
            "mevana_profit_slippage_lamports_total",
            "Cumulative (estimated - realized) profit delta (lamports, positive means overestimation)",
        )?;
        registry.register(Box::new(profit_slippage_lamports.clone()))?;
        let profit_overestimation_lamports = Counter::new(
            "mevana_profit_overestimation_lamports_total",
            "Sum of (estimated - realized) where estimated > realized (lamports)",
        )?;
        registry.register(Box::new(profit_overestimation_lamports.clone()))?;
        let profit_underestimation_lamports = Counter::new(
            "mevana_profit_underestimation_lamports_total",
            "Sum of (realized - estimated) where realized > estimated (lamports)",
        )?;
        registry.register(Box::new(profit_underestimation_lamports.clone()))?;
        let profit_slippage_ratio = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "mevana_profit_slippage_ratio",
                "Distribution of realized/estimated profit ratio",
            )
            .buckets(vec![0.0, 0.25, 0.5, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 2.0]),
        )?;
        registry.register(Box::new(profit_slippage_ratio.clone()))?;
        let account_sets_required_missing = IntGauge::new(
            "mevana_account_sets_required_missing",
            "Count of hard-coded required account keys missing (MISSING_REQUIRED)",
        )?;
        registry.register(Box::new(account_sets_required_missing.clone()))?;

        // Admin / security metrics
        let admin_requests_total = IntCounter::new(
            "mevana_admin_requests_total",
            "Total admin endpoint requests (any method)",
        )?;
        registry.register(Box::new(admin_requests_total.clone()))?;
        let admin_refresh_requests_total = IntCounter::new(
            "mevana_admin_refresh_requests_total",
            "Total registry refresh attempts",
        )?;
        registry.register(Box::new(admin_refresh_requests_total.clone()))?;
        let admin_unauthorized_total = IntCounter::new(
            "mevana_admin_unauthorized_total",
            "Admin requests rejected due to failed auth (token or subject)",
        )?;
        registry.register(Box::new(admin_unauthorized_total.clone()))?;
        let admin_rate_limited_total = IntCounter::new(
            "mevana_admin_rate_limited_total",
            "Admin requests rejected due to rate limiting",
        )?;
        registry.register(Box::new(admin_rate_limited_total.clone()))?;
        let admin_dual_auth_required_total = IntCounter::new(
            "mevana_admin_dual_auth_required_total",
            "Admin requests where dual auth (token+subject) was enforced",
        )?;
        registry.register(Box::new(admin_dual_auth_required_total.clone()))?;
        let admin_subject_missing_total = IntCounter::new(
            "mevana_admin_subject_missing_total",
            "Admin requests missing client subject when subject allowlist configured",
        )?;
        registry.register(Box::new(admin_subject_missing_total.clone()))?;
        let admin_subject_allowed_total = IntCounter::new(
            "mevana_admin_subject_allowed_total",
            "Admin requests with allowed client subject",
        )?;
        registry.register(Box::new(admin_subject_allowed_total.clone()))?;
        let admin_token_valid_total = IntCounter::new(
            "mevana_admin_token_valid_total",
            "Admin requests with valid API token",
        )?;
        registry.register(Box::new(admin_token_valid_total.clone()))?;
        let admin_token_invalid_total = IntCounter::new(
            "mevana_admin_token_invalid_total",
            "Admin requests with invalid API token",
        )?;
        registry.register(Box::new(admin_token_invalid_total.clone()))?;

        // Trading control metrics
        let trading_enable_events_total = IntCounter::new(
            "mevana_trading_enable_events_total",
            "Total number of trading enable events (kill switch released)",
        )?;
        registry.register(Box::new(trading_enable_events_total.clone()))?;
        let trading_disable_events_total = IntCounter::new(
            "mevana_trading_disable_events_total",
            "Total number of trading disable events (kill switch engaged)",
        )?;
        registry.register(Box::new(trading_disable_events_total.clone()))?;
        let trading_enabled_gauge = IntGauge::new(
            "mevana_trading_enabled",
            "Gauge 1 when live trading enabled (kill switch off)",
        )?;
        registry.register(Box::new(trading_enabled_gauge.clone()))?;

        // Circuit breaker metrics
        let circuit_breaker_trips_total = IntCounter::new(
            "mevana_circuit_breaker_trips_total",
            "Total number of times the circuit breaker tripped (auto disabled trading)",
        )?;
        registry.register(Box::new(circuit_breaker_trips_total.clone()))?;
        let circuit_breaker_open_gauge = IntGauge::new(
            "mevana_circuit_breaker_open",
            "Gauge 1 if circuit breaker currently open (trading blocked)",
        )?;
        registry.register(Box::new(circuit_breaker_open_gauge.clone()))?;
        let circuit_breaker_consec_failures = IntGauge::new(
            "mevana_circuit_breaker_consec_failures",
            "Current consecutive failed/rejected executions",
        )?;
        registry.register(Box::new(circuit_breaker_consec_failures.clone()))?;
        let circuit_breaker_consec_unlanded = IntGauge::new(
            "mevana_circuit_breaker_consec_unlanded",
            "Current consecutive unlanded executions",
        )?;
        registry.register(Box::new(circuit_breaker_consec_unlanded.clone()))?;

        // Dedupe metrics
        let opportunity_dedupe_hits_total = IntCounter::new(
            "mevana_opportunity_dedupe_hits_total",
            "Total number of opportunities dropped due to dedupe hit (recent duplicate)",
        )?;
        registry.register(Box::new(opportunity_dedupe_hits_total.clone()))?;
        let opportunity_dedupe_inserts_total = IntCounter::new(
            "mevana_opportunity_dedupe_inserts_total",
            "Total number of opportunities inserted into dedupe cache",
        )?;
        registry.register(Box::new(opportunity_dedupe_inserts_total.clone()))?;
        let opportunity_dedupe_evictions_total = IntCounter::new(
            "mevana_opportunity_dedupe_evictions_total",
            "Total number of dedupe cache evictions (TTL or capacity)",
        )?;
        registry.register(Box::new(opportunity_dedupe_evictions_total.clone()))?;

        // Reconciliation metrics
        let reconciliation_started_total = IntCounter::new(
            "mevana_reconciliation_started_total",
            "Total reconciliation tasks started",
        )?;
        registry.register(Box::new(reconciliation_started_total.clone()))?;
        let reconciliation_success_total = IntCounter::new(
            "mevana_reconciliation_success_total",
            "Total reconciliation tasks succeeded",
        )?;
        registry.register(Box::new(reconciliation_success_total.clone()))?;
        let reconciliation_failure_total = IntCounter::new(
            "mevana_reconciliation_failure_total",
            "Total reconciliation tasks failed",
        )?;
        registry.register(Box::new(reconciliation_failure_total.clone()))?;
        let reconciliation_adjustments_total = IntCounter::new(
            "mevana_reconciliation_adjustments_total",
            "Count of daily PnL adjustments applied after reconciliation",
        )?;
        registry.register(Box::new(reconciliation_adjustments_total.clone()))?;
        let reconciliation_adjustment_lamports_total = Counter::new(
            "mevana_reconciliation_adjustment_lamports_total",
            "Net lamports added (positive) or removed (negative aggregated) from daily PnL via reconciliation adjustments",
        )?;
        registry.register(Box::new(reconciliation_adjustment_lamports_total.clone()))?;
        let reconciliation_divergence_bps = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "mevana_reconciliation_divergence_bps",
                "Distribution of reconciliation divergence (abs divergence in bps)",
            )
            .buckets(vec![
                10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
            ]),
        )?;
        registry.register(Box::new(reconciliation_divergence_bps.clone()))?;
        let reconciliation_last_divergence_bps = IntGauge::new(
            "mevana_reconciliation_last_divergence_bps",
            "Most recent reconciliation divergence in bps (signed)",
        )?;
        registry.register(Box::new(reconciliation_last_divergence_bps.clone()))?;

        Ok(Self {
            registry,
            port,
            opportunities_received,
            opportunities_executed,
            opportunities_rejected,
            execution_time_seconds,
            profit_lamports,
            loss_lamports,
            wallet_balance_sol,
            daily_pnl_sol,
            active_trades,
            flash_loan_requests,
            flash_loan_successes,
            flash_loan_failures,
            flash_loan_fees_lamports,
            bundle_submissions,
            bundle_confirmations,
            bundle_failures,
            bundle_unlanded,
            bundle_failed_submit,
            system_errors,
            hotpath_ws_reconnections,
            hotpath_ws_errors,
            hotpath_ws_messages,
            hotpath_opportunities_processed,
            hotpath_processing_time_seconds,
            route_template_hits,
            route_template_misses,
            blockhash_refreshes,
            simulation_requests,
            simulation_successes,
            simulation_failures,
            simulation_latency_seconds,
            transactions_built,
            instructions_instantiated,
            route_template_build_time_seconds,
            account_sets_missing,
            account_sets_loaded,
            account_metas_placeholders,
            account_metas_real,
            account_registry_validation_errors,
            account_registry_freshness_unix,
            min_profit_rejections,
            realized_profit_lamports,
            profit_slippage_lamports,
            profit_overestimation_lamports,
            profit_underestimation_lamports,
            account_sets_required_missing,
            profit_slippage_ratio,
            admin_requests_total,
            admin_refresh_requests_total,
            admin_unauthorized_total,
            admin_rate_limited_total,
            admin_dual_auth_required_total,
            admin_subject_missing_total,
            admin_subject_allowed_total,
            admin_token_valid_total,
            admin_token_invalid_total,
            trading_enable_events_total,
            trading_disable_events_total,
            trading_enabled_gauge,
            circuit_breaker_trips_total,
            circuit_breaker_open_gauge,
            circuit_breaker_consec_failures,
            circuit_breaker_consec_unlanded,
            opportunity_dedupe_hits_total,
            opportunity_dedupe_inserts_total,
            opportunity_dedupe_evictions_total,
            reconciliation_started_total,
            reconciliation_success_total,
            reconciliation_failure_total,
            reconciliation_adjustments_total,
            reconciliation_adjustment_lamports_total,
            reconciliation_divergence_bps,
            reconciliation_last_divergence_bps,
        })
    }

    pub async fn start(&self) -> Result<()> {
        let registry = Arc::clone(&self.registry);

        let metrics_route = warp::path("metrics").and(warp::get()).map(move || {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();

            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                return warp::reply::with_status(
                    format!("Failed to encode metrics: {}", e),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                );
            }

            warp::reply::with_status(
                String::from_utf8_lossy(&buffer).to_string(),
                warp::http::StatusCode::OK,
            )
        });

        let health_route = warp::path("health")
            .and(warp::get())
            .map(|| warp::reply::with_status("OK".to_string(), warp::http::StatusCode::OK));

        let routes = metrics_route.or(health_route);

        let addr: SocketAddr = format!("0.0.0.0:{}", self.port).parse()?;
        log::info!("Starting metrics server on {}", addr);
        // Simpler run to avoid std listener -> tokio registration panic.
        warp::serve(routes).run(addr).await;
        Ok(())
    }

    /// Record execution result metrics
    pub fn record_execution_result(&self, success: bool, profit_loss: i64, execution_time: f64) {
        if success {
            self.opportunities_executed.inc();
            if profit_loss > 0 {
                self.profit_lamports.inc_by(profit_loss as f64);
            } else {
                self.loss_lamports.inc_by((-profit_loss) as f64);
            }
        } else {
            self.opportunities_rejected.inc();
        }

        self.execution_time_seconds.observe(execution_time);
    }

    pub fn record_flash_loan_quote_attempt(&self) {
        self.flash_loan_requests.inc();
    }

    pub fn record_flash_loan_plan_success(&self, fee_lamports: u64) {
        self.flash_loan_successes.inc();
        self.flash_loan_fees_lamports.inc_by(fee_lamports as f64);
    }

    pub fn record_flash_loan_plan_failure(&self) {
        self.flash_loan_failures.inc();
    }

    /// Update wallet metrics
    pub fn update_wallet_metrics(
        &self,
        wallet_label: &str,
        balance_sol: f64,
        daily_pnl_sol: f64,
        active_trades: i64,
        wallet_labels: &HashMap<String, String>,
    ) {
        if wallet_labels.is_empty() {
            self.wallet_balance_sol
                .with_label_values(&[wallet_label])
                .set(balance_sol);
        } else {
            for label in wallet_labels.values() {
                let gauge = self.wallet_balance_sol.with_label_values(&[label]);
                if label == wallet_label {
                    gauge.set(balance_sol);
                } else {
                    gauge.set(0.0);
                }
            }
        }
        self.daily_pnl_sol.set(daily_pnl_sol);
        self.active_trades.set(active_trades);
    }

    /// Record flash loan metrics
    pub fn record_flash_loan_result(&self, success: bool, fee_lamports: Option<u64>) {
        self.flash_loan_requests.inc();

        if success {
            self.flash_loan_successes.inc();
            if let Some(fee) = fee_lamports {
                self.flash_loan_fees_lamports.inc_by(fee as f64);
            }
        } else {
            self.flash_loan_failures.inc();
        }
    }

    /// Record bundle submission metrics
    pub fn record_bundle_submission(&self) {
        self.bundle_submissions.inc();
    }

    /// Record bundle confirmation
    pub fn record_bundle_confirmation(&self, success: bool) {
        if success {
            self.bundle_confirmations.inc();
        } else {
            self.bundle_failures.inc();
        }
    }

    /// Record bundle that never landed within polling window
    pub fn record_bundle_unlanded(&self) {
        self.bundle_unlanded.inc();
    }

    /// Record bundle submission failure (pre-polling)
    pub fn record_bundle_submit_failure(&self) {
        self.bundle_failed_submit.inc();
    }

    /// Record system error
    pub fn record_system_error(&self) {
        self.system_errors.inc();
    }

    pub fn record_simulation(&self, success: bool, latency_secs: f64) {
        self.simulation_requests.inc();
        if success {
            self.simulation_successes.inc();
        } else {
            self.simulation_failures.inc();
        }
        self.simulation_latency_seconds.observe(latency_secs);
    }

    pub fn record_instantiation(&self, instr_count: usize, tx_count: usize) {
        self.instructions_instantiated.inc_by(instr_count as u64);
        self.transactions_built.inc_by(tx_count as u64);
    }

    pub fn observe_route_template_build_time(&self, secs: f64) {
        self.route_template_build_time_seconds.observe(secs);
    }

    pub fn update_account_registry_stats(&self, missing: usize, loaded: usize) {
        self.account_sets_missing.set(missing as i64);
        self.account_sets_loaded.set(loaded as i64);
    }

    pub fn record_account_meta_resolution(&self, real: usize, placeholders: usize) {
        if real > 0 {
            self.account_metas_real.inc_by(real as u64);
        }
        if placeholders > 0 {
            self.account_metas_placeholders.inc_by(placeholders as u64);
        }
    }

    pub fn update_account_registry_validation_errors(&self, count: usize) {
        self.account_registry_validation_errors.set(count as i64);
    }

    pub fn update_account_registry_freshness(&self, ts: i64) {
        self.account_registry_freshness_unix.set(ts);
    }

    pub fn record_min_profit_rejection(&self) {
        self.min_profit_rejections.inc();
    }

    pub fn record_realized_profit(&self, estimated: u64, realized: i64) {
        if realized > 0 {
            self.realized_profit_lamports.inc_by(realized as f64);
        }
        // overestimation positive, underestimation negative in counter naming convention (store abs separately?)
        let delta = estimated as i64 - realized; // positive => over, negative => under
        if delta > 0 {
            self.profit_slippage_lamports.inc_by(delta as f64);
            self.profit_overestimation_lamports.inc_by(delta as f64);
        } else if delta < 0 {
            let under = (-delta) as f64;
            self.profit_slippage_lamports.inc_by(under);
            self.profit_underestimation_lamports.inc_by(under);
        }
        if estimated > 0 {
            let ratio = (realized as f64) / (estimated as f64);
            self.profit_slippage_ratio.observe(ratio);
        }
    }

    pub fn update_required_missing_accounts(&self, count: usize) {
        self.account_sets_required_missing.set(count as i64);
    }

    // Dedupe metrics recording
    pub fn record_dedupe_hit(&self) {
        self.opportunity_dedupe_hits_total.inc();
    }
    pub fn record_dedupe_insert(&self) {
        self.opportunity_dedupe_inserts_total.inc();
    }
    pub fn record_dedupe_eviction(&self) {
        self.opportunity_dedupe_evictions_total.inc();
    }

    // Reconciliation helpers
    pub fn reconciliation_started(&self) {
        self.reconciliation_started_total.inc();
    }
    pub fn reconciliation_success(&self) {
        self.reconciliation_success_total.inc();
    }
    pub fn reconciliation_failure(&self) {
        self.reconciliation_failure_total.inc();
    }
    pub fn reconciliation_adjustment(&self, diff: i64) {
        self.reconciliation_adjustments_total.inc();
        self.reconciliation_adjustment_lamports_total
            .inc_by(diff as f64);
    }
    pub fn reconciliation_observe_divergence(&self, bps: i64) {
        self.reconciliation_last_divergence_bps.set(bps);
        self.reconciliation_divergence_bps.observe(bps.abs() as f64);
    }
}
