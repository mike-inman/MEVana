use crate::model::TokenRecord;
use anyhow::Result;
use parking_lot::RwLock;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{collections::HashMap, sync::Arc};

pub struct HotStore {
    inner: RwLock<HashMap<String, TokenRecord>>,
    #[allow(dead_code)]
    pub registry_hash: RwLock<String>,
}
impl HotStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            registry_hash: RwLock::new(String::new()),
        }
    }
    pub fn get(&self, mint: &str) -> Option<TokenRecord> {
        self.inner.read().get(mint).cloned()
    }
    pub fn upsert(&self, rec: TokenRecord) {
        self.inner.write().insert(rec.mint.clone(), rec);
    }
    pub fn all(&self) -> Vec<TokenRecord> {
        self.inner.read().values().cloned().collect()
    }
    pub fn count(&self) -> usize {
        self.inner.read().len()
    }
}

pub struct ColdStore {
    pub pool: Pool<Postgres>,
    tokens_table: String,
    versions_table: String,
}
impl ColdStore {
    pub async fn connect(url: &str, schema: &str) -> Result<Self> {
        let safe_schema = sanitize_identifier(schema);
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

        // Decide table naming strategy: if public, create in public schema but reference unqualified for ease of access.
        let (tokens_table, versions_table) = if safe_schema == "public" {
            ("tokens".to_string(), "token_versions".to_string())
        } else {
            // Create schema (no error if exists)
            let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS {safe_schema}");
            let _ = sqlx::query(&create_schema_sql).execute(&pool).await;
            (
                format!("{safe_schema}.tokens"),
                format!("{safe_schema}.token_versions"),
            )
        };

        // Create tables (idempotent)
        let create_tokens = format!(
            "CREATE TABLE IF NOT EXISTS {tokens_table} (mint TEXT PRIMARY KEY, json JSONB NOT NULL, token_hash TEXT NOT NULL, last_refreshed BIGINT NOT NULL)"
        );
        let create_versions = format!(
            "CREATE TABLE IF NOT EXISTS {versions_table} (mint TEXT NOT NULL, version_id BIGINT NOT NULL, recorded_at BIGINT NOT NULL, hash TEXT NOT NULL, json JSONB NOT NULL, PRIMARY KEY(mint, version_id))"
        );
        sqlx::query(&create_tokens).execute(&pool).await?;
        sqlx::query(&create_versions).execute(&pool).await?;

        // Create helpful indexes if not present
        // json symbol index (expression) and refreshed time for TTL scans
        let _ = sqlx::query(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_symbol ON {tokens_table} ((json->>'symbol'))",
            tokens_table.replace('.', "_")
        ))
        .execute(&pool)
        .await;
        let _ = sqlx::query(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_last_refreshed ON {tokens_table} (last_refreshed)",
            tokens_table.replace('.', "_")
        ))
        .execute(&pool)
        .await;

        // Add generated (virtual) columns for visibility if they don't exist yet.
        if safe_schema == "public" {
            // only attempt once on public
            let add_cols = [
                format!("ALTER TABLE {tokens_table} ADD COLUMN IF NOT EXISTS symbol TEXT GENERATED ALWAYS AS ((json->>'symbol')) STORED"),
                format!("ALTER TABLE {tokens_table} ADD COLUMN IF NOT EXISTS decimals INT GENERATED ALWAYS AS ((json->>'decimals')::int) STORED"),
                format!("ALTER TABLE {tokens_table} ADD COLUMN IF NOT EXISTS logo_url TEXT GENERATED ALWAYS AS ((json->>'logo_url')) STORED"),
            ];
            for sql in add_cols {
                let _ = sqlx::query(&sql).execute(&pool).await;
            }
        }

        // One-time migration: if we are using public schema now, attempt to migrate legacy token_registry.* tables.
        if safe_schema == "public" {
            if let Err(e) = migrate_from_legacy(&pool, &tokens_table, &versions_table).await {
                log::warn!("legacy token_registry schema migration skipped: {}", e);
            }
        }

        Ok(Self {
            pool,
            tokens_table,
            versions_table,
        })
    }
    pub async fn write_current(&self, rec: &TokenRecord) -> Result<()> {
        let json = serde_json::to_value(rec)?;
        let sql = format!("INSERT INTO {} (mint,json,token_hash,last_refreshed) VALUES ($1,$2,$3,$4) ON CONFLICT(mint) DO UPDATE SET json=EXCLUDED.json, token_hash=EXCLUDED.token_hash, last_refreshed=EXCLUDED.last_refreshed", self.tokens_table);
        sqlx::query(&sql)
            .bind(&rec.mint)
            .bind(&json)
            .bind(&rec.token_hash)
            .bind(rec.last_refreshed.timestamp())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn append_version(&self, rec: &TokenRecord) -> Result<()> {
        let json = serde_json::to_value(rec)?;
        let sel = format!(
            "SELECT COALESCE(MAX(version_id)+1,1) FROM {} WHERE mint = $1",
            self.versions_table
        );
        let next: i64 = sqlx::query_scalar(&sel)
            .bind(&rec.mint)
            .fetch_one(&self.pool)
            .await?;
        let ins = format!(
            "INSERT INTO {} (mint,version_id,recorded_at,hash,json) VALUES ($1,$2,$3,$4,$5)",
            self.versions_table
        );
        sqlx::query(&ins)
            .bind(&rec.mint)
            .bind(next)
            .bind(rec.last_refreshed.timestamp())
            .bind(&rec.token_hash)
            .bind(&json)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// Migrate legacy data from token_registry schema into public if legacy tables exist and public tables are empty.
async fn migrate_from_legacy(
    pool: &Pool<Postgres>,
    public_tokens: &str,
    public_versions: &str,
) -> Result<()> {
    // Only proceed if legacy schema tables exist.
    let legacy_tokens = "token_registry.tokens";
    let legacy_versions = "token_registry.token_versions";
    let legacy_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='token_registry' AND table_name='tokens')"
    )
    .fetch_one(pool)
    .await?;
    if !legacy_exists.0 {
        return Ok(());
    }

    // If public tokens already has rows, skip.
    let (public_count,): (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {public_tokens}"))
        .fetch_one(pool)
        .await?;
    if public_count > 0 {
        return Ok(());
    }

    // Copy structure already created; copy rows.
    let copy_tokens_sql = format!("INSERT INTO {public_tokens} (mint,json,token_hash,last_refreshed) SELECT mint,json,token_hash,last_refreshed FROM {legacy_tokens} ON CONFLICT (mint) DO NOTHING");
    let copy_versions_sql = format!("INSERT INTO {public_versions} (mint,version_id,recorded_at,hash,json) SELECT mint,version_id,recorded_at,hash,json FROM {legacy_versions} ON CONFLICT DO NOTHING");
    let _ = sqlx::query(&copy_tokens_sql).execute(pool).await?;
    let _ = sqlx::query(&copy_versions_sql).execute(pool).await?;
    log::info!("migrated legacy token_registry.* tables into public");
    Ok(())
}

fn sanitize_identifier(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        }
    }
    if out.is_empty() {
        out.push_str("public");
    }
    out
}

#[derive(Clone)]
pub struct Stores {
    pub hot: Arc<HotStore>,
    pub cold: Arc<ColdStore>,
}
