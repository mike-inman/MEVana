use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
use env_logger::Env;
use log::{error, info};
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use serde::Serialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::interval;
use whirlpool_core::math::{
    apply_input_fee, dx_raw_within_tick_ab, dy_raw_within_tick_ba, tick_to_sqrt_price_x64_approx,
};
use whirlpool_core::{
    quoter::ClmmQuoter,
    quoter::Direction,
    tick_window::{TickArrayView, TickWindow},
    types::Tick,
};

#[derive(Parser, Debug, Clone)]
#[command(name = "whirlpool-quote-surface", version)]
struct Args {
    #[arg(long, env = "REDIS_URL", default_value = "redis://localhost:6379/0")]
    redis_url: String,

    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:29092")]
    kafka_brokers: String,

    #[arg(long, env = "OUT_TOPIC", default_value = "quotes.whirlpool.v1")]
    out_topic: String,

    #[arg(long, env = "QUOTE_SIZES", default_value = "100,1000,10000")]
    quote_sizes: String,

    #[arg(long, env = "QUOTE_INTERVAL_MS", default_value_t = 1000)]
    quote_interval_ms: u64,

    #[arg(
        long,
        env = "RPC_URL",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    rpc_url: String,

    #[arg(long, env = "MIN_LIQUIDITY", default_value_t = 0u128)]
    min_liquidity: u128,

    #[arg(long, env = "QUOTE_POOL_ALLOWLIST", default_value = "")]
    quote_pool_allowlist: String,
}

#[derive(Serialize)]
struct QuoteOut {
    pool: String,
    side: String,
    size_in: u128,
    out: u128,
    ticks_crossed: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args = Args::parse();
    info!("Starting whirlpool-quote-surface → {}", args.out_topic);

    let redis_client = redis::Client::open(args.redis_url.clone())?;
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
    let rpc = RpcClient::new(args.rpc_url.clone());

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &args.kafka_brokers)
        .create()?;

    let mut ticker = interval(Duration::from_millis(args.quote_interval_ms));
    let sizes: Vec<u128> = args
        .quote_sizes
        .split(',')
        .filter_map(|s| s.trim().parse::<u128>().ok())
        .collect();

    // Simple decimals cache
    let mut mint_decimals: HashMap<String, u8> = HashMap::new();
    let allowlist: std::collections::HashSet<String> = args
        .quote_pool_allowlist
        .split(',')
        .filter_map(|s| {
            let t = s.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        })
        .collect();

    loop {
        ticker.tick().await;

        // Enumerate available Whirlpool pools from Redis (dev: KEYS; prod: SCAN)
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg("whirlpool:pool:*")
            .query_async(&mut redis_conn)
            .await
            .unwrap_or_default();

        // SCAN returns a vector; we keep a small sample
        for key in keys
            .into_iter()
            .filter(|k| k.starts_with("whirlpool:pool:"))
            .take(10)
        {
            let pool = key.trim_start_matches("whirlpool:pool:").to_string();
            let raw: Option<String> = redis::cmd("GET")
                .arg(&key)
                .query_async(&mut redis_conn)
                .await
                .ok();
            let Some(raw_json) = raw else { continue };
            let v: serde_json::Value =
                serde_json::from_str(&raw_json).unwrap_or(serde_json::json!({}));
            let sqrt_price_x64 = v
                .get("sqrt_price_x64")
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse::<u128>().ok())
                .unwrap_or(0);
            let liquidity = v
                .get("liquidity")
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse::<u128>().ok())
                .unwrap_or(0);
            let tick_current = v.get("tick_current").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
            let tick_spacing = v.get("tick_spacing").and_then(|x| x.as_i64()).unwrap_or(64) as i32;
            let fee_bps = v.get("fee_bps").and_then(|x| x.as_i64()).unwrap_or(30) as u16;

            // Active pool filter: liquidity threshold and at least one initialized tick
            if liquidity < args.min_liquidity && !allowlist.contains(&pool) {
                continue;
            }

            // Decimals lookup for token mints
            let token_a = v
                .get("token_a")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let token_b = v
                .get("token_b")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let _dec_a = get_or_fetch_decimals(&rpc, &mut mint_decimals, &mut redis_conn, &token_a)
                .await
                .unwrap_or(6);
            let _dec_b = get_or_fetch_decimals(&rpc, &mut mint_decimals, &mut redis_conn, &token_b)
                .await
                .unwrap_or(6);

            // Assemble a TickWindow from Redis tick-array summaries
            let window = match build_tick_window_from_redis(
                &mut redis_conn,
                &v,
                &pool,
                tick_spacing,
            )
            .await
            {
                Some(w) => w,
                None => TickWindow::default(),
            };
            // If not allowlisted, skip pools with no initialized ticks across window
            if window.current.is_none() && !allowlist.contains(&pool) {
                continue;
            }
            let quoter = ClmmQuoter {
                sqrt_price_x64,
                liquidity,
                tick_current,
                tick_spacing,
                total_fee_bps: fee_bps,
                window,
            };

            for &sz in &sizes {
                for (dir, side) in [(Direction::AtoB, "AtoB"), (Direction::BtoA, "BtoA")] {
                    let mut q = quoter.quote(dir, sz, 1);
                    if q.out == 0 {
                        if let Some(fallback_out) = fallback_one_grid_step(
                            dir,
                            sz,
                            quoter.sqrt_price_x64,
                            quoter.liquidity,
                            quoter.tick_current,
                            quoter.tick_spacing,
                            quoter.total_fee_bps,
                        ) {
                            q.out = fallback_out;
                        }
                    }
                    let out = QuoteOut {
                        pool: pool.clone(),
                        side: side.to_string(),
                        size_in: sz,
                        out: q.out,
                        ticks_crossed: q.ticks_crossed,
                    };
                    let payload = serde_json::to_string(&out)?;
                    let _ = producer
                        .send(
                            FutureRecord::<(), _>::to(&args.out_topic).payload(&payload),
                            Duration::from_secs(5),
                        )
                        .await;
                }
            }
        }

        info!("Emitted quotes batch");
    }
}

