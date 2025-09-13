use anyhow::Result;
use log::{error, warn, info};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;

/// System health events that can trigger alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HealthEvent {
    /// Critical: Execution queue closed due to configuration issue
    ExecutionQueueClosed {
        timestamp: u64,
        paper_trading: bool,
        cause: String,
        impact: String,
    },
    /// Critical: Database record flow stopped
    RecordFlowStopped {
        timestamp: u64,
        last_record_time: u64,
        expected_rate_per_minute: u64,
        actual_rate_per_minute: u64,
    },
    /// Warning: Queue send failures detected
    QueueSendFailures {
        timestamp: u64,
        failure_count: u64,
        failure_rate_per_minute: u64,
        queue_status: String,
    },
    /// Warning: Memory usage growing unexpectedly
    MemoryGrowthDetected {
        timestamp: u64,
        current_usage_mb: u64,
        growth_rate_mb_per_minute: f64,
        component: String,
    },
    /// Info: Configuration validation failure
    ConfigurationIssue {
        timestamp: u64,
        parameter: String,
        expected_value: String,
        actual_value: String,
        recommendation: String,
    },
}

/// Health monitoring statistics
#[derive(Debug, Default)]
pub struct HealthStats {
    pub opportunities_detected_per_minute: AtomicU64,
    pub opportunities_processed_per_minute: AtomicU64,
    pub queue_send_failures_per_minute: AtomicU64,
    pub database_records_per_minute: AtomicU64,
    pub queue_is_healthy: AtomicBool,
    pub last_database_record: AtomicU64,
    pub last_opportunity_processed: AtomicU64,
    pub memory_usage_mb: AtomicU64,
    pub consecutive_failures: AtomicU64,
}

/// Comprehensive system health monitor to prevent silent failures
pub struct SystemHealthMonitor {
    stats: Arc<HealthStats>,
    event_handlers: Arc<RwLock<Vec<Box<dyn Fn(HealthEvent) + Send + Sync>>>>,
    monitoring_enabled: AtomicBool,
    last_health_check: AtomicU64,
    
    // Configuration validation
    paper_trading_enabled: bool,
    expected_min_records_per_minute: u64,
    max_queue_failures_per_minute: u64,
}

impl SystemHealthMonitor {
    pub fn new() -> Self {
        let paper_trading = std::env::var("PAPER_TRADING")
            .ok()
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
            
        Self {
            stats: Arc::new(HealthStats::default()),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
            monitoring_enabled: AtomicBool::new(true),
            last_health_check: AtomicU64::new(0),
            paper_trading_enabled: paper_trading,
            expected_min_records_per_minute: 100, // Minimum expected database records
            max_queue_failures_per_minute: 10,    // Max tolerable queue failures
        }
    }

    /// Start the health monitoring system
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("🏥 Starting System Health Monitor");
        
        // Validate configuration on startup
        self.validate_startup_configuration().await?;
        
        let stats_clone = Arc::clone(&self.stats);
        let handlers_clone = Arc::clone(&self.event_handlers);
        let expected_min_records = self.expected_min_records_per_minute;
        let max_failures = self.max_queue_failures_per_minute;
        let paper_trading = self.paper_trading_enabled;
        
        // Health check task - runs every 30 seconds
        tokio::spawn(async move {
            let mut health_check_interval = interval(Duration::from_secs(30));
            let mut consecutive_no_records = 0u64;
            let mut last_database_count = 0u64;
            
            loop {
                health_check_interval.tick().await;
                
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                // Check database record flow
                let current_db_records = stats_clone.database_records_per_minute.load(Ordering::Relaxed);
                let last_record_time = stats_clone.last_database_record.load(Ordering::Relaxed);
                
                // Detect record flow stoppage
                if current_db_records < expected_min_records {
                    consecutive_no_records += 1;
                    
                    if consecutive_no_records >= 2 { // 1 minute of low/no records
                        let event = HealthEvent::RecordFlowStopped {
                            timestamp: now,
                            last_record_time,
                            expected_rate_per_minute: expected_min_records,
                            actual_rate_per_minute: current_db_records,
                        };
                        
                        Self::emit_event(&handlers_clone, event).await;
                    }
                } else {
                    consecutive_no_records = 0; // Reset counter on good flow
                }
                
                // Check queue health
                let queue_failures = stats_clone.queue_send_failures_per_minute.load(Ordering::Relaxed);
                if queue_failures > max_failures {
                    let queue_healthy = stats_clone.queue_is_healthy.load(Ordering::Relaxed);
                    
                    let event = HealthEvent::QueueSendFailures {
                        timestamp: now,
                        failure_count: stats_clone.consecutive_failures.load(Ordering::Relaxed),
                        failure_rate_per_minute: queue_failures,
                        queue_status: if queue_healthy { "healthy".to_string() } else { "degraded".to_string() },
                    };
                    
                    Self::emit_event(&handlers_clone, event).await;
                }
                
                // Check for configuration issues if queue is failing
                if !stats_clone.queue_is_healthy.load(Ordering::Relaxed) && !paper_trading {
                    let event = HealthEvent::ConfigurationIssue {
                        timestamp: now,
                        parameter: "PAPER_TRADING".to_string(),
                        expected_value: "true".to_string(),
                        actual_value: "false".to_string(),
                        recommendation: "Set PAPER_TRADING=true to prevent safety system queue closure".to_string(),
                    };
                    
                    Self::emit_event(&handlers_clone, event).await;
                }
                
                // Reset per-minute counters
                stats_clone.opportunities_detected_per_minute.store(0, Ordering::Relaxed);
                stats_clone.opportunities_processed_per_minute.store(0, Ordering::Relaxed);
                stats_clone.queue_send_failures_per_minute.store(0, Ordering::Relaxed);
                stats_clone.database_records_per_minute.store(0, Ordering::Relaxed);
                
                last_database_count = current_db_records;
            }
        });
        
