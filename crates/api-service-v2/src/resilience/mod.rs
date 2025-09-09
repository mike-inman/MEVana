//! Resilience framework integration
//!
//! Provides enterprise-grade resilience patterns with:
//! - Circuit breaker for database and external services
//! - Retry policies with exponential backoff
//! - Rate limiting and bulkhead isolation
//! - Health checking and service discovery

use crate::error::{ApiError, ApiResult};
use resilience_framework::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState},
    retry::{RetryPolicy, ExponentialBackoff},
    health::{HealthChecker, ServiceHealth},
};
use std::{sync::Arc, time::Duration};
use tracing::{debug, warn, error, instrument};

pub mod database;
pub mod external_services;
pub mod health;

/// Resilience service coordinator
pub struct ResilienceService {
    database_circuit_breaker: Arc<CircuitBreaker>,
    external_service_circuit_breaker: Arc<CircuitBreaker>,
    health_checker: Arc<HealthChecker>,
    retry_policy: RetryPolicy,
}

impl ResilienceService {
    /// Create new resilience service
    pub async fn new(config: ResilienceConfig) -> ApiResult<Self> {
        // Database circuit breaker
        let db_cb_config = CircuitBreakerConfig {
            failure_threshold: config.database_failure_threshold,
            timeout_duration: Duration::from_millis(config.database_timeout_ms),
            reset_timeout: Duration::from_secs(config.database_reset_timeout_secs),
            health_check_interval: Some(Duration::from_secs(30)),
        };
        let database_circuit_breaker = Arc::new(CircuitBreaker::new(db_cb_config).await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to create database circuit breaker: {:?}", e)))?);
        
        // External services circuit breaker
        let ext_cb_config = CircuitBreakerConfig {
            failure_threshold: config.external_service_failure_threshold,
            timeout_duration: Duration::from_millis(config.external_service_timeout_ms),
            reset_timeout: Duration::from_secs(config.external_service_reset_timeout_secs),
            health_check_interval: Some(Duration::from_secs(60)),
        };
        let external_service_circuit_breaker = Arc::new(CircuitBreaker::new(ext_cb_config).await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to create external service circuit breaker: {:?}", e)))?);
        
        // Health checker
        let health_checker = Arc::new(HealthChecker::new().await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to create health checker: {:?}", e)))?);
        
        // Retry policy
        let retry_policy = RetryPolicy::ExponentialBackoff(ExponentialBackoff {
            max_retries: config.max_retries,
            base_delay: Duration::from_millis(config.base_delay_ms),
            max_delay: Duration::from_secs(config.max_delay_secs),
            jitter: true,
        });
        
        Ok(Self {
            database_circuit_breaker,
            external_service_circuit_breaker,
            health_checker,
            retry_policy,
        })
    }
    
    /// Execute database operation with resilience
    #[instrument(skip_all)]
    pub async fn execute_database_operation<T, F, Fut>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> ApiResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T, sqlx::Error>> + Send,
        T: Send,
    {
        debug!("Executing database operation: {}", operation_name);
        
        // Check circuit breaker state
        match self.database_circuit_breaker.state().await {
            CircuitBreakerState::Open => {
                warn!("Database circuit breaker is open for operation: {}", operation_name);
                return Err(ApiError::ServiceUnavailable("Database service unavailable".to_string()));
            }
            CircuitBreakerState::HalfOpen => {
                debug!("Database circuit breaker is half-open for operation: {}", operation_name);
            }
            CircuitBreakerState::Closed => {
                debug!("Database circuit breaker is closed for operation: {}", operation_name);
            }
        }
        
        // Execute with retry policy
        let result = self.retry_policy.execute(|| async {
            match operation().await {
                Ok(value) => {
                    self.database_circuit_breaker.record_success().await;
                    Ok(value)
                }
                Err(e) => {
                    self.database_circuit_breaker.record_failure().await;
                    Err(e)
                }
            }
        }).await;
        
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                error!("Database operation failed after retries: {} - {:?}", operation_name, e);
                Err(ApiError::from(e))
            }
        }
    }
    
    /// Execute external service call with resilience
    #[instrument(skip_all)]
    pub async fn execute_external_service_call<T, F, Fut>(
        &self,
        service_name: &str,
        operation: F,
    ) -> ApiResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T, reqwest::Error>> + Send,
        T: Send,
    {
        debug!("Executing external service call: {}", service_name);
        
        // Check circuit breaker state
        match self.external_service_circuit_breaker.state().await {
            CircuitBreakerState::Open => {
                warn!("External service circuit breaker is open for service: {}", service_name);
                return Err(ApiError::ServiceUnavailable(format!("External service '{}' unavailable", service_name)));
            }
            CircuitBreakerState::HalfOpen => {
                debug!("External service circuit breaker is half-open for service: {}", service_name);
            }
            CircuitBreakerState::Closed => {
                debug!("External service circuit breaker is closed for service: {}", service_name);
            }
        }
        
        // Execute with retry policy
        let result = self.retry_policy.execute(|| async {
            match operation().await {
                Ok(value) => {
                    self.external_service_circuit_breaker.record_success().await;
                    Ok(value)
                }
                Err(e) => {
                    self.external_service_circuit_breaker.record_failure().await;
                    Err(e)
                }
            }
        }).await;
        
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                error!("External service call failed after retries: {} - {:?}", service_name, e);
                Err(ApiError::ExternalServiceError(format!("Service '{}' failed: {}", service_name, e)))
            }
        }
    }
    
    /// Get overall system health
    pub async fn get_system_health(&self) -> SystemHealth {
        let database_health = match self.database_circuit_breaker.state().await {
            CircuitBreakerState::Closed => ServiceHealth::Healthy,
            CircuitBreakerState::HalfOpen => ServiceHealth::Degraded,
            CircuitBreakerState::Open => ServiceHealth::Unhealthy,
        };
        
        let external_services_health = match self.external_service_circuit_breaker.state().await {
            CircuitBreakerState::Closed => ServiceHealth::Healthy,
            CircuitBreakerState::HalfOpen => ServiceHealth::Degraded,
            CircuitBreakerState::Open => ServiceHealth::Unhealthy,
        };
        
        // Get detailed health from health checker
        let detailed_health = self.health_checker.check_all().await.unwrap_or_default();
        
        SystemHealth {
            overall: calculate_overall_health(&[database_health, external_services_health]),
            database: database_health,
            external_services: external_services_health,
            details: detailed_health,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Reset circuit breakers (admin operation)
    pub async fn reset_circuit_breakers(&self) -> ApiResult<()> {
        debug!("Resetting all circuit breakers");
        
        self.database_circuit_breaker.reset().await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to reset database circuit breaker: {:?}", e)))?;
        
        self.external_service_circuit_breaker.reset().await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to reset external service circuit breaker: {:?}", e)))?;
        
        debug!("All circuit breakers reset successfully");
        Ok(())
    }
    
    /// Get circuit breaker statistics
    pub async fn get_circuit_breaker_stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            database: CircuitBreakerInfo {
                state: format!("{:?}", self.database_circuit_breaker.state().await),
                failure_count: self.database_circuit_breaker.failure_count().await.unwrap_or(0),
                success_count: self.database_circuit_breaker.success_count().await.unwrap_or(0),
                last_failure_time: self.database_circuit_breaker.last_failure_time().await,
            },
            external_services: CircuitBreakerInfo {
                state: format!("{:?}", self.external_service_circuit_breaker.state().await),
                failure_count: self.external_service_circuit_breaker.failure_count().await.unwrap_or(0),
                success_count: self.external_service_circuit_breaker.success_count().await.unwrap_or(0),
                last_failure_time: self.external_service_circuit_breaker.last_failure_time().await,
            },
        }
    }
}

