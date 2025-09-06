use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::time::{Duration, Instant};
use anyhow::Result;

/// Data source for arbitrage engine with automatic failover
pub enum DataSource {
    /// Primary: Kafka for high throughput
    Kafka { healthy: bool, last_message: Instant },
    /// Fallback: Direct from blockchain
    DirectGeyser { connected: bool },
    /// Emergency: Local in-memory queue
    InMemory { queue_size: usize },
}

/// Health status of data sources
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub kafka_healthy: bool,
    pub geyser_healthy: bool,
    pub last_kafka_message: Option<Instant>,
    pub last_geyser_message: Option<Instant>,
    pub failover_count: u32,
    pub current_mode: String,
}

/// Manages resilient data flow with automatic failover
pub struct ResilientDataManager {
    current_source: Arc<RwLock<DataSource>>,
    health_status: Arc<RwLock<HealthStatus>>,
    kafka_retry_interval: Duration,
    max_kafka_silence: Duration,
    emergency_queue: Arc<RwLock<Vec<EdgeDelta>>>,
}

/// Placeholder for edge delta type
#[derive(Debug, Clone)]
pub struct EdgeDelta {
    pub edge_id: String,
    pub timestamp: u64,
    pub data: Vec<u8>,
}

impl ResilientDataManager {
    pub fn new() -> Self {
        Self {
            current_source: Arc::new(RwLock::new(DataSource::Kafka {
                healthy: true,
                last_message: Instant::now(),
            })),
            health_status: Arc::new(RwLock::new(HealthStatus {
                kafka_healthy: true,
                geyser_healthy: false,
                last_kafka_message: Some(Instant::now()),
                last_geyser_message: None,
                failover_count: 0,
                current_mode: "Kafka".to_string(),
            })),
            kafka_retry_interval: Duration::from_secs(30),
            max_kafka_silence: Duration::from_secs(5),
            emergency_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Main processing loop with automatic failover
    pub async fn process_with_resilience(&self) -> Result<()> {
        loop {
            let source = self.current_source.read().await.clone();
            
            match source {
                DataSource::Kafka { healthy, last_message } => {
                    if healthy && last_message.elapsed() < self.max_kafka_silence {
                        // Process from Kafka
                        if let Some(edge) = self.consume_kafka().await? {
                            self.process_edge(edge).await?;
                            self.update_kafka_health(true).await;
                        }
                    } else {
                        // Kafka unhealthy, switch to Geyser
                        log::warn!("Kafka unhealthy, switching to direct Geyser mode");
                        self.switch_to_geyser().await?;
                    }
                }
                
                DataSource::DirectGeyser { connected } => {
                    if connected {
                        // Process directly from Geyser
                        if let Some(edge) = self.consume_geyser().await? {
                            self.process_edge(edge).await?;
                        }
                        
                        // Try to restore Kafka in background
                        self.try_restore_kafka().await;
                    } else {
                        // Geyser disconnected, switch to emergency mode
                        log::error!("Geyser disconnected, entering emergency mode");
                        self.switch_to_emergency().await?;
                    }
                }
                
                DataSource::InMemory { queue_size } => {
                    // Process from local queue
                    if queue_size > 0 {
                        if let Some(edge) = self.consume_local_queue().await {
                            self.process_edge(edge).await?;
                        }
                    }
                    
                    // Keep trying to restore primary sources
                    self.try_restore_all().await;
                }
            }
            
            // Brief sleep to prevent tight loop
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    /// Process an edge delta (arbitrage opportunity)
    async fn process_edge(&self, edge: EdgeDelta) -> Result<()> {
        // This would contain actual arbitrage logic
        log::debug!("Processing edge: {}", edge.edge_id);
        Ok(())
    }

    /// Consume from Kafka (stub)
    async fn consume_kafka(&self) -> Result<Option<EdgeDelta>> {
        // In production, this would consume from actual Kafka
        Ok(None)
    }

    /// Consume from Geyser (stub)
    async fn consume_geyser(&self) -> Result<Option<EdgeDelta>> {
        // In production, this would consume from Geyser stream
        Ok(None)
    }

    /// Consume from local queue
    async fn consume_local_queue(&self) -> Option<EdgeDelta> {
        let mut queue = self.emergency_queue.write().await;
        queue.pop()
    }

    /// Update Kafka health status
    async fn update_kafka_health(&self, healthy: bool) {
        let mut source = self.current_source.write().await;
        let mut health = self.health_status.write().await;
        
        if let DataSource::Kafka { .. } = *source {
            *source = DataSource::Kafka {
                healthy,
                last_message: if healthy { Instant::now() } else { Instant::now() - self.max_kafka_silence },
            };
            
            health.kafka_healthy = healthy;
            if healthy {
                health.last_kafka_message = Some(Instant::now());
            }
        }
    }

    /// Switch to Geyser mode
    async fn switch_to_geyser(&self) -> Result<()> {
        let mut source = self.current_source.write().await;
        let mut health = self.health_status.write().await;
        
        *source = DataSource::DirectGeyser { connected: true };
        health.current_mode = "DirectGeyser".to_string();
        health.failover_count += 1;
        
        log::info!("Switched to Direct Geyser mode (failover #{})", health.failover_count);
        
        // TODO: Initialize Geyser connection
        
        Ok(())
    }

    /// Switch to emergency mode
    async fn switch_to_emergency(&self) -> Result<()> {
        let mut source = self.current_source.write().await;
        let mut health = self.health_status.write().await;
        let queue = self.emergency_queue.read().await;
        
        *source = DataSource::InMemory { 
            queue_size: queue.len() 
        };
        health.current_mode = "Emergency".to_string();
        health.failover_count += 1;
        
        log::error!("EMERGENCY MODE ACTIVATED - Running on local queue");
        
        // Send alert to admin
        self.send_emergency_alert().await;
        
        Ok(())
    }

    /// Try to restore Kafka connection
    async fn try_restore_kafka(&self) {
        let health = self.health_status.read().await;
        
        if !health.kafka_healthy {
            if let Some(last) = health.last_kafka_message {
                if last.elapsed() > self.kafka_retry_interval {
                    drop(health);
                    
                    log::info!("Attempting to restore Kafka connection...");
                    
                    // TODO: Actually try to reconnect to Kafka
                    // If successful, switch back
                    
                    // For now, just update the attempt time
                    let mut health = self.health_status.write().await;
                    health.last_kafka_message = Some(Instant::now());
                }
            }
        }
    }

    /// Try to restore all data sources
    async fn try_restore_all(&self) {
        self.try_restore_kafka().await;
        // TODO: Try to restore Geyser
    }

    /// Send emergency alert
    async fn send_emergency_alert(&self) {
        // TODO: Send to Telegram, Discord, email, etc.
        log::error!("ALERT: System in emergency mode - manual intervention may be required");
    }

    /// Get current health status
    pub async fn get_health(&self) -> HealthStatus {
        self.health_status.read().await.clone()
    }

    /// Force switch to a specific mode (for testing/manual override)
    pub async fn force_mode(&self, mode: &str) -> Result<()> {
        match mode {
            "kafka" => {
                let mut source = self.current_source.write().await;
                *source = DataSource::Kafka {
                    healthy: true,
                    last_message: Instant::now(),
                };
                log::info!("Forced switch to Kafka mode");
            }
            "geyser" => {
                self.switch_to_geyser().await?;
            }
            "emergency" => {
                self.switch_to_emergency().await?;
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown mode: {}", mode));
            }
        }
        Ok(())
    }
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    failure_count: Arc<RwLock<u32>>,
    last_failure: Arc<RwLock<Option<Instant>>>,
    threshold: u32,
    reset_timeout: Duration,
    state: Arc<RwLock<CircuitState>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,     // Normal operation
    Open,       // Failing, reject requests
    HalfOpen,   // Testing if service recovered
}

impl CircuitBreaker {
    pub fn new(threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_count: Arc::new(RwLock::new(0)),
            last_failure: Arc::new(RwLock::new(None)),
            threshold,
            reset_timeout,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
        }
    }

    /// Check if request should be allowed
    pub async fn should_allow(&self) -> bool {
        let mut state = self.state.write().await;
        let last_failure = self.last_failure.read().await;
        
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should try half-open
                if let Some(last) = *last_failure {
                    if last.elapsed() > self.reset_timeout {
                        *state = CircuitState::HalfOpen;
                        log::info!("Circuit breaker entering half-open state");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;
        let mut failure_count = self.failure_count.write().await;
        
        match *state {
            CircuitState::HalfOpen => {
                *state = CircuitState::Closed;
                *failure_count = 0;
                log::info!("Circuit breaker closed - service recovered");
            }
            _ => {
                *failure_count = 0;
            }
        }
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        let mut state = self.state.write().await;
        let mut failure_count = self.failure_count.write().await;
        let mut last_failure = self.last_failure.write().await;
        
        *failure_count += 1;
        *last_failure = Some(Instant::now());
        
        if *failure_count >= self.threshold {
            *state = CircuitState::Open;
            log::error!("Circuit breaker OPEN - too many failures ({}/{})", 
                *failure_count, self.threshold);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));
        
        // Should start closed
        assert!(cb.should_allow().await);
        
        // Record failures
        for _ in 0..3 {
            cb.record_failure().await;
        }
        
        // Should be open
        assert!(!cb.should_allow().await);
        
        // Wait for reset timeout
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Should be half-open
        assert!(cb.should_allow().await);
        
        // Success should close it
        cb.record_success().await;
        assert!(cb.should_allow().await);
    }
}