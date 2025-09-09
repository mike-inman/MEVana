//! Database connection pool management
//!
//! Handles database connections with:
//! - Automatic connection pooling
//! - Health checks and reconnection
//! - Integration with resilience framework
//! - Connection lifecycle management

use super::DatabaseConfig;
use crate::error::{ApiError, ApiResult};
use resilience_framework::{ResilientDatabase, DatabaseConfig as ResilienceDbConfig};
use sqlx::{PgPool, postgres::PgPoolOptions, Pool, Postgres};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error, instrument};

/// Type alias for our database pool
pub type DatabasePool = Arc<PgPool>;

/// Create a new database connection pool with resilience features
#[instrument(skip(config))]
pub async fn create_connection_pool(config: &DatabaseConfig) -> ApiResult<DatabasePool> {
    info!("Creating database connection pool");
    
    let pool_options = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.connection_timeout_seconds))
        .idle_timeout(Some(Duration::from_secs(config.idle_timeout_seconds)))
        .max_lifetime(Some(Duration::from_secs(config.max_lifetime_seconds)))
        .test_before_acquire(true);
    
    let pool = if config.enable_logging {
        pool_options
            .connect(&config.url)
            .await
            .map_err(|e| {
                error!("Failed to create database pool: {:?}", e);
                ApiError::DatabaseError(format!("Failed to connect to database: {}", e))
            })?
    } else {
        pool_options
            .connect(&config.url)
            .await
            .map_err(|e| {
                error!("Failed to create database pool: {:?}", e);
                ApiError::DatabaseError(format!("Failed to connect to database: {}", e))
            })?
    };
    
    // Test the connection
    test_connection(&pool).await?;
    
    info!(
        "Database connection pool created successfully with {} max connections",
        config.max_connections
    );
    
    Ok(Arc::new(pool))
}

/// Create a resilient database wrapper for advanced features
#[instrument(skip(config))]
pub async fn create_resilient_database(config: &DatabaseConfig) -> ApiResult<ResilientDatabase> {
    info!("Creating resilient database connection");
    
    let resilience_config = ResilienceDbConfig {
        primary_url: config.url.clone(),
        replica_url: None, // TODO: Add replica support
        max_connections: config.max_connections,
        connection_timeout: Duration::from_secs(config.connection_timeout_seconds),
        circuit_breaker: resilience_framework::CircuitBreakerConfig::default(),
        retry_policy: resilience_framework::RetryPolicy::default(),
        enable_metrics: true,
    };
    
    let resilient_db = ResilientDatabase::new(resilience_config)
        .await
        .map_err(|e| {
            error!("Failed to create resilient database: {:?}", e);
            ApiError::DatabaseError(format!("Failed to create resilient database: {}", e))
        })?;
    
    info!("Resilient database connection created successfully");
    
    Ok(resilient_db)
}

/// Test database connection health
#[instrument(skip(pool))]
async fn test_connection(pool: &PgPool) -> ApiResult<()> {
    info!("Testing database connection");
    
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(|e| {
            error!("Database health check failed: {:?}", e);
            ApiError::DatabaseError(format!("Database health check failed: {}", e))
        })?;
    
    info!("Database connection test successful");
    Ok(())
}

/// Database health check structure
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseHealth {
    pub status: HealthStatus,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub max_connections: u32,
    pub response_time_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Perform comprehensive database health check
#[instrument(skip(pool))]
pub async fn check_database_health(pool: &DatabasePool) -> DatabaseHealth {
    let start_time = std::time::Instant::now();
    
    // Test basic connectivity
    let status = match sqlx::query("SELECT 1").execute(pool.as_ref()).await {
        Ok(_) => {
            let active = pool.num_idle();
            let max_conn = pool.options().get_max_connections();
            
            if active as f32 / max_conn as f32 > 0.9 {
                warn!("Database connection pool usage high: {}/{}", active, max_conn);
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            }
        }
        Err(e) => {
            error!("Database health check failed: {:?}", e);
            HealthStatus::Unhealthy
        }
    };
    
    let response_time = start_time.elapsed();
    
    DatabaseHealth {
        status,
        active_connections: pool.size() as usize,
        idle_connections: pool.num_idle() as usize,
        max_connections: pool.options().get_max_connections(),
        response_time_ms: response_time.as_millis() as u64,
    }
}

/// Connection pool metrics for monitoring
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolMetrics {
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub max_connections: u32,
    pub utilization_percent: f32,
}

/// Get current pool metrics
pub fn get_pool_metrics(pool: &DatabasePool) -> PoolMetrics {
    let total = pool.size() as usize;
    let idle = pool.num_idle() as usize;
    let active = total - idle;
    let max_connections = pool.options().get_max_connections();
    let utilization = (total as f32 / max_connections as f32) * 100.0;
    
    PoolMetrics {
        total_connections: total,
        active_connections: active,
        idle_connections: idle,
        max_connections,
        utilization_percent: utilization,
    }
}

/// Gracefully close database pool
#[instrument(skip(pool))]
pub async fn close_pool(pool: DatabasePool) {
    info!("Closing database connection pool");
    
    // Wait for active connections to finish (with timeout)
    let close_timeout = Duration::from_secs(30);
    
    tokio::select! {
        _ = tokio::time::sleep(close_timeout) => {
            warn!("Database pool close timeout reached, forcing closure");
        }
        _ = async {
            pool.close().await;
        } => {
            info!("Database pool closed gracefully");
        }
    }
}