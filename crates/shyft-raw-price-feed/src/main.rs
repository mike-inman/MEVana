use anyhow::{Context, Result};
use chrono::Utc;
use futures_util::TryStreamExt;
use log::{error, info, warn};
use lru::LruCache;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{ClientConfig, Message};
use serde::Deserialize;
use serde_json::Value;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

fn mint_to_symbol(mint: &str) -> &'static str {
    match mint {
        // Stablecoins
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => "USDC",
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => "USDT",
        // Wrapped SOL
        "So11111111111111111111111111111111111111112" => "WSOL",
        // Common mints (extend as needed)
        "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj" => "stSOL",
        "SLERF7cThpFksUHvd8rtK5ZCPDbB6rCRV3tjFfxuG7F" => "SLERF",
        _ => "UNKNOWN",
    }
}

#[derive(Clone)]
struct Config {
    kafka_broker: String,
    input_topic: String,
    output_topic: String,
    group_id: String,
    token_registry_url: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            kafka_broker: std::env::var("KAFKA_BROKER")
                .unwrap_or_else(|_| "kafka:9092".to_string()),
            input_topic: std::env::var("INPUT_TOPIC")
                .unwrap_or_else(|_| "shyft-full-transactions-raw".to_string()),
            output_topic: std::env::var("OUTPUT_TOPIC")
                .unwrap_or_else(|_| "prices.latest.raw".to_string()),
            group_id: std::env::var("GROUP_ID")
                .unwrap_or_else(|_| "shyft-raw-price-feed".to_string()),
            token_registry_url: std::env::var("TOKEN_REGISTRY_URL").ok(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryTokenEntry {
    mint: String,
    symbol: Option<String>,
    decimals: Option<u8>,
    is_stable: bool,
}

struct RegistryClient {
    base_url: String,
    http: reqwest::Client,
    cache: tokio::sync::Mutex<LruCache<String, RegistryTokenEntry>>,
}

impl RegistryClient {
    fn new(base_url: String, capacity: usize) -> Self {
        Self {
            base_url,
            http: reqwest::Client::new(),
            cache: tokio::sync::Mutex::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())),
        }
    }

    async fn get(&self, mint: &str) -> Option<RegistryTokenEntry> {
        // fast path: cache
        if let Some(v) = self.cache.lock().await.get(mint).cloned() {
            return Some(v);
        }
        // http call with short timeout
        let url = format!("{}/v1/token/{}", self.base_url, mint);
        let req = self.http.get(&url).timeout(Duration::from_millis(300));
        match req.send().await.and_then(|r| r.error_for_status()) {
            Ok(resp) => match resp.json::<RegistryTokenEntry>().await {
                Ok(entry) => {
                    self.cache.lock().await.put(mint.to_string(), entry.clone());
                    Some(entry)
                }
                Err(_) => None,
            },
            Err(_) => None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let cfg = Config::from_env()?;
    let registry_client = cfg
        .token_registry_url
        .clone()
        .map(|u| Arc::new(RegistryClient::new(u, 1024)));
    info!(
        "Starting shyft-raw-price-feed: input={}, output={}",
        cfg.input_topic, cfg.output_topic
    );

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.kafka_broker)
        .set("group.id", &cfg.group_id)
        .set("auto.offset.reset", "latest")
        .create()
        .context("create consumer")?;
    consumer.subscribe(&[&cfg.input_topic])?;

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.kafka_broker)
        .create()
        .context("create producer")?;

    let mut stream = consumer.stream();
    loop {
        match stream.try_next().await {
            Ok(Some(msg)) => {
                if let Some(payload) = msg.payload() {
                    if let Err(e) =
                        process_message(payload, &producer, &cfg, registry_client.clone()).await
                    {
                        warn!("failed to process message: {}", e);
                    }
                }
            }
            Ok(None) => break,
            Err(e) => warn!("kafka consumer error: {}", e),
        }
    }
    Ok(())
}