/// Resilience configuration
#[derive(Debug, Clone)]
pub struct ResilienceConfig {
    // Database circuit breaker
    pub database_failure_threshold: u32,
    pub database_timeout_ms: u64,
    pub database_reset_timeout_secs: u64,
    
    // External service circuit breaker
    pub external_service_failure_threshold: u32,
    pub external_service_timeout_ms: u64,
    pub external_service_reset_timeout_secs: u64,
    
    // Retry policy
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_secs: u64,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            database_failure_threshold: 5,
            database_timeout_ms: 5000,
            database_reset_timeout_secs: 30,
            external_service_failure_threshold: 10,
            external_service_timeout_ms: 10000,
            external_service_reset_timeout_secs: 60,
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_secs: 5,
        }
    }
}

/// System health information
#[derive(Debug, serde::Serialize)]
pub struct SystemHealth {
    pub overall: ServiceHealth,
    pub database: ServiceHealth,
    pub external_services: ServiceHealth,
    pub details: std::collections::HashMap<String, ServiceHealth>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Circuit breaker statistics
#[derive(Debug, serde::Serialize)]
pub struct CircuitBreakerStats {
    pub database: CircuitBreakerInfo,
    pub external_services: CircuitBreakerInfo,
}

/// Circuit breaker information
#[derive(Debug, serde::Serialize)]
pub struct CircuitBreakerInfo {
    pub state: String,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Calculate overall health from individual service health
fn calculate_overall_health(healths: &[ServiceHealth]) -> ServiceHealth {
    if healths.iter().all(|h| matches!(h, ServiceHealth::Healthy)) {
        ServiceHealth::Healthy
    } else if healths.iter().any(|h| matches!(h, ServiceHealth::Unhealthy)) {
        ServiceHealth::Unhealthy
    } else {
        ServiceHealth::Degraded
    }
}