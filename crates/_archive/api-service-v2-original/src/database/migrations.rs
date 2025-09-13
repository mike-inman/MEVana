//! Database migrations
//!
//! Handles database schema creation and evolution with:
//! - Version-controlled migrations
//! - Rollback support
//! - Schema validation
//! - Data integrity checks

use crate::error::{ApiError, ApiResult};
use sqlx::{migrate::Migrator, PgPool};
use std::path::Path;
use tracing::{info, error, instrument};

/// Database migrator instance
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Run all pending database migrations
#[instrument(skip(pool))]
pub async fn run_migrations(pool: &PgPool) -> ApiResult<()> {
    info!("Running database migrations");
    
    match MIGRATOR.run(pool).await {
        Ok(_) => {
            info!("Database migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Database migration failed: {:?}", e);
            Err(ApiError::DatabaseError(format!("Migration failed: {}", e)))
        }
    }
}

/// Check migration status
#[instrument(skip(pool))]
pub async fn check_migration_status(pool: &PgPool) -> ApiResult<MigrationStatus> {
    info!("Checking migration status");
    
    // Get applied migrations
    let applied_migrations = sqlx::query!(
        "SELECT version, description, success FROM _sqlx_migrations ORDER BY version"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error!("Failed to check migration status: {:?}", e);
        ApiError::DatabaseError(format!("Failed to check migration status: {}", e))
    })?;
    
    let applied_count = applied_migrations.len();
    let total_count = MIGRATOR.iter().count();
    
    let status = if applied_count == total_count {
        MigrationStatusType::UpToDate
    } else if applied_count < total_count {
        MigrationStatusType::PendingMigrations
    } else {
        MigrationStatusType::Unknown
    };
    
    let migrations = applied_migrations
        .into_iter()
        .map(|row| AppliedMigration {
            version: row.version,
            description: row.description,
            success: row.success,
        })
        .collect();
    
    Ok(MigrationStatus {
        status,
        applied_count,
        total_count,
        migrations,
    })
}

/// Migration status information
#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct MigrationStatus {
    pub status: MigrationStatusType,
    pub applied_count: usize,
    pub total_count: usize,
    pub migrations: Vec<AppliedMigration>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub enum MigrationStatusType {
    UpToDate,
    PendingMigrations,
    Unknown,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AppliedMigration {
    pub version: i64,
    pub description: String,
    pub success: bool,
}

/// Create initial database schema
pub const INITIAL_SCHEMA: &str = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(100) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    is_active BOOLEAN NOT NULL DEFAULT true,
    is_verified BOOLEAN NOT NULL DEFAULT false,
    last_login_at TIMESTAMPTZ,
    roles TEXT[] NOT NULL DEFAULT '{}',
    tier TEXT NOT NULL DEFAULT 'Free',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by UUID,
    version BIGINT NOT NULL DEFAULT 1
);

-- API keys table
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    key_hash VARCHAR(255) NOT NULL,
    key_prefix VARCHAR(8) NOT NULL,
    permissions TEXT[] NOT NULL DEFAULT '{}',
    rate_limit_per_hour INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by UUID,
    version BIGINT NOT NULL DEFAULT 1
);

-- Trading sessions table
CREATE TABLE IF NOT EXISTS trading_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token VARCHAR(255) NOT NULL,
    wallet_id UUID,
    strategy TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Active',
    max_position_size DECIMAL(20, 8),
    stop_loss_percent REAL,
    take_profit_percent REAL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    trades_executed INTEGER NOT NULL DEFAULT 0,
    total_volume DECIMAL(30, 18) NOT NULL DEFAULT 0,
    realized_pnl DECIMAL(30, 18) NOT NULL DEFAULT 0,
    unrealized_pnl DECIMAL(30, 18) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by UUID,
    version BIGINT NOT NULL DEFAULT 1
);

-- Audit logs table
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,
    session_id UUID,
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id UUID,
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    user_agent TEXT,
    request_id UUID,
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- System configuration table
CREATE TABLE IF NOT EXISTS system_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(255) UNIQUE NOT NULL,
    value JSONB NOT NULL,
    description TEXT,
    is_sensitive BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by UUID,
    version BIGINT NOT NULL DEFAULT 1
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active) WHERE is_active = true;

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_active ON api_keys(is_active) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(key_prefix);

CREATE INDEX IF NOT EXISTS idx_trading_sessions_user_id ON trading_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_trading_sessions_status ON trading_sessions(status);
CREATE INDEX IF NOT EXISTS idx_trading_sessions_started_at ON trading_sessions(started_at);

CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource_type, resource_id);

CREATE INDEX IF NOT EXISTS idx_system_config_key ON system_config(key);
"#;

/// Execute raw SQL for development/testing
#[instrument(skip(pool, sql))]
pub async fn execute_sql(pool: &PgPool, sql: &str) -> ApiResult<()> {
    info!("Executing custom SQL");
    
    sqlx::query(sql)
        .execute(pool)
        .await
        .map_err(|e| {
            error!("SQL execution failed: {:?}", e);
            ApiError::DatabaseError(format!("SQL execution failed: {}", e))
        })?;
    
    Ok(())
}

/// Reset database (WARNING: Destructive operation for testing only)
#[cfg(feature = "testing")]
#[instrument(skip(pool))]
pub async fn reset_database(pool: &PgPool) -> ApiResult<()> {
    use tracing::warn;
    
    warn!("RESETTING DATABASE - ALL DATA WILL BE LOST!");
    
    let tables = vec![
        "audit_logs",
        "trading_sessions", 
        "api_keys",
        "users",
        "system_config",
        "_sqlx_migrations",
    ];
    
    for table in tables {
        let sql = format!("DROP TABLE IF EXISTS {} CASCADE", table);
        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| {
                error!("Failed to drop table {}: {:?}", table, e);
                ApiError::DatabaseError(format!("Failed to drop table {}: {}", table, e))
            })?;
    }
    
    // Recreate schema
    execute_sql(pool, INITIAL_SCHEMA).await?;
    
    warn!("Database reset completed");
    Ok(())
}