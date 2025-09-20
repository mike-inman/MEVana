use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

use super::{DexProgram, DiscoveryQueue, IdlCache, RegistryStats, Token};

// DEX Program queries
pub async fn insert_dex_program(pool: &PgPool, program: &DexProgram) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO dex_programs 
        (program_id, dex_name, protocol_type, fee_structure, liquidity_model, idl_version, quirks, 
         is_active, discovered_at, last_seen, transaction_volume_24h, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (program_id) DO UPDATE SET
        dex_name = EXCLUDED.dex_name,
        protocol_type = EXCLUDED.protocol_type,
        fee_structure = EXCLUDED.fee_structure,
        liquidity_model = EXCLUDED.liquidity_model,
        idl_version = EXCLUDED.idl_version,
        quirks = EXCLUDED.quirks,
        is_active = EXCLUDED.is_active,
        last_seen = EXCLUDED.last_seen,
        transaction_volume_24h = EXCLUDED.transaction_volume_24h,
        metadata = EXCLUDED.metadata
        "#,
    )
    .bind(&program.program_id)
    .bind(&program.dex_name)
    .bind(&program.protocol_type)
    .bind(&program.fee_structure)
    .bind(&program.liquidity_model)
    .bind(&program.idl_version)
    .bind(&program.quirks)
    .bind(program.is_active)
    .bind(program.discovered_at)
    .bind(program.last_seen)
    .bind(program.transaction_volume_24h)
    .bind(&program.metadata)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_dex_program(pool: &PgPool, program_id: &str) -> Result<Option<DexProgram>> {
    let program =
        sqlx::query_as::<_, DexProgram>("SELECT * FROM dex_programs WHERE program_id = $1")
            .bind(program_id)
            .fetch_optional(pool)
            .await?;

    Ok(program)
}

pub async fn list_dex_programs(pool: &PgPool, active_only: bool) -> Result<Vec<DexProgram>> {
    let query = if active_only {
        "SELECT * FROM dex_programs WHERE is_active = true ORDER BY dex_name"
    } else {
        "SELECT * FROM dex_programs ORDER BY dex_name"
    };

    let programs = sqlx::query_as::<_, DexProgram>(query)
        .fetch_all(pool)
        .await?;

    Ok(programs)
}

pub async fn update_dex_program_activity(pool: &PgPool, program_id: &str) -> Result<()> {
    sqlx::query("UPDATE dex_programs SET last_seen = $1 WHERE program_id = $2")
        .bind(Utc::now())
        .bind(program_id)
        .execute(pool)
        .await?;

    Ok(())
}

// Token queries
pub async fn insert_token(pool: &PgPool, token: &Token) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO tokens 
        (mint_address, symbol, name, decimals, is_stable_coin, is_wrapped, underlying_asset,
         coingecko_id, price_feeds, market_cap, daily_volume, discovered_at, verified, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (mint_address) DO UPDATE SET
        symbol = EXCLUDED.symbol,
        name = EXCLUDED.name,
        decimals = EXCLUDED.decimals,
        is_stable_coin = EXCLUDED.is_stable_coin,
        is_wrapped = EXCLUDED.is_wrapped,
        underlying_asset = EXCLUDED.underlying_asset,
        coingecko_id = EXCLUDED.coingecko_id,
        price_feeds = EXCLUDED.price_feeds,
        market_cap = EXCLUDED.market_cap,
        daily_volume = EXCLUDED.daily_volume,
        verified = EXCLUDED.verified,
        metadata = EXCLUDED.metadata
        "#,
    )
    .bind(&token.mint_address)
    .bind(&token.symbol)
    .bind(&token.name)
    .bind(token.decimals)
    .bind(token.is_stable_coin)
    .bind(token.is_wrapped)
    .bind(&token.underlying_asset)
    .bind(&token.coingecko_id)
    .bind(&token.price_feeds)
    .bind(token.market_cap)
    .bind(token.daily_volume)
    .bind(token.discovered_at)
    .bind(token.verified)
    .bind(&token.metadata)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_token(pool: &PgPool, mint_address: &str) -> Result<Option<Token>> {
    let token = sqlx::query_as::<_, Token>("SELECT * FROM tokens WHERE mint_address = $1")
        .bind(mint_address)
        .fetch_optional(pool)
        .await?;

    Ok(token)
}

pub async fn list_tokens(pool: &PgPool, verified_only: bool) -> Result<Vec<Token>> {
    let query = if verified_only {
        "SELECT * FROM tokens WHERE verified = true ORDER BY symbol"
    } else {
        "SELECT * FROM tokens ORDER BY symbol"
    };

    let tokens = sqlx::query_as::<_, Token>(query).fetch_all(pool).await?;

    Ok(tokens)
}

