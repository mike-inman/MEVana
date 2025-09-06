use anyhow::Result;
use log::info;
use sqlx::SqlitePool;

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    info!("Running database migrations...");

    // Create dex_programs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS dex_programs (
            program_id TEXT PRIMARY KEY,
            dex_name TEXT NOT NULL,
            protocol_type TEXT NOT NULL,
            fee_structure TEXT, -- JSON
            liquidity_model TEXT NOT NULL,
            idl_version TEXT,
            quirks TEXT, -- JSON
            is_active BOOLEAN NOT NULL DEFAULT true,
            discovered_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            transaction_volume_24h REAL,
            metadata TEXT -- JSON
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create tokens table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tokens (
            mint_address TEXT PRIMARY KEY,
            symbol TEXT,
            name TEXT,
            decimals INTEGER,
            is_stable_coin BOOLEAN NOT NULL DEFAULT false,
            is_wrapped BOOLEAN NOT NULL DEFAULT false,
            underlying_asset TEXT,
            coingecko_id TEXT,
            price_feeds TEXT, -- JSON
            market_cap REAL,
            daily_volume REAL,
            discovered_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            verified BOOLEAN NOT NULL DEFAULT false,
            metadata TEXT -- JSON
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create idl_cache table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS idl_cache (
            program_id TEXT PRIMARY KEY,
            idl_hash TEXT NOT NULL,
            idl_content TEXT NOT NULL, -- JSON
            version TEXT NOT NULL,
            fetched_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            source TEXT NOT NULL, -- 'manual', 'anchor_cli', 'solana_cli', 'github'
            is_valid BOOLEAN NOT NULL DEFAULT true
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create discovery_queue table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS discovery_queue (
            id TEXT PRIMARY KEY,
            program_id TEXT NOT NULL,
            discovery_type TEXT NOT NULL, -- 'dex_program', 'token_mint'
            priority INTEGER NOT NULL DEFAULT 0,
            attempts INTEGER NOT NULL DEFAULT 0,
            last_attempt TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            metadata TEXT, -- JSON
            status TEXT NOT NULL DEFAULT 'pending' -- 'pending', 'processing', 'completed', 'failed'
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_dex_programs_active ON dex_programs(is_active)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_dex_programs_last_seen ON dex_programs(last_seen)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tokens_verified ON tokens(verified)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tokens_symbol ON tokens(symbol)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_idl_cache_fetched_at ON idl_cache(fetched_at)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_discovery_queue_status ON discovery_queue(status)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_discovery_queue_priority ON discovery_queue(priority DESC)",
    )
    .execute(pool)
    .await?;

    // Insert initial DEX program data
    insert_initial_dex_programs(pool).await?;

    info!("Database migrations completed successfully");
    Ok(())
}

async fn insert_initial_dex_programs(pool: &SqlitePool) -> Result<()> {
    let initial_dexs = vec![
        (
            "whirLbMi2vNtMeJ6z18WZb98sPNT1L4GzWk2vuyZgq2U",
            "Orca",
            "CLMM",
            "constant_product_concentrated",
        ),
        (
            "675kPX9MWYgmL7z3vNL1SWqjc9p3t7g8a8PNg4K8d5m",
            "Raydium",
            "AMM",
            "constant_product",
        ),
        (
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
            "Raydium",
            "CLMM",
            "constant_product_concentrated",
        ),
        (
            "PhoeNiX7JJR8KLjc9JTvnR7K5s1E8K8o5C8m4D9w3pN",
            "Phoenix",
            "CLOB",
            "order_book",
        ),
        (
            "METALmNJxYdxiMExY7hhE6RTfQ3QqpTDUxqrnWBH8dG",
            "Meteora",
            "DLMM",
            "dynamic_liquidity",
        ),
        (
            "EewxydAPCCVuNEyrVN68PuSYdQ7wKn27V9Gjeoi8dy3S",
            "Lifinity",
            "AMM",
            "proactive_market_maker",
        ),
        (
            "srmqPiTfBJqh2DTFqzjLKV4y8bF9BYdj4tB9qzq3u7K",
            "OpenBook",
            "CLOB",
            "order_book",
        ),
        (
            "invPNzJKHEKXZnTjvhF6E8DCT5LQ4bMkP3h8Kd6F9xL",
            "Invariant",
            "CLMM",
            "constant_product_concentrated",
        ),
        (
            "FLUXfqQ5J8k3vNjF6X2mTKs8q7e9L3d4RYp6z5w8VcD",
            "Fluxbeam",
            "AMM",
            "cross_chain",
        ),
        (
            "GFXsq4j9yNMd3z8vA5q7f3E6kW8L7x2BNc5T9e4RyHP",
            "GooseFX",
            "AMM",
            "full_stack_defi",
        ),
    ];

    for (program_id, dex_name, protocol_type, liquidity_model) in initial_dexs {
        let quirks = match dex_name {
            "Orca" => r#"["q64_math", "tick_arrays", "position_nfts", "fee_tiers"]"#,
            "Raydium" => r#"["serum_integration", "farm_rewards", "concentrated_liquidity"]"#,
            "Phoenix" => r#"["order_book", "market_makers", "limit_orders"]"#,
            "Meteora" => r#"["dynamic_fees", "bin_steps", "auto_rebalancing"]"#,
            "Lifinity" => r#"["delta_neutral", "proactive_mm", "lazy_liquidity"]"#,
            "OpenBook" => r#"["serum_successor", "order_book", "crank_operations"]"#,
            "Invariant" => r#"["custom_fee_tiers", "concentrated_liquidity", "multiple_pools"]"#,
            "Fluxbeam" => r#"["cross_chain", "bridge_integration", "multi_asset"]"#,
            "GooseFX" => r#"["full_stack", "derivatives", "synthetics"]"#,
            _ => r#"[]"#,
        };

        let fee_structure = match protocol_type {
            "CLMM" => r#"{"swap_fee_bps": 30, "protocol_fee_bps": 1, "variable": true}"#,
            "AMM" => r#"{"swap_fee_bps": 25, "protocol_fee_bps": 1, "variable": false}"#,
            "CLOB" => r#"{"maker_fee_bps": -2, "taker_fee_bps": 8, "variable": false}"#,
            "DLMM" => r#"{"base_fee_bps": 20, "variable_fee_bps": 80, "variable": true}"#,
            _ => r#"{"swap_fee_bps": 30, "protocol_fee_bps": 1, "variable": false}"#,
        };

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO dex_programs 
            (program_id, dex_name, protocol_type, fee_structure, liquidity_model, quirks, is_active)
            VALUES (?, ?, ?, ?, ?, ?, true)
            "#,
        )
        .bind(program_id)
        .bind(dex_name)
        .bind(protocol_type)
        .bind(fee_structure)
        .bind(liquidity_model)
        .bind(quirks)
        .execute(pool)
        .await?;
    }

    // Insert common tokens
    let initial_tokens = vec![
        (
            "So11111111111111111111111111111111111111112",
            "SOL",
            "Solana",
            9,
            false,
        ),
        (
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "USDC",
            "USD Coin",
            6,
            true,
        ),
        (
            "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
            "USDT",
            "Tether USD",
            6,
            true,
        ),
        (
            "7i5KKsX2weiTkry7jA4ZwSuXGhs5eJBEjY8vVxR4pfRx",
            "GMT",
            "STEPN",
            9,
            false,
        ),
        (
            "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",
            "mSOL",
            "Marinade SOL",
            9,
            false,
        ),
        (
            "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj",
            "stSOL",
            "Lido Staked SOL",
            9,
            false,
        ),
    ];

    for (mint_address, symbol, name, decimals, is_stable) in initial_tokens {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO tokens 
            (mint_address, symbol, name, decimals, is_stable_coin, verified)
            VALUES (?, ?, ?, ?, ?, true)
            "#,
        )
        .bind(mint_address)
        .bind(symbol)
        .bind(name)
        .bind(decimals)
        .bind(is_stable)
        .execute(pool)
        .await?;
    }

    Ok(())
}
