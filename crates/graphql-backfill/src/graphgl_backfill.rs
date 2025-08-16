// shyft_graphql_backfill.rs
// ------------------------------------------------------------
// Purpose: Seed Redis with Whirlpool pool snapshots + mint metadata
// using Shyft GraphQL (pools) and Shyft REST (mint info). Adds:
//  - Tiny in-memory LRU for mint metadata (decimals/symbol/name)
//  - Writes both address-based pair mapping and optional symbol-based mapping
//
// ✅ Hot path remains your Geyser streamer → Redis. This file is an offline backfill/helper.
//
// USAGE
// -----
// Cargo.toml (add these deps):
//
// [dependencies]
// anyhow = "1"
// bs58 = "0.5"
// chrono = "0.4"
// clap = { version = "4.5", features = ["derive", "env"] }
// dotenv = "0.15"
// env_logger = "0.11"
// lru = "0.12"                # ← LRU cache
// log = "0.4"
// redis = { version = "0.32", features = ["tokio-comp"] }
// reqwest = { version = "0.12", features = ["json", "gzip", "brotli", "deflate", "rustls-tls"] }
// serde = { version = "1", features = ["derive"] }
// serde_json = "1"
// tokio = { version = "1", features = ["full"] }
//
// .env (examples):
// SHYFT_API_KEY=fWrVh_mDygK358FC
// SHYFT_NETWORK=mainnet-beta
// REDIS_URL=redis://127.0.0.1:6379/0
//
// Run:
// cargo run --release --bin shyft_graphql_backfill -- \
//   --page-size 250 --max-pages 200
//
// Optional:
//   --mint-filter EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
//
// Redis keys written
// ------------------
// HSET dex:pool:{POOL} => mint_a, mint_b, decimals_a, decimals_b, fee_bps,
//                         sqrt_price_x64, tick, updated_at, pair, [pair_symbol]
// SET  dex:pair:latest_pool:{mintA/mintB} -> {POOL}
// SET  dex:pair_symbol:latest_pool:{SYM_A/SYM_B} -> {POOL}    (only if both symbols known)
// HSET dex:mint:{MINT} => decimals, [symbol], [name], updated_at

use anyhow::{bail, Context, Result};
use log::info;
use chrono::Utc;
use clap::{Parser, ArgAction};
use dotenv::dotenv;
use lru::LruCache;
use redis::{aio::MultiplexedConnection, AsyncCommands, pipe};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{num::NonZeroUsize, time::Duration};

#[derive(Debug, Parser)]
#[command(author, version, about = "Shyft backfill: Whirlpool pools + mint metadata → Redis (with LRU)")]
struct Args {
    /// Shyft API key
    #[arg(long, env = "SHYFT_API_KEY")]
    shyft_api_key: String,

    /// Network: devnet | mainnet-beta
    #[arg(long, env = "SHYFT_NETWORK", default_value = "mainnet-beta")]
    network: String,

    /// Redis URL
    #[arg(long, env = "REDIS_URL", default_value = "redis://127.0.0.1:6379/0")]
    redis_url: String,

    /// Page size (client-side truncation)
    #[arg(long, default_value_t = 250)]
    page_size: usize,

    /// Max pages to fetch
    #[arg(long, default_value_t = 200)]
    max_pages: usize,

    /// Only pull pools where either mint equals this address
    #[arg(long)]
    mint_filter: Option<String>,

    /// LRU capacity for mint metadata
    #[arg(long, default_value_t = 1024)]
    mint_cache_cap: usize,

    /// Dry run (don’t write to Redis)
    #[arg(long, action = ArgAction::SetTrue)]
    dry_run: bool,
}

// ---------- Shyft endpoints ----------
fn graphql_endpoint(api_key: &str, network: &str) -> String {
    format!("https://programs.shyft.to/v0/graphql/?api_key={api_key}&network={network}")
}
fn token_info_url(network: &str, mint: &str) -> String {
    format!("https://api.shyft.to/sol/v1/token/get_info?network={network}&token_address={mint}")
}

// ---------- Models ----------
#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> { data: T }

