use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::retry::{RetryExecutor, RetryPolicy};
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct ConnectionPool {
    primary: Pool,
    replica: Option<Pool>,
    circuit_breaker: Arc<CircuitBreaker>,
    _retry_executor: RetryExecutor,
    metrics: Arc<RwLock<DatabaseMetrics>>,
}

#[derive(Clone)]
pub struct DatabaseConfig {
    pub primary_url: String,
    pub replica_url: Option<String>,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub retry_policy: RetryPolicy,
    pub circuit_breaker: CircuitBreakerConfig,
    pub enable_metrics: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            primary_url: "postgresql://localhost/mevana".to_string(),
            replica_url: None,
            max_connections: 20,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
            retry_policy: RetryPolicy::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
            enable_metrics: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseMetrics {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub retried_queries: u64,
    pub circuit_breaker_trips: u64,
    pub average_query_time_ms: f64,
    pub connection_pool_size: usize,
    pub active_connections: usize,
}

impl Default for DatabaseMetrics {
    fn default() -> Self {
        DatabaseMetrics {
            total_queries: 0,
            successful_queries: 0,
            failed_queries: 0,
            retried_queries: 0,
            circuit_breaker_trips: 0,
            average_query_time_ms: 0.0,
            connection_pool_size: 0,
            active_connections: 0,
        }
    }
}

impl ConnectionPool {
    pub async fn new(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let mut pg_config = Config::new();
        pg_config.url = Some(config.primary_url.clone());
        pg_config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        pg_config.pool = Some(deadpool::managed::PoolConfig {
            max_size: config.max_connections,
            timeouts: deadpool::managed::Timeouts::default(),
            ..Default::default()
        });

        let primary = pg_config
            .create_pool(None, tokio_postgres::NoTls)
            .map_err(|e| DatabaseError::PoolCreation(e.to_string()))?;

        let replica = if let Some(replica_url) = config.replica_url {
            let mut replica_config = pg_config.clone();
            replica_config.url = Some(replica_url);
            Some(
                replica_config
                    .create_pool(None, tokio_postgres::NoTls)
                    .map_err(|e| DatabaseError::PoolCreation(e.to_string()))?,
            )
        } else {
            None
        };

        let circuit_breaker = Arc::new(CircuitBreaker::new(config.circuit_breaker));
        let retry_executor = RetryExecutor::new(config.retry_policy);
        let metrics = Arc::new(RwLock::new(DatabaseMetrics::default()));

        Ok(ConnectionPool {
            primary,
            replica,
            circuit_breaker,
            _retry_executor: retry_executor,
            metrics,
        })
    }

    pub async fn execute_query<T, F>(&self, query_fn: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(
                deadpool_postgres::Client,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T, tokio_postgres::Error>> + Send>,
            > + Send
            + 'static,
        T: Send + 'static,
    {
        let start_time = std::time::Instant::now();
        let mut metrics = self.metrics.write().await;
        metrics.total_queries += 1;
        drop(metrics);

        let pool = &self.primary;
        let client = pool
            .get()
            .await
            .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        let result = query_fn(client)
            .await
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()));

        let duration = start_time.elapsed();
        let mut metrics = self.metrics.write().await;

        match &result {
            Ok(_) => {
                metrics.successful_queries += 1;
            }
            Err(_) => {
                metrics.failed_queries += 1;
            }
        }

        metrics.average_query_time_ms = (metrics.average_query_time_ms
            * (metrics.total_queries - 1) as f64
            + duration.as_millis() as f64)
            / metrics.total_queries as f64;

        result
    }

