use crate::edge_integration::edge_threshold_snapshot;
use crate::models::ArbSignal;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct TokenRecord {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    rec: TokenRecord,
    fetched: Instant,
}

static CACHE: Lazy<DashMap<String, CacheEntry>> = Lazy::new(DashMap::new);
static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

fn base_url() -> String {
    std::env::var("TOKEN_REGISTRY_BASE").unwrap_or_else(|_| "http://127.0.0.1:8085".into())
}

/// Fetch token metadata, cached for ttl_secs (default 30 min). Non-blocking to hotpath: network done in background if stale.
pub async fn get_token(mint: &str) -> Option<TokenRecord> {
    let ttl = std::env::var("TOKEN_META_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);
    let ttl = Duration::from_secs(ttl);
    if let Some(entry) = CACHE.get(mint) {
        if entry.fetched.elapsed() < ttl {
            return Some(entry.rec.clone());
        }
    }
    // Spawn background refresh so caller isn't delayed
    let mint_s = mint.to_string();
    tokio::spawn(async move {
        if let Ok(resp) = CLIENT
            .get(format!("{}/v1/token/{}", base_url(), mint_s))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(rec) = resp.json::<TokenRecord>().await {
                    CACHE.insert(
                        mint_s.clone(),
                        CacheEntry {
                            rec: rec.clone(),
                            fetched: Instant::now(),
                        },
                    );
                    metrics::counter!("token_meta_fetch_success_total").increment(1);
                }
            } else {
                let sc = resp.status();
                let sc_s = sc.as_str().to_string();
                metrics::counter!("token_meta_fetch_error_total", "status" => sc_s).increment(1);
            }
        }
    });
    CACHE.get(mint).map(|e| e.rec.clone())
}

/// Fire-and-forget background priming for a list of mints; increments counters for misses.
pub fn prime_tokens<'a, I: IntoIterator<Item = &'a str>>(mints: I) {
    for m in mints {
        if !CACHE.contains_key(m) {
            metrics::counter!("token_meta_prime_miss_total").increment(1);
            let m_owned = m.to_string();
            tokio::spawn(async move {
                let _ = get_token(&m_owned).await;
            });
        } else {
            metrics::counter!("token_meta_prime_hit_total").increment(1);
        }
    }
}

/// Force a synchronous (awaited) fetch from the registry if not cached or stale; returns fresh record if successful.
pub async fn fetch_token_now(mint: &str) -> Option<TokenRecord> {
    let url = format!("{}/v1/token/{}", base_url(), mint);
    if let Ok(resp) = CLIENT.get(url).send().await {
        if resp.status().is_success() {
            if let Ok(rec) = resp.json::<TokenRecord>().await {
                CACHE.insert(
                    mint.to_string(),
                    CacheEntry {
                        rec: rec.clone(),
                        fetched: Instant::now(),
                    },
                );
                metrics::counter!("token_meta_fetch_success_total", "force" => "1").increment(1);
                return Some(rec);
            }
        } else {
            let scs = resp.status().as_str().to_string();
            metrics::counter!(
                "token_meta_fetch_error_total",
                "status" => scs,
                "force" => "1"
            )
            .increment(1);
        }
    } else {
        metrics::counter!(
            "token_meta_fetch_error_total",
            "status" => "network",
            "force" => "1"
        )
        .increment(1);
    }
    None
}
/// Snapshot decimals if known (no network); None if unknown or record absent.
pub fn decimals_snapshot(mint: &str) -> Option<u8> {
    CACHE.get(mint).and_then(|c| c.rec.decimals)
}

/// Snapshot symbol if known
pub fn symbol_snapshot(mint: &str) -> Option<String> {
    CACHE.get(mint).and_then(|c| c.rec.symbol.clone())
}

/// Snapshot logo URL if known
pub fn logo_snapshot(mint: &str) -> Option<String> {
    CACHE.get(mint).and_then(|c| c.rec.logo_url.clone())
}

