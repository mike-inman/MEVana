use redis::{Client, Connection};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{debug, error, warn};
use tokio::sync::mpsc;

/// Ultra-fast Redis metrics publisher for hotpath engine
/// Publishes metrics asynchronously without blocking trading operations
pub struct RedisMetricsPublisher {
    sender: mpsc::UnboundedSender<MetricsEvent>,
    _handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Clone)]
pub enum MetricsEvent {
    ArbitrageStats {
        opportunities_detected: u64,
        opportunities_executed: u64,
        total_profit_bps: i64,
        avg_detection_time_us: u64,
        success_rate_pct: f64,
    },
    PerformanceMetrics {
        detection_latency_p50_us: u64,
        detection_latency_p95_us: u64,
        detection_latency_p99_us: u64,
        execution_latency_p50_ms: u64,
        execution_latency_p95_ms: u64,
        execution_latency_p99_ms: u64,
    },
    SystemStatus {
        current_slot: u64,
        streams_active: u32,
        transactions_per_second: f64,
        cpu_cores_used: Vec<u8>,
        memory_usage_mb: u64,
        uptime_seconds: u64,
    },
    VenueStats {
        venue: String,
        volume_24h: f64,
        opportunities_count: u64,
        avg_profit_bps: i32,
        success_rate: f64,
    },
}

impl RedisMetricsPublisher {
    /// Create new Redis publisher (non-blocking)
    pub async fn new(redis_url: Option<String>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let redis_url = redis_url.unwrap_or_else(|| "redis://localhost:6379".to_string());
        
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // Spawn background Redis worker
        let handle = tokio::spawn(async move {
            let worker = RedisWorker::new(redis_url, receiver).await;
            worker.run().await;
        });
        
        Ok(Self {
            sender,
            _handle: handle,
        })
    }
    
    /// Publish arbitrage stats (NON-BLOCKING)
    #[inline(always)]
    pub fn publish_arbitrage_stats(
        &self,
        opportunities_detected: u64,
        opportunities_executed: u64,
        total_profit_bps: i64,
        avg_detection_time_us: u64,
        success_rate_pct: f64,
    ) {
        let _ = self.sender.send(MetricsEvent::ArbitrageStats {
            opportunities_detected,
            opportunities_executed,
            total_profit_bps,
            avg_detection_time_us,
            success_rate_pct,
        });
    }
    
    /// Publish performance metrics (NON-BLOCKING)
    #[inline(always)]
    pub fn publish_performance_metrics(
        &self,
        detection_latency_p50_us: u64,
        detection_latency_p95_us: u64,
        detection_latency_p99_us: u64,
        execution_latency_p50_ms: u64,
        execution_latency_p95_ms: u64,
        execution_latency_p99_ms: u64,
    ) {
        let _ = self.sender.send(MetricsEvent::PerformanceMetrics {
            detection_latency_p50_us,
            detection_latency_p95_us,
            detection_latency_p99_us,
            execution_latency_p50_ms,
            execution_latency_p95_ms,
            execution_latency_p99_ms,
        });
    }
    
    /// Publish system status (NON-BLOCKING)
    #[inline(always)]
    pub fn publish_system_status(
        &self,
        current_slot: u64,
        streams_active: u32,
        transactions_per_second: f64,
        cpu_cores_used: Vec<u8>,
        memory_usage_mb: u64,
        uptime_seconds: u64,
    ) {
        let _ = self.sender.send(MetricsEvent::SystemStatus {
            current_slot,
            streams_active,
            transactions_per_second,
            cpu_cores_used,
            memory_usage_mb,
            uptime_seconds,
        });
    }
}

struct RedisWorker {
    redis_client: Option<Client>,
    receiver: mpsc::UnboundedReceiver<MetricsEvent>,
    connection_attempts: u32,
    last_connection_attempt: SystemTime,
}

impl RedisWorker {
    async fn new(redis_url: String, receiver: mpsc::UnboundedReceiver<MetricsEvent>) -> Self {
        let redis_client = match Client::open(redis_url.as_str()) {
            Ok(client) => {
                debug!("✅ Redis client created for metrics publishing");
                Some(client)
            }
            Err(e) => {
                warn!("⚠️  Failed to create Redis client: {}. Metrics will be dropped.", e);
                None
            }
        };
        
        Self {
            redis_client,
            receiver,
            connection_attempts: 0,
            last_connection_attempt: UNIX_EPOCH,
        }
    }
    
