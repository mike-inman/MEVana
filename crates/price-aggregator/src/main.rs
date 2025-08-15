use anyhow::Result;
use chrono::Utc;
use dotenv::dotenv;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

#[derive(Debug)]
struct Cfg {
    broker: String,
    in_topic: String,  // e.g. "orca-swaps.parsed"
    out_topic: String, // e.g. "prices.latest"
    group_id: String,  // e.g. "price-aggregator"
    emit_secs: u64,    // e.g. 5
}

impl Cfg {
    fn load() -> Self {
        dotenv().ok();
        env_logger::init();

        let broker = env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());
        let in_topic = env::var("IN_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed".to_string());
        let out_topic = env::var("OUT_TOPIC").unwrap_or_else(|_| "prices.latest".to_string());
        let group_id = env::var("GROUP_ID").unwrap_or_else(|_| "price-aggregator".to_string());
        let emit_secs = env::var("EMIT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        log::info!(
            "config: broker={} in_topic={} out_topic={} group_id={} emit_secs={}",
            broker,
            in_topic,
            out_topic,
            group_id,
            emit_secs
        );

        Self {
            broker,
            in_topic,
            out_topic,
            group_id,
            emit_secs,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct NormalizedSwap {
    ts: i64,
    slot: u64,
    sig: String,
    dex: String,
    token_in: Option<String>,
    amount_in: Option<f64>,
    token_out: Option<String>,
    amount_out: Option<f64>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PriceSnapshot {
    pub ts: i64,
    pub dex: String,
    pub pair: String,
    pub price: f64,
    pub src_sig: String,
    pub window_secs: u64,
}

fn make_consumer(cfg: &Cfg) -> Result<StreamConsumer> {
    let c: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.broker)
        .set("group.id", &cfg.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()?;
    c.subscribe(&[&cfg.in_topic])?;
    Ok(c)
}

fn make_producer(cfg: &Cfg) -> Result<FutureProducer> {
    let p: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.broker)
        .set("acks", "all")
        .set("linger.ms", "5")
        .set("compression.type", "lz4")
        .create()?;
    Ok(p)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Cfg::load();
    let consumer = make_consumer(&cfg)?;
    let producer = make_producer(&cfg)?;

    // Shared in-memory latest price per (dex, pair)
    let state: Arc<RwLock<HashMap<(String, String), PriceSnapshot>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // let state = Arc::new(tokio::sync::RwLock::new(HashMap::<
    //     (String, String),
    //     PriceSnapshot,
    // >::new()));

    //Periodic emitter → prices.latest
    {
        let state = Arc::clone(&state);
        let producer = producer.clone();
        let out_topic = cfg.out_topic.clone();
        let window_secs = cfg.emit_secs;

        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(window_secs));
            loop {
                tick.tick().await;
                let now = Utc::now().timestamp();
                let snap = {
                    let s = state.read().await;
                    s.values().cloned().collect::<Vec<_>>()
                };
                let count = snap.len(); // keep length before consuming the vec

                for mut p in snap {
                    p.ts = now;
                    let key = format!("{}:{}", p.dex, p.pair);
                    let out = serde_json::to_vec(&p).unwrap_or_default();
                    if let Err((e, _)) = producer
                        .send(
                            FutureRecord::to(&out_topic).key(&key).payload(&out),
                            Duration::from_secs(5),
                        )
                        .await
                    {
                        log::error!("emit error: {:?}", e);
                    }
                }

                log::debug!("emitted {} price snapshots", count);
            }
        });
    }

    log::info!(
        "price-aggregator started: in='{}' out='{}'",
        cfg.in_topic,
        cfg.out_topic
    );

    // Main consume loop
    loop {
        match consumer.recv().await {
            Err(e) => log::error!("consumer error: {:?}", e),
            Ok(m) => {
                if let Some(p) = m.payload() {
                    match serde_json::from_slice::<NormalizedSwap>(p) {
                        Ok(ns) => {
                            if let (Some(ti), Some(ai), Some(to), Some(ao)) = (
                                ns.token_in.clone(),
                                ns.amount_in,
                                ns.token_out.clone(),
                                ns.amount_out,
                            ) {
                                if ai > 0.0 && ao > 0.0 {
                                    let price = ao / ai;
                                    let pair = format!("{}/{}", ti, to);
                                    let key = (ns.dex.clone(), pair.clone());
                                    let snapshot = PriceSnapshot {
                                        ts: ns.ts,
                                        dex: ns.dex,
                                        pair,
                                        price,
                                        src_sig: ns.sig,
                                        window_secs: cfg.emit_secs,
                                    };
                                    let mut s = state.write().await;
                                    s.insert(key, snapshot);
                                }
                            }
                        }
                        Err(e) => {
                            // Not our JSON? Ignore gracefully (normalizer would DLQ its own failures).
                            log::debug!("parse normalized json failed: {:?}", e);
                        }
                    }
                }
                if let Err(e) = consumer.commit_message(&m, CommitMode::Async) {
                    log::warn!("commit error: {:?}", e);
                }
            }
        }
    }
}
