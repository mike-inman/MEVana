use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use reqwest::Client;
use std::time::Duration;
use tokio::time::timeout;

use crate::config::{HealthCheckConfig, ServiceConfig};

#[derive(Debug, Clone)]
pub struct HealthChecker {
    client: Client,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
    Timeout,
    Unknown,
}

impl HealthChecker {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for health checks");

        Self { client }
    }

    pub async fn check_service_health(&self, service: &ServiceConfig) -> HealthStatus {
        debug!("🔍 Health checking service: {}", service.name);

        // Try multiple health check methods
        if let Some(endpoint) = &service.health_check.endpoint {
            match self
                .check_http_endpoint(endpoint, &service.health_check)
                .await
            {
                HealthStatus::Healthy => return HealthStatus::Healthy,
                status => debug!(
                    "HTTP endpoint check failed for {}: {:?}",
                    service.name, status
                ),
            }
        }

        if let Some(port) = service.health_check.port {
            match self
                .check_port_connectivity(port, &service.health_check)
                .await
            {
                HealthStatus::Healthy => return HealthStatus::Healthy,
                status => debug!(
                    "Port connectivity check failed for {}: {:?}",
                    service.name, status
                ),
            }
        }

        if let Some(custom_check) = &service.health_check.custom_check {
            return self
                .run_custom_health_check(custom_check, &service.health_check)
                .await;
        }

        // If no specific health check is configured, assume healthy
        // (This is for services that don't expose health endpoints)
        warn!(
            "⚠️  No health check configured for service: {}",
            service.name
        );
        HealthStatus::Unknown
    }

    async fn check_http_endpoint(
        &self,
        endpoint: &str,
        config: &HealthCheckConfig,
    ) -> HealthStatus {
        for attempt in 1..=config.retries {
            match timeout(config.timeout, self.client.get(endpoint).send()).await {
                Ok(Ok(response)) => {
                    if response.status().is_success() {
                        debug!("✅ HTTP health check passed: {}", endpoint);
                        return HealthStatus::Healthy;
                    } else {
                        debug!(
                            "❌ HTTP health check failed with status {}: {}",
                            response.status(),
                            endpoint
                        );
                        if attempt == config.retries {
                            return HealthStatus::Unhealthy(format!("HTTP {}", response.status()));
                        }
                    }
                }
                Ok(Err(e)) => {
                    debug!(
                        "❌ HTTP health check request failed (attempt {}): {} - {}",
                        attempt, endpoint, e
                    );
                    if attempt == config.retries {
                        return HealthStatus::Unhealthy(format!("Request failed: {}", e));
                    }
                }
                Err(_) => {
                    debug!(
                        "⏰ HTTP health check timed out (attempt {}): {}",
                        attempt, endpoint
                    );
                    if attempt == config.retries {
                        return HealthStatus::Timeout;
                    }
                }
            }

            if attempt < config.retries {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        HealthStatus::Unhealthy("Max retries exceeded".to_string())
    }

    async fn check_port_connectivity(&self, port: u16, config: &HealthCheckConfig) -> HealthStatus {
        for attempt in 1..=config.retries {
            match timeout(
                config.timeout,
                tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)),
            )
            .await
            {
                Ok(Ok(_)) => {
                    debug!("✅ Port connectivity check passed: {}", port);
                    return HealthStatus::Healthy;
                }
                Ok(Err(e)) => {
                    debug!(
                        "❌ Port connectivity check failed (attempt {}): {} - {}",
                        attempt, port, e
                    );
                    if attempt == config.retries {
                        return HealthStatus::Unhealthy(format!("Port connection failed: {}", e));
                    }
                }
                Err(_) => {
                    debug!(
                        "⏰ Port connectivity check timed out (attempt {}): {}",
                        attempt, port
                    );
                    if attempt == config.retries {
                        return HealthStatus::Timeout;
                    }
                }
            }

            if attempt < config.retries {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }

        HealthStatus::Unhealthy("Max retries exceeded".to_string())
    }

    async fn run_custom_health_check(
        &self,
        command: &str,
        config: &HealthCheckConfig,
    ) -> HealthStatus {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(command);

        match timeout(config.timeout, cmd.output()).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    debug!("✅ Custom health check passed: {}", command);
                    HealthStatus::Healthy
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    debug!("❌ Custom health check failed: {} - {}", command, stderr);
                    HealthStatus::Unhealthy(format!("Command failed: {}", stderr))
                }
            }
            Ok(Err(e)) => {
                debug!(
                    "❌ Custom health check execution failed: {} - {}",
                    command, e
                );
                HealthStatus::Unhealthy(format!("Execution failed: {}", e))
            }
            Err(_) => {
                debug!("⏰ Custom health check timed out: {}", command);
                HealthStatus::Timeout
            }
        }
    }

    pub async fn wait_for_service_healthy(
        &self,
        service: &ServiceConfig,
        max_wait: Duration,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();
        let check_interval = Duration::from_secs(5);

        info!(
            "⏳ Waiting for service '{}' to become healthy (max wait: {:?})",
            service.name, max_wait
        );

        loop {
            if start_time.elapsed() >= max_wait {
                return Err(anyhow::anyhow!(
                    "Service '{}' failed to become healthy within {:?}",
                    service.name,
                    max_wait
                ));
            }

            match self.check_service_health(service).await {
                HealthStatus::Healthy => {
                    info!("✅ Service '{}' is healthy", service.name);
                    return Ok(());
                }
                HealthStatus::Unknown => {
                    warn!(
                        "⚠️  Service '{}' health status unknown, assuming healthy",
                        service.name
                    );
                    return Ok(());
                }
                status => {
                    debug!(
                        "⏳ Service '{}' not yet healthy: {:?}, retrying...",
                        service.name, status
                    );
                    tokio::time::sleep(check_interval).await;
                }
            }
        }
    }

    #[allow(dead_code)]
    pub async fn check_infrastructure_services(&self) -> Result<()> {
        info!("🔍 Checking infrastructure services...");

        // Check Redis
        match self.check_redis().await {
            Ok(_) => info!("✅ Redis is healthy"),
            Err(e) => {
                error!("❌ Redis health check failed: {}", e);
                return Err(anyhow::anyhow!("Redis is not healthy: {}", e));
            }
        }

        // Check Kafka
        match self.check_kafka().await {
            Ok(_) => info!("✅ Kafka is healthy"),
            Err(e) => {
                error!("❌ Kafka health check failed: {}", e);
                return Err(anyhow::anyhow!("Kafka is not healthy: {}", e));
            }
        }

        // Check PostgreSQL
        match self.check_postgres().await {
            Ok(_) => info!("✅ PostgreSQL is healthy"),
            Err(e) => {
                error!("❌ PostgreSQL health check failed: {}", e);
                return Err(anyhow::anyhow!("PostgreSQL is not healthy: {}", e));
            }
        }

        info!("✅ All infrastructure services are healthy");
        Ok(())
    }

    #[allow(dead_code)]
    async fn check_redis(&self) -> Result<()> {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let client = redis::Client::open(redis_url)?;
        let mut conn = client.get_multiplexed_tokio_connection().await?;
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    async fn check_kafka(&self) -> Result<()> {
        // Simple TCP connection check for Kafka
        tokio::net::TcpStream::connect("localhost:9092")
            .await
            .context("Failed to connect to Kafka")?;
        Ok(())
    }

    #[allow(dead_code)]
    async fn check_postgres(&self) -> Result<()> {
        let postgres_url = std::env::var("DATABASE_URL").context(
            "DATABASE_URL environment variable is required - no hardcoded fallbacks allowed",
        )?;

        let (client, connection) =
            tokio_postgres::connect(&postgres_url, tokio_postgres::NoTls).await?;

        // Spawn the connection to run in the background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        // Simple query to check connectivity
        client.execute("SELECT 1", &[]).await?;
        Ok(())
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}
