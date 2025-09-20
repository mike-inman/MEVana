use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// Centralized database configuration for the entire MEVana system
/// This is the SINGLE SOURCE OF TRUTH for all database connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

impl DatabaseConfig {
    /// Load database configuration from environment variables
    /// This is the ONLY way database configuration should be loaded
    pub fn load() -> Result<Self> {
        let database_url =
            env::var("DATABASE_URL").context("DATABASE_URL environment variable must be set")?;

        // Parse DATABASE_URL to extract components
        Self::from_url(&database_url)
    }

    /// Parse a PostgreSQL connection URL into DatabaseConfig
    pub fn from_url(url: &str) -> Result<Self> {
        // Expected format: postgresql://username:password@host:port/database
        if !url.starts_with("postgresql://") {
            return Err(anyhow::anyhow!(
                "DATABASE_URL must start with 'postgresql://'"
            ));
        }

        let url = url.strip_prefix("postgresql://").unwrap();

        // Split on '@' to separate credentials from host/port/db
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid DATABASE_URL format: missing '@' separator"
            ));
        }

        // Parse credentials (username:password)
        let credentials = parts[0];
        let cred_parts: Vec<&str> = credentials.splitn(2, ':').collect();
        if cred_parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid DATABASE_URL format: missing ':' in credentials"
            ));
        }
        let username = cred_parts[0].to_string();
        let password = cred_parts[1].to_string();

        // Parse host:port/database
        let host_port_db = parts[1];
        let host_db_parts: Vec<&str> = host_port_db.splitn(2, '/').collect();
        if host_db_parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid DATABASE_URL format: missing '/' separator"
            ));
        }

        let database = host_db_parts[1].to_string();

        // Parse host:port
        let host_port = host_db_parts[0];
        let host_port_parts: Vec<&str> = host_port.splitn(2, ':').collect();
        if host_port_parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid DATABASE_URL format: missing ':' in host:port"
            ));
        }

        let host = host_port_parts[0].to_string();
        let port = host_port_parts[1]
            .parse::<u16>()
            .context("Invalid port number in DATABASE_URL")?;

        Ok(Self {
            host,
            port,
            database,
            username,
            password,
            max_connections: Self::get_env_u32("DB_MAX_CONNECTIONS", 20),
            min_connections: Self::get_env_u32("DB_MIN_CONNECTIONS", 1),
            connection_timeout_seconds: Self::get_env_u64("DB_CONNECTION_TIMEOUT", 30),
            idle_timeout_seconds: Self::get_env_u64("DB_IDLE_TIMEOUT", 600),
        })
    }

    /// Load database configuration - NO hardcoded fallbacks allowed
    /// All configuration MUST come from environment variables only
    /// Convert back to a PostgreSQL connection URL
    pub fn to_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }

    /// Get a connection URL with masked password for logging
    pub fn to_safe_url(&self) -> String {
        format!(
            "postgresql://{}:***@{}:{}/{}",
            self.username, self.host, self.port, self.database
        )
    }

    /// Validate that we're using the correct MEVana database
    /// This prevents database confusion by ensuring we use 'mevdb' only
    pub fn validate_mevana_standards(&self) -> Result<()> {
        // Enforce that we only use the mevdb database (prevents the mevana/mevdb confusion)
        if self.database != "mevdb" {
            return Err(anyhow::anyhow!(
                "CRITICAL: Only 'mevdb' database is allowed. Found: '{}'. \
                This prevents database confusion and ensures single source of truth.",
                self.database
            ));
        }

        Ok(())
    }

    fn get_env_u32(key: &str, default: u32) -> u32 {
        env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    fn get_env_u64(key: &str, default: u64) -> u64 {
        env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
}

lazy_static::lazy_static! {
    pub static ref DATABASE_CONFIG: DatabaseConfig = DatabaseConfig::load()
        .expect("Failed to load database configuration - ensure DATABASE_URL is set");
}

/// Get the database URL from environment variables ONLY
/// This is the ONLY function that should be used to get database URLs
/// NO hardcoded fallbacks - environment must be properly configured
pub fn get_database_url() -> String {
    env::var("DATABASE_URL").expect(
        "DATABASE_URL environment variable is required. \
        No hardcoded fallbacks allowed - ensure environment is configured.",
    )
}

/// Get the safe database URL for logging (password masked)
pub fn get_safe_database_url() -> String {
    DATABASE_CONFIG.to_safe_url()
}

/// Validate that the current database configuration meets MEVana standards
pub fn validate_database_config() -> Result<()> {
    DATABASE_CONFIG.validate_mevana_standards()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing() {
        let config = DatabaseConfig::from_url("postgresql://mev:mevpassword@localhost:5433/mevdb")
            .expect("Should parse valid URL");

        assert_eq!(config.username, "mev");
        assert_eq!(config.password, "mevpassword");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5433);
        assert_eq!(config.database, "mevdb");
    }

    #[test]
    fn test_validation() {
        let config = DatabaseConfig::from_url("postgresql://mev:mevpassword@localhost:5433/mevdb")
            .expect("Should parse valid URL");

        // Should pass validation
        config
            .validate_mevana_standards()
            .expect("Should pass validation");
    }

    #[test]
    fn test_invalid_database_rejected() {
        let config =
            DatabaseConfig::from_url("postgresql://mev:mevpassword@localhost:5433/wrongdb")
                .expect("Should parse URL");

        // Should fail validation
        assert!(config.validate_mevana_standards().is_err());
    }

    #[test]
    fn test_safe_url() {
        let config = DatabaseConfig::from_url("postgresql://mev:mevpassword@localhost:5433/mevdb")
            .expect("Should parse valid URL");

        let safe_url = config.to_safe_url();
        assert!(safe_url.contains("***"));
        assert!(!safe_url.contains("mevpassword"));
    }
}
