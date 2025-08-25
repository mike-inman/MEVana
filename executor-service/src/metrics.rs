use anyhow::Result;
use prometheus::{Counter, Gauge, Histogram, IntCounter, IntGauge, Registry, TextEncoder, Encoder};
use std::net::SocketAddr;
use std::sync::Arc;
use warp::Filter;

/// Prometheus metrics server for monitoring executor performance
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
    pub wallet_balance_sol: Gauge,
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
    pub system_errors: IntCounter,
}

impl MetricsServer {
    pub fn new(port: u16) -> Result<Self> {
        let registry = Arc::new(Registry::new());

        // Initialize execution metrics
        let opportunities_received = IntCounter::new(
            "mevana_opportunities_received_total",
            "Total number of arbitrage opportunities received"
        )?;
        registry.register(Box::new(opportunities_received.clone()))?;

        let opportunities_executed = IntCounter::new(
            "mevana_opportunities_executed_total",
            "Total number of arbitrage opportunities executed"
        )?;
        registry.register(Box::new(opportunities_executed.clone()))?;

        let opportunities_rejected = IntCounter::new(
            "mevana_opportunities_rejected_total",
            "Total number of arbitrage opportunities rejected"
        )?;
        registry.register(Box::new(opportunities_rejected.clone()))?;

        let execution_time_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "mevana_execution_time_seconds",
                "Time taken to execute arbitrage opportunities"
            ).buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0])
        )?;
        registry.register(Box::new(execution_time_seconds.clone()))?;

        let profit_lamports = Counter::new(
            "mevana_profit_lamports_total",
            "Total profit in lamports"
        )?;
        registry.register(Box::new(profit_lamports.clone()))?;

        let loss_lamports = Counter::new(
            "mevana_loss_lamports_total",
            "Total losses in lamports"
        )?;
        registry.register(Box::new(loss_lamports.clone()))?;

        // Initialize wallet metrics
        let wallet_balance_sol = Gauge::new(
            "mevana_wallet_balance_sol",
            "Current wallet balance in SOL"
        )?;
        registry.register(Box::new(wallet_balance_sol.clone()))?;

        let daily_pnl_sol = Gauge::new(
            "mevana_daily_pnl_sol",
            "Daily profit/loss in SOL"
        )?;
        registry.register(Box::new(daily_pnl_sol.clone()))?;

        let active_trades = IntGauge::new(
            "mevana_active_trades",
            "Number of currently active trades"
        )?;
        registry.register(Box::new(active_trades.clone()))?;

        // Initialize flash loan metrics
        let flash_loan_requests = IntCounter::new(
            "mevana_flash_loan_requests_total",
            "Total number of flash loan requests"
        )?;
        registry.register(Box::new(flash_loan_requests.clone()))?;

        let flash_loan_successes = IntCounter::new(
            "mevana_flash_loan_successes_total",
            "Total number of successful flash loans"
        )?;
        registry.register(Box::new(flash_loan_successes.clone()))?;

        let flash_loan_failures = IntCounter::new(
            "mevana_flash_loan_failures_total",
            "Total number of failed flash loans"
        )?;
        registry.register(Box::new(flash_loan_failures.clone()))?;

        let flash_loan_fees_lamports = Counter::new(
            "mevana_flash_loan_fees_lamports_total",
            "Total flash loan fees paid in lamports"
        )?;
        registry.register(Box::new(flash_loan_fees_lamports.clone()))?;

        // Initialize system metrics
        let bundle_submissions = IntCounter::new(
            "mevana_bundle_submissions_total",
            "Total number of Jito bundle submissions"
        )?;
        registry.register(Box::new(bundle_submissions.clone()))?;

        let bundle_confirmations = IntCounter::new(
            "mevana_bundle_confirmations_total",
            "Total number of confirmed Jito bundles"
        )?;
        registry.register(Box::new(bundle_confirmations.clone()))?;

        let bundle_failures = IntCounter::new(
            "mevana_bundle_failures_total",
            "Total number of failed Jito bundles"
        )?;
        registry.register(Box::new(bundle_failures.clone()))?;

        let system_errors = IntCounter::new(
            "mevana_system_errors_total",
            "Total number of system errors"
        )?;
        registry.register(Box::new(system_errors.clone()))?;

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
            system_errors,
        })
    }

    pub async fn start(&self) -> Result<()> {
        let registry = Arc::clone(&self.registry);
        
        let metrics_route = warp::path("metrics")
            .and(warp::get())
            .map(move || {
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
            .map(|| {
                warp::reply::with_status(
                    "OK".to_string(),
                    warp::http::StatusCode::OK,
                )
            });

        let routes = metrics_route.or(health_route);

        let addr: SocketAddr = format!("0.0.0.0:{}", self.port).parse()?;
        
        log::info!("Starting metrics server on {}", addr);
        
        warp::serve(routes)
            .run(addr)
            .await;

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

    /// Update wallet metrics
    pub fn update_wallet_metrics(&self, balance_sol: f64, daily_pnl_sol: f64, active_trades: i64) {
        self.wallet_balance_sol.set(balance_sol);
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

    /// Record system error
    pub fn record_system_error(&self) {
        self.system_errors.inc();
    }
}