        info!("✅ System Health Monitor started successfully");
        Ok(())
    }
    
    /// Validate configuration on startup to prevent known issues
    async fn validate_startup_configuration(&self) -> Result<()> {
        info!("🔍 Validating system configuration...");
        
        // Check PAPER_TRADING configuration
        let paper_trading = std::env::var("PAPER_TRADING")
            .unwrap_or_else(|_| "false".to_string());
            
        if paper_trading.eq_ignore_ascii_case("false") {
            warn!("⚠️ PAPER_TRADING=false detected - this may cause execution queue closure!");
            
            let event = HealthEvent::ConfigurationIssue {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                parameter: "PAPER_TRADING".to_string(),
                expected_value: "true (for development/testing)".to_string(),
                actual_value: paper_trading,
                recommendation: "Set PAPER_TRADING=true unless explicitly deploying to production".to_string(),
            };
            
            let handlers = self.event_handlers.read().await;
            for handler in handlers.iter() {
                handler(event.clone());
            }
        }
        
        // Check database connection
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "not_set".to_string());
            
        if !db_url.contains("localhost:5433") {
            warn!("⚠️ DATABASE_URL may not be using correct Docker PostgreSQL port (5433)");
            
            let event = HealthEvent::ConfigurationIssue {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                parameter: "DATABASE_URL".to_string(),
                expected_value: "postgresql://mev:mevpassword@localhost:5433/mevdb".to_string(),
                actual_value: db_url,
                recommendation: "Use Docker PostgreSQL on port 5433 to avoid connection conflicts".to_string(),
            };
            
            let handlers = self.event_handlers.read().await;
            for handler in handlers.iter() {
                handler(event.clone());
            }
        }
        
        info!("✅ Configuration validation complete");
        Ok(())
    }
    
    /// Record that an opportunity was detected
    pub fn record_opportunity_detected(&self) {
        self.stats.opportunities_detected_per_minute.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record that an opportunity was successfully processed
    pub fn record_opportunity_processed(&self) {
        self.stats.opportunities_processed_per_minute.fetch_add(1, Ordering::Relaxed);
        self.stats.last_opportunity_processed.store(
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            Ordering::Relaxed,
        );
    }
    
    /// Record a queue send failure (critical indicator)
    pub fn record_queue_send_failure(&self, reason: &str) {
        self.stats.queue_send_failures_per_minute.fetch_add(1, Ordering::Relaxed);
        let consecutive = self.stats.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
        
        // Mark queue as unhealthy after multiple failures
        if consecutive > 5 {
            self.stats.queue_is_healthy.store(false, Ordering::Relaxed);
            
            // Emit immediate alert for queue closure
            if reason.contains("queue closed") || reason.contains("Execution queue closed") {
                let event = HealthEvent::ExecutionQueueClosed {
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    paper_trading: self.paper_trading_enabled,
                    cause: reason.to_string(),
                    impact: "All opportunity processing stopped - no database records will be written".to_string(),
                };
                
                // Emit synchronously for critical events
                tokio::spawn(async move {
                    error!("🚨 CRITICAL: Execution queue closed - system processing halted!");
                    error!("🚨 This indicates a configuration issue preventing operation");
                    error!("🚨 Check PAPER_TRADING setting and safety system logs");
                });
            }
        }
    }
    
    /// Record successful queue send (reset failure counter)
    pub fn record_queue_send_success(&self) {
        self.stats.consecutive_failures.store(0, Ordering::Relaxed);
        self.stats.queue_is_healthy.store(true, Ordering::Relaxed);
    }
    
    /// Record database record written
    pub fn record_database_write(&self) {
        self.stats.database_records_per_minute.fetch_add(1, Ordering::Relaxed);
        self.stats.last_database_record.store(
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            Ordering::Relaxed,
        );
    }
    
    /// Add event handler for health events
    pub async fn add_event_handler<F>(&self, handler: F) 
    where
        F: Fn(HealthEvent) + Send + Sync + 'static,
    {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(Box::new(handler));
    }
    
    /// Get current health statistics
    pub fn get_health_stats(&self) -> HealthStats {
        HealthStats {
            opportunities_detected_per_minute: AtomicU64::new(
                self.stats.opportunities_detected_per_minute.load(Ordering::Relaxed)
            ),
            opportunities_processed_per_minute: AtomicU64::new(
                self.stats.opportunities_processed_per_minute.load(Ordering::Relaxed)
            ),
            queue_send_failures_per_minute: AtomicU64::new(
                self.stats.queue_send_failures_per_minute.load(Ordering::Relaxed)
            ),
            database_records_per_minute: AtomicU64::new(
                self.stats.database_records_per_minute.load(Ordering::Relaxed)
            ),
            queue_is_healthy: AtomicBool::new(
                self.stats.queue_is_healthy.load(Ordering::Relaxed)
            ),
            last_database_record: AtomicU64::new(
                self.stats.last_database_record.load(Ordering::Relaxed)
            ),
            last_opportunity_processed: AtomicU64::new(
                self.stats.last_opportunity_processed.load(Ordering::Relaxed)
            ),
            memory_usage_mb: AtomicU64::new(
                self.stats.memory_usage_mb.load(Ordering::Relaxed)
            ),
            consecutive_failures: AtomicU64::new(
                self.stats.consecutive_failures.load(Ordering::Relaxed)
            ),
        }
    }
    
    /// Emit health event to all handlers
    async fn emit_event(
        handlers: &Arc<RwLock<Vec<Box<dyn Fn(HealthEvent) + Send + Sync>>>>,
        event: HealthEvent,
    ) {
        let handlers_guard = handlers.read().await;
        for handler in handlers_guard.iter() {
            handler(event.clone());
        }
    }
}

