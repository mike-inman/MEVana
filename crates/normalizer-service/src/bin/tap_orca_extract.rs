// crates/normalizer-service/src/bin/tap_orca_extract.rs

// Pull in the generated registry (built by build.rs via normalizer-codegen)
mod generated {
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

use anyhow::{Context, Result};
use bs58;
use chrono::Utc;
use dotenv::dotenv;
use env_logger;
use log::{error, info, warn};
use normalizer_runtime as rt;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use rdkafka::Message as _;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde_json;
use std::collections::HashMap;
use std::time::Instant;
use yellowstone_grpc_proto::prelude as ypp;
use yellowstone_grpc_proto::prost::Message as _;

// Simple metrics collection
#[derive(Default)]
struct Metrics {
    total_messages: u64,
    successful_decodes: u64,
    decode_errors: u64,
    redis_errors: u64,
    kafka_errors: u64,
    latency_samples: Vec<f64>,
}

impl Metrics {
    fn record_latency(&mut self, latency_ms: f64) {
        self.latency_samples.push(latency_ms);
        // Keep only last 1000 samples for memory efficiency
        if self.latency_samples.len() > 1000 {
            self.latency_samples.remove(0);
        }
    }

    fn get_p50_latency(&self) -> Option<f64> {
        if self.latency_samples.is_empty() {
            return None;
        }
        let mut sorted = self.latency_samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted.len() / 2;
        Some(sorted[mid])
    }

    fn get_p99_latency(&self) -> Option<f64> {
        if self.latency_samples.is_empty() {
            return None;
        }
        let mut sorted = self.latency_samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = (sorted.len() as f64 * 0.99) as usize;
        Some(sorted[idx.min(sorted.len() - 1)])
    }

