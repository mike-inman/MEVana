use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::Arc;
use uuid::Uuid;

pub mod migrations;
pub mod queries;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DexProgram {
    pub program_id: String,
    pub dex_name: String,
    pub protocol_type: String,
    pub fee_structure: Option<String>, // JSON
    pub liquidity_model: String,
    pub idl_version: Option<String>,
    pub quirks: Option<String>, // JSON
    pub is_active: bool,
    pub discovered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub transaction_volume_24h: Option<f64>,
    pub metadata: Option<String>, // JSON
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Token {
    pub mint_address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<i32>,
    pub is_stable_coin: bool,
    pub is_wrapped: bool,
    pub underlying_asset: Option<String>,
    pub coingecko_id: Option<String>,
    pub price_feeds: Option<String>, // JSON
    pub market_cap: Option<f64>,
    pub daily_volume: Option<f64>,
    pub discovered_at: DateTime<Utc>,
    pub verified: bool,
    pub metadata: Option<String>, // JSON
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IdlCache {
    pub program_id: String,
    pub idl_hash: String,
    pub idl_content: String, // JSON
    pub version: String,
    pub fetched_at: DateTime<Utc>,
    pub source: String, // 'manual', 'anchor_cli', 'solana_cli', 'github'
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DiscoveryQueue {
    pub id: String, // UUID
    pub program_id: String,
    pub discovery_type: String, // 'dex_program', 'token_mint'
    pub priority: i32,
    pub attempts: i32,
    pub last_attempt: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<String>, // JSON
    pub status: String,           // 'pending', 'processing', 'completed', 'failed'
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self> {
        // Create database directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(database_url).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;

        let db = Self { pool };

        // Run migrations
        db.migrate().await?;

        Ok(db)
    }

    pub async fn migrate(&self) -> Result<()> {
        migrations::run_migrations(&self.pool).await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // DEX Program operations
    pub async fn insert_dex_program(&self, program: &DexProgram) -> Result<()> {
        queries::insert_dex_program(&self.pool, program).await
    }

    pub async fn get_dex_program(&self, program_id: &str) -> Result<Option<DexProgram>> {
        queries::get_dex_program(&self.pool, program_id).await
    }

    pub async fn list_dex_programs(&self, active_only: bool) -> Result<Vec<DexProgram>> {
        queries::list_dex_programs(&self.pool, active_only).await
    }

    pub async fn update_dex_program_activity(&self, program_id: &str) -> Result<()> {
        queries::update_dex_program_activity(&self.pool, program_id).await
    }

    // Token operations
    pub async fn insert_token(&self, token: &Token) -> Result<()> {
        queries::insert_token(&self.pool, token).await
    }

    pub async fn get_token(&self, mint_address: &str) -> Result<Option<Token>> {
        queries::get_token(&self.pool, mint_address).await
    }

    pub async fn list_tokens(&self, verified_only: bool) -> Result<Vec<Token>> {
        queries::list_tokens(&self.pool, verified_only).await
    }

    pub async fn update_token_metadata(&self, mint_address: &str, metadata: &str) -> Result<()> {
        queries::update_token_metadata(&self.pool, mint_address, metadata).await
    }

    // IDL cache operations
    pub async fn insert_idl_cache(&self, idl_entry: &IdlCache) -> Result<()> {
        queries::insert_idl_cache(&self.pool, idl_entry).await
    }

    pub async fn get_idl_cache(&self, program_id: &str) -> Result<Option<IdlCache>> {
        queries::get_idl_cache(&self.pool, program_id).await
    }

    pub async fn list_cached_idls(&self) -> Result<Vec<IdlCache>> {
        queries::list_cached_idls(&self.pool).await
    }

    // Discovery queue operations
    pub async fn enqueue_discovery(&self, discovery: &DiscoveryQueue) -> Result<()> {
        queries::enqueue_discovery(&self.pool, discovery).await
    }

    pub async fn dequeue_discovery(&self, batch_size: usize) -> Result<Vec<DiscoveryQueue>> {
        queries::dequeue_discovery(&self.pool, batch_size).await
    }

    pub async fn complete_discovery(&self, id: &str, status: &str) -> Result<()> {
        queries::complete_discovery(&self.pool, id, status).await
    }

    // Analytics and metrics
    pub async fn get_registry_stats(&self) -> Result<RegistryStats> {
        queries::get_registry_stats(&self.pool).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    pub total_dex_programs: i64,
    pub active_dex_programs: i64,
    pub total_tokens: i64,
    pub verified_tokens: i64,
    pub cached_idls: i64,
    pub pending_discoveries: i64,
    pub last_updated: DateTime<Utc>,
}