    pub async fn execute_read_query<T, F>(&self, query_fn: F) -> Result<T, DatabaseError>
    where
        F: Fn(
                deadpool_postgres::Client,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T, tokio_postgres::Error>> + Send>,
            > + Clone
            + Send
            + 'static,
        T: Send + 'static,
    {
        if let Some(replica) = &self.replica {
            let replica_result = self
                .circuit_breaker
                .call(async {
                    let client = replica
                        .get()
                        .await
                        .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

                    query_fn.clone()(client)
                        .await
                        .map_err(|e| DatabaseError::QueryFailed(e.to_string()))
                })
                .await;

            match replica_result {
                Ok(result) => Ok(result),
                Err(_) => {
                    log::warn!("Replica query failed, falling back to primary");
                    self.execute_query(query_fn).await
                }
            }
        } else {
            self.execute_query(query_fn).await
        }
    }

    pub async fn health_check(&self) -> Result<DatabaseHealthStatus, DatabaseError> {
        let start = std::time::Instant::now();

        let primary_health = self.check_pool_health(&self.primary).await;
        let replica_health = if let Some(replica) = &self.replica {
            Some(self.check_pool_health(replica).await)
        } else {
            None
        };

        let response_time = start.elapsed();

        let status = match (&primary_health, &replica_health) {
            (Ok(_), Some(Ok(_))) | (Ok(_), None) => DatabaseHealthStatus::Healthy,
            (Ok(_), Some(Err(_))) => DatabaseHealthStatus::Degraded,
            (Err(_), Some(Ok(_))) => DatabaseHealthStatus::Degraded,
            (Err(_), Some(Err(_))) | (Err(_), None) => DatabaseHealthStatus::Unhealthy,
        };

        log::info!(
            "Database health check: {:?} (response_time: {:?})",
            status,
            response_time
        );

        Ok(status)
    }

    async fn check_pool_health(&self, pool: &Pool) -> Result<(), DatabaseError> {
        let client = pool
            .get()
            .await
            .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        client
            .query_one("SELECT 1", &[])
            .await
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;

        Ok(())
    }

    pub async fn get_metrics(&self) -> DatabaseMetrics {
        let mut metrics = self.metrics.read().await.clone();

        metrics.connection_pool_size = self.primary.status().max_size;
        metrics.active_connections = self.primary.status().size;

        metrics
    }

    pub async fn reset_circuit_breaker(&self) {
        self.circuit_breaker.reset().await;
    }
}

#[derive(Debug, Clone)]
pub enum DatabaseHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub struct ResilientDatabase {
    pool: Arc<ConnectionPool>,
}

impl ResilientDatabase {
    pub async fn new(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let pool = Arc::new(ConnectionPool::new(config).await?);
        Ok(ResilientDatabase { pool })
    }

    // Helper method for executing queries with owned parameters that can be moved into closures
    pub async fn query_with_params(
        &self,
        sql: String,
        params: Vec<serde_json::Value>,
    ) -> Result<Vec<tokio_postgres::Row>, DatabaseError> {
        self.pool
            .execute_query(move |client| {
                Box::pin(async move {
                    // Special handling for common parameter types to work around trait bounds
                    match params.len() {
                        0 => client.query(&sql, &[]).await,
                        1 => {
                            let p0 = convert_json_to_string(&params[0]);
                            client.query(&sql, &[&p0]).await
                        }
                        2 => {
                            let p0 = convert_json_to_string(&params[0]);
                            let p1 = convert_json_timestamp_or_string(&params[1]);
                            client.query(&sql, &[&p0, &p1]).await
                        }
                        13 => {
                            // Handle the specific case for arbitrage opportunities
                            let p0 = convert_json_to_string(&params[0]);
                            let p1 = convert_json_timestamp_or_string(&params[1]);
                            let p2 = convert_json_to_i64(&params[2]);
                            let p3 = convert_json_to_string(&params[3]);
                            let p4 = convert_json_to_string(&params[4]);
                            let p5 = convert_json_to_string(&params[5]);
                            let p6 = convert_json_to_i32(&params[6]);
                            let p7 = convert_json_to_f64(&params[7]);
                            let p8 = convert_json_to_f64(&params[8]);
                            let p9 = convert_json_to_optional_f64(&params[9]);
                            let p10 = convert_json_to_i64(&params[10]);
                            let p11 = convert_json_to_string(&params[11]);
                            let p12 = convert_json_to_jsonb(&params[12]);

                            client
                                .query(
                                    &sql,
                                    &[
                                        &p0, &p1, &p2, &p3, &p4, &p5, &p6, &p7, &p8, &p9, &p10,
                                        &p11, &p12,
                                    ],
                                )
                                .await
                        }
                        _ => {
                            // Fallback: convert all to strings
                            let string_params: Vec<String> =
                                params.iter().map(convert_json_to_string).collect();
                            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                                string_params
                                    .iter()
                                    .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
                                    .collect();
                            client.query(&sql, &param_refs).await
                        }
                    }
                })
            })
            .await
    }

