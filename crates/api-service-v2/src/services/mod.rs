//! Service layer coordination
//!
//! Provides service orchestration with:
//! - Database connection management
//! - Authentication service setup
//! - Resilience service coordination
//! - External service integrations

use crate::{
    auth::JwtAuthService,
    config::ApiConfig,
    error::{ApiError, ApiResult},
    resilience::ResilienceService,
};
use redis::aio::ConnectionManager;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tracing::{info, error};

/// Main services container
pub struct ApiServices {
    pub database: Pool<Postgres>,
    pub redis: ConnectionManager,
    pub auth: Arc<JwtAuthService>,
    pub resilience: Arc<ResilienceService>,
}

impl ApiServices {
    /// Initialize all services
    pub async fn new(config: &ApiConfig) -> ApiResult<Self> {
        info!("🔧 Initializing API services");
        
        // Database connection
        let database = Self::setup_database(config).await?;
        
        // Redis connection
        let redis = Self::setup_redis(config).await?;
        
        // Authentication service
        let auth = Self::setup_auth_service(config, redis.clone()).await?;
        
        // Resilience service
        let resilience = Self::setup_resilience_service(config).await?;
        
        info!("✅ All services initialized successfully");
        
        Ok(Self {
            database,
            redis,
            auth,
            resilience,
        })
    }
    
    /// Setup database connection pool
    async fn setup_database(config: &ApiConfig) -> ApiResult<Pool<Postgres>> {
        info!("🗄️  Connecting to database");
        
        let options = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .min_connections(config.database.min_connections)
            .acquire_timeout(config.database.connect_timeout)
            .idle_timeout(config.database.idle_timeout)
            .max_lifetime(config.database.max_lifetime);
        
        let pool = options
            .connect(&config.database.url)
            .await
            .map_err(|e| {
                error!("Failed to connect to database: {:?}", e);
                ApiError::DatabaseConnectionError
            })?;
        
        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| {
                error!("Failed to run database migrations: {:?}", e);
                ApiError::DatabaseError("Migration failed".to_string())
            })?;
        
        info!("✅ Database connected and migrated");
        Ok(pool)
    }
    
    /// Setup Redis connection
    async fn setup_redis(config: &ApiConfig) -> ApiResult<ConnectionManager> {
        info!("🔴 Connecting to Redis");
        
        let client = redis::Client::open(config.redis.url.clone())
            .map_err(|e| {
                error!("Failed to create Redis client: {:?}", e);
                ApiError::ServiceUnavailable("Redis connection failed".to_string())
            })?;
        
        let connection_manager = ConnectionManager::new(client)
            .await
            .map_err(|e| {
                error!("Failed to connect to Redis: {:?}", e);
                ApiError::ServiceUnavailable("Redis connection failed".to_string())
            })?;
        
        info!("✅ Redis connected");
        Ok(connection_manager)
    }
    
    /// Setup JWT authentication service
    async fn setup_auth_service(
        config: &ApiConfig,
        redis: ConnectionManager,
    ) -> ApiResult<Arc<JwtAuthService>> {
        info!("🔐 Setting up authentication service");
        
        let auth_config = crate::auth::AuthConfig {
            jwt_secret: config.auth.jwt_secret.clone(),
            access_token_lifetime_seconds: config.auth.access_token_lifetime.as_secs(),
            refresh_token_lifetime_seconds: config.auth.refresh_token_lifetime.as_secs(),
            enable_token_rotation: config.auth.enable_token_rotation,
        };
        
        let jwt_service = JwtAuthService::new(auth_config, redis)?;
        
        info!("✅ Authentication service ready");
        Ok(Arc::new(jwt_service))
    }
    
    /// Setup resilience service
    async fn setup_resilience_service(config: &ApiConfig) -> ApiResult<Arc<ResilienceService>> {
        info!("🛡️  Setting up resilience service");
        
        let resilience_config = crate::resilience::ResilienceConfig {
            database_failure_threshold: config.resilience.database_circuit_breaker.failure_threshold,
            database_timeout_ms: config.resilience.database_circuit_breaker.timeout_duration.as_millis() as u64,
            database_reset_timeout_secs: config.resilience.database_circuit_breaker.reset_timeout.as_secs(),
            external_service_failure_threshold: config.resilience.external_service_circuit_breaker.failure_threshold,
            external_service_timeout_ms: config.resilience.external_service_circuit_breaker.timeout_duration.as_millis() as u64,
            external_service_reset_timeout_secs: config.resilience.external_service_circuit_breaker.reset_timeout.as_secs(),
            max_retries: config.resilience.retry_max_attempts,
            base_delay_ms: config.resilience.retry_base_delay.as_millis() as u64,
            max_delay_secs: config.resilience.retry_max_delay.as_secs(),
        };
        
        let resilience_service = ResilienceService::new(resilience_config).await?;
        
        info!("✅ Resilience service ready");
        Ok(Arc::new(resilience_service))
    }
}