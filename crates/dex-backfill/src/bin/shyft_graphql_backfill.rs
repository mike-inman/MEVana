use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::Parser;
use redis::{aio::MultiplexedConnection, pipe};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Parser)]
struct Args {
    /// Shyft API key (or set SHYFT_API_KEY env)
    #[clap(long, env = "SHYFT_API_KEY", default_value = "")]
    shyft_api_key: String,

    /// Shyft network: devnet | mainnet-beta
    #[clap(long, env = "SHYFT_NETWORK", default_value = "mainnet-beta")]
    network: String,

    /// Redis URL
    #[clap(long, env = "REDIS_URL", default_value = "redis://localhost:6380")]
    redis_url: String,

    /// How many records per page to request (Shyft paginates via `offset`)
    #[clap(long, env = "PAGE_SIZE", default_value = "250")]
    page_size: usize,

    /// Maximum number of pages to pull (stop early on empty page)
    #[clap(long, env = "MAX_PAGES", default_value = "200")]
    max_pages: usize,

    /// Optional: only pull pools where either mint matches this string
    #[clap(long, env = "MINT_FILTER")]
    mint_filter: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct WhirlpoolRows {
    #[serde(rename = "ORCA_WHIRLPOOLS_whirlpool")]
    rows: Vec<PoolRow>,
}

#[derive(Debug, Deserialize)]
struct PoolRow {
    pubkey: String,
    tokenMintA: String,
    tokenMintB: String,
    feeRate: i64,
    tickCurrentIndex: i32,
    // sqrtPrice: u128,
    sqrtPrice: f64,
}

#[derive(Debug, Deserialize)]
struct ShyftTokenInfoResp {
    success: bool,
    result: Option<ShyftTokenInfo>,
    message: Option<String>,
}
#[derive(Debug, Deserialize)]
struct ShyftTokenInfo {
    decimals: u8,
    address: String,
}

/// Build Shyft GraphQL endpoint
fn make_graphql_endpoint(api_key: &str, network: &str) -> String {
    // docs: https://programs.shyft.to/v0/graphql/?api_key=...&network=mainnet-beta
    format!(
        "https://programs.shyft.to/v0/graphql/?api_key={}&network={}",
        api_key, network
    )
}

async fn get_decimals(client: &Client, api_key: &str, network: &str, mint: &str) -> Result<u8> {
    // docs: GET /sol/v1/token/get_info?network=...&token_address=...
    let url = format!(
        "https://api.shyft.to/sol/v1/token/get_info?network={}&token_address={}",
        network, mint
    );
    let resp = client
        .get(&url)
        .header("x-api-key", api_key)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .context("token get_info request failed")?
        .error_for_status()
        .context("token get_info non-200")?
        .json::<ShyftTokenInfoResp>()
        .await
        .context("token get_info json decode failed")?;

    match (resp.success, resp.result) {
        (true, Some(r)) => Ok(r.decimals),
        _ => bail!("decimals lookup failed for {}: {:?}", mint, resp.message),
    }
}

async fn backfill_page(
    client: &Client,
    redis: &mut MultiplexedConnection,
    graphql_endpoint: &str,
    shyft_api_key: &str,
    network: &str,
    offset: usize,
    page_size: usize,
    mint_filter: &Option<String>,
) -> Result<usize> {
    // Shyft GraphQL supports `offset`, and you can add filters with `where` and sort with `order_by`.
    // We'll pull by liquidity desc so the most relevant pools arrive early.
    // Ref: docs on filters/order/pagination.
    // https://docs.shyft.to/solana-indexers/graphql-apis/paginating-response
    // https://docs.shyft.to/solana-indexers/graphql-apis/applying-filters
    // https://docs.shyft.to/solana-indexers/graphql-apis/ordering-and-sorting-data

    // Build optional mint filter
    let where_clause = if let Some(m) = mint_filter {
        // either side equals mint AND liquidity > 0
        format!(
            r#"where: {{ _and: [
                {{ liquidity: {{ _gt: "0" }} }},
                {{ _or: [ {{ tokenMintA: {{ _eq: "{}" }} }}, {{ tokenMintB: {{ _eq: "{}" }} }} ] }}
            ]}}"#,
            m, m
        )
    } else {
        r#"where: { liquidity: { _gt: "0" } }"#.to_string()
    };

    // Note: Shyft doesn't document a `limit` param in the page; offset is supported.
    // We emulate "page_size" by slicing client-side.
    let query = format!(
        r#"
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
        }}"#,
        where_clause = where_clause
    );

    #[derive(serde::Serialize)]
    struct Body<'a> {
        query: &'a str,
        variables: serde_json::Value,
    }

    let body = Body {
        query: &query,
        variables: serde_json::json!({ "offset": offset as i64 }),
    };

    let response_text = client
        .post(graphql_endpoint)
        .json(&body)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .context("graphQL request failed")?
        .error_for_status()
        .context("graphQL non-200")?
        .text()  // Get response as text first
        .await
        .context("graphQL text decode failed")?;

    // Log the raw response
    println!("Raw GraphQL response: {}", response_text);

    // Add debug logging around column 285
    let start = 285_usize.saturating_sub(50); // 50 chars before column 285
    let end = (285_usize + 50).min(response_text.len()); // 50 chars after column 285
    println!("Characters around column 285 ({} to {}):", start, end);
    println!("'{}'", &response_text[start..end]);



    // Then parse it as JSON
    let resp: GraphQLResponse<WhirlpoolRows> = serde_json::from_str(&response_text)
        .context("graphQL json decode failed")?;

    let mut rows = resp.data.rows;
    if rows.is_empty() {
        return Ok(0);
    }

    // If page_size < server batch, keep only first N
    if rows.len() > page_size {
        rows.truncate(page_size);
    }

    let row_count = rows.len(); // Store the count before moving rows

    for pool in rows {
        // decimals for each mint (cache would be nicer; keep it simple)
        let dec_a = get_decimals(client, shyft_api_key, network, &pool.tokenMintA).await
            .unwrap_or(6); // fall back to a sane default if a lookup fails
        let dec_b = get_decimals(client, shyft_api_key, network, &pool.tokenMintB).await
            .unwrap_or(6);

        let pair = format!("{}/{}", pool.tokenMintA, pool.tokenMintB);
        let pool_key = format!("dex:pool:{}", pool.pubkey);
        let now = Utc::now().timestamp();

        // One round-trip per pool
        let mut p = pipe();
        p.cmd("HSET").arg(&pool_key)
            .arg("mint_a").arg(&pool.tokenMintA)
            .arg("mint_b").arg(&pool.tokenMintB)
            .arg("decimals_a").arg(dec_a as i64)
            .arg("decimals_b").arg(dec_b as i64)
            .arg("fee_bps").arg(pool.feeRate as i64)
            .arg("sqrt_price_x64").arg(&pool.sqrtPrice) // Shyft returns Q64.64 as string named sqrtPrice
            .arg("tick").arg(pool.tickCurrentIndex as i64)
            .arg("updated_at").arg(now)
            .arg("pair").arg(&pair)
         .cmd("SET").arg(format!("dex:pair:latest_pool:{}", pair)).arg(&pool.pubkey).ignore();

        // Let type inference pick the return type; don't specify generics.
        p.query_async::<()>(redis).await
            .with_context(|| format!("redis write failed for pool {}", pool.pubkey))?;
    }

    Ok(row_count)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    let args = Args::parse();

    let client = Client::builder()
        .user_agent("dex-backfill/0.1")
        .pool_idle_timeout(Duration::from_secs(30))
        .build()?;

    let endpoint = make_graphql_endpoint(&args.shyft_api_key, &args.network);

    // Redis connect
    let redis_client = redis::Client::open(args.redis_url.clone())?;
    let mut redis_conn: MultiplexedConnection = redis_client
        .get_multiplexed_tokio_connection()
        .await
        .context("redis connect failed")?;

    // Simple paginated crawl with offset
    let mut total = 0usize;
    for page in 0..args.max_pages {
        let offset = page * args.page_size;
        let got = backfill_page(
            &client,
            &mut redis_conn,
            &endpoint,
            &args.shyft_api_key,
            &args.network,
            offset,
            args.page_size,
            &args.mint_filter,
        )
        .await
        .with_context(|| format!("pull failed for offset {}", offset))?;

        if got == 0 {
            println!("Done. No more rows at offset {}", offset);
            break;
        }
        total += got;
        println!("Wrote {} pools (this page: {}, offset: {})", total, got, offset);
        // Be nice to the API
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Ok(())
}