async fn process_message(
    payload: &[u8],
    producer: &FutureProducer,
    cfg: &Config,
    registry: Option<Arc<RegistryClient>>,
) -> Result<()> {
    let v: Value = serde_json::from_slice(payload).context("parse raw shyft json")?;
    let result = v
        .get("full_shyft_response")
        .and_then(|x| x.get("result"))
        .context("missing result in full_shyft_response")?;

    // Prefer parsed.actions if present to identify swaps cleanly
    if let Some(actions) = result
        .get("parsed")
        .and_then(|p| p.get("actions"))
        .and_then(|a| a.as_array())
    {
        for act in actions {
            let typ = act.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if typ != "SWAP" {
                continue;
            }
            let info = match act.get("info") {
                Some(i) => i,
                None => continue,
            };

            // tokens_swapped.in/out amounts (already scaled by Shyft)
            let tin = info
                .get("tokens_swapped")
                .and_then(|ts| ts.get("in"))
                .context("missing tokens_swapped.in")?;
            let tout = info
                .get("tokens_swapped")
                .and_then(|ts| ts.get("out"))
                .context("missing tokens_swapped.out")?;

            let in_mint = tin
                .get("token_address")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            let out_mint = tout
                .get("token_address")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            let in_sym_shyft = tin.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
            let out_sym_shyft = tout.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
            let mut in_decimals: Option<u8> = None;
            let mut out_decimals: Option<u8> = None;
            let mut in_is_stable: Option<bool> = None;
            let mut out_is_stable: Option<bool> = None;
            let mut in_symbol: String;
            let mut out_symbol: String;
            // Fallback to mint-derived symbols when Shyft does not provide
            let (rin, rout) = if let Some(reg) = registry.as_ref() {
                tokio::join!(reg.get(in_mint), reg.get(out_mint))
            } else {
                (None, None)
            };
            // Build in_symbol with precedence: registry -> shyft -> mint mapping
            if let Some(ref e) = rin {
                if let Some(sym) = e.symbol.as_ref() {
                    if !sym.is_empty() {
                        in_symbol = sym.clone();
                    } else {
                        in_symbol = if in_sym_shyft.is_empty() {
                            mint_to_symbol(in_mint).to_string()
                        } else {
                            in_sym_shyft.to_string()
                        };
                    }
                } else {
                    in_symbol = if in_sym_shyft.is_empty() {
                        mint_to_symbol(in_mint).to_string()
                    } else {
                        in_sym_shyft.to_string()
                    };
                }
                in_decimals = e.decimals;
                in_is_stable = Some(e.is_stable);
            } else {
                in_symbol = if in_sym_shyft.is_empty() {
                    mint_to_symbol(in_mint).to_string()
                } else {
                    in_sym_shyft.to_string()
                };
            }
            // Build out_symbol
            if let Some(ref e) = rout {
                if let Some(sym) = e.symbol.as_ref() {
                    if !sym.is_empty() {
                        out_symbol = sym.clone();
                    } else {
                        out_symbol = if out_sym_shyft.is_empty() {
                            mint_to_symbol(out_mint).to_string()
                        } else {
                            out_sym_shyft.to_string()
                        };
                    }
                } else {
                    out_symbol = if out_sym_shyft.is_empty() {
                        mint_to_symbol(out_mint).to_string()
                    } else {
                        out_sym_shyft.to_string()
                    };
                }
                out_decimals = e.decimals;
                out_is_stable = Some(e.is_stable);
            } else {
                out_symbol = if out_sym_shyft.is_empty() {
                    mint_to_symbol(out_mint).to_string()
                } else {
                    out_sym_shyft.to_string()
                };
            }
            let amount_in = tin.get("amount").and_then(|n| n.as_f64()).unwrap_or(0.0);
            let amount_out = tout.get("amount").and_then(|n| n.as_f64()).unwrap_or(0.0);

            // pool (if present)
            let pool = info
                .get("swaps")
                .and_then(|s| s.as_array())
                .and_then(|arr| arr.get(0))
                .and_then(|s0| s0.get("liquidity_pool_address"))
                .and_then(|s| s.as_str())
                .unwrap_or("");

            // choose quote if USDC/USDT present; else publish both directions
            let is_stable = |m: &str| {
                m == "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
                    || m == "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            };

            let ts = result
                .get("timestamp")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let signature = v
                .get("transaction_signature")
                .and_then(|s| s.as_str())
                .unwrap_or("");

            if amount_in > 0.0 && amount_out > 0.0 {
                if is_stable(out_mint) {
                    // price = stable_out / base_in
                    publish_price(
                        producer,
                        cfg,
                        in_mint,
                        out_mint,
                        &in_symbol,
                        &out_symbol,
                        in_decimals,
                        out_decimals,
                        in_is_stable,
                        out_is_stable,
                        amount_out / amount_in,
                        amount_in,
                        amount_out,
                        pool,
                        &ts,
                        signature,
                    )
                    .await?;
                } else if is_stable(in_mint) {
                    // price = stable_in / base_out (invert)
                    publish_price(
                        producer,
                        cfg,
                        out_mint,
                        in_mint,
                        &out_symbol,
                        &in_symbol,
                        out_decimals,
                        in_decimals,
                        out_is_stable,
                        in_is_stable,
                        amount_in / amount_out,
                        amount_in,
                        amount_out,
                        pool,
                        &ts,
                        signature,
                    )
                    .await?;
                } else {
                    // publish both directions (no assumptions)
                    publish_price(
                        producer,
                        cfg,
                        in_mint,
                        out_mint,
                        &in_symbol,
                        &out_symbol,
                        in_decimals,
                        out_decimals,
                        in_is_stable,
                        out_is_stable,
                        amount_out / amount_in,
                        amount_in,
                        amount_out,
                        pool,
                        &ts,
                        signature,
                    )
                    .await?;
                    publish_price(
                        producer,
                        cfg,
                        out_mint,
                        in_mint,
                        &out_symbol,
                        &in_symbol,
                        out_decimals,
                        in_decimals,
                        out_is_stable,
                        in_is_stable,
                        amount_in / amount_out,
                        amount_in,
                        amount_out,
                        pool,
                        &ts,
                        signature,
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}

async fn publish_price(
    producer: &FutureProducer,
    cfg: &Config,
    base_mint: &str,
    quote_mint: &str,
    base_symbol: &str,
    quote_symbol: &str,
    base_decimals: Option<u8>,
    quote_decimals: Option<u8>,
    base_is_stable: Option<bool>,
    quote_is_stable: Option<bool>,
    price: f64,
    amount_in: f64,
    amount_out: f64,
    pool: &str,
    ts_str: &str,
    signature: &str,
) -> Result<()> {
    // compute price_usd if quoted in USDC/USDT
    let mut price_usdc: Option<f64> = None;
    let mut price_usdt: Option<f64> = None;
    if quote_mint == "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" {
        price_usdc = Some(price);
    } else if quote_mint == "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" {
        price_usdt = Some(price);
    }
    let now = Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "label": "shyft_raw_price_tick_v1",
        "source": "shyft_raw",
        "timestamp": now,
        "tx_time": ts_str,
        "signature": signature,
        "pair": format!("{}/{}", base_symbol, quote_symbol),
        "base_mint": base_mint,
        "quote_mint": quote_mint,
        "base_symbol": base_symbol,
        "quote_symbol": quote_symbol,
        "base_decimals": base_decimals,
        "quote_decimals": quote_decimals,
        "base_is_stable": base_is_stable,
        "quote_is_stable": quote_is_stable,
        "price": price,
        "price_usdc": price_usdc,
        "price_usdt": price_usdt,
        "amount_in": amount_in,
        "amount_out": amount_out,
        "pool_address": pool,
    });
    producer
        .send(
            FutureRecord::to(&cfg.output_topic)
                .key(signature)
                .payload(&serde_json::to_vec(&payload)?),
            Duration::from_secs(5),
        )
        .await
        .map_err(|(e, _)| anyhow::anyhow!("producer error: {}", e))?;
    info!(
        "price published: {} {} -> {} = {}",
        signature, base_mint, quote_mint, price
    );
    Ok(())
}
