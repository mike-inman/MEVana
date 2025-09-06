use anyhow::Result;
use dashmap::DashMap;
use log::{debug, info, warn};
use once_cell::sync::Lazy;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use crate::database::{DexProgram, Token};

/// Hot-path optimized cache with Redis backing
pub struct RegistryCache {
    // In-memory caches (nanosecond lookups)
    dex_programs: Arc<DashMap<String, CachedDexProgram>>,
    tokens: Arc<DashMap<String, CachedToken>>,

    // Redis connection
    redis_client: Client,
    redis_conn: Option<ConnectionManager>,

    // Cache configuration
    ttl_seconds: u64,
    refresh_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDexProgram {
    pub program_id: String,
    pub dex_name: String,
    pub protocol_type: String,
    pub fee_bps: u16,
    pub liquidity_model: String,
    pub quirks: Vec<String>,
    pub is_active: bool,
    pub cached_at: u64, // timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToken {
    pub mint_address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub is_stable_coin: bool,
    pub is_wrapped: bool,
    pub verified: bool,
    pub cached_at: u64, // timestamp
}

// Global in-memory caches for hot path access
static DEX_CACHE: Lazy<DashMap<String, CachedDexProgram>> = Lazy::new(DashMap::new);
static TOKEN_CACHE: Lazy<DashMap<String, CachedToken>> = Lazy::new(DashMap::new);

impl RegistryCache {
    pub async fn new(redis_url: &str, ttl_seconds: u64) -> Result<Self> {
        let redis_client = Client::open(redis_url)?;

        let cache = Self {
            dex_programs: Arc::new(DashMap::new()),
            tokens: Arc::new(DashMap::new()),
            redis_client,
            redis_conn: None,
            ttl_seconds,
            refresh_interval: Duration::from_secs(60), // Refresh every minute
        };

        info!("Registry cache initialized with TTL: {}s", ttl_seconds);
        Ok(cache)
    }

    pub async fn connect(&mut self) -> Result<()> {
        let manager = self.redis_client.get_connection_manager().await?;
        self.redis_conn = Some(manager);
        info!("Connected to Redis cache");
        Ok(())
    }

    /// Hot path: Get DEX program info (nanosecond lookup)
    pub fn get_dex_program_fast(program_id: &str) -> Option<CachedDexProgram> {
        DEX_CACHE.get(program_id).map(|entry| entry.value().clone())
    }

    /// Hot path: Get token info (nanosecond lookup)
    pub fn get_token_fast(mint_address: &str) -> Option<CachedToken> {
        TOKEN_CACHE
            .get(mint_address)
            .map(|entry| entry.value().clone())
    }

    /// Warm path: Get DEX program with Redis fallback
    pub async fn get_dex_program(&self, program_id: &str) -> Result<Option<CachedDexProgram>> {
        // Try in-memory first
        if let Some(cached) = self.dex_programs.get(program_id) {
            if !self.is_expired(cached.cached_at) {
                return Ok(Some(cached.clone()));
            }
        }

        // Try Redis
        if let Some(conn) = &self.redis_conn {
            if let Ok(cached_json) = conn
                .clone()
                .get::<String, String>(format!("dex:{}", program_id))
                .await
            {
                if let Ok(cached_dex) = serde_json::from_str::<CachedDexProgram>(&cached_json) {
                    // Update in-memory cache
                    self.dex_programs
                        .insert(program_id.to_string(), cached_dex.clone());
                    DEX_CACHE.insert(program_id.to_string(), cached_dex.clone());
                    return Ok(Some(cached_dex));
                }
            }
        }

        Ok(None)
    }

    /// Warm path: Get token with Redis fallback
    pub async fn get_token(&self, mint_address: &str) -> Result<Option<CachedToken>> {
        // Try in-memory first
        if let Some(cached) = self.tokens.get(mint_address) {
            if !self.is_expired(cached.cached_at) {
                return Ok(Some(cached.clone()));
            }
        }

        // Try Redis
        if let Some(conn) = &self.redis_conn {
            if let Ok(cached_json) = conn
                .clone()
                .get::<String, String>(format!("token:{}", mint_address))
                .await
            {
                if let Ok(cached_token) = serde_json::from_str::<CachedToken>(&cached_json) {
                    // Update in-memory cache
                    self.tokens
                        .insert(mint_address.to_string(), cached_token.clone());
                    TOKEN_CACHE.insert(mint_address.to_string(), cached_token.clone());
                    return Ok(Some(cached_token));
                }
            }
        }

        Ok(None)
    }

    /// Update DEX program in all cache layers
    pub async fn set_dex_program(&self, program: &DexProgram) -> Result<()> {
        let cached = CachedDexProgram {
            program_id: program.program_id.clone(),
            dex_name: program.dex_name.clone(),
            protocol_type: program.protocol_type.clone(),
            fee_bps: self.extract_fee_bps(&program.fee_structure),
            liquidity_model: program.liquidity_model.clone(),
            quirks: self.parse_quirks(&program.quirks),
            is_active: program.is_active,
            cached_at: chrono::Utc::now().timestamp() as u64,
        };

        // Update in-memory cache
        self.dex_programs
            .insert(program.program_id.clone(), cached.clone());
        DEX_CACHE.insert(program.program_id.clone(), cached.clone());

        // Update Redis
        if let Some(mut conn) = self.redis_conn.clone() {
            let cached_json = serde_json::to_string(&cached)?;
            let _: Result<(), _> = conn
                .set_ex(
                    format!("dex:{}", program.program_id),
                    cached_json,
                    self.ttl_seconds,
                )
                .await;
        }

        debug!(
            "Cached DEX program: {} ({})",
            program.dex_name, program.program_id
        );
        Ok(())
    }

