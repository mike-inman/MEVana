use sha2::{Digest, Sha256};
use std::env;
use std::net::{TcpStream, SocketAddr};
use std::time::Duration;
use tokio::time::sleep;

pub fn anchor_disc(name: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(format!("global:{name}"));
    let x = h.finalize();
    let mut d = [0u8; 8];
    d.copy_from_slice(&x[..8]);
    d
}

/// Network context detection for intelligent configuration
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkContext {
    DockerContainer,      // Inside Docker network
    HostNetwork,         // Direct host access
    Kubernetes,          // K8s cluster
    Production,          // Production environment
    Unknown,             // Unable to determine
}

/// Network configuration with intelligent fallbacks
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub kafka_broker: String,
    pub redis_host: String,
    pub redis_port: u16,
    pub prometheus_host: String,
    pub prometheus_port: u16,
    pub context: NetworkContext,
    pub health_status: NetworkHealthStatus,
}

/// Network health status for monitoring
#[derive(Debug, Clone)]
pub struct NetworkHealthStatus {
    pub kafka_healthy: bool,
    pub redis_healthy: bool,
    pub prometheus_healthy: bool,
    pub last_check: std::time::Instant,
    pub consecutive_failures: u32,
}

impl NetworkHealthStatus {
    pub fn new() -> Self {
        Self {
            kafka_healthy: false,
            redis_healthy: false,
            prometheus_healthy: false,
            last_check: std::time::Instant::now(),
            consecutive_failures: 0,
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.kafka_healthy && self.redis_healthy && self.prometheus_healthy
    }

    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }
}

impl NetworkConfig {
    /// Create network configuration with automatic context detection
    pub fn from_env() -> Self {
        let context = Self::detect_network_context();
        let mut config = Self::resolve_config_for_context(&context);
        config.context = context;
        config
    }

    /// Detect the current network context intelligently
    fn detect_network_context() -> NetworkContext {
        // Check for Docker environment
        if env::var("DOCKER_CONTAINER").is_ok() || env::var("KUBERNETES_SERVICE_HOST").is_ok() {
            return NetworkContext::DockerContainer;
        }

        // Check for production environment
        if env::var("ENVIRONMENT").unwrap_or_default() == "production" {
            return NetworkContext::Production;
        }

        // Check for Kubernetes
        if env::var("KUBERNETES_SERVICE_HOST").is_ok() {
            return NetworkContext::Kubernetes;
        }

        // Default to host network
        NetworkContext::HostNetwork
    }