/// Build enrichment JSON object for a signal using only locally cached metadata (non-blocking)
pub fn build_enrichment(sig: &ArbSignal) -> serde_json::Value {
    let a = sig.pair.a.as_str();
    let b = sig.pair.b.as_str();
    let dec_a = decimals_snapshot(a);
    let dec_b = decimals_snapshot(b);
    let sym_a = symbol_snapshot(a);
    let sym_b = symbol_snapshot(b);
    let pair_symbol = match (&sym_a, &sym_b) {
        (Some(sa), Some(sb)) => Some(format!("{}/{}", sa, sb)),
        _ => None,
    };

    // UI conversions
    let (amount_in_ui, expected_profit_ui) = if let Some(d) = dec_a {
        let denom = 10u128.saturating_pow(d as u32) as f64;
        if denom > 0.0 {
            (
                (sig.amount_in as f64) / denom,
                (sig.expected_profit_out as f64) / denom,
            )
        } else {
            (0.0, 0.0)
        }
    } else {
        (0.0, 0.0)
    };

    // Threshold snapshot
    let (min_profit_bps, max_slippage_bps) = edge_threshold_snapshot();

    // Route kind & hop count
    let route_kind = if sig.mid_pool_id.is_some() {
        "3leg"
    } else {
        "2leg"
    };

    // Latency (detection to now) if emitted_at_ns present
    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let detection_latency_ns = if sig.emitted_at_ns > 0 && now_ns > sig.emitted_at_ns {
        Some(now_ns - sig.emitted_at_ns)
    } else {
        None
    };

    // Stablecoin classification (env var STABLECOIN_MINTS comma separated)
    static STABLES: once_cell::sync::OnceCell<std::collections::HashSet<String>> =
        once_cell::sync::OnceCell::new();
    let stables = STABLES.get_or_init(|| {
        std::env::var("STABLECOIN_MINTS")
            .ok()
            .map(|s| {
                s.split(',')
                    .filter(|x| !x.is_empty())
                    .map(|v| v.trim().to_string())
                    .collect::<std::collections::HashSet<String>>()
            })
            .unwrap_or_default()
    });
    let is_stable_a = stables.contains(a);
    let is_stable_b = stables.contains(b);

    json!({
        "decimals_a": dec_a,
        "decimals_b": dec_b,
        "symbol_a": sym_a,
        "symbol_b": sym_b,
        "pair_symbol": pair_symbol,
        "decimals_known": dec_a.is_some() && dec_b.is_some(),
        "amount_in_ui": amount_in_ui,
        "expected_profit_ui": expected_profit_ui,
        "min_profit_bps": min_profit_bps,
        "max_slippage_bps": max_slippage_bps,
        "route_kind": route_kind,
        "detection_latency_ns": detection_latency_ns,
        "is_stable_a": is_stable_a,
        "is_stable_b": is_stable_b,
    })
}

/// Enrich JSON signal object (serde_json::Value) with token metadata if present
pub fn enrich_signal_json(mut v: serde_json::Value) -> serde_json::Value {
    use serde_json::json;
    let pair_sym = if let Some(obj) = v.get("pair") {
        let a = obj.get("a").and_then(|x| x.as_str()).unwrap_or("");
        let b = obj.get("b").and_then(|x| x.as_str()).unwrap_or("");
        let ta = CACHE.get(a).and_then(|c| c.rec.symbol.clone());
        let tb = CACHE.get(b).and_then(|c| c.rec.symbol.clone());
        match (ta, tb) {
            (Some(sa), Some(sb)) => Some(format!("{}/{}", sa, sb)),
            _ => None,
        }
    } else {
        None
    };
    if let Some(ps) = pair_sym {
        if let Some(obj) = v.as_object_mut() {
            obj.insert("pair_symbol".into(), json!(ps));
        }
    }
    // Add UI normalization if decimals known
    if let Some(obj) = v.as_object_mut() {
        if let (Some(pair), Some(amount_in), Some(profit)) = (
            obj.get("pair"),
            obj.get("amount_in"),
            obj.get("expected_profit_out"),
        ) {
            if let (Some(a_mint), Some(amount_in_u128), Some(profit_i128)) = (
                pair.get("a").and_then(|x| x.as_str()),
                amount_in.as_u64().map(|x| x as u128), // u128 serialized may truncate if >u64
                profit.as_i64().map(|x| x as i128),
            ) {
                if let Some(dec) = decimals_snapshot(a_mint) {
                    let denom = 10u128.saturating_pow(dec as u32);
                    if denom > 0 {
                        let amount_ui = (amount_in_u128 as f64) / (denom as f64);
                        let profit_ui = (profit_i128 as f64) / (denom as f64);
                        obj.insert("amount_in_ui".into(), json!(amount_ui));
                        obj.insert("expected_profit_ui".into(), json!(profit_ui));
                    }
                }
            }
        }
    }
    v
}
