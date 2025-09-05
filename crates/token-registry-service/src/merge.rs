use crate::model::{PartialToken, SourceAttribution, TokenRecord};
use blake3::Hasher;
use chrono::Utc;

pub struct MergeResult {
    pub record: TokenRecord,
    pub changed: bool,
    pub conflicts: Vec<String>,
}

pub fn merge(mut base: Option<TokenRecord>, parts: Vec<PartialToken>) -> MergeResult {
    let mut record = base.take().unwrap_or_else(|| {
        TokenRecord::minimal(parts.get(0).map(|p| p.mint.as_str()).unwrap_or(""))
    });
    let before_hash = record.token_hash.clone();
    let mut conflicts = Vec::new();

    // Helper macro to merge simple Option fields
    macro_rules! merge_field {
        ($field:ident) => {{
            let mut winner: Option<(String, f64)> = None; // (value_json, score)
            for p in &parts {
                if let Some(val) = p.$field.clone() {
                    let score = p.weight * freshness_decay(p.fetched_at);
                    let encoded = serde_json::to_string(&val).unwrap();
                    match &mut winner {
                        Some((cur_val, cur_score)) => {
                            if encoded != *cur_val {
                                if score > *cur_score + f64::EPSILON {
                                    *cur_val = encoded;
                                    *cur_score = score;
                                } else if (score - *cur_score).abs() <= f64::EPSILON {
                                    conflicts.push(stringify!($field).to_string());
                                }
                            }
                        }
                        None => winner = Some((encoded, score)),
                    }
                }
            }
            if let Some((val_json, _)) = &winner {
                record.$field = serde_json::from_str(val_json).ok();
            }
        }};
    }

    merge_field!(symbol);
    merge_field!(name);
    merge_field!(decimals);
    merge_field!(logo_url);

    // Boolean-ish or vector fields
    if record.is_stable == false {
        // only upgrade to true
        for p in &parts {
            if p.is_stable == Some(true) {
                record.is_stable = true;
            }
        }
    }
    for p in &parts {
        if let Some(flags) = &p.risk_flags {
            if !flags.is_empty() {
                record.risk_flags.extend(flags.clone());
            }
        }
    }
    for p in &parts {
        if let Some(tags) = &p.tags {
            if !tags.is_empty() {
                record.tags.extend(tags.clone());
            }
        }
    }
    record.tags.sort();
    record.tags.dedup();
    record.risk_flags.sort();
    record.risk_flags.dedup();

    // Trust score simple average of weights that contributed any field
    if !parts.is_empty() {
        let mut contributing = 0.0;
        let mut total = 0.0;
        for p in &parts {
            total += p.weight;
            if contributed(p) {
                contributing += p.weight;
            }
        }
        record.trust_score = if total > 0.0 {
            contributing / total
        } else {
            0.0
        };
    }

    // Sources attribution (append & truncate to reasonable size)
    for p in &parts {
        record.sources.push(SourceAttribution {
            source_id: p.source_id.clone(),
            fetched_at: p.fetched_at,
            fields: collected_fields(&p),
        });
    }
    if record.sources.len() > 20 {
        record.sources = record.sources.split_off(record.sources.len() - 20);
    }

    record.last_refreshed = Utc::now();
    record.token_hash = compute_hash(&record);
    let changed = record.token_hash != before_hash;
    MergeResult {
        record,
        changed,
        conflicts,
    }
}

fn freshness_decay(fetched: chrono::DateTime<Utc>) -> f64 {
    let age_secs = (Utc::now() - fetched).num_seconds().max(0) as f64;
    let half_life = 3600.0; // 1h half-life
    (-(age_secs / half_life) * std::f64::consts::LN_2).exp()
}

fn contributed(p: &PartialToken) -> bool {
    p.symbol.is_some() || p.name.is_some() || p.decimals.is_some() || p.logo_url.is_some()
}

fn collected_fields(p: &PartialToken) -> Vec<String> {
    let mut v = Vec::new();
    if p.symbol.is_some() {
        v.push("symbol".into());
    }
    if p.name.is_some() {
        v.push("name".into());
    }
    if p.decimals.is_some() {
        v.push("decimals".into());
    }
    if p.logo_url.is_some() {
        v.push("logo_url".into());
    }
    v
}

pub fn compute_hash(rec: &TokenRecord) -> String {
    let mut h = Hasher::new();
    // Only hash stable canonical subset
    h.update(rec.mint.as_bytes());
    if let Some(s) = &rec.symbol {
        h.update(s.as_bytes());
    }
    if let Some(n) = &rec.name {
        h.update(n.as_bytes());
    }
    if let Some(d) = &rec.decimals {
        h.update(&[*d]);
    }
    if let Some(l) = &rec.logo_url {
        h.update(l.as_bytes());
    }
    h.update(&[rec.is_stable as u8]);
    h.finalize().to_hex().to_string()
}