fn floor_div_i32(x: i32, y: i32) -> i32 {
    let mut q = x / y;
    if (x ^ y) < 0 && x % y != 0 {
        q -= 1;
    }
    q
}
fn ceil_div_i32(x: i32, y: i32) -> i32 {
    let mut q = x / y;
    if (x ^ y) >= 0 && x % y != 0 {
        q += 1;
    }
    q
}

fn fallback_one_grid_step(
    dir: Direction,
    dx_in: u128,
    sp_current: u128,
    liquidity: u128,
    tick_current: i32,
    tick_spacing: i32,
    fee_bps: u16,
) -> Option<u128> {
    if dx_in == 0 || sp_current == 0 || tick_spacing <= 0 {
        return None;
    }
    let boundary_tick = match dir {
        Direction::AtoB => floor_div_i32(tick_current, tick_spacing) * tick_spacing,
        Direction::BtoA => ceil_div_i32(tick_current + 1, tick_spacing) * tick_spacing,
    };
    let sp_next = tick_to_sqrt_price_x64_approx(boundary_tick);
    match dir {
        Direction::AtoB => {
            if sp_next == 0 || sp_next >= sp_current {
                return None;
            }
            let max_dx_raw = dx_raw_within_tick_ab(sp_current, sp_next, liquidity);
            if max_dx_raw == 0 {
                return None;
            }
            let max_dx_in = apply_input_fee(max_dx_raw, fee_bps);
            let dx_clip = dx_in.min(max_dx_in);
            let fee_denom = 10_000u64;
            let fee_num = (fee_denom - fee_bps as u64) as u128;
            let dx_eff_clip = (dx_clip as u128) * fee_num / (fee_denom as u128);
            let dy_raw = (liquidity as u128)
                .saturating_mul(sp_current.saturating_sub(sp_next))
                .saturating_mul(dx_eff_clip)
                / max_dx_raw;
            Some(dy_raw)
        }
        Direction::BtoA => {
            if sp_next <= sp_current {
                return None;
            }
            let max_dy_raw = dy_raw_within_tick_ba(sp_current, sp_next, liquidity);
            if max_dy_raw == 0 {
                return None;
            }
            let max_dx_in = apply_input_fee(max_dy_raw, fee_bps);
            let dx_clip = dx_in.min(max_dx_in);
            let fee_denom = 10_000u64;
            let fee_num = (fee_denom - fee_bps as u64) as u128;
            let dx_eff_clip = (dx_clip as u128) * fee_num / (fee_denom as u128);
            let num = (liquidity as u128)
                .saturating_mul(sp_next.saturating_sub(sp_current))
                .saturating_mul(dx_eff_clip);
            let dy_out = if max_dy_raw > 0 { num / max_dy_raw } else { 0 };
            Some(dy_out)
        }
    }
}