    async fn run(mut self) {
        debug!("🚀 Redis metrics publisher worker started");
        
        let mut connection: Option<Connection> = None;
        
        while let Some(event) = self.receiver.recv().await {
            // Try to establish connection if we don't have one
            if connection.is_none() && self.redis_client.is_some() {
                connection = self.try_connect().await;
            }
            
            // Publish metrics if we have a connection
            if let Some(ref mut conn) = connection {
                if let Err(e) = self.publish_event(conn, &event).await {
                    error!("❌ Failed to publish metrics: {}. Reconnecting...", e);
                    connection = None; // Force reconnection
                }
            }
        }
        
        debug!("📤 Redis metrics publisher worker stopped");
    }
    
    async fn try_connect(&mut self) -> Option<Connection> {
        if let Some(ref client) = self.redis_client {
            // Rate limit connection attempts
            let now = SystemTime::now();
            if now.duration_since(self.last_connection_attempt).unwrap_or_default().as_secs() < 5 {
                return None;
            }
            
            self.last_connection_attempt = now;
            self.connection_attempts += 1;
            
            match client.get_connection() {
                Ok(conn) => {
                    debug!("✅ Connected to Redis for metrics publishing");
                    self.connection_attempts = 0;
                    Some(conn)
                }
                Err(e) => {
                    if self.connection_attempts % 10 == 1 { // Log every 10 attempts
                        warn!("⚠️  Redis connection attempt {} failed: {}", self.connection_attempts, e);
                    }
                    None
                }
            }
        } else {
            None
        }
    }
    
    async fn publish_event(&self, connection: &mut Connection, event: &MetricsEvent) -> Result<(), redis::RedisError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        match event {
            MetricsEvent::ArbitrageStats {
                opportunities_detected,
                opportunities_executed,
                total_profit_bps,
                avg_detection_time_us,
                success_rate_pct,
            } => {
                let data = json!({
                    "opportunities_detected": opportunities_detected,
                    "opportunities_executed": opportunities_executed,
                    "total_profit_bps": total_profit_bps,
                    "avg_detection_time_us": avg_detection_time_us,
                    "success_rate_pct": success_rate_pct,
                    "last_updated": chrono::Utc::now(),
                    "timestamp": timestamp
                });
                
                redis::cmd("SET")
                    .arg("hotpath:stats:arbitrage")
                    .arg(data.to_string())
                    .arg("EX")
                    .arg(300) // Expire after 5 minutes
                    .query::<()>(connection)?;
            }
            
            MetricsEvent::PerformanceMetrics {
                detection_latency_p50_us,
                detection_latency_p95_us,
                detection_latency_p99_us,
                execution_latency_p50_ms,
                execution_latency_p95_ms,
                execution_latency_p99_ms,
            } => {
                let data = json!({
                    "detection_latency_p50_us": detection_latency_p50_us,
                    "detection_latency_p95_us": detection_latency_p95_us,
                    "detection_latency_p99_us": detection_latency_p99_us,
                    "execution_latency_p50_ms": execution_latency_p50_ms,
                    "execution_latency_p95_ms": execution_latency_p95_ms,
                    "execution_latency_p99_ms": execution_latency_p99_ms,
                    "timestamp": timestamp
                });
                
                redis::cmd("SET")
                    .arg("hotpath:stats:performance")
                    .arg(data.to_string())
                    .arg("EX")
                    .arg(300)
                    .query::<()>(connection)?;
            }
            
            MetricsEvent::SystemStatus {
                current_slot,
                streams_active,
                transactions_per_second,
                cpu_cores_used,
                memory_usage_mb,
                uptime_seconds,
            } => {
                let data = json!({
                    "current_slot": current_slot,
                    "streams_active": streams_active,
                    "transactions_per_second": transactions_per_second,
                    "cpu_cores_used": cpu_cores_used,
                    "memory_usage_mb": memory_usage_mb,
                    "uptime_seconds": uptime_seconds,
                    "timestamp": timestamp
                });
                
                redis::cmd("SET")
                    .arg("hotpath:stats:system")
                    .arg(data.to_string())
                    .arg("EX")
                    .arg(300)
                    .query::<()>(connection)?;
            }
            
            MetricsEvent::VenueStats {
                venue,
                volume_24h,
                opportunities_count,
                avg_profit_bps,
                success_rate,
            } => {
                let data = json!({
                    "venue": venue,
                    "volume_24h": volume_24h,
                    "opportunities_count": opportunities_count,
                    "avg_profit_bps": avg_profit_bps,
                    "success_rate": success_rate,
                    "timestamp": timestamp
                });
                
                redis::cmd("HSET")
                    .arg("hotpath:stats:venues")
                    .arg(venue)
                    .arg(data.to_string())
                    .query::<()>(connection)?;
                
                // Set expiry on the hash
                redis::cmd("EXPIRE")
                    .arg("hotpath:stats:venues")
                    .arg(300)
                    .query::<()>(connection)?;
            }
        }
        
        Ok(())
    }
}