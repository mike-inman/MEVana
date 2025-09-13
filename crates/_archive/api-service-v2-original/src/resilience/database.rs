//! Database resilience patterns
//!
//! Provides database-specific resilience implementations with:
//! - Connection pool management
//! - Query timeout handling
//! - Deadlock detection and retry
//! - Database health monitoring

use crate::{
    database::{DatabaseRepository, Entity},
    error::{ApiError, ApiResult},
};
use super::ResilienceService;
use sqlx::{Pool, Postgres, Row};
use std::{sync::Arc, time::Duration};
use tracing::{debug, warn, error, instrument};
use uuid::Uuid;

/// Resilient database repository wrapper
pub struct ResilientDatabaseRepository<T: Entity> {
    inner: Arc<dyn DatabaseRepository<T>>,
    resilience_service: Arc<ResilienceService>,
    pool: Pool<Postgres>,
}

impl<T: Entity> ResilientDatabaseRepository<T> {
    /// Create new resilient repository wrapper
    pub fn new(
        inner: Arc<dyn DatabaseRepository<T>>,
        resilience_service: Arc<ResilienceService>,
        pool: Pool<Postgres>,
    ) -> Self {
        Self {
            inner,
            resilience_service,
            pool,
        }
    }
    
    /// Execute operation with database resilience
    async fn execute_with_resilience<R, F, Fut>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> ApiResult<R>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<R, sqlx::Error>> + Send,
        R: Send,
    {
        self.resilience_service
            .execute_database_operation(operation_name, operation)
            .await
    }
}