    /// Resolve configuration based on detected context
    fn resolve_config_for_context(context: &NetworkContext) -> Self {
        match context {
            NetworkContext::DockerContainer => Self {
                kafka_broker: env::var("KAFKA_BROKER").unwrap_or_else(|_| "kafka:9092".to_string()),
                redis_host: env::var("REDIS_HOST").unwrap_or_else(|_| "redis".to_string()),
                redis_port: env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string()).parse().unwrap_or(6379),
                prometheus_host: env::var("PROMETHEUS_HOST").unwrap_or_else(|_| "prometheus".to_string()),
                prometheus_port: env::var("PROMETHEUS_PORT").unwrap_or_else(|_| "9090".to_string()).parse().unwrap_or(9090),
                context: context.clone(),
                health_status: NetworkHealthStatus::new(),
            },
            NetworkContext::HostNetwork | NetworkContext::Unknown => Self {
                kafka_broker: env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string()),
                redis_host: env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                redis_port: env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string()).parse().unwrap_or(6379),
                prometheus_host: env::var("PROMETHEUS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                prometheus_port: env::var("PROMETHEUS_PORT").unwrap_or_else(|_| "9090".to_string()).parse().unwrap_or(9090),
                context: context.clone(),
                health_status: NetworkHealthStatus::new(),
            },
            NetworkContext::Production => Self {
                kafka_broker: env::var("KAFKA_BROKER").expect("KAFKA_BROKER required in production"),
                redis_host: env::var("REDIS_HOST").expect("REDIS_HOST required in production"),
                redis_port: env::var("REDIS_PORT").expect("REDIS_PORT required in production").parse().expect("Invalid REDIS_PORT"),
                prometheus_host: env::var("PROMETHEUS_HOST").expect("PROMETHEUS_HOST required in production"),
                prometheus_port: env::var("PROMETHEUS_PORT").expect("PROMETHEUS_PORT required in production").parse().expect("Invalid PROMETHEUS_PORT"),
                context: context.clone(),
                health_status: NetworkHealthStatus::new(),
            },
            NetworkContext::Kubernetes => Self {
                kafka_broker: env::var("KAFKA_BROKER").unwrap_or_else(|_| "kafka-service:9092".to_string()),
                redis_host: env::var("REDIS_HOST").unwrap_or_else(|_| "redis-service".to_string()),
                redis_port: env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string()).parse().unwrap_or(6379),
                prometheus_host: env::var("PROMETHEUS_HOST").unwrap_or_else(|_| "prometheus-service".to_string()),
                prometheus_port: env::var("PROMETHEUS_PORT").unwrap_or_else(|_| "9090".to_string()).parse().unwrap_or(9090),
                context: context.clone(),
                health_status: NetworkHealthStatus::new(),
            },
        }
    }

    /// Self-healing network configuration with intelligent fallbacks
    pub async fn self_heal(&mut self) -> Result<(), NetworkError> {
        // Check current health
        self.check_health().await;

        // If healthy, no action needed
        if self.health_status.is_healthy() {
            return Ok(());
        }

        // Attempt self-healing based on context
        match self.context {
            NetworkContext::HostNetwork => self.attempt_host_network_healing().await,
            NetworkContext::DockerContainer => self.attempt_docker_healing().await,
            _ => self.attempt_generic_healing().await,
        }
    }

    /// Check network health for all services
    async fn check_health(&mut self) {
        let kafka_healthy = self.test_kafka_connectivity().await.is_ok();
        let redis_healthy = self.test_redis_connectivity().await.is_ok();
        let prometheus_healthy = self.test_prometheus_connectivity().await.is_ok();

        self.health_status.kafka_healthy = kafka_healthy;
        self.health_status.redis_healthy = redis_healthy;
        self.health_status.prometheus_healthy = prometheus_healthy;
        self.health_status.last_check = std::time::Instant::now();

        if !self.health_status.is_healthy() {
            self.health_status.record_failure();
        } else {
            self.health_status.record_success();
        }
    }

    /// Attempt healing for host network context
    async fn attempt_host_network_healing(&mut self) -> Result<(), NetworkError> {
        // Try alternative host addresses
        let host_alternatives = vec![
            "localhost",
            "127.0.0.1",
        ];

        for host in &host_alternatives {
            // Test Kafka alternatives
            let kafka_alt = format!("{}:29092", host);
            if self.test_kafka_connectivity_with_broker(&kafka_alt).await.is_ok() {
                self.kafka_broker = kafka_alt.clone();
                log::info!("Self-healed Kafka broker to: {}", kafka_alt);
                break;
            }
        }

        // Test Redis alternatives
        for host in &host_alternatives {
            let redis_alt = format!("{}:6379", host);
            if self.test_redis_connectivity_with_host(host, 6379).await.is_ok() {
                self.redis_host = host.to_string();
                log::info!("Self-healed Redis host to: {}", host);
                break;
            }
        }

        Ok(())
    }

    /// Attempt healing for Docker context
    async fn attempt_docker_healing(&mut self) -> Result<(), NetworkError> {
        // Try alternative Docker service names
        let kafka_alternatives = vec![
            "kafka:9092",
            "kafka-service:9092",
            "broker:9092",
        ];

        for alt in kafka_alternatives {
            if self.test_kafka_connectivity_with_broker(alt).await.is_ok() {
                self.kafka_broker = alt.to_string();
                log::info!("Self-healed Kafka broker to: {}", alt);
                break;
            }
        }

        Ok(())
    }

    /// Generic healing attempt
    async fn attempt_generic_healing(&mut self) -> Result<(), NetworkError> {
        // Try to reconnect with exponential backoff
        let max_attempts = 3;
        for attempt in 1..=max_attempts {
            log::info!("Attempting network healing (attempt {}/{})", attempt, max_attempts);
            
            self.check_health().await;
            if self.health_status.is_healthy() {
                log::info!("Network healing successful on attempt {}", attempt);
                return Ok(());
            }

            if attempt < max_attempts {
                let delay = Duration::from_secs(2u64.pow(attempt as u32));
                log::info!("Waiting {} seconds before next healing attempt", delay.as_secs());
                sleep(delay).await;
            }
        }

        Err(NetworkError::HealingFailed)
    }

    /// Test Kafka connectivity
    async fn test_kafka_connectivity(&self) -> Result<(), NetworkError> {
        self.test_kafka_connectivity_with_broker(&self.kafka_broker).await
    }

    /// Test Kafka connectivity with specific broker
    async fn test_kafka_connectivity_with_broker(&self, broker: &str) -> Result<(), NetworkError> {
        // Parse broker address
        let parts: Vec<&str> = broker.split(':').collect();
        if parts.len() != 2 {
            return Err(NetworkError::InvalidAddress);
        }

        let host = parts[0];
        let port: u16 = parts[1].parse().map_err(|_| NetworkError::InvalidAddress)?;

        // Test TCP connectivity
        let addr = format!("{}:{}", host, port);
        let socket_addr: SocketAddr = addr.parse().map_err(|_| NetworkError::InvalidAddress)?;

        match tokio::net::TcpStream::connect(socket_addr).await {
            Ok(_) => Ok(()),
            Err(_) => Err(NetworkError::ConnectionFailed),
        }
    }

    /// Test Redis connectivity
    async fn test_redis_connectivity(&self) -> Result<(), NetworkError> {
        self.test_redis_connectivity_with_host(&self.redis_host, self.redis_port).await
    }

    /// Test Redis connectivity with specific host
    async fn test_redis_connectivity_with_host(&self, host: &str, port: u16) -> Result<(), NetworkError> {
        let addr = format!("{}:{}", host, port);
        let socket_addr: SocketAddr = addr.parse().map_err(|_| NetworkError::InvalidAddress)?;

        match tokio::net::TcpStream::connect(socket_addr).await {
            Ok(_) => Ok(()),
            Err(_) => Err(NetworkError::ConnectionFailed),
        }
    }

    /// Test Prometheus connectivity
    async fn test_prometheus_connectivity(&self) -> Result<(), NetworkError> {
        let addr = format!("{}:{}", self.prometheus_host, self.prometheus_port);
        let socket_addr: SocketAddr = addr.parse().map_err(|_| NetworkError::InvalidAddress)?;

        match tokio::net::TcpStream::connect(socket_addr).await {
            Ok(_) => Ok(()),
            Err(_) => Err(NetworkError::ConnectionFailed),
        }
    }

    /// Get Kafka broker address
    pub fn kafka_broker(&self) -> &str {
        &self.kafka_broker
    }

    /// Get Redis connection string
    pub fn redis_url(&self) -> String {
        format!("{}:{}", self.redis_host, self.redis_port)
    }

    /// Get Prometheus endpoint
    pub fn prometheus_endpoint(&self) -> String {
        format!("{}:{}", self.prometheus_host, self.prometheus_port)
    }

    /// Get current health status
    pub fn health_status(&self) -> &NetworkHealthStatus {
        &self.health_status
    }

    /// Check if network is healthy
    pub fn is_healthy(&self) -> bool {
        self.health_status.is_healthy()
    }
}

/// Network error types
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Invalid network address")]
    InvalidAddress,
    #[error("Connection failed")]
    ConnectionFailed,
    #[error("Network healing failed after multiple attempts")]
    HealingFailed,
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Network health monitor for continuous monitoring
pub struct NetworkHealthMonitor {
    config: NetworkConfig,
    check_interval: Duration,
}

impl NetworkHealthMonitor {
    pub fn new(config: NetworkConfig, check_interval: Duration) -> Self {
        Self {
            config,
            check_interval,
        }
    }

    /// Start continuous health monitoring
    pub async fn start_monitoring(mut self) {
        loop {
            // Check health
            self.config.check_health().await;

            // Attempt self-healing if unhealthy
            if !self.config.is_healthy() {
                log::warn!("Network unhealthy, attempting self-healing...");
                if let Err(e) = self.config.self_heal().await {
                    log::error!("Self-healing failed: {}", e);
                    
                    // Alert for manual intervention if needed
                    if self.config.health_status.consecutive_failures > 5 {
                        log::error!("CRITICAL: Network requires manual intervention after {} consecutive failures", 
                                   self.config.health_status.consecutive_failures);
                    }
                }
            }

            // Wait before next check
            sleep(self.check_interval).await;
        }
    }
}
