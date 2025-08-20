use anyhow::{Context, Result};
use axum::serve;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use chrono::Utc;
use log::{info, warn};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone)]
struct ServiceConfig {
    rpc_url: String,
    bind_addr: String,
    redis_url: String,
    cache_ttl_secs: usize,
}

impl ServiceConfig {
    fn from_env() -> Result<Self> {
        let rpc_url = std::env::var("RPC_URL")
            .context("RPC_URL env var is required (e.g. https://rpc.shyft.to?api_key=...) ")?;
        let bind_addr =
            std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8083".to_string());
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_string());
        let cache_ttl_secs = std::env::var("CACHE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600);
        Ok(Self {
            rpc_url,
            bind_addr,
            redis_url,
            cache_ttl_secs,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct TokenEntry {
    mint: String,
    symbol: Option<String>,
    name: Option<String>,
    decimals: Option<u8>,
    is_stable: bool,
    tags: Vec<String>,
    sources: Vec<SourceInfo>,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct SourceInfo {
    name: String,
    fetched_at: String,
}

fn is_known_stable(mint: &str) -> Option<&'static str> {
    match mint {
        // USDC, USDT
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => Some("USDC"),
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => Some("USDT"),
        _ => None,
    }
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<RpcResult>,
}

#[derive(Deserialize)]
struct RpcResult {
    value: Option<RpcAccount>,
}

#[derive(Deserialize)]
struct RpcAccount {
    data: Option<(String, String)>, // (base64, encoding)
}

async fn get_token(Path(mint): Path<String>, Json(cfg): Json<ServiceConfig>) -> impl IntoResponse {
    match get_token_impl(&mint, &cfg).await {
        Ok(entry) => (StatusCode::OK, Json(entry)).into_response(),
        Err(e) => {
            warn!("token lookup failed for {}: {}", mint, e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

async fn get_token_impl(mint: &str, cfg: &ServiceConfig) -> Result<TokenEntry> {
    // Try Redis cache first (read-through)
    let cache_key = format!("TOKEN_REGISTRY:{}", mint);
    if let Ok(client) = redis::Client::open(cfg.redis_url.clone()) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let cached: Result<Option<String>, _> = conn.get(&cache_key).await;
            if let Ok(Some(json_str)) = cached {
                if let Ok(entry) = serde_json::from_str::<TokenEntry>(&json_str) {
                    info!("cache hit for {}", mint);
                    return Ok(entry);
                }
            }
        }
    }

    let now = Utc::now().to_rfc3339();
    let client = reqwest::Client::new();

    // JSON-RPC getAccountInfo (base64) for the Mint account
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [ mint, { "encoding": "base64" } ]
    });
    let resp = client
        .post(&cfg.rpc_url)
        .json(&payload)
        .send()
        .await
        .context("rpc request failed")?;
    let status = resp.status();
    let text = resp.text().await.context("rpc read body")?;
    if !status.is_success() {
        anyhow::bail!("rpc status {}: {}", status, text);
    }
    let rpc: RpcResponse = serde_json::from_str(&text).context("rpc json parse")?;
    let data_b64 = rpc
        .result
        .and_then(|r| r.value)
        .and_then(|v| v.data)
        .map(|t| t.0)
        .context("no account data returned")?;
    let data = base64::decode(data_b64).context("base64 decode mint account")?;
    let decimals = data
        .get(44)
        .copied()
        .context("mint data too short for decimals")?;

    let stable_sym = is_known_stable(mint);
    let entry = TokenEntry {
        mint: mint.to_string(),
        symbol: stable_sym.map(|s| s.to_string()),
        name: None,
        decimals: Some(decimals),
        is_stable: stable_sym.is_some(),
        tags: if stable_sym.is_some() {
            vec!["stablecoin".to_string()]
        } else {
            vec![]
        },
        sources: vec![SourceInfo {
            name: "rpc_mint".to_string(),
            fetched_at: now.clone(),
        }],
        updated_at: now,
    };
    // Write to Redis with TTL
    if let Ok(client) = redis::Client::open(cfg.redis_url.clone()) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(s) = serde_json::to_string(&entry) {
                let _: Result<(), _> = conn.set_ex(cache_key, s, cfg.cache_ttl_secs as u64).await;
            }
        }
    }
    Ok(entry)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let cfg = ServiceConfig::from_env()?;
    info!("token-registry-service starting on {}", cfg.bind_addr);

    let app = Router::new()
        .route("/healthz", get(health))
        // Inject cfg per-request by posting it as JSON with the path.
        // For simplicity in this first step, use a closure capturing cfg.
        .route(
            "/v1/token/:mint",
            get({
                let cfg = cfg.clone();
                move |Path(mint): Path<String>| {
                    let cfg = cfg.clone();
                    async move {
                        match get_token_impl(&mint, &cfg).await {
                            Ok(entry) => (StatusCode::OK, Json(entry)).into_response(),
                            Err(e) => (
                                StatusCode::BAD_REQUEST,
                                Json(json!({"error": e.to_string()})),
                            )
                                .into_response(),
                        }
                    }
                }
            }),
        );

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    serve(listener, app).await.context("server failed")?;
    Ok(())
}
