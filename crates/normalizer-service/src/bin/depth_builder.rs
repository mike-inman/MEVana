use anyhow::{Context, Result};
use dotenv::dotenv;
use env_logger;
use log::{info, warn};
use redis::{AsyncCommands, Value};
use serde_json::json;
use std::collections::HashMap;

// NEW: Configuration structure with validation
#[derive(Debug, Clone)]
struct ServiceConfig {
    redis_url: String,
    start_id: String,
}

impl ServiceConfig {
    fn from_env() -> Result<Self> {
        // Load environment variables with validation
        let redis_url = std::env::var("REDIS_URL")
            .map_err(|_| anyhow::anyhow!("REDIS_URL environment variable is required"))?;

        let start_id = std::env::var("START_ID").unwrap_or_else(|_| "$".to_string());

        Ok(Self {
            redis_url,
            start_id,
        })
    }

    fn log_config(&self) {
        info!("=== DEPTH BUILDER SERVICE CONFIGURATION ===");
        info!("Redis URL: {}", self.redis_url);
        info!("Start ID: {}", self.start_id);
        info!("==========================================");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    // Load and validate configuration
    let config = ServiceConfig::from_env().context("Failed to load service configuration")?;

    config.log_config();

    // Validate Redis connection
    let client =
        redis::Client::open(config.redis_url.clone()).context("Failed to create Redis client")?;

    let mut conn = client
        .get_multiplexed_tokio_connection()
        .await
        .context("Failed to connect to Redis")?;

    info!("depth-builder: Connected to Redis successfully");

    // Simple loop: read from XREAD on tickarray stream; compute/update per-pool summaries
    // Stream: dex:whirlpool:tickarray_updates
    let mut last_id = config.start_id.clone();

    info!("Starting depth builder loop with start_id: {}", last_id);

    loop {
        let resp: Value = match redis::cmd("XREAD")
            .arg("BLOCK")
            .arg(5000)
            .arg("STREAMS")
            .arg("dex:whirlpool:tickarray_updates")
            .arg(&last_id)
            .query_async(&mut conn)
            .await
        {
            Ok(value) => value,
            Err(e) => {
                warn!("Redis XREAD error: {}", e);
                Value::Nil
            }
        };
        // Expect structure: [[stream_key, [[id, [field, value, ...]], ...]]]
        if let Value::Array(streams) = resp {
            if let Some(Value::Array(mut top)) = streams.into_iter().next() {
                if top.len() >= 2 {
                    if let Value::Array(items) = top.remove(1) {
                        for item in items {
                            if let Value::Array(mut pair) = item {
                                if pair.len() != 2 {
                                    continue;
                                }
                                let id = match &pair[0] {
                                    Value::BulkString(b) => String::from_utf8_lossy(b).to_string(),
                                    _ => continue,
                                };
                                last_id = id;
                                let mut pool = String::new();
                                let mut start: i64 = 0;
                                let mut num_init: i64 = 0;
                                let mut sum_gross = String::new();
                                if let Value::Array(kvs) = &pair[1] {
                                    let mut it = kvs.iter();
                                    while let (Some(k), Some(v)) = (it.next(), it.next()) {
                                        let key = match k {
                                            Value::BulkString(b) => {
                                                String::from_utf8_lossy(b).to_string()
                                            }
                                            _ => continue,
                                        };
                                        match key.as_str() {
                                            "pool" => {
                                                if let Value::BulkString(b) = v {
                                                    pool = String::from_utf8_lossy(b).to_string()
                                                }
                                            }
                                            "start" => {
                                                if let Value::BulkString(b) = v {
                                                    start = String::from_utf8_lossy(b)
                                                        .parse()
                                                        .unwrap_or(0)
                                                }
                                            }
                                            "num_init" => {
                                                if let Value::BulkString(b) = v {
                                                    num_init = String::from_utf8_lossy(b)
                                                        .parse()
                                                        .unwrap_or(0)
                                                }
                                            }
                                            "sum_gross" => {
                                                if let Value::BulkString(b) = v {
                                                    sum_gross =
                                                        String::from_utf8_lossy(b).to_string()
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                if pool.is_empty() {
                                    continue;
                                }
                                let key = format!("dex:pool_depth:{}", pool);
                                let _: () = redis::cmd("HSET")
                                    .arg(&key)
                                    .arg("last_start")
                                    .arg(start)
                                    .arg("last_num_init")
                                    .arg(num_init)
                                    .arg("last_sum_gross")
                                    .arg(&sum_gross)
                                    .query_async(&mut conn)
                                    .await
                                    .unwrap_or(());

                                // Read current pool snapshot for sqrt_price_x64 and decimals
                                let pool_meta_key = format!("dex:pool:{}", pool);
                                let snap_val: Value = redis::cmd("HGETALL")
                                    .arg(&pool_meta_key)
                                    .query_async(&mut conn)
                                    .await
                                    .unwrap_or(Value::Nil);
                                let mut meta: HashMap<String, String> = HashMap::new();
                                if let Value::Array(kvs) = snap_val.clone() {
                                    let mut it = kvs.into_iter();
                                    while let (Some(k), Some(v)) = (it.next(), it.next()) {
                                        if let (Value::BulkString(kb), Value::BulkString(vb)) =
                                            (k, v)
                                        {
                                            meta.insert(
                                                String::from_utf8_lossy(&kb).to_string(),
                                                String::from_utf8_lossy(&vb).to_string(),
                                            );
                                        }
                                    }
                                }

                                let sqrt_price_x64_opt = meta
                                    .get("sqrt_price_x64")
                                    .and_then(|s| s.parse::<u128>().ok());
                                let dec_a_opt =
                                    meta.get("decimals_a").and_then(|s| s.parse::<i32>().ok());
                                let dec_b_opt =
                                    meta.get("decimals_b").and_then(|s| s.parse::<i32>().ok());

                                // First-cut mid price in UI units for Whirlpool
                                let p0_ui: Option<f64> =
                                    match (sqrt_price_x64_opt, dec_a_opt, dec_b_opt) {
                                        (Some(sp), Some(da), Some(db)) => {
                                            const Q64: f64 = 18446744073709551616.0; // 2^64
                                            let sqrt = (sp as f64) / Q64;
                                            let price = (sqrt * sqrt) * 10f64.powi(db - da);
                                            Some(price)
                                        }
                                        _ => None,
                                    };

                                let sum_gross_f: f64 = sum_gross.parse::<f64>().unwrap_or(0.0);
                                // Heuristic impact coefficient (smaller when liquidity larger)
                                let k_coeff: f64 = if sum_gross_f > 0.0 {
                                    1.0 / (sum_gross_f * 1000.0)
                                } else {
                                    0.0
                                };
                                let sample_sizes = [0.1_f64, 1.0, 5.0, 10.0];
                                let quotes = if let Some(p0) = p0_ui {
                                    let q: Vec<_> = sample_sizes
                                        .iter()
                                        .map(|s| {
                                            let px = (p0 * (1.0 - k_coeff * *s)).max(0.0);
                                            json!({"size_ui": s, "price_ui": px})
                                        })
                                        .collect();
                                    Some(serde_json::to_string(&q).unwrap_or("[]".to_string()))
                                } else {
                                    None
                                };

                                let now_ts = chrono::Utc::now().timestamp();
                                let mut hset = redis::cmd("HSET");
                                hset.arg(&key).arg("updated_at").arg(now_ts);
                                if let Some(p0) = p0_ui {
                                    hset.arg("p0_ui").arg(p0);
                                }
                                hset.arg("k_coeff").arg(k_coeff);
                                if let Some(q) = quotes {
                                    hset.arg("sample_quotes").arg(q);
                                }
                                let _: () = hset.query_async(&mut conn).await.unwrap_or(());

                                info!(
                                    "updated {} start={} num_init={} sum_gross={} p0_ui={:?} k_coeff={:.6}",
                                    key, start, num_init, sum_gross, p0_ui, k_coeff
                                );
                            }
                        }
                    }
                }
            }
        }
        // else Nil or unexpected → loop
    }
}