    pub async fn query_one(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<tokio_postgres::Row, DatabaseError> {
        let sql = sql.to_string();
        // Convert parameters to strings for safe moving into closure
        let string_params: Vec<String> =
            params.iter().map(|param| format!("{:?}", param)).collect();

        self.pool
            .execute_query(move |client| {
                Box::pin(async move {
                    // Create parameter references
                    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = string_params
                        .iter()
                        .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
                        .collect();
                    client.query_one(&sql, &param_refs).await
                })
            })
            .await
    }

    pub async fn query(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>, DatabaseError> {
        let sql = sql.to_string();
        // Convert parameters to strings for safe moving into closure
        let string_params: Vec<String> =
            params.iter().map(|param| format!("{:?}", param)).collect();

        self.pool
            .execute_query(move |client| {
                Box::pin(async move {
                    // Create parameter references
                    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = string_params
                        .iter()
                        .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
                        .collect();
                    client.query(&sql, &param_refs).await
                })
            })
            .await
    }

    pub async fn execute(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<u64, DatabaseError> {
        let sql = sql.to_string();
        // Convert parameters to strings for safe moving into closure
        let string_params: Vec<String> =
            params.iter().map(|param| format!("{:?}", param)).collect();

        self.pool
            .execute_query(move |client| {
                Box::pin(async move {
                    // Create parameter references
                    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = string_params
                        .iter()
                        .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
                        .collect();
                    client.execute(&sql, &param_refs).await
                })
            })
            .await
    }

    pub async fn transaction<T, F>(&self, transaction_fn: F) -> Result<T, DatabaseError>
    where
        F: for<'a> FnOnce(
                &'a tokio_postgres::Transaction<'a>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T, tokio_postgres::Error>> + Send + 'a>,
            > + Send
            + 'static,
        T: Send + 'static,
    {
        self.pool
            .execute_query(move |mut client| {
                Box::pin(async move {
                    let transaction = client.transaction().await?;
                    let result = transaction_fn(&transaction).await?;
                    transaction.commit().await?;
                    Ok(result)
                })
            })
            .await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Pool creation failed: {0}")]
    PoolCreation(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Query failed: {0}")]
    QueryFailed(String),
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Helper functions for JSON to SQL type conversion
fn convert_json_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "".to_string(),
        _ => value.to_string(),
    }
}

fn convert_json_timestamp_or_string(value: &serde_json::Value) -> chrono::DateTime<chrono::Utc> {
    match value {
        serde_json::Value::String(s) => {
            // Try to parse as RFC3339 timestamp
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now())
        }
        _ => chrono::Utc::now(),
    }
}

fn convert_json_to_i64(value: &serde_json::Value) -> i64 {
    match value {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
        _ => 0,
    }
}

fn convert_json_to_i32(value: &serde_json::Value) -> i32 {
    match value {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0) as i32,
        _ => 0,
    }
}

fn convert_json_to_f64(value: &serde_json::Value) -> f64 {
    match value {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn convert_json_to_optional_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::Null => None,
        _ => None,
    }
}

fn convert_json_to_jsonb(value: &serde_json::Value) -> serde_json::Value {
    // With the with-serde_json-1 feature, serde_json::Value implements ToSql for JSONB
    value.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.max_connections, 20);
        assert!(config.enable_metrics);
    }
}