    fn log_summary(&self) {
        info!("METRICS: total={} decodes={} errors={} redis_errors={} kafka_errors={} p50={:.2}ms p99={:.2}ms", 
            self.total_messages,
            self.successful_decodes,
            self.decode_errors,
            self.redis_errors,
            self.kafka_errors,
            self.get_p50_latency().unwrap_or(0.0),
            self.get_p99_latency().unwrap_or(0.0)
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    let in_topic = std::env::var("IN_TOPIC").unwrap_or_else(|_| "orca-swaps".into());
    let out_topic = std::env::var("OUT_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed.v2".into());
    let group_id = std::env::var("GROUP_ID").unwrap_or_else(|_| "tap-orca-extract".into());
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".into());

    info!("tap_orca_extract: broker={broker} in={in_topic} out={out_topic} group_id={group_id}");

    let fast = std::env::var("FAST_MODE").ok().as_deref() == Some("1");
    let mut cconf = ClientConfig::new();
    cconf
        .set("bootstrap.servers", &broker)
        .set("group.id", &group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest");
    if fast {
        cconf
            .set("fetch.min.bytes", "1")
            .set("fetch.wait.max.ms", "1")
            .set("queued.min.messages", "1");
    }
    let consumer: StreamConsumer = cconf.create().context("create consumer")?;

    consumer.subscribe(&[&in_topic]).context("subscribe")?;

    // Producer for v2 output
    let mut pconf = ClientConfig::new();
    pconf.set("bootstrap.servers", &broker);
    if fast {
        pconf
            .set("linger.ms", "0")
            .set("acks", "1")
            .set("compression.type", "lz4")
            .set("enable.idempotence", "false")
            .set("message.timeout.ms", "100");
    } else {
        pconf
            .set("compression.type", "lz4")
            .set("linger.ms", "5")
            .set("acks", "all");
    }
    let producer: FutureProducer = pconf.create().context("create producer")?;

    // Redis (optional enrich) with local read-through caches
    let redis_client = redis::Client::open(redis_url.clone())?;
    let mut redis_conn: MultiplexedConnection = redis_client
        .get_multiplexed_tokio_connection()
        .await
        .context("redis connect")?;

    // Verify decoders are registered
    {
        let regs = generated::registry();
        if regs.is_empty() {
            warn!("No generated extractors registered. Check idls/ and profiles/.");
        } else {
            info!("{} extractor(s) registered", regs.len());
            for e in regs {
                info!(" - program_id={}", e.program_id());
            }
        }
    }

    let mut metrics = Metrics::default();

    loop {
        match consumer.recv().await {
            Err(e) => {
                metrics.kafka_errors += 1;
                warn!("kafka recv error: {e:?}");
            }
            Ok(m) => {
                if let Some(bytes) = m.payload() {
                    let t0 = Instant::now();
                    if let Err(e) = handle(bytes, &producer, &out_topic, &mut redis_conn).await {
                        metrics.decode_errors += 1;
                        error!("handle error: {e:#}");
                    } else {
                        metrics.successful_decodes += 1;
                    }
                    let dt_ms = t0.elapsed().as_micros() as f64 / 1000.0;
                    metrics.record_latency(dt_ms);
                    if dt_ms > 25.0 {
                        warn!("extract path latency high: {:.2} ms", dt_ms);
                    }
                } else {
                    metrics.kafka_errors += 1;
                    warn!("empty payload");
                }
                let _ = consumer.commit_message(&m, CommitMode::Async);
            }
        }
        metrics.total_messages += 1;
        if metrics.total_messages % 1000 == 0 {
            metrics.log_summary();
        }
    }
}

async fn handle(
    payload: &[u8],
    producer: &FutureProducer,
    out_topic: &str,
    redis: &mut MultiplexedConnection,
) -> Result<()> {
    // Decode the Yellowstone SubscribeUpdateTransaction
    let update = ypp::SubscribeUpdateTransaction::decode(payload)
        .context("prost decode SubscribeUpdateTransaction")?;

    let Some(info) = update.transaction.as_ref() else {
        return Ok(());
    };

    let (keys_b58, tb_changes) = build_keys_and_balance_changes(info);

    // Iterate compiled instructions and attempt IDL decode + normalization
    if let Some(msg) = info.transaction.as_ref().and_then(|t| t.message.as_ref()) {
        for (ix_index, ci) in msg.instructions.iter().enumerate() {
            let pid_idx = ci.program_id_index as usize;
            if pid_idx >= keys_b58.len() {
                continue;
            }
            let pid = &keys_b58[pid_idx];

            // IDL decode
            let Some(dec) = generated::decoder_for(pid) else {
                continue;
            };
            let Some((decoded_name, roles)) = dec(&ci.data, &ci.accounts, &keys_b58) else {
                continue;
            };

            // Find extractor for this program id
            let extractor = generated::registry()
                .into_iter()
                .find(|e| e.program_id() == pid);
            let Some(extractor) = extractor else {
                continue;
            };

            let sig = bs58::encode(&info.signature).into_string();

            // Build TxContext
            let ctx = rt::TxContext {
                slot: update.slot,
                block_time_unix: Utc::now().timestamp(),
                signature: sig,
                ix_index: ix_index as u16,
                program_id: pid.clone(),
                meta_ok: true,
                token_balance_changes: &tb_changes,
                account_data_by_pubkey: None,
                decoded_name: Some(decoded_name),
                decoded_args_json: None,
                decoded_accounts_by_role: Some(roles),
            };

            if let Some(n) = extractor.try_extract(&ctx) {
                // Enrich with pool/mint info from Redis if present
                // Expect keys: dex:pool:{pool_id}, dex:mint:{mint}
                let pool_id = ctx
                    .decoded_accounts_by_role
                    .as_ref()
                    .and_then(|m| ["whirlpool", "pool"].iter().find_map(|r| m.get(*r)))
                    .cloned()
                    .unwrap_or_default();
                let pool_key = format!("dex:pool:{}", pool_id);
                let mut tick_hint: Option<i64> = None;
                let mut sqrt_price_hint: Option<String> = None;
                if !pool_key.ends_with(':') {
                    if let Some(snap) = get_pool_snapshot_cached(redis, &pool_key).await {
                        if let Some(sp) = snap.get("sqrt_price_x64") {
                            sqrt_price_hint = Some(sp.clone());
                        }
                        if let Some(t) = snap.get("tick").and_then(|s| s.parse::<i64>().ok()) {
                            tick_hint = Some(t);
                        }
                    }
                }

                // Optional decimals → UI amounts (with local LRU + TTL read-through)
                let dec_in = get_mint_decimals_cached(redis, &n.token_in_mint).await;
                let dec_out = get_mint_decimals_cached(redis, &n.token_out_mint).await;
                let amount_in_ui = dec_in.map(|d| to_ui(n.amount_in, d));
                let amount_out_ui = dec_out.map(|d| to_ui(n.amount_out, d));
                let price_ui = match (amount_in_ui, amount_out_ui) {
                    (Some(ai), Some(ao)) if ai > 0.0 => Some(ao / ai),
                    _ => None,
                };

                // Merge into JSON object
                let mut obj = serde_json::to_value(&n)?
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                if let Some(v) = amount_in_ui {
                    obj.insert("amount_in_ui".into(), serde_json::json!(v));
                }
                if let Some(v) = amount_out_ui {
                    obj.insert("amount_out_ui".into(), serde_json::json!(v));
                }
                if let Some(v) = price_ui {
                    obj.insert("price_ui".into(), serde_json::json!(v));
                }
                if let Some(v) = tick_hint {
                    obj.insert("tick_hint".into(), serde_json::json!(v));
                }
                if let Some(v) = sqrt_price_hint {
                    obj.insert("sqrt_price_x64_hint".into(), serde_json::json!(v));
                }

                // Emit to Kafka as JSON with key = signature
                let key = ctx.signature.clone();
                let payload = serde_json::to_vec(&obj)?;
                let _ = producer
                    .send(
                        FutureRecord::to(out_topic).key(&key).payload(&payload),
                        std::time::Duration::from_secs(5),
                    )
                    .await;
                // Also echo to stdout for quick inspection
                println!("{}", serde_json::to_string(&obj)?);
            }
        }
    }

    Ok(())
}

fn build_keys_and_balance_changes(
    info: &ypp::SubscribeUpdateTransactionInfo,
) -> (Vec<String>, Vec<rt::TokenBalanceChange>) {
    // Account keys (base58)
    let keys_b58: Vec<String> = info
        .transaction
        .as_ref()
        .and_then(|t| t.message.as_ref())
        .map(|msg| {
            msg.account_keys
                .iter()
                .map(|k| bs58::encode(k).into_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Build pre/post maps over (owner, mint)
    let mut pre_map: HashMap<(String, String), u128> = HashMap::new();
    let mut post_map: HashMap<(String, String), u128> = HashMap::new();

    if let Some(meta) = info.meta.as_ref() {
        for b in &meta.pre_token_balances {
            let owner = b.owner.clone();
            let mint = b.mint.clone();
            let raw = b
                .ui_token_amount
                .as_ref()
                .map(|x| x.amount.clone())
                .unwrap_or_else(|| "0".to_string());
            let val = raw.parse::<u128>().unwrap_or(0);
            pre_map.insert((owner, mint), val);
        }
        for b in &meta.post_token_balances {
            let owner = b.owner.clone();
            let mint = b.mint.clone();
            let raw = b
                .ui_token_amount
                .as_ref()
                .map(|x| x.amount.clone())
                .unwrap_or_else(|| "0".to_string());
            let val = raw.parse::<u128>().unwrap_or(0);
            post_map.insert((owner, mint), val);
        }
    }

    // Union of keys
    let mut out: Vec<rt::TokenBalanceChange> = Vec::new();
    let mut seen: HashMap<(String, String), bool> = HashMap::new();
    for (k, v) in pre_map.iter() {
        seen.insert(k.clone(), true);
        let post = post_map.get(k).copied().unwrap_or(0);
        out.push(rt::TokenBalanceChange {
            owner: k.0.clone(),
            mint: k.1.clone(),
            pre: *v,
            post,
        });
    }
    for (k, post) in post_map.into_iter() {
        if seen.contains_key(&k) {
            continue;
        }
        out.push(rt::TokenBalanceChange {
            owner: k.0,
            mint: k.1,
            pre: 0,
            post,
        });
    }

    (keys_b58, out)
}

fn to_ui(raw: u128, decimals: u8) -> f64 {
    if decimals == 0 {
        return raw as f64;
    }
    let scale = 10u128.saturating_pow(decimals as u32);
    (raw as f64) / (scale as f64)
}

use once_cell::sync::Lazy;
use std::num::NonZeroUsize;
use std::sync::Mutex as StdMutex;

struct TtlEntry<T> {
    value: T,
    expires_at_unix_ms: i64,
}

static MINT_DECIMALS_CACHE: Lazy<StdMutex<lru::LruCache<String, TtlEntry<u8>>>> =
    Lazy::new(|| StdMutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap())));

static POOL_SNAPSHOT_CACHE: Lazy<
    StdMutex<lru::LruCache<String, TtlEntry<HashMap<String, String>>>>,
> = Lazy::new(|| StdMutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap())));

fn now_unix_ms() -> i64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64)
}

async fn get_mint_decimals_cached(redis: &mut MultiplexedConnection, mint: &str) -> Option<u8> {
    let ttl_ms: i64 = std::env::var("MINT_DECIMALS_TTL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60_000);
    {
        let mut guard = MINT_DECIMALS_CACHE.lock().unwrap();
        if let Some(entry) = guard.get(mint) {
            if entry.expires_at_unix_ms > now_unix_ms() {
                return Some(entry.value);
            }
        }
    }
    let key = format!("dex:mint:{}", mint);
    if let Ok(Some(d)) = redis::cmd("HGET")
        .arg(&key)
        .arg("decimals")
        .query_async::<Option<i64>>(redis)
        .await
    {
        if d >= 0 && d <= 255 {
            let entry = TtlEntry {
                value: d as u8,
                expires_at_unix_ms: now_unix_ms() + ttl_ms,
            };
            let mut guard = MINT_DECIMALS_CACHE.lock().unwrap();
            guard.put(mint.to_string(), entry);
            return Some(d as u8);
        }
    }
    None
}

async fn get_pool_snapshot_cached(
    redis: &mut MultiplexedConnection,
    pool_key: &str,
) -> Option<HashMap<String, String>> {
    let ttl_ms: i64 = std::env::var("POOL_SNAPSHOT_TTL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60_000);
    {
        let mut guard = POOL_SNAPSHOT_CACHE.lock().unwrap();
        if let Some(entry) = guard.get(pool_key) {
            if entry.expires_at_unix_ms > now_unix_ms() {
                return Some(entry.value.clone());
            }
        }
    }
    match redis.hgetall::<_, HashMap<String, String>>(pool_key).await {
        Ok(map) if !map.is_empty() => {
            let entry = TtlEntry {
                value: map.clone(),
                expires_at_unix_ms: now_unix_ms() + ttl_ms,
            };
            let mut guard = POOL_SNAPSHOT_CACHE.lock().unwrap();
            guard.put(pool_key.to_string(), entry);
            Some(map)
        }
        _ => None,
    }
}