pub async fn update_token_metadata(
    pool: &PgPool,
    mint_address: &str,
    metadata: &str,
) -> Result<()> {
    sqlx::query("UPDATE tokens SET metadata = $1 WHERE mint_address = $1")
        .bind(metadata)
        .bind(mint_address)
        .execute(pool)
        .await?;

    Ok(())
}

// IDL cache queries
pub async fn insert_idl_cache(pool: &PgPool, idl_entry: &IdlCache) -> Result<()> {
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO idl_cache 
        (program_id, idl_hash, idl_content, version, fetched_at, source, is_valid)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&idl_entry.program_id)
    .bind(&idl_entry.idl_hash)
    .bind(&idl_entry.idl_content)
    .bind(&idl_entry.version)
    .bind(idl_entry.fetched_at)
    .bind(&idl_entry.source)
    .bind(idl_entry.is_valid)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_idl_cache(pool: &PgPool, program_id: &str) -> Result<Option<IdlCache>> {
    let idl_entry = sqlx::query_as::<_, IdlCache>(
        "SELECT * FROM idl_cache WHERE program_id = $1 AND is_valid = true",
    )
    .bind(program_id)
    .fetch_optional(pool)
    .await?;

    Ok(idl_entry)
}

pub async fn list_cached_idls(pool: &PgPool) -> Result<Vec<IdlCache>> {
    let idl_entries = sqlx::query_as::<_, IdlCache>(
        "SELECT * FROM idl_cache WHERE is_valid = true ORDER BY fetched_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(idl_entries)
}

// Discovery queue queries
pub async fn enqueue_discovery(pool: &PgPool, discovery: &DiscoveryQueue) -> Result<()> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO discovery_queue 
        (id, program_id, discovery_type, priority, attempts, last_attempt, created_at, metadata, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&discovery.id)
    .bind(&discovery.program_id)
    .bind(&discovery.discovery_type)
    .bind(discovery.priority)
    .bind(discovery.attempts)
    .bind(discovery.last_attempt)
    .bind(discovery.created_at)
    .bind(&discovery.metadata)
    .bind(&discovery.status)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn dequeue_discovery(pool: &PgPool, batch_size: usize) -> Result<Vec<DiscoveryQueue>> {
    // First, mark items as processing
    sqlx::query(
        r#"
        UPDATE discovery_queue 
        SET status = 'processing', last_attempt = $1
        WHERE id IN (
            SELECT id FROM discovery_queue 
            WHERE status = 'pending' 
            ORDER BY priority DESC, created_at ASC 
            LIMIT ?
        )
        "#,
    )
    .bind(Utc::now())
    .bind(batch_size as i64)
    .execute(pool)
    .await?;

    // Then fetch the processing items
    let discoveries = sqlx::query_as::<_, DiscoveryQueue>(
        r#"
        SELECT * FROM discovery_queue 
        WHERE status = 'processing' 
        ORDER BY priority DESC, created_at ASC 
        LIMIT ?
        "#,
    )
    .bind(batch_size as i64)
    .fetch_all(pool)
    .await?;

    Ok(discoveries)
}

pub async fn complete_discovery(pool: &PgPool, id: &str, status: &str) -> Result<()> {
    sqlx::query("UPDATE discovery_queue SET status = $1, attempts = attempts + 1 WHERE id = $1")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

// Analytics queries
pub async fn get_registry_stats(pool: &PgPool) -> Result<RegistryStats> {
    let total_dex_programs: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dex_programs")
        .fetch_one(pool)
        .await?;

    let active_dex_programs: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM dex_programs WHERE is_active = true")
            .fetch_one(pool)
            .await?;

    let total_tokens: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tokens")
        .fetch_one(pool)
        .await?;

    let verified_tokens: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM tokens WHERE verified = true")
            .fetch_one(pool)
            .await?;

    let cached_idls: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM idl_cache WHERE is_valid = true")
            .fetch_one(pool)
            .await?;

    let pending_discoveries: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM discovery_queue WHERE status = 'pending'")
            .fetch_one(pool)
            .await?;

    Ok(RegistryStats {
        total_dex_programs: total_dex_programs.0,
        active_dex_programs: active_dex_programs.0,
        total_tokens: total_tokens.0,
        verified_tokens: verified_tokens.0,
        cached_idls: cached_idls.0,
        pending_discoveries: pending_discoveries.0,
        last_updated: Utc::now(),
    })
}
