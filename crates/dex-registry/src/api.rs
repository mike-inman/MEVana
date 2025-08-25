use anyhow::Result;
use log::info;
use prometheus::{Encoder, TextEncoder};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use warp::{Filter, Rejection, Reply};

use crate::cache::RegistryCache;
use crate::database::Database;
use crate::plugin::PluginManager;

/// HTTP API server for DEX registry
pub struct ApiServer {
    http_port: u16,
    metrics_port: u16,
    database: Arc<Database>,
    cache: Arc<RegistryCache>,
    plugin_manager: Arc<PluginManager>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DexLookupResponse {
    pub program_id: String,
    pub dex_name: String,
    pub protocol_type: String,
    pub is_active: bool,
    pub cached: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenLookupResponse {
    pub mint_address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub is_stable_coin: bool,
    pub verified: bool,
    pub cached: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryStatusResponse {
    pub status: String,
    pub database_stats: crate::database::RegistryStats,
    pub cache_stats: crate::cache::CacheStats,
    pub plugin_stats: crate::plugin::PluginStats,
}

impl ApiServer {
    pub fn new(
        http_port: u16,
        metrics_port: u16,
        database: Arc<Database>,
        cache: Arc<RegistryCache>,
        plugin_manager: Arc<PluginManager>,
    ) -> Self {
        Self {
            http_port,
            metrics_port,
            database,
            cache,
            plugin_manager,
        }
    }

    pub async fn start(&self) -> Result<()> {
        // Start metrics server
        let metrics_handle = self.start_metrics_server();

        // Start main API server
        let api_handle = self.start_api_server();

        // Run both servers concurrently
        tokio::try_join!(metrics_handle, api_handle)?;

        Ok(())
    }

    async fn start_metrics_server(&self) -> Result<()> {
        let metrics_route = warp::path("metrics")
            .and(warp::get())
            .map(|| {
                let encoder = TextEncoder::new();
                let metric_families = prometheus::gather();
                let mut buffer = Vec::new();
                
                match encoder.encode(&metric_families, &mut buffer) {
                    Ok(_) => warp::reply::with_status(
                        String::from_utf8_lossy(&buffer).to_string(),
                        warp::http::StatusCode::OK,
                    ),
                    Err(e) => warp::reply::with_status(
                        format!("Failed to encode metrics: {}", e),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ),
                }
            });

        let health_route = warp::path("health")
            .and(warp::get())
            .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

        let routes = metrics_route.or(health_route);
        let addr: SocketAddr = format!("0.0.0.0:{}", self.metrics_port).parse()?;

        info!("Starting metrics server on {}", addr);
        warp::serve(routes).run(addr).await;

        Ok(())
    }

    async fn start_api_server(&self) -> Result<()> {
        let database = Arc::clone(&self.database);
        let cache = Arc::clone(&self.cache);
        let plugin_manager = Arc::clone(&self.plugin_manager);

        // DEX lookup endpoint
        let dex_lookup = warp::path!("api" / "v1" / "dex" / String)
            .and(warp::get())
            .and(with_cache(Arc::clone(&cache)))
            .and(with_database(Arc::clone(&database)))
            .and_then(handle_dex_lookup);

        // Token lookup endpoint
        let token_lookup = warp::path!("api" / "v1" / "token" / String)
            .and(warp::get())
            .and(with_cache(Arc::clone(&cache)))
            .and(with_database(Arc::clone(&database)))
            .and_then(handle_token_lookup);

        // Registry status endpoint
        let status = warp::path!("api" / "v1" / "status")
            .and(warp::get())
            .and(with_database(Arc::clone(&database)))
            .and(with_cache(Arc::clone(&cache)))
            .and(with_plugin_manager(Arc::clone(&plugin_manager)))
            .and_then(handle_status);

        // List DEXs endpoint
        let list_dexs = warp::path!("api" / "v1" / "dexs")
            .and(warp::get())
            .and(with_database(Arc::clone(&database)))
            .and_then(handle_list_dexs);

        // List tokens endpoint
        let list_tokens = warp::path!("api" / "v1" / "tokens")
            .and(warp::get())
            .and(with_database(Arc::clone(&database)))
            .and_then(handle_list_tokens);

        // List plugins endpoint
        let list_plugins = warp::path!("api" / "v1" / "plugins")
            .and(warp::get())
            .and(with_plugin_manager(Arc::clone(&plugin_manager)))
            .and_then(handle_list_plugins);

        let api_routes = dex_lookup
            .or(token_lookup)
            .or(status)
            .or(list_dexs)
            .or(list_tokens)
            .or(list_plugins)
            .with(warp::cors().allow_any_origin().allow_headers(vec!["content-type"]).allow_methods(vec!["GET", "POST"]));

        let addr: SocketAddr = format!("0.0.0.0:{}", self.http_port).parse()?;

        info!("Starting API server on {}", addr);
        warp::serve(api_routes).run(addr).await;

        Ok(())
    }
}

// Helper functions for dependency injection
fn with_database(
    database: Arc<Database>,
) -> impl Filter<Extract = (Arc<Database>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&database))
}

fn with_cache(
    cache: Arc<RegistryCache>,
) -> impl Filter<Extract = (Arc<RegistryCache>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&cache))
}

