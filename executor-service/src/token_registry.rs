use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub logo_url: Option<String>,
    pub tags: Vec<String>,
    pub first_seen: DateTime<Utc>,
    pub last_refreshed: DateTime<Utc>,
    pub needs_refresh: bool,
    pub sources: Vec<MetaSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSource {
    pub source: String,
    pub fetched_at: DateTime<Utc>,
    pub fields: Vec<String>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct TokenRegistryInner {
    pub by_mint: HashMap<String, TokenMetadata>,
    pub unknown_queue: Vec<String>,
}

#[derive(Clone, Default)]
pub struct TokenRegistry {
    inner: Arc<RwLock<TokenRegistryInner>>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, mint: &str) -> Option<TokenMetadata> {
        self.inner.read().unwrap().by_mint.get(mint).cloned()
    }

    pub fn note_seen_mint(&self, mint: &str) {
        let mut w = self.inner.write().unwrap();
        if !w.by_mint.contains_key(mint) {
            let now = Utc::now();
            w.by_mint.insert(
                mint.to_string(),
                TokenMetadata {
                    mint: mint.to_string(),
                    first_seen: now,
                    last_refreshed: now,
                    needs_refresh: true,
                    ..Default::default()
                },
            );
            w.unknown_queue.push(mint.to_string());
        }
    }

    // Return a snapshot of all token metadata (cheap clone for administration / persistence)
    pub fn all_metadata(&self) -> Vec<TokenMetadata> {
        self.inner
            .read()
            .unwrap()
            .by_mint
            .values()
            .cloned()
            .collect()
    }

    pub fn dequeue_unknown(&self) -> Option<String> {
        let mut w = self.inner.write().unwrap();
        if w.unknown_queue.is_empty() {
            return None;
        }
        Some(w.unknown_queue.remove(0))
    }

    pub fn upsert(&self, meta: TokenMetadata) {
        let mut w = self.inner.write().unwrap();
        w.by_mint.insert(meta.mint.clone(), meta);
    }

    pub fn load_snapshot_file(&self, path: &str) -> anyhow::Result<usize> {
        if !Path::new(path).exists() {
            return Ok(0);
        }
        let raw = fs::read_to_string(path)?;
        match serde_json::from_str::<TokenRegistryInner>(&raw) {
            Ok(inner) => {
                let count = inner.by_mint.len();
                *self.inner.write().unwrap() = inner;
                Ok(count)
            }
            Err(e) => {
                warn!(
                    "token_registry_snapshot_parse_error path={} err={}",
                    path, e
                );
                Ok(0)
            }
        }
    }

    pub fn write_snapshot_file(&self, path: &str) -> anyhow::Result<()> {
        let inner = self.inner.read().unwrap();
        let json = serde_json::to_string_pretty(&*inner)?;
        fs::write(path, json)?;
        Ok(())
    }
}

// Placeholder enrichment trait – implementations will call external APIs (Shyft / Helius) later.
#[allow(dead_code)]
#[async_trait::async_trait]
pub trait TokenEnricher: Send + Sync {
    async fn enrich(&self, mint: &str) -> Option<TokenMetadata>;
}

pub struct NoopEnricher;

#[async_trait]
impl TokenEnricher for NoopEnricher {
    async fn enrich(&self, mint: &str) -> Option<TokenMetadata> {
        let now = Utc::now();
        Some(TokenMetadata {
            mint: mint.to_string(),
            symbol: None,
            name: None,
            decimals: None,
            logo_url: None,
            tags: Vec::new(),
            first_seen: now,
            last_refreshed: now,
            needs_refresh: false,
            sources: Vec::new(),
        })
    }
}

impl TokenRegistry {
    pub async fn start_background_enrichment<E: TokenEnricher + 'static>(&self, enricher: E) {
        let reg = self.clone();
        // Optional snapshot path
        let snapshot_path = std::env::var("TOKEN_REGISTRY_SNAPSHOT_PATH")
            .unwrap_or_else(|_| "token_registry_snapshot.json".into());
        // Load existing snapshot (non-fatal)
        match reg.load_snapshot_file(&snapshot_path) {
            Ok(count) => {
                if count > 0 {
                    info!(
                        "token_registry_snapshot_loaded count={} path={} ",
                        count, snapshot_path
                    );
                }
            }
            Err(e) => warn!(
                "token_registry_snapshot_load_error path={} err={}",
                snapshot_path, e
            ),
        }
        tokio::spawn(async move {
            let mut last_snapshot = std::time::Instant::now();
            let ttl_secs: u64 = std::env::var("TOKEN_META_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600);
            let refresh_batch_limit: usize = std::env::var("TOKEN_META_REFRESH_BATCH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(25);
            loop {
                // Requeue stale entries
                {
                    let now = Utc::now();
                    let mut stale: Vec<String> = Vec::new();
                    if let Ok(inner) = reg.inner.read() {
                        for (mint, meta) in inner.by_mint.iter() {
                            if (now - meta.last_refreshed).num_seconds() as u64 >= ttl_secs {
                                stale.push(mint.clone());
                                if stale.len() >= refresh_batch_limit {
                                    break;
                                }
                            }
                        }
                    }
                    if !stale.is_empty() {
                        let mut w = reg.inner.write().unwrap();
                        for m in stale {
                            if !w.unknown_queue.contains(&m) {
                                w.unknown_queue.push(m);
                            }
                        }
                    }
                }
                if let Some(mint) = reg.dequeue_unknown() {
                    if let Some(mut meta) = enricher.enrich(&mint).await {
                        meta.last_refreshed = Utc::now();
                        reg.upsert(meta);
                        info!("token_enriched mint={}", mint);
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
                // Periodic snapshot (every 60s)
                if last_snapshot.elapsed().as_secs() >= 60 {
                    if let Err(e) = reg.write_snapshot_file(&snapshot_path) {
                        warn!(
                            "token_registry_snapshot_write_error path={} err={}",
                            snapshot_path, e
                        );
                    } else {
                        info!("token_registry_snapshot_written path={}", snapshot_path);
                    }
                    last_snapshot = std::time::Instant::now();
                }
            }
        });
    }
}

// Example Shyft-based enricher (simplified / placeholder). It attempts to fetch metadata from an assumed REST endpoint.
pub struct ShyftEnricher {
    pub base_url: String,
    pub api_key: String,
    pub client: reqwest::Client,
}

impl ShyftEnricher {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl TokenEnricher for ShyftEnricher {
    async fn enrich(&self, mint: &str) -> Option<TokenMetadata> {
        let url = format!("{}/token/{}", self.base_url.trim_end_matches('/'), mint);
        match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    return None;
                }
                match resp.json::<serde_json::Value>().await {
                    Ok(v) => {
                        let now = Utc::now();
                        Some(TokenMetadata {
                            mint: mint.to_string(),
                            symbol: v["symbol"].as_str().map(|s| s.to_string()),
                            name: v["name"].as_str().map(|s| s.to_string()),
                            decimals: v["decimals"].as_u64().map(|d| d as u8),
                            logo_url: v["logoURI"].as_str().map(|s| s.to_string()),
                            tags: v["tags"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            first_seen: now,
                            last_refreshed: now,
                            needs_refresh: false,
                            sources: vec![MetaSource {
                                source: "shyft".into(),
                                fetched_at: now,
                                fields: vec![
                                    "symbol".into(),
                                    "name".into(),
                                    "decimals".into(),
                                    "logoURI".into(),
                                ],
                            }],
                        })
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }
}
