use crate::metrics::*;
use crate::model::PartialToken;
use crate::{merge::merge, model::TokenRecord, sources::TokenSource, storage::Stores};
use anyhow::Result;
use chrono::Utc;
use std::{sync::Arc, time::Duration};

pub struct EnrichmentConfig {
    pub ttl_secs: u64,
    pub concurrency: usize,
}

pub async fn run_enrichment_loop(
    stores: Stores,
    sources: Vec<Arc<dyn TokenSource>>,
    cfg: EnrichmentConfig,
) {
    let interval = Duration::from_secs(10);
    loop {
        let now = Utc::now();
        // Collect candidates needing refresh
        let candidates: Vec<String> = stores
            .hot
            .all()
            .into_iter()
            .filter(|t| (now - t.last_refreshed).num_seconds() as u64 >= cfg.ttl_secs)
            .map(|t| t.mint)
            .take(100)
            .collect();
        for mint in candidates {
            let _ = enrich_one(&stores, &sources, &mint).await;
        }
        tokio::time::sleep(interval).await;
    }
}

pub async fn enrich_one(
    stores: &Stores,
    sources: &Vec<Arc<dyn TokenSource>>,
    mint: &str,
) -> Result<()> {
    let mut partials: Vec<PartialToken> = Vec::new();
    for s in sources {
        let start = std::time::Instant::now();
        TOKEN_ENRICH_REQUESTS.with_label_values(&[s.id()]).inc();
        match tokio::time::timeout(std::time::Duration::from_secs(5), s.fetch(mint)).await {
            Ok(Ok(opt)) => {
                if let Some(pt) = opt {
                    partials.push(pt);
                }
            }
            Ok(Err(e)) => {
                TOKEN_ENRICH_FAILURES
                    .with_label_values(&[s.id(), "error"])
                    .inc();
                log::warn!(
                    "enrich source error mint={} source={} err={}",
                    mint,
                    s.id(),
                    e
                );
            }
            Err(_) => {
                TOKEN_ENRICH_FAILURES
                    .with_label_values(&[s.id(), "timeout"])
                    .inc();
            }
        }
        TOKEN_ENRICH_LATENCY
            .with_label_values(&[s.id()])
            .observe(start.elapsed().as_secs_f64());
    }
    if partials.is_empty() {
        return Ok(());
    }
    let existing = stores.hot.get(mint);
    let result = merge(existing, partials);
    for c in &result.conflicts {
        TOKEN_CONFLICTS.with_label_values(&[c]).inc();
    }
    if result.changed {
        stores.hot.upsert(result.record.clone());
        TOKEN_TOTAL.set(stores.hot.count() as i64);
        let _ = stores.cold.write_current(&result.record).await;
        let _ = stores.cold.append_version(&result.record).await;
    }
    Ok(())
}

pub async fn ensure_token_present(stores: &Stores, mint: &str) {
    if stores.hot.get(mint).is_none() {
        let rec = TokenRecord::minimal(mint);
        stores.hot.upsert(rec);
        TOKEN_TOTAL.set(stores.hot.count() as i64);
    }
}