#[async_trait::async_trait]
impl<T: Entity> DatabaseRepository<T> for ResilientDatabaseRepository<T>
where
    T: Send + Sync + 'static,
    T::Id: Send + Sync,
{
    #[instrument(skip(self, entity))]
    async fn create(&self, entity: &mut T, user_id: Uuid) -> ApiResult<T> {
        self.execute_with_resilience("create", || {
            let inner = self.inner.clone();
            let mut entity_clone = entity.clone();
            async move {
                inner.create(&mut entity_clone, user_id).await
                    .map_err(|e| match e {
                        ApiError::DatabaseError(msg) => sqlx::Error::Protocol(msg),
                        _ => sqlx::Error::Protocol("Repository error".to_string()),
                    })
            }
        }).await
    }
    
    #[instrument(skip(self))]
    async fn find_by_id(&self, id: T::Id) -> ApiResult<Option<T>> {
        self.execute_with_resilience("find_by_id", || {
            let inner = self.inner.clone();
            let id_clone = id.clone();
            async move {
                inner.find_by_id(id_clone).await
                    .map_err(|e| match e {
                        ApiError::DatabaseError(msg) => sqlx::Error::Protocol(msg),
                        _ => sqlx::Error::Protocol("Repository error".to_string()),
                    })
            }
        }).await
    }
    
    #[instrument(skip(self, entity))]
    async fn update(&self, entity: &T, user_id: Uuid) -> ApiResult<T> {
        self.execute_with_resilience("update", || {
            let inner = self.inner.clone();
            let entity_clone = entity.clone();
            async move {
                inner.update(&entity_clone, user_id).await
                    .map_err(|e| match e {
                        ApiError::DatabaseError(msg) => sqlx::Error::Protocol(msg),
                        _ => sqlx::Error::Protocol("Repository error".to_string()),
                    })
            }
        }).await
    }
    
    #[instrument(skip(self))]
    async fn delete(&self, id: T::Id, user_id: Uuid) -> ApiResult<bool> {
        self.execute_with_resilience("delete", || {
            let inner = self.inner.clone();
            let id_clone = id.clone();
            async move {
                inner.delete(id_clone, user_id).await
                    .map_err(|e| match e {
                        ApiError::DatabaseError(msg) => sqlx::Error::Protocol(msg),
                        _ => sqlx::Error::Protocol("Repository error".to_string()),
                    })
            }
        }).await
    }
    
    #[instrument(skip(self))]
    async fn find_all(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> ApiResult<Vec<T>> {
        self.execute_with_resilience("find_all", || {
            let inner = self.inner.clone();
            async move {
                inner.find_all(limit, offset).await
                    .map_err(|e| match e {
                        ApiError::DatabaseError(msg) => sqlx::Error::Protocol(msg),
                        _ => sqlx::Error::Protocol("Repository error".to_string()),
                    })
            }
        }).await
    }
    
    #[instrument(skip(self))]
    async fn count(&self) -> ApiResult<i64> {
        self.execute_with_resilience("count", || {
            let inner = self.inner.clone();
            async move {
                inner.count().await
                    .map_err(|e| match e {
                        ApiError::DatabaseError(msg) => sqlx::Error::Protocol(msg),
                        _ => sqlx::Error::Protocol("Repository error".to_string()),
                    })
            }
        }).await
    }
}

/// Database health checker
pub struct DatabaseHealthChecker {
    pool: Pool<Postgres>,
}

impl DatabaseHealthChecker {
    /// Create new database health checker
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
    
    /// Check database connectivity
    #[instrument(skip(self))]
    pub async fn check_connectivity(&self) -> ApiResult<DatabaseHealthInfo> {
        let start_time = std::time::Instant::now();
        
        // Simple connectivity test
        let connectivity_result = sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await;
        
        let connectivity_duration = start_time.elapsed();
        
        match connectivity_result {
            Ok(_) => {
                debug!("Database connectivity check passed in {:?}", connectivity_duration);
                
                // Get pool stats
                let pool_stats = self.get_pool_stats().await;
                
                Ok(DatabaseHealthInfo {
                    status: resilience_framework::health::ServiceHealth::Healthy,
                    connectivity_check_duration: connectivity_duration,
                    pool_stats,
                    last_check: chrono::Utc::now(),
                    error_message: None,
                })
            }
            Err(e) => {
                error!("Database connectivity check failed: {:?}", e);
                
                Ok(DatabaseHealthInfo {
                    status: resilience_framework::health::ServiceHealth::Unhealthy,
                    connectivity_check_duration: connectivity_duration,
                    pool_stats: None,
                    last_check: chrono::Utc::now(),
                    error_message: Some(e.to_string()),
                })
            }
        }
    }
    
    /// Get connection pool statistics
    async fn get_pool_stats(&self) -> Option<PoolStats> {
        Some(PoolStats {
            connections: self.pool.size(),
            idle_connections: self.pool.num_idle(),
            active_connections: self.pool.size() - self.pool.num_idle(),
        })
    }
    
    /// Check database performance
    #[instrument(skip(self))]
    pub async fn check_performance(&self) -> ApiResult<DatabasePerformanceInfo> {
        let start_time = std::time::Instant::now();
        
        // Execute a simple query to test performance
        let query_result = sqlx::query("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'")
            .fetch_one(&self.pool)
            .await;
        
        let query_duration = start_time.elapsed();
        
        match query_result {
            Ok(row) => {
                let table_count: i64 = row.try_get(0).unwrap_or(0);
                
                let performance_status = if query_duration > Duration::from_millis(1000) {
                    resilience_framework::health::ServiceHealth::Degraded
                } else {
                    resilience_framework::health::ServiceHealth::Healthy
                };
                
                Ok(DatabasePerformanceInfo {
                    status: performance_status,
                    query_duration,
                    table_count,
                    last_check: chrono::Utc::now(),
                })
            }
            Err(e) => {
                error!("Database performance check failed: {:?}", e);
                
                Ok(DatabasePerformanceInfo {
                    status: resilience_framework::health::ServiceHealth::Unhealthy,
                    query_duration,
                    table_count: 0,
                    last_check: chrono::Utc::now(),
                })
            }
        }
    }
    
    /// Comprehensive database health check
    pub async fn comprehensive_check(&self) -> DatabaseComprehensiveHealth {
        let connectivity = self.check_connectivity().await.unwrap_or_else(|_| {
            DatabaseHealthInfo {
                status: resilience_framework::health::ServiceHealth::Unhealthy,
                connectivity_check_duration: Duration::from_secs(0),
                pool_stats: None,
                last_check: chrono::Utc::now(),
                error_message: Some("Health check failed".to_string()),
            }
        });
        
        let performance = self.check_performance().await.unwrap_or_else(|_| {
            DatabasePerformanceInfo {
                status: resilience_framework::health::ServiceHealth::Unhealthy,
                query_duration: Duration::from_secs(0),
                table_count: 0,
                last_check: chrono::Utc::now(),
            }
        });
        
        let overall_status = match (&connectivity.status, &performance.status) {
            (resilience_framework::health::ServiceHealth::Healthy, resilience_framework::health::ServiceHealth::Healthy) => {
                resilience_framework::health::ServiceHealth::Healthy
            }
            (resilience_framework::health::ServiceHealth::Unhealthy, _) | 
            (_, resilience_framework::health::ServiceHealth::Unhealthy) => {
                resilience_framework::health::ServiceHealth::Unhealthy
            }
            _ => resilience_framework::health::ServiceHealth::Degraded,
        };
        
        DatabaseComprehensiveHealth {
            overall_status,
            connectivity,
            performance,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Database health information
#[derive(Debug, serde::Serialize)]
pub struct DatabaseHealthInfo {
    pub status: resilience_framework::health::ServiceHealth,
    pub connectivity_check_duration: Duration,
    pub pool_stats: Option<PoolStats>,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub error_message: Option<String>,
}

/// Database performance information
#[derive(Debug, serde::Serialize)]
pub struct DatabasePerformanceInfo {
    pub status: resilience_framework::health::ServiceHealth,
    pub query_duration: Duration,
    pub table_count: i64,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Connection pool statistics
#[derive(Debug, serde::Serialize)]
pub struct PoolStats {
    pub connections: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
}

/// Comprehensive database health
#[derive(Debug, serde::Serialize)]
pub struct DatabaseComprehensiveHealth {
    pub overall_status: resilience_framework::health::ServiceHealth,
    pub connectivity: DatabaseHealthInfo,
    pub performance: DatabasePerformanceInfo,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}