    /// Update token in all cache layers
    pub async fn set_token(&self, token: &Token) -> Result<()> {
        let cached = CachedToken {
            mint_address: token.mint_address.clone(),
            symbol: token.symbol.clone(),
            name: token.name.clone(),
            decimals: token.decimals.map(|d| d as u8),
            is_stable_coin: token.is_stable_coin,
            is_wrapped: token.is_wrapped,
            verified: token.verified,
            cached_at: chrono::Utc::now().timestamp() as u64,
        };

        // Update in-memory cache
        self.tokens
            .insert(token.mint_address.clone(), cached.clone());
        TOKEN_CACHE.insert(token.mint_address.clone(), cached.clone());

        // Update Redis
        if let Some(mut conn) = self.redis_conn.clone() {
            let cached_json = serde_json::to_string(&cached)?;
            let _: Result<(), _> = conn
                .set_ex(
                    format!("token:{}", token.mint_address),
                    cached_json,
                    self.ttl_seconds,
                )
                .await;
        }

        debug!("Cached token: {:?} ({})", token.symbol, token.mint_address);
        Ok(())
    }

    /// Invalidate cache entry
    pub async fn invalidate_dex_program(&self, program_id: &str) -> Result<()> {
        self.dex_programs.remove(program_id);
        DEX_CACHE.remove(program_id);

        if let Some(mut conn) = self.redis_conn.clone() {
            let _: Result<(), _> = conn.del(format!("dex:{}", program_id)).await;
        }

        debug!("Invalidated DEX program cache: {}", program_id);
        Ok(())
    }

    /// Invalidate token cache entry
    pub async fn invalidate_token(&self, mint_address: &str) -> Result<()> {
        self.tokens.remove(mint_address);
        TOKEN_CACHE.remove(mint_address);

        if let Some(mut conn) = self.redis_conn.clone() {
            let _: Result<(), _> = conn.del(format!("token:{}", mint_address)).await;
        }

        debug!("Invalidated token cache: {}", mint_address);
        Ok(())
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            dex_programs_cached: self.dex_programs.len(),
            tokens_cached: self.tokens.len(),
            global_dex_cache_size: DEX_CACHE.len(),
            global_token_cache_size: TOKEN_CACHE.len(),
            ttl_seconds: self.ttl_seconds,
        }
    }

    /// Background task: Refresh stale cache entries
    pub async fn start_refresh_task(&self) -> Result<()> {
        let refresh_interval = self.refresh_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);

            loop {
                interval.tick().await;

                // Clean up expired entries from global caches
                let now = chrono::Utc::now().timestamp() as u64;
                let ttl = 3600; // 1 hour for global cache

                DEX_CACHE.retain(|_, cached| now - cached.cached_at < ttl);

                TOKEN_CACHE.retain(|_, cached| now - cached.cached_at < ttl);

                debug!("Cache refresh completed");
            }
        });

        info!("Cache refresh task started");
        Ok(())
    }

    // Helper methods

    fn is_expired(&self, cached_at: u64) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        now - cached_at > self.ttl_seconds
    }

    fn extract_fee_bps(&self, fee_structure: &Option<String>) -> u16 {
        fee_structure
            .as_ref()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
            .and_then(|v| v["swap_fee_bps"].as_u64())
            .map(|f| f as u16)
            .unwrap_or(30) // Default 0.3%
    }

    fn parse_quirks(&self, quirks: &Option<String>) -> Vec<String> {
        quirks
            .as_ref()
            .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub dex_programs_cached: usize,
    pub tokens_cached: usize,
    pub global_dex_cache_size: usize,
    pub global_token_cache_size: usize,
    pub ttl_seconds: u64,
}

/// Global hot-path functions for maximum performance
impl RegistryCache {
    /// Check if program ID is a known DEX (hot path optimized)
    pub fn is_known_dex(program_id: &str) -> bool {
        DEX_CACHE.contains_key(program_id)
    }

    /// Check if mint address is a known token (hot path optimized)
    pub fn is_known_token(mint_address: &str) -> bool {
        TOKEN_CACHE.contains_key(mint_address)
    }

    /// Get DEX name for program ID (hot path optimized)
    pub fn get_dex_name(program_id: &str) -> Option<String> {
        DEX_CACHE
            .get(program_id)
            .map(|entry| entry.dex_name.clone())
    }

    /// Get token symbol for mint address (hot path optimized)
    pub fn get_token_symbol(mint_address: &str) -> Option<String> {
        TOKEN_CACHE
            .get(mint_address)
            .and_then(|entry| entry.symbol.clone())
    }
}
