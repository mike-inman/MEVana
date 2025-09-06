use anyhow::Result;
use clap::Parser;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "localhost:29092")]
    broker: String,
    #[arg(long, default_value = "normalized-swaps.v1")]
    topic: String,
    #[arg(long, default_value = "arb-watch")]
    group: String,
    /// pct threshold in % (e.g. 0.20 = 20 bps)
    #[arg(long, default_value_t = 0.20)]
    threshold_pct: f64,
    /// expire book entries without updates
    #[arg(long, default_value = "15")]
    ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwapEvent {
    dex: String,
    pool_id: String,
    slot: u64,
    sig: String,
    in_mint: String,
    out_mint: String,
    amount_in: String,
    amount_out: String,
    fee_paid: String,
}

#[derive(Debug, Clone)]
struct Quote {
    price_out_per_in: f64,
    pool_id: String,
    dex: String,
    ts: Instant,
}

fn parse_decimal(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}

fn price_from_swap(se: &SwapEvent) -> Option<f64> {
    let ain = parse_decimal(&se.amount_in)?;
    let aout = parse_decimal(&se.amount_out)?;
    if ain <= 0.0 {
        return None;
    }
    Some(aout / ain)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &args.broker)
        .set("group.id", &args.group)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()?;
    consumer.subscribe(&[&args.topic])?;

    // pair_key = "in_mint/out_mint"
    let mut best_by_pair: HashMap<String, HashMap<String, Quote>> = HashMap::new();
    let ttl = Duration::from_secs(args.ttl_seconds);

    eprintln!(
        "arb_watch: broker={} topic={} threshold_pct={}%",
        args.broker, args.topic, args.threshold_pct
    );

    loop {
        let m = match consumer.recv().await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("kafka recv error: {e}");
                continue;
            }
        };
        let Some(payload) = m.payload() else {
            continue;
        };

        let se: SwapEvent = match serde_json::from_slice(payload) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let Some(p) = price_from_swap(&se) else {
            continue;
        };

        let pair = format!("{}/{}", se.in_mint, se.out_mint);
        let now = Instant::now();
        let q = Quote {
            price_out_per_in: p,
            pool_id: se.pool_id.clone(),
            dex: se.dex.clone(),
            ts: now,
        };

        // upsert
        let book = best_by_pair.entry(pair.clone()).or_default();
        book.insert(se.dex.clone(), q);

        // evict stale
        book.retain(|_, q| now.duration_since(q.ts) <= ttl);

        // need at least 2 venues
        if book.len() < 2 {
            continue;
        }

        // find best buy (max out per 1 in) and worst
        let mut quotes: Vec<_> = book.values().cloned().collect();
        quotes.sort_by(|a, b| a.price_out_per_in.partial_cmp(&b.price_out_per_in).unwrap());
        let best = quotes.last().unwrap();
        let worst = quotes.first().unwrap();

        // potential arb: buy on best (in->out), sell out->in on another venue (reverse leg)
        let gross_edge = (best.price_out_per_in / worst.price_out_per_in - 1.0) * 100.0;
        if gross_edge >= args.threshold_pct {
            println!(
                "🚨 ARB CANDIDATE pair={} edge={:.3}% buy@{}:{}({}) sell@{}:{}({})",
                pair,
                gross_edge,
                best.dex,
                best.pool_id,
                best.price_out_per_in,
                worst.dex,
                worst.pool_id,
                worst.price_out_per_in
            );
            // TODO: publish candidate JSON to "arb-candidates.v1" if you want another stage to consume.
        }

        let _ = consumer.commit_message(&m, rdkafka::consumer::CommitMode::Async);
    }
}
