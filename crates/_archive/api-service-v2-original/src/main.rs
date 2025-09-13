use std::env;
use api_service_v2::{run_service, ApiConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api_service_v2=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting MEVana API Service v2");

    // Load configuration from environment
    let config = ApiConfig::from_env().unwrap_or_else(|_| {
        tracing::warn!("Could not load config from environment, using defaults");
        ApiConfig::default_with_overrides()
    });

    tracing::info!("📊 Configuration loaded:");
    tracing::info!("  - Server: {}", config.server.bind_address);
    tracing::info!("  - Database: {}", config.database.url.replace(":mevpassword@", ":***@"));
    tracing::info!("  - JWT Secret: [REDACTED]");
    tracing::info!("  - Wallet Vault: http://localhost:{}", config.wallet_vault.port);

    // Run the service
    if let Err(e) = run_service(config).await {
        tracing::error!("❌ Failed to run API service: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

impl ApiConfig {
    /// Create configuration with overrides for local development
    fn default_with_overrides() -> Self {
        use api_service_v2::config::*;
        
        ApiConfig {
            server: ServerConfig {
                bind_address: env::var("API_BIND_ADDRESS")
                    .unwrap_or_else(|_| "0.0.0.0:8091".to_string()),
                environment: env::var("ENVIRONMENT")
                    .unwrap_or_else(|_| "development".to_string()),
            },
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://mev:mevpassword@localhost:5433/mevdb".to_string()),
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10),
                connect_timeout: std::time::Duration::from_secs(
                    env::var("DB_CONNECT_TIMEOUT_SECS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(30)
                ),
                idle_timeout: Some(std::time::Duration::from_secs(
                    env::var("DB_IDLE_TIMEOUT_SECS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(600)
                )),
            },
            jwt: JwtConfig {
                secret: env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "mevana_dev_secret_change_in_production".to_string()),
                expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(24),
                issuer: env::var("JWT_ISSUER")
                    .unwrap_or_else(|_| "mevana-api-v2".to_string()),
                audience: env::var("JWT_AUDIENCE")
                    .unwrap_or_else(|_| "mevana-users".to_string()),
            },
            clerk: ClerkConfig {
                publishable_key: env::var("CLERK_PUBLISHABLE_KEY")
                    .unwrap_or_else(|_| "pk_test_placeholder".to_string()),
                secret_key: env::var("CLERK_SECRET_KEY")
                    .unwrap_or_else(|_| "sk_test_placeholder".to_string()),
                jwt_template: env::var("CLERK_JWT_TEMPLATE")
                    .unwrap_or_else(|_| "mevana-template".to_string()),
            },
            wallet_vault: WalletVaultConfig {
                url: env::var("WALLET_VAULT_URL")
                    .unwrap_or_else(|_| "http://localhost:8090".to_string()),
                port: env::var("WALLET_VAULT_PORT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8090),
                timeout: std::time::Duration::from_secs(
                    env::var("WALLET_VAULT_TIMEOUT_SECS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(10)
                ),
            },
            metrics: MetricsConfig {
                enabled: env::var("METRICS_ENABLED")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(true),
                bind_address: env::var("METRICS_BIND_ADDRESS")
                    .unwrap_or_else(|_| "0.0.0.0:9091".to_string()),
            },
            rate_limiting: RateLimitingConfig {
                enabled: env::var("RATE_LIMITING_ENABLED")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(true),
                requests_per_minute: env::var("RATE_LIMIT_RPM")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100),
                burst_size: env::var("RATE_LIMIT_BURST")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10),
            },
        }
    }
}