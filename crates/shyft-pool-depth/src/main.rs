use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::Parser;
use dotenv::dotenv;
use env_logger::Env;
use log::{error, info, warn};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(author, version, about = "Shyft Pool Depth Service - Get pool depth data directly from Shyft APIs")]
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

    /// Update interval in seconds
    #[arg(long, default_value_t = 30)]
    update_interval: u64,

    /// Max pools to process per update
    #[arg(long, default_value_t = 100)]
    max_pools: usize,

    /// Dry run (don't write to Redis)
    #[arg(long, action = clap::ArgAction::SetTrue)]
    dry_run: bool,
}

// ---------- Shyft API Models ----------
#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct WhirlpoolRows {
    #[serde(rename = "ORCA_WHIRLPOOLS_whirlpool")]
    rows: Vec<PoolRow>,
}

#[derive(Debug, Deserialize, Clone)]
struct PoolRow {
    pubkey: String,
    tokenMintA: String,
    tokenMintB: String,
    feeRate: i64,
    tickCurrentIndex: i32,
    sqrtPrice: String,
    liquidity: String,
    tickSpacing: i32,
}

#[derive(Debug, Deserialize)]
struct ShyftTokenInfoResp {
    success: bool,
    result: Option<ShyftTokenInfo>,
    message: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ShyftTokenInfo {
    address: String,
    decimals: u8,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

// ---------- Pool Depth Models ----------
#[derive(Debug, Serialize, Clone)]
struct PoolDepth {
    pool: String,
    mint_a: String,
    mint_b: String,
    decimals_a: u8,
    decimals_b: u8,
    fee_bps: i64,
    tick_current: i32,
    sqrt_price: String,
    liquidity: String,
    tick_spacing: i32,
    depth_levels: Vec<DepthLevel>,
    updated_at: i64,
}

#[derive(Debug, Serialize, Clone)]
struct DepthLevel {
    tick: i32,
    liquidity_net: i64,
    price: f64,
    size_usd: f64,
}

// ---------- Shyft API Client ----------
struct ShyftClient {
    client: Client,
    api_key: String,
    network: String,
}

impl ShyftClient {
    fn new(api_key: String, network: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        
        Self {
            client,
            api_key,
            network,
        }
    }

    fn graphql_endpoint(&self) -> String {
        format!("https://programs.shyft.to/v0/graphql/?api_key={}&network={}", 
                self.api_key, self.network)
    }

    fn token_info_url(&self, mint: &str) -> String {
        format!("https://api.shyft.to/sol/v1/token/get_info?network={}&token_address={}", 
                self.network, mint)
    }

    async fn get_pools(&self, limit: usize, offset: usize) -> Result<Vec<PoolRow>> {
        let query = format!(
            r#"
            query Pools($limit: Int!, $offset: Int!) {{
              ORCA_WHIRLPOOLS_whirlpool(
                where: {{ liquidity: {{ _gt: "0" }} }}
                order_by: {{ liquidity: desc }}
                limit: $limit
                offset: $offset
              ) {{
                pubkey
                tokenMintA
                tokenMintB
                feeRate
                tickCurrentIndex
                sqrtPrice
                liquidity
                tickSpacing
              }}
            }}
            "#
        );

        let body = serde_json::json!({
            "query": query,
            "variables": {
                "limit": limit,
                "offset": offset
            }
        });

        let response = self.client
            .post(self.graphql_endpoint())
            .json(&body)
            .send()
            .await
            .context("GraphQL request failed")?
            .error_for_status()
            .context("GraphQL non-200 response")?;

        let response_text = response.text().await?;
        let graphql_response: GraphQLResponse<WhirlpoolRows> = serde_json::from_str(&response_text)
            .context("Failed to parse GraphQL response")?;

        Ok(graphql_response.data.rows)
    }

    async fn get_token_info(&self, mint: &str) -> Result<ShyftTokenInfo> {
        let url = self.token_info_url(mint);
        let resp = self.client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .context("Token info request failed")?
            .error_for_status()
            .context("Token info non-200 response")?;

        let info: ShyftTokenInfoResp = resp.json().await?;
        info.result.ok_or_else(|| {
            anyhow::anyhow!("No token info for {}: {:?}", mint, info.message)
        })
    }

    async fn calculate_pool_depth(&self, pool: &PoolRow) -> Result<PoolDepth> {
        // Get token metadata
        let token_a = self.get_token_info(&pool.tokenMintA).await?;
        let token_b = self.get_token_info(&pool.tokenMintB).await?;

        // Calculate depth levels around current tick
        let current_tick = pool.tickCurrentIndex;
        let tick_spacing = pool.tickSpacing;
        let mut depth_levels = Vec::new();

        // Generate depth levels around current tick
        for tick_offset in -10..=10 {
            let tick = current_tick + (tick_offset * tick_spacing);
            let liquidity_net = if tick_offset == 0 {
                pool.liquidity.parse::<i64>().unwrap_or(0)
            } else {
                // Simulate liquidity distribution (in real implementation, this would come from TickArray data)
                (pool.liquidity.parse::<i64>().unwrap_or(0) / 20).max(0)
            };

            // Calculate price from tick (simplified)
            let price = self.tick_to_price(tick, &token_a, &token_b);
            let size_usd = self.liquidity_to_usd_size(liquidity_net, price, &token_a, &token_b);

            depth_levels.push(DepthLevel {
                tick,
                liquidity_net,
                price,
                size_usd,
            });
        }

        Ok(PoolDepth {
            pool: pool.pubkey.clone(),
            mint_a: pool.tokenMintA.clone(),
            mint_b: pool.tokenMintB.clone(),
            decimals_a: token_a.decimals,
            decimals_b: token_b.decimals,
            fee_bps: pool.feeRate,
            tick_current: pool.tickCurrentIndex,
            sqrt_price: pool.sqrtPrice.clone(),
            liquidity: pool.liquidity.clone(),
            tick_spacing: pool.tickSpacing,
            depth_levels,
            updated_at: Utc::now().timestamp(),
        })
    }

    fn tick_to_price(&self, tick: i32, token_a: &ShyftTokenInfo, token_b: &ShyftTokenInfo) -> f64 {
        // Simplified tick to price conversion
        // In reality, this would use the proper Whirlpool formula
        let tick_ratio = (tick as f64 / 1000.0).exp();
        tick_ratio * (10.0_f64.powi((token_b.decimals - token_a.decimals) as i32))
    }

    fn liquidity_to_usd_size(&self, liquidity_net: i64, price: f64, token_a: &ShyftTokenInfo, token_b: &ShyftTokenInfo) -> f64 {
        // Simplified liquidity to USD size conversion
        // In reality, this would use proper liquidity math
        let liquidity_f64 = liquidity_net as f64;
        let base_size = liquidity_f64 / (10.0_f64.powi(token_a.decimals as i32));
        base_size * price
    }
}

// ---------- Redis Operations ----------
async fn write_pool_depth_to_redis(
    redis: &mut MultiplexedConnection,
    depth: &PoolDepth,
) -> Result<()> {
    let pool_key = format!("dex:pool_depth:{}", depth.pool);
    
    // Write pool depth data
    let depth_json = serde_json::to_string(depth)?;
    redis.set_ex(&pool_key, depth_json, 300).await?; // 5 minute TTL

    // Write summary data for quick access
    let summary_key = format!("dex:pool_summary:{}", depth.pool);
    let summary = serde_json::json!({
        "pool": depth.pool,
        "mint_a": depth.mint_a,
        "mint_b": depth.mint_b,
        "fee_bps": depth.fee_bps,
        "tick_current": depth.tick_current,
        "sqrt_price": depth.sqrt_price,
        "liquidity": depth.liquidity,
        "depth_levels_count": depth.depth_levels.len(),
        "updated_at": depth.updated_at,
    });
    
    redis.set_ex(&summary_key, summary.to_string(), 300).await?;

    info!("Updated pool depth for {} with {} depth levels", depth.pool, depth.depth_levels.len());
    Ok(())
}

// ---------- Main Service Loop ----------
async fn run_service(args: Args) -> Result<()> {
    let shyft_client = ShyftClient::new(args.shyft_api_key, args.network);
    
    let redis_client = redis::Client::open(args.redis_url)?;
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;

    info!("Starting Shyft Pool Depth Service");
    info!("Update interval: {} seconds", args.update_interval);
    info!("Max pools per update: {}", args.max_pools);

    let mut offset = 0;
    
    loop {
        info!("Fetching pools (offset: {})", offset);
        
        match shyft_client.get_pools(args.max_pools, offset).await {
            Ok(pools) => {
                if pools.is_empty() {
                    info!("No more pools, resetting offset");
                    offset = 0;
                    continue;
                }

                info!("Processing {} pools", pools.len());
                
                for pool in pools {
                    match shyft_client.calculate_pool_depth(&pool).await {
                        Ok(depth) => {
                            if !args.dry_run {
                                if let Err(e) = write_pool_depth_to_redis(&mut redis_conn, &depth).await {
                                    error!("Failed to write pool depth to Redis: {}", e);
                                }
                            } else {
                                info!("[DRY RUN] Would write pool depth for {}", pool.pubkey);
                            }
                        }
                        Err(e) => {
                            error!("Failed to calculate depth for pool {}: {}", pool.pubkey, e);
                        }
                    }
                }

                offset += pools.len();
            }
            Err(e) => {
                error!("Failed to fetch pools: {}", e);
            }
        }

        tokio::time::sleep(Duration::from_secs(args.update_interval)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args = Args::parse();
    
    if args.shyft_api_key.is_empty() {
        bail!("SHYFT_API_KEY is required");
    }

    run_service(args).await
}
