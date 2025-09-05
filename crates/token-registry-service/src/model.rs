use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceAttribution {
    pub source_id: String,
    pub fetched_at: DateTime<Utc>,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVersionRow {
    pub mint: String,
    pub version_id: i64,
    pub recorded_at: DateTime<Utc>,
    pub hash: String,
    pub json_blob: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenRecord {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub logo_url: Option<String>,
    pub is_stable: bool,
    pub is_wrapped: bool,
    pub is_lp: bool,
    pub risk_flags: Vec<String>,
    pub tags: Vec<String>,
    pub first_seen: DateTime<Utc>,
    pub last_refreshed: DateTime<Utc>,
    pub last_trade_seen: Option<DateTime<Utc>>,
    pub liquidity_tier: Option<String>,
    pub active_pool_count: Option<u32>,
    pub trust_score: f64,
    pub sources: Vec<SourceAttribution>,
    pub token_hash: String,
}

impl TokenRecord {
    pub fn minimal(mint: &str) -> Self {
        let now = Utc::now();
        Self {
            mint: mint.to_string(),
            first_seen: now,
            last_refreshed: now,
            trust_score: 0.0,
            token_hash: String::new(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialToken {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub logo_url: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_stable: Option<bool>,
    pub is_wrapped: Option<bool>,
    pub is_lp: Option<bool>,
    pub risk_flags: Option<Vec<String>>,
    pub source_id: String,
    pub fetched_at: DateTime<Utc>,
    pub weight: f64,
}