/// Default event handler that logs health events
pub fn create_default_event_handler() -> impl Fn(HealthEvent) + Send + Sync + 'static {
    |event: HealthEvent| {
        match event {
            HealthEvent::ExecutionQueueClosed { timestamp, paper_trading, cause, impact } => {
                error!("🚨 CRITICAL ALERT: Execution Queue Closed at {}", timestamp);
                error!("🚨 Paper Trading: {}, Cause: {}", paper_trading, cause);
                error!("🚨 Impact: {}", impact);
                error!("🚨 IMMEDIATE ACTION REQUIRED: Check configuration and restart with PAPER_TRADING=true");
            }
            HealthEvent::RecordFlowStopped { timestamp, expected_rate_per_minute, actual_rate_per_minute, .. } => {
                error!("🚨 DATABASE RECORD FLOW STOPPED at {}", timestamp);
                error!("🚨 Expected: {}/min, Actual: {}/min", expected_rate_per_minute, actual_rate_per_minute);
                error!("🚨 This indicates system processing has halted - investigate immediately");
            }
            HealthEvent::QueueSendFailures { timestamp, failure_count, failure_rate_per_minute, .. } => {
                warn!("⚠️ Queue Send Failures Detected at {}", timestamp);
                warn!("⚠️ Consecutive failures: {}, Rate: {}/min", failure_count, failure_rate_per_minute);
                warn!("⚠️ Check execution queue health and configuration");
            }
            HealthEvent::ConfigurationIssue { parameter, expected_value, actual_value, recommendation, .. } => {
                warn!("⚠️ Configuration Issue: {} = {}", parameter, actual_value);
                warn!("⚠️ Expected: {}", expected_value);
                warn!("⚠️ Recommendation: {}", recommendation);
            }
            HealthEvent::MemoryGrowthDetected { component, current_usage_mb, growth_rate_mb_per_minute, .. } => {
                warn!("⚠️ Memory Growth in {}: {}MB (+{:.2}MB/min)", 
                      component, current_usage_mb, growth_rate_mb_per_minute);
            }
        }
    }
}

/// Create Prometheus metrics for health monitoring
pub fn create_health_metrics() -> prometheus::Registry {
    use prometheus::{Counter, Gauge, Registry, Opts};
    
    let registry = Registry::new();
    
    // Queue health metrics
    let queue_failures = Counter::with_opts(Opts::new(
        "mevana_queue_failures_total",
        "Total number of execution queue send failures",
    )).unwrap();
    
    let queue_health = Gauge::with_opts(Opts::new(
        "mevana_queue_healthy",
        "1 if execution queue is healthy, 0 if degraded",
    )).unwrap();
    
    // Database flow metrics
    let db_records_rate = Gauge::with_opts(Opts::new(
        "mevana_database_records_per_minute",
        "Rate of database records written per minute",
    )).unwrap();
    
    // Opportunity processing metrics
    let opportunities_detected = Counter::with_opts(Opts::new(
        "mevana_opportunities_detected_total",
        "Total arbitrage opportunities detected",
    )).unwrap();
    
    let opportunities_processed = Counter::with_opts(Opts::new(
        "mevana_opportunities_processed_total",
        "Total arbitrage opportunities successfully processed",
    )).unwrap();
    
    registry.register(Box::new(queue_failures)).unwrap();
    registry.register(Box::new(queue_health)).unwrap();
    registry.register(Box::new(db_records_rate)).unwrap();
    registry.register(Box::new(opportunities_detected)).unwrap();
    registry.register(Box::new(opportunities_processed)).unwrap();
    
    registry
}