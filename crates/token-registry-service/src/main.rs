mod enrich;
mod merge;
mod metrics;
mod model;
mod sources;
mod storage;
use crate::metrics::*;
use crate::{
    enrich::{enrich_one, ensure_token_present, run_enrichment_loop, EnrichmentConfig},
    sources::{OnChainMintSource, StaticOverrideSource, TokenSource},
    storage::{ColdStore, HotStore, Stores},
};
use anyhow::Result;
use chrono::Utc;
use prometheus::Encoder;
use serde_json::Value;
use sqlx::Row;
use std::fs;
use std::sync::Arc;
use warp::Filter;

// Removed auto-create logic: database must already exist (mevdb).

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let bind_addr = std::env::var("TOKEN_REGISTRY_BIND").unwrap_or_else(|_| "0.0.0.0:8085".into());
    let rpc_url =
        std::env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into());
    let pg_url = std::env::var("TOKEN_REGISTRY_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://mev:mevpassword@localhost:5433/mevdb".into());
    let schema = std::env::var("TOKEN_REGISTRY_SCHEMA").unwrap_or_else(|_| "public".into());
    log::info!("connecting postgres at {} schema={} (set TOKEN_REGISTRY_DATABASE_URL / TOKEN_REGISTRY_SCHEMA to override)", pg_url, schema);

    let hot = Arc::new(HotStore::new());
    let cold = Arc::new(ColdStore::connect(&pg_url, &schema).await.map_err(|e| {
        log::error!("failed to connect postgres url={} err={}", pg_url, e);
        e
    })?);
    if let Ok(row) = sqlx::query("SELECT current_database(), current_user")
        .fetch_one(&cold.pool)
        .await
    {
        let db: String = row.get(0);
        let user: String = row.get(1);
        log::info!("connected db={} user={}", db, user);
    }
    let stores = storage::Stores {
        hot: hot.clone(),
        cold: cold.clone(),
    };

    // Static overrides
    let mut overrides = std::collections::HashMap::new();
    // Add common stablecoins (symbol, name, decimals, logo)
    overrides.insert("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(), (Some("USDC".into()), Some("USD Coin".into()), Some(6u8), Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png".into())));
    overrides.insert("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".into(), (Some("USDT".into()), Some("Tether USD".into()), Some(6u8), Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.png".into())));

    let sources: Vec<Arc<dyn TokenSource>> = vec![
        Arc::new(StaticOverrideSource { map: overrides }),
        Arc::new(OnChainMintSource {
            rpc_url: rpc_url.clone(),
            client: reqwest::Client::new(),
        }),
    ];

    // Seed tokens: (1) snapshot file (merged) ; (2) Solana list (optional) ; (3) static overrides ; (4) env list
    if let Ok(path) = std::env::var("TOKEN_REGISTRY_SNAPSHOT") {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                if let Some(arr) = val.as_array() {
                    let mut loaded = 0usize;
                    for v in arr.iter().take(5000) {
                        // safety cap
                        if let Some(mint) = v.get("mint").and_then(|m| m.as_str()) {
                            let pt = crate::model::PartialToken {
                                mint: mint.to_string(),
                                symbol: v
                                    .get("symbol")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string()),
                                name: v
                                    .get("name")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string()),
                                decimals: v
                                    .get("decimals")
                                    .and_then(|x| x.as_u64())
                                    .and_then(|d| if d <= 255 { Some(d as u8) } else { None }),
                                logo_url: v
                                    .get("logo_url")
                                    .or_else(|| v.get("logoURI"))
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string()),
                                tags: None,
                                is_stable: None,
                                is_wrapped: None,
                                is_lp: None,
                                risk_flags: None,
                                source_id: "snapshot".into(),
                                fetched_at: Utc::now(),
                                weight: 8.0,
                            };
                            ensure_token_present(&stores, mint).await;
                            let existing = stores.hot.get(mint);
                            let merged = crate::merge::merge(existing, vec![pt]);
                            if merged.changed {
                                stores.hot.upsert(merged.record.clone());
                                let _ = stores.cold.write_current(&merged.record).await;
                                let _ = stores.cold.append_version(&merged.record).await;
                                loaded += 1;
                            }
                        }
                    }
                    TOKEN_TOTAL.set(stores.hot.count() as i64);
                    log::info!("snapshot merged {} entries", loaded);
                }
            }
        }
    }

    // Optional bulk ingest from Solana token list (large) when TOKEN_REGISTRY_FETCH_SOLANA_LIST=1
    if std::env::var("TOKEN_REGISTRY_FETCH_SOLANA_LIST")
        .ok()
        .as_deref()
        == Some("1")
    {
        if let Ok(resp) = reqwest::Client::new()
            .get("https://raw.githubusercontent.com/solana-labs/token-list/main/src/tokens/solana.tokenlist.json")
            .send().await
        {
            if let Ok(text) = resp.text().await {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    if let Some(tokens) = v.get("tokens").and_then(|t| t.as_array()) {
                        let mut loaded = 0usize;
                        for t in tokens.iter().take(5000) { // safety cap
                            if let Some(mint) = t.get("address").and_then(|x| x.as_str()) {
                                let pt = crate::model::PartialToken {
                                    mint: mint.to_string(),
                                    symbol: t.get("symbol").and_then(|x| x.as_str()).map(|s| s.to_string()),
                                    name: t.get("name").and_then(|x| x.as_str()).map(|s| s.to_string()),
                                    decimals: t.get("decimals").and_then(|x| x.as_u64()).and_then(|d| if d<=255 {Some(d as u8)} else {None}),
                                    logo_url: t.get("logoURI").and_then(|x| x.as_str()).map(|s| s.to_string()),
                                    tags: None,
                                    is_stable: None,
                                    is_wrapped: None,
                                    is_lp: None,
                                    risk_flags: None,
                                    source_id: "solana_list".into(),
                                    fetched_at: Utc::now(),
                                    weight: 7.0,
                                };
                                ensure_token_present(&stores, mint).await;
                                let existing = stores.hot.get(mint);
                                let merged = crate::merge::merge(existing, vec![pt]);
                                if merged.changed {
                                    stores.hot.upsert(merged.record.clone());
                                    let _ = stores.cold.write_current(&merged.record).await;
                                    let _ = stores.cold.append_version(&merged.record).await;
                                }
                                loaded += 1;
                            }
                        }
                        TOKEN_TOTAL.set(stores.hot.count() as i64);
                        log::info!("solana token list merged {} entries (capped)", loaded);
                    }
                }
            }
        }
    }
    if let Some(static_source) = sources
        .iter()
        .find_map(|s| s.as_any().downcast_ref::<StaticOverrideSource>())
    {
        for mint in static_source.map.keys() {
            ensure_token_present(&stores, mint).await;
            let _ = enrich_one(&stores, &sources, mint).await;
        }
    }
    if let Ok(extra) = std::env::var("TOKEN_REGISTRY_SEED_MINTS") {
        for mint in extra.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            ensure_token_present(&stores, mint).await;
            let _ = enrich_one(&stores, &sources, mint).await;
        }
    }

    // Kick off enrichment loop
    let loop_stores = stores.clone();
    let loop_sources = sources.clone();
    tokio::spawn(async move {
        run_enrichment_loop(
            loop_stores,
            loop_sources,
            EnrichmentConfig {
                ttl_secs: 3600,
                concurrency: 4,
            },
        )
        .await;
    });

    // Routes
    let stores_filter = warp::any().map(move || stores.clone());
    let sources_filter = warp::any().map(move || sources.clone());

    let token_route = warp::path!("v1" / "token" / String)
        .and(stores_filter.clone())
        .and(sources_filter.clone())
        .and_then(
            |mint: String, stores: Stores, sources: Vec<Arc<dyn TokenSource>>| async move {
                ensure_token_present(&stores, &mint).await;
                if let Err(e) = enrich_one(&stores, &sources, &mint).await {
                    log::warn!("on-demand enrich error mint={} err={}", mint, e);
                }
                match stores.hot.get(&mint) {
                    Some(rec) => Ok::<_, warp::Rejection>(warp::reply::json(&rec)),
                    None => Ok(warp::reply::json(&serde_json::json!({"error":"not_found"}))),
                }
            },
        );

    let stats_route =
        warp::path!("v1" / "stats")
            .and(stores_filter.clone())
            .map(|stores: Stores| {
                let count = stores.hot.count();
                warp::reply::json(&serde_json::json!({"count":count}))
            });

    let list_route = warp::path!("v1" / "tokens")
        .and(stores_filter.clone())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .map(
            |stores: Stores, params: std::collections::HashMap<String, String>| {
                let mut all = stores.hot.all();
                if let Some(sym) = params.get("symbol") {
                    all.retain(|t| t.symbol.as_deref() == Some(sym.as_str()));
                }
                // sort by symbol then mint for determinism
                all.sort_by(|a, b| a.symbol.cmp(&b.symbol).then(a.mint.cmp(&b.mint)));
                let limit: usize = params
                    .get("limit")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(100);
                let offset: usize = params
                    .get("offset")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let slice = if offset >= all.len() {
                    vec![]
                } else {
                    all.into_iter().skip(offset).take(limit.min(1000)).collect()
                };
                warp::reply::json(&serde_json::json!({
                    "data": slice,
                    "offset": offset,
                    "limit": limit.min(1000),
                    "total": stores.hot.count()
                }))
            },
        );

    let metrics_route = warp::path("metrics").and(warp::get()).map(|| {
        let mf = prometheus::gather();
        let enc = prometheus::TextEncoder::new();
        let mut buf = Vec::new();
        let _ = enc.encode(&mf, &mut buf);
        warp::reply::with_header(
            String::from_utf8_lossy(&buf).to_string(),
            "Content-Type",
            enc.format_type(),
        )
    });

    let health = warp::path("health")
        .map(|| warp::reply::json(&serde_json::json!({"status":"ok","ts":Utc::now()})));

    let routes = token_route
        .or(list_route)
        .or(stats_route)
        .or(metrics_route)
        .or(health);

    let addr: std::net::SocketAddr = bind_addr.parse().expect("invalid bind address");
    log::info!("token-registry-service listening on {}", addr);
    warp::serve(routes).run(addr).await;
    Ok(())
}
