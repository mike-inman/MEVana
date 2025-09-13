//! Configuration management
//!
//! Provides comprehensive configuration with:
//! - Environment-based configuration
//! - Security defaults for production
//! - Database and Redis connection settings
//! - JWT and authentication configuration

use crate::error::{ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use std::{env, time::Duration};

/// Main API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
    pub security: SecurityConfig,
    pub observability: ObservabilityConfig,
    pub resilience: ResilienceConfig,
}

impl ApiConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> ApiResult<Self> {
        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()
            .map_err(|e| ApiError::ConfigurationError(e.to_string()))?;
        
        config
            .try_deserialize()
            .map_err(|e| ApiError::ConfigurationError(e.to_string()))
    }
    
    /// Create development configuration
    pub fn development() -> Self {
        Self {
            server: ServerConfig {
                bind_address: "127.0.0.1:3000".to_string(),
                workers: num_cpus::get(),
                keep_alive_timeout: Duration::from_secs(75),
                request_timeout: Duration::from_secs(30),
                max_connections: 1000,
                enable_https: false,
                tls_cert_path: None,
                tls_key_path: None,
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://postgres:password@localhost/mevana_dev".to_string()),
                max_connections: 10,
                min_connections: 1,
                connect_timeout: Duration::from_secs(10),
                idle_timeout: Some(Duration::from_secs(600)),
                max_lifetime: Some(Duration::from_secs(1800)),
                enable_logging: true,
            },
            redis: RedisConfig {
                url: env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_string()),
                pool_size: 10,
                connect_timeout: Duration::from_secs(5),
                command_timeout: Duration::from_secs(5),
                enable_cluster: false,
            },
            auth: AuthConfig {
                jwt_secret: env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "dev-secret-key-change-in-production".to_string()),
                access_token_lifetime: Duration::from_secs(900), // 15 minutes
                refresh_token_lifetime: Duration::from_secs(86400 * 7), // 7 days
                enable_token_rotation: false,
                password_min_length: 8,
                password_require_special_chars: false,
                max_failed_login_attempts: 10,
                lockout_duration: Duration::from_secs(300),
            },
            security: SecurityConfig {
                cors_allowed_origins: vec!["http://localhost:3000".to_string(), "http://localhost:3001".to_string()],
                cors_allow_credentials: true,
                enable_csrf_protection: false, // Disabled for development
                csrf_token_lifetime: Duration::from_secs(3600),
                enable_rate_limiting: true,
                rate_limit_requests_per_minute: 1000,
                enable_request_logging: true,
                max_request_body_size: 10 * 1024 * 1024, // 10MB
            },
            observability: ObservabilityConfig {
                enable_tracing: true,
                tracing_endpoint: None,
                enable_metrics: true,
                metrics_endpoint: "127.0.0.1:9090".to_string(),
                log_level: "debug".to_string(),
                enable_structured_logging: true,
            },
            resilience: ResilienceConfig {
                database_circuit_breaker: CircuitBreakerConfig {
                    failure_threshold: 5,
                    timeout_duration: Duration::from_secs(5),
                    reset_timeout: Duration::from_secs(30),
                },
                external_service_circuit_breaker: CircuitBreakerConfig {
                    failure_threshold: 10,
                    timeout_duration: Duration::from_secs(10),
                    reset_timeout: Duration::from_secs(60),
                },
                retry_max_attempts: 3,
                retry_base_delay: Duration::from_millis(100),
                retry_max_delay: Duration::from_secs(5),
            },
        }
    }
    
    /// Create production configuration
    pub fn production() -> ApiResult<Self> {
        // Production requires all sensitive values from environment
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| ApiError::ConfigurationError("JWT_SECRET environment variable required".to_string()))?;
        
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ApiError::ConfigurationError("DATABASE_URL environment variable required".to_string()))?;
        
        Ok(Self {
            server: ServerConfig {
                bind_address: env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
                workers: env::var("WORKERS")
                    .ok()
                    .and_then(|w| w.parse().ok())
                    .unwrap_or_else(|| num_cpus::get()),
                keep_alive_timeout: Duration::from_secs(75),
                request_timeout: Duration::from_secs(30),
                max_connections: env::var("MAX_CONNECTIONS")
                    .ok()
                    .and_then(|c| c.parse().ok())
                    .unwrap_or(10000),
                enable_https: env::var("ENABLE_HTTPS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(false),
                tls_cert_path: env::var("TLS_CERT_PATH").ok(),
                tls_key_path: env::var("TLS_KEY_PATH").ok(),
            },
            database: DatabaseConfig {
                url: database_url,
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|c| c.parse().ok())
                    .unwrap_or(50),
                min_connections: env::var("DB_MIN_CONNECTIONS")
                    .ok()
                    .and_then(|c| c.parse().ok())
                    .unwrap_or(5),
                connect_timeout: Duration::from_secs(10),
                idle_timeout: Some(Duration::from_secs(600)),
                max_lifetime: Some(Duration::from_secs(1800)),
                enable_logging: false,
            },
            redis: RedisConfig {
                url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_string()),
                pool_size: env::var("REDIS_POOL_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(20),
                connect_timeout: Duration::from_secs(5),
                command_timeout: Duration::from_secs(5),
                enable_cluster: env::var("REDIS_CLUSTER_MODE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(false),
            },
            auth: AuthConfig {
                jwt_secret,
                access_token_lifetime: Duration::from_secs(900), // 15 minutes
                refresh_token_lifetime: Duration::from_secs(86400 * 7), // 7 days
                enable_token_rotation: true,
                password_min_length: 12,
                password_require_special_chars: true,
                max_failed_login_attempts: 5,
                lockout_duration: Duration::from_secs(900),
            },
            security: SecurityConfig {
                cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "https://app.mevana.com".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                cors_allow_credentials: true,
                enable_csrf_protection: true,
                csrf_token_lifetime: Duration::from_secs(3600),
                enable_rate_limiting: true,
                rate_limit_requests_per_minute: env::var("RATE_LIMIT_RPM")
                    .ok()
                    .and_then(|r| r.parse().ok())
                    .unwrap_or(100),
                enable_request_logging: true,
                max_request_body_size: 5 * 1024 * 1024, // 5MB
            },
            observability: ObservabilityConfig {
                enable_tracing: true,
                tracing_endpoint: env::var("TRACING_ENDPOINT").ok(),
                enable_metrics: true,
                metrics_endpoint: env::var("METRICS_ENDPOINT").unwrap_or_else(|_| "0.0.0.0:9090".to_string()),
                log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                enable_structured_logging: true,
            },
            resilience: ResilienceConfig {
                database_circuit_breaker: CircuitBreakerConfig {
                    failure_threshold: 5,
                    timeout_duration: Duration::from_secs(5),
                    reset_timeout: Duration::from_secs(30),
                },
                external_service_circuit_breaker: CircuitBreakerConfig {
                    failure_threshold: 10,
                    timeout_duration: Duration::from_secs(10),
                    reset_timeout: Duration::from_secs(60),
                },
                retry_max_attempts: 3,
                retry_base_delay: Duration::from_millis(100),
                retry_max_delay: Duration::from_secs(5),
            },
        })
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub workers: usize,
    pub keep_alive_timeout: Duration,
    pub request_timeout: Duration,
    pub max_connections: usize,
    pub enable_https: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
    pub enable_logging: bool,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connect_timeout: Duration,
    pub command_timeout: Duration,
    pub enable_cluster: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_lifetime: Duration,
    pub refresh_token_lifetime: Duration,
    pub enable_token_rotation: bool,
    pub password_min_length: usize,
    pub password_require_special_chars: bool,
    pub max_failed_login_attempts: u32,
    pub lockout_duration: Duration,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub cors_allowed_origins: Vec<String>,
    pub cors_allow_credentials: bool,
    pub enable_csrf_protection: bool,
    pub csrf_token_lifetime: Duration,
    pub enable_rate_limiting: bool,
    pub rate_limit_requests_per_minute: u32,
    pub enable_request_logging: bool,
    pub max_request_body_size: usize,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub enable_tracing: bool,
    pub tracing_endpoint: Option<String>,
    pub enable_metrics: bool,
    pub metrics_endpoint: String,
    pub log_level: String,
    pub enable_structured_logging: bool,
}

/// Resilience configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceConfig {
    pub database_circuit_breaker: CircuitBreakerConfig,
    pub external_service_circuit_breaker: CircuitBreakerConfig,
    pub retry_max_attempts: u32,
    pub retry_base_delay: Duration,
    pub retry_max_delay: Duration,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub timeout_duration: Duration,
    pub reset_timeout: Duration,
}