fn with_plugin_manager(
    plugin_manager: Arc<PluginManager>,
) -> impl Filter<Extract = (Arc<PluginManager>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&plugin_manager))
}

// API handlers
async fn handle_dex_lookup(
    program_id: String,
    cache: Arc<RegistryCache>,
    database: Arc<Database>,
) -> Result<impl Reply, Rejection> {
    // Try cache first
    if let Some(cached_dex) = RegistryCache::get_dex_program_fast(&program_id) {
        let response = DexLookupResponse {
            program_id: cached_dex.program_id,
            dex_name: cached_dex.dex_name,
            protocol_type: cached_dex.protocol_type,
            is_active: cached_dex.is_active,
            cached: true,
        };
        return Ok(warp::reply::json(&response));
    }

    // Try database
    match database.get_dex_program(&program_id).await {
        Ok(Some(dex_program)) => {
            let response = DexLookupResponse {
                program_id: dex_program.program_id.clone(),
                dex_name: dex_program.dex_name.clone(),
                protocol_type: dex_program.protocol_type.clone(),
                is_active: dex_program.is_active,
                cached: false,
            };

            // Update cache asynchronously
            let cache_clone = Arc::clone(&cache);
            let dex_clone = dex_program.clone();
            tokio::spawn(async move {
                let _ = cache_clone.set_dex_program(&dex_clone).await;
            });

            Ok(warp::reply::json(&response))
        }
        Ok(None) => Ok(warp::reply::json(&serde_json::json!({
            "error": "DEX program not found",
            "program_id": program_id
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "error": format!("Database error: {}", e),
            "program_id": program_id
        }))),
    }
}

async fn handle_token_lookup(
    mint_address: String,
    cache: Arc<RegistryCache>,
    database: Arc<Database>,
) -> Result<impl Reply, Rejection> {
    // Try cache first
    if let Some(cached_token) = RegistryCache::get_token_fast(&mint_address) {
        let response = TokenLookupResponse {
            mint_address: cached_token.mint_address,
            symbol: cached_token.symbol,
            name: cached_token.name,
            decimals: cached_token.decimals,
            is_stable_coin: cached_token.is_stable_coin,
            verified: cached_token.verified,
            cached: true,
        };
        return Ok(warp::reply::json(&response));
    }

    // Try database
    match database.get_token(&mint_address).await {
        Ok(Some(token)) => {
            let response = TokenLookupResponse {
                mint_address: token.mint_address.clone(),
                symbol: token.symbol.clone(),
                name: token.name.clone(),
                decimals: token.decimals.map(|d| d as u8),
                is_stable_coin: token.is_stable_coin,
                verified: token.verified,
                cached: false,
            };

            // Update cache asynchronously
            let cache_clone = Arc::clone(&cache);
            let token_clone = token.clone();
            tokio::spawn(async move {
                let _ = cache_clone.set_token(&token_clone).await;
            });

            Ok(warp::reply::json(&response))
        }
        Ok(None) => Ok(warp::reply::json(&serde_json::json!({
            "error": "Token not found",
            "mint_address": mint_address
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "error": format!("Database error: {}", e),
            "mint_address": mint_address
        }))),
    }
}

async fn handle_status(
    database: Arc<Database>,
    cache: Arc<RegistryCache>,
    plugin_manager: Arc<PluginManager>,
) -> Result<impl Reply, Rejection> {
    let database_stats = database.get_registry_stats().await.unwrap_or_else(|_| {
        crate::database::RegistryStats {
            total_dex_programs: 0,
            active_dex_programs: 0,
            total_tokens: 0,
            verified_tokens: 0,
            cached_idls: 0,
            pending_discoveries: 0,
            last_updated: chrono::Utc::now(),
        }
    });

    let response = RegistryStatusResponse {
        status: "operational".to_string(),
        database_stats,
        cache_stats: cache.get_stats(),
        plugin_stats: plugin_manager.get_stats(),
    };

    Ok(warp::reply::json(&response))
}

async fn handle_list_dexs(database: Arc<Database>) -> Result<impl Reply, Rejection> {
    match database.list_dex_programs(true).await {
        Ok(dexs) => Ok(warp::reply::json(&dexs)),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "error": format!("Database error: {}", e)
        }))),
    }
}

async fn handle_list_tokens(database: Arc<Database>) -> Result<impl Reply, Rejection> {
    match database.list_tokens(true).await {
        Ok(tokens) => Ok(warp::reply::json(&tokens)),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "error": format!("Database error: {}", e)
        }))),
    }
}

async fn handle_list_plugins(plugin_manager: Arc<PluginManager>) -> Result<impl Reply, Rejection> {
    let plugins = plugin_manager.list_plugins();
    Ok(warp::reply::json(&plugins))
}