async fn build_tick_window_from_redis(
    redis_conn: &mut MultiplexedConnection,
    pool_json: &serde_json::Value,
    pool_b58: &str,
    tick_spacing: i32,
) -> Option<TickWindow<'static>> {
    let starts = pool_json.get("tick_array_starts")?.as_array()?;
    if starts.len() != 3 {
        return None;
    }
    let start_vals: Vec<i32> = starts
        .iter()
        .filter_map(|x| x.as_i64())
        .map(|x| x as i32)
        .collect();
    if start_vals.len() != 3 {
        return None;
    }

    // Helper to fetch initialized indices for a start
    async fn fetch_init_indices(
        conn: &mut MultiplexedConnection,
        pool: &str,
        start: i32,
    ) -> Option<Vec<i32>> {
        let key = format!("whirlpool:tickarray:{}:{}", start, pool);
        let raw: String = redis::cmd("GET").arg(&key).query_async(conn).await.ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let arr = v.get("initialized_indices")?.as_array()?;
        Some(
            arr.iter()
                .filter_map(|x| x.as_i64())
                .map(|x| x as i32)
                .collect(),
        )
    }

    // Build three arrays; we must own the memory backing TickArrayView
    let mut lower_ticks: [Tick; 88] = std::array::from_fn(|_| Tick {
        liquidity_net: 0,
        liquidity_gross: 0,
        fee_growth_outside_a: 0,
        fee_growth_outside_b: 0,
        reward_growths_outside: [0, 0, 0],
    });
    let mut current_ticks = lower_ticks.clone();
    let mut upper_ticks = lower_ticks.clone();

    // Populate initialized flags based on indices
    if let Some(idxs) = fetch_init_indices(redis_conn, pool_b58, start_vals[0]).await {
        for abs_tick in idxs {
            let off = (abs_tick - start_vals[0]) / tick_spacing;
            if off >= 0 && off < 88 {
                lower_ticks[off as usize].liquidity_gross = 1;
            }
        }
    }
    if let Some(idxs) = fetch_init_indices(redis_conn, pool_b58, start_vals[1]).await {
        for abs_tick in idxs {
            let off = (abs_tick - start_vals[1]) / tick_spacing;
            if off >= 0 && off < 88 {
                current_ticks[off as usize].liquidity_gross = 1;
            }
        }
    }
    if let Some(idxs) = fetch_init_indices(redis_conn, pool_b58, start_vals[2]).await {
        for abs_tick in idxs {
            let off = (abs_tick - start_vals[2]) / tick_spacing;
            if off >= 0 && off < 88 {
                upper_ticks[off as usize].liquidity_gross = 1;
            }
        }
    }

    // Leak arrays to obtain 'static refs for TickWindow lifetime within this call
    let lower_box = Box::new(lower_ticks);
    let current_box = Box::new(current_ticks);
    let upper_box = Box::new(upper_ticks);
    let lower_ref: &'static [Tick; 88] = Box::leak(lower_box);
    let current_ref: &'static [Tick; 88] = Box::leak(current_box);
    let upper_ref: &'static [Tick; 88] = Box::leak(upper_box);

    Some(TickWindow {
        lower: Some(TickArrayView {
            start_tick_index: start_vals[0],
            ticks: lower_ref,
        }),
        current: Some(TickArrayView {
            start_tick_index: start_vals[1],
            ticks: current_ref,
        }),
        upper: Some(TickArrayView {
            start_tick_index: start_vals[2],
            ticks: upper_ref,
        }),
    })
}

async fn get_or_fetch_decimals(
    rpc: &RpcClient,
    cache: &mut HashMap<String, u8>,
    redis_conn: &mut MultiplexedConnection,
    mint_b58: &str,
) -> Option<u8> {
    if mint_b58.is_empty() {
        return None;
    }
    if let Some(&d) = cache.get(mint_b58) {
        return Some(d);
    }
    // Try Redis first
    let key = format!("mint:decimals:{}", mint_b58);
    if let Ok(Some(s)) = redis::cmd("GET")
        .arg(&key)
        .query_async::<_, Option<String>>(redis_conn)
        .await
    {
        if let Ok(val) = s.parse::<u8>() {
            cache.insert(mint_b58.to_string(), val);
            return Some(val);
        }
    }
    // Fallback to RPC
    let pk = Pubkey::from_str(mint_b58).ok()?;
    let acc = rpc.get_account(&pk).ok()?;
    if acc.data.len() >= 45 {
        let dec = acc.data[44];
        cache.insert(mint_b58.to_string(), dec);
        let _: Result<(), _> = redis::cmd("SETEX")
            .arg(&key)
            .arg(3600)
            .arg(dec.to_string())
            .query_async::<_, ()>(redis_conn)
            .await;
        Some(dec)
    } else {
        None
    }
}