#[derive(Debug, Deserialize)]
struct WhirlpoolRows { #[serde(rename = "ORCA_WHIRLPOOLS_whirlpool")] rows: Vec<PoolRow> }

#[derive(Debug, Deserialize, Clone)]
struct PoolRow {
    pubkey: String,
    tokenMintA: String,
    tokenMintB: String,
    feeRate: i64,
    tickCurrentIndex: i32,
    sqrtPrice: String, // Shyft returns as string
}

#[derive(Debug, Deserialize)]
struct ShyftTokenInfoResp { success: bool, result: Option<ShyftTokenInfo>, message: Option<String> }
#[derive(Debug, Deserialize, Clone)]
struct ShyftTokenInfo { address: String, decimals: u8, #[serde(default)] symbol: Option<String>, #[serde(default)] name: Option<String> }

#[derive(Debug, Clone)]
struct MintMeta { decimals: u8, symbol: Option<String>, name: Option<String> }

// ---------- LRU cache for mint metadata ----------
struct MintCache {
    lru: LruCache<String, MintMeta>,
    client: Client,
    api_key: String,
    network: String,
}
impl MintCache {
    fn new(cap: usize, client: Client, api_key: String, network: String) -> Self {
        let cap = NonZeroUsize::new(cap.max(16)).unwrap();
        Self { lru: LruCache::new(cap), client, api_key, network }
    }
    async fn get(&mut self, mint: &str) -> Result<MintMeta> {
        if let Some(v) = self.lru.get(mint).cloned() { return Ok(v); }
        let url = token_info_url(&self.network, mint);
        let resp = self.client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .timeout(Duration::from_secs(10))
            .send().await?.error_for_status()?;
        let info: ShyftTokenInfoResp = resp.json().await?;
        let r = info.result.ok_or_else(|| anyhow::anyhow!("No token info for {mint}: {:?}", info.message))?;
        let meta = MintMeta { decimals: r.decimals, symbol: r.symbol, name: r.name };
        self.lru.put(mint.to_string(), meta.clone());
        Ok(meta)
    }
}

// ---------- GraphQL page pull ----------
async fn backfill_page(
    http: &Client,
    redis: &mut MultiplexedConnection,
    endpoint: &str,
    mint_cache: &mut MintCache,
    offset: usize,
    page_size: usize,
    mint_filter: &Option<String>,
    dry_run: bool,
) -> Result<usize> {
    // Build filter (liquidity > 0 is a good heuristic; Shyft supports where/order_by/offset)
    let where_clause = if let Some(m) = mint_filter {
        format!(r#"where: {{ _and: [ {{ liquidity: {{ _gt: "0" }} }}, {{ _or: [ {{ tokenMintA: {{ _eq: "{m}" }} }}, {{ tokenMintB: {{ _eq: "{m}" }} }} ] }} ] }}"#)
    } else {
        r#"where: { liquidity: { _gt: "0" } }"#.to_string()
    };

    let query = format!(r#"
        query Pools($offset: Int!) {{
          ORCA_WHIRLPOOLS_whirlpool(
            {where_clause}
            order_by: {{ liquidity: desc }}
            offset: $offset
          ) {{
            pubkey
            tokenMintA
            tokenMintB
            feeRate
            tickCurrentIndex
            sqrtPrice
          }}
        }}
    "#, where_clause = where_clause);

    #[derive(serde::Serialize)]
    struct Body<'a> { query: &'a str, variables: serde_json::Value }
    let body = Body { query: &query, variables: json!({ "offset": offset as i64 }) };

    let resp: GraphQLResponse<WhirlpoolRows> = http
        .post(endpoint)
        .json(&body)
        .timeout(Duration::from_secs(20))
        .send().await?.error_for_status()?
        .json().await?;

    let mut rows = resp.data.rows;
    if rows.is_empty() { return Ok(0); }
    if rows.len() > page_size { rows.truncate(page_size); }

    for pool in rows {
        // LRU-backed mint metadata
        let a = mint_cache.get(&pool.tokenMintA).await?;
        let b = mint_cache.get(&pool.tokenMintB).await?;

        let pair_addr = format!("{}/{}", pool.tokenMintA, pool.tokenMintB);
        let pair_sym = match (&a.symbol, &b.symbol) {
            (Some(sa), Some(sb)) if !sa.is_empty() && !sb.is_empty() => Some(format!("{}/{}", sa, sb)),
            _ => None,
        };

        if dry_run {
            info!("DRY: pool={} pair={} decA={} decB={} symA={:?} symB={:?}",
                pool.pubkey, pair_addr, a.decimals, b.decimals, a.symbol, b.symbol);
            continue;
        }

        // Single pipeline per pool: write pool hash, pair mappings, and both mint metas
        let now = Utc::now().timestamp();
        let pool_key = format!("dex:pool:{}", pool.pubkey);
        let mint_a_key = format!("dex:mint:{}", pool.tokenMintA);
        let mint_b_key = format!("dex:mint:{}", pool.tokenMintB);

        let mut p = pipe();
        // Pool snapshot
        p.cmd("HSET").arg(&pool_key)
            .arg("mint_a").arg(&pool.tokenMintA)
            .arg("mint_b").arg(&pool.tokenMintB)
            .arg("decimals_a").arg(a.decimals as i64)
            .arg("decimals_b").arg(b.decimals as i64)
            .arg("fee_bps").arg(pool.feeRate as i64)
            .arg("sqrt_price_x64").arg(&pool.sqrtPrice)
            .arg("tick").arg(pool.tickCurrentIndex as i64)
            .arg("updated_at").arg(now)
            .arg("pair").arg(&pair_addr);
        if let Some(ps) = &pair_sym { p.arg("pair_symbol").arg(ps); }

        // Pair mappings
        p.cmd("SET").arg(format!("dex:pair:latest_pool:{}", pair_addr)).arg(&pool.pubkey).ignore();
        if let Some(ps) = &pair_sym {
            p.cmd("SET").arg(format!("dex:pair_symbol:latest_pool:{}", ps)).arg(&pool.pubkey).ignore();
        }

        // Mint A meta
        p.cmd("HSET").arg(&mint_a_key)
            .arg("decimals").arg(a.decimals as i64)
            .arg("updated_at").arg(now);
        if let Some(sym) = &a.symbol { p.arg("symbol").arg(sym); }
        if let Some(name) = &a.name { p.arg("name").arg(name); }

        // Mint B meta
        p.cmd("HSET").arg(&mint_b_key)
            .arg("decimals").arg(b.decimals as i64)
            .arg("updated_at").arg(now);
        if let Some(sym) = &b.symbol { p.arg("symbol").arg(sym); }
        if let Some(name) = &b.name { p.arg("name").arg(name); }

        p.ignore();
        p.query_async::<()>(&mut *redis).await
            .with_context(|| format!("redis write failed for pool {}", pool.pubkey))?;
    }

    Ok(page_size)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();
    let args = Args::parse();

    let http = Client::builder()
        .user_agent("shyft-backfill/1.0")
        .pool_idle_timeout(Duration::from_secs(30))
        .build()?;

    let endpoint = graphql_endpoint(&args.shyft_api_key, &args.network);

    // Redis connect
    let redis_client = redis::Client::open(args.redis_url.clone())?;
    let mut redis_conn: MultiplexedConnection = redis_client
        .get_multiplexed_tokio_connection().await?;

    // LRU for mint meta
    let mut mint_cache = MintCache::new(
        args.mint_cache_cap,
        http.clone(),
        args.shyft_api_key.clone(),
        args.network.clone(),
    );

    let mut total = 0usize;
    for page in 0..args.max_pages {
        let offset = page * args.page_size;
        let got = backfill_page(
            &http,
            &mut redis_conn,
            &endpoint,
            &mut mint_cache,
            offset,
            args.page_size,
            &args.mint_filter,
            args.dry_run,
        ).await.with_context(|| format!("pull failed at offset {}", offset))?;

        if got == 0 { break; }
        total += got;
        println!("wrote {} pools (page {}, offset {})", total, page + 1, offset);
        tokio::time::sleep(Duration::from_millis(200)).await; // be polite
    }

    println!("backfill complete: {} pools", total);
    Ok(())
}
