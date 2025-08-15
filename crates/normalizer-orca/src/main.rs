use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use dotenv::dotenv;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::{BorrowedHeaders, BorrowedMessage, Headers, Message as _};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::env;
use std::time::Duration;

//use prost::Message;
use yellowstone_grpc_proto::prost::Message as ProstMessage;

// Yellowstone gRPC (prost-generated types)
use yellowstone_grpc_proto::prelude as ypp;

// Solana pubkey encoding
//use bs58;

// modern base64 API

// ---------- Config ----------

#[derive(Debug)]
struct Cfg {
    broker: String,
    in_topic: String,  // e.g., "orca-swaps"
    out_topic: String, // e.g., "orca-swaps.parsed"
    dlq_topic: String, // e.g., "dlq"
    group_id: String,  // e.g., "normalizer-orca"
}

impl Cfg {
    fn load() -> Self {
        dotenv().ok();
        env_logger::init();

        let broker = env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
        let in_topic = env::var("IN_TOPIC").unwrap_or_else(|_| "orca-swaps".into());
        let out_topic = env::var("OUT_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed".into());
        let dlq_topic = env::var("DLQ_TOPIC").unwrap_or_else(|_| "dlq".into());
        let group_id = env::var("GROUP_ID").unwrap_or_else(|_| "normalizer-orca".into());

        log::info!(
            "config: broker={} in_topic={} out_topic={} dlq_topic={} group_id={}",
            broker,
            in_topic,
            out_topic,
            dlq_topic,
            group_id
        );

        Self {
            broker,
            in_topic,
            out_topic,
            dlq_topic,
            group_id,
        }
    }
}

// ---------- Normalized schema ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NormalizedSwap {
    ts: i64,
    slot: u64,
    sig: String,
    dex: String, // "orca"
    // Best-effort inference from signer balance deltas
    token_in: Option<String>,  // mint
    amount_in: Option<f64>,    // UI amount
    token_out: Option<String>, // mint
    amount_out: Option<f64>,   // UI amount
    // Extras for debugging
    signers: Vec<String>,
    mints_in_tx: Vec<String>,
}

// ---------- Utilities ----------

fn ui_amount_to_f64(amount_str: &str, decimals: u32) -> Option<f64> {
    let v = amount_str.parse::<u128>().ok()?;
    let scale = 10u128.checked_pow(decimals).unwrap_or(1);
    Some((v as f64) / (scale as f64))
}

fn headers_to_map(headers: Option<&BorrowedHeaders>) -> serde_json::Value {
    match headers {
        None => serde_json::json!([]),
        Some(hs) => {
            let mut out = Vec::with_capacity(hs.count());
            for i in 0..hs.count() {
                let h = hs.get(i);
                out.push(serde_json::json!({
                    "key": h.key,
                    "value_b64": h.value
                        .map(|b| general_purpose::STANDARD.encode(b))
                        .unwrap_or_default(),
                }));
            }
            serde_json::json!(out)
        }
    }
}

fn decode_transaction(payload: &[u8]) -> anyhow::Result<ypp::SubscribeUpdateTransaction> {
    // `dex-stream-orca` sent prost-encoded `SubscribeUpdateTransaction`
    Ok(ypp::SubscribeUpdateTransaction::decode(payload)?)
}

#[derive(Default)]
struct MintDelta {
    pos: f64,
    neg: f64,
}

fn signer_set_from_info(info: &ypp::SubscribeUpdateTransactionInfo) -> HashSet<String> {
    // In v4.2.x:
    // info.transaction: Option<VersionedTransaction>
    //   -> .message: Option<VersionedMessage> { header, account_keys, ... }
    let mut signers = HashSet::new();
    if let Some(vtx) = info.transaction.as_ref() {
        if let Some(msg) = vtx.message.as_ref() {
            if let Some(h) = msg.header.as_ref() {
                let req = h.num_required_signatures as usize;
                for (i, k) in msg.account_keys.iter().enumerate() {
                    if i < req {
                        signers.insert(bs58::encode(k).into_string());
                    }
                }
            }
        }
    }
    signers
}

fn infer_swap_from_balances(
    signers: &HashSet<String>,
    pre: &[ypp::TokenBalance],
    post: &[ypp::TokenBalance],
) -> (
    Option<String>,
    Option<f64>,
    Option<String>,
    Option<f64>,
    Vec<String>,
) {
    #[derive(Hash, Eq, PartialEq)]
    struct Key {
        owner: String,
        mint: String,
    }
    let mut pre_map: HashMap<Key, (u32, String)> = HashMap::new();
    let mut post_map: HashMap<Key, (u32, String)> = HashMap::new();
    let mut all_mints: HashSet<String> = HashSet::new();

    for b in pre {
        let owner = b.owner.clone(); // String (not Option) in these protos
        let mint = b.mint.clone();
        all_mints.insert(mint.clone());
        let dec = b
            .ui_token_amount
            .as_ref()
            .map(|x| x.decimals as u32)
            .unwrap_or(0);
        let raw = b
            .ui_token_amount
            .as_ref()
            .map(|x| x.amount.clone())
            .unwrap_or_else(|| "0".to_string());
        pre_map.insert(Key { owner, mint }, (dec, raw));
    }
    for b in post {
        let owner = b.owner.clone();
        let mint = b.mint.clone();
        all_mints.insert(mint.clone());
        let dec = b
            .ui_token_amount
            .as_ref()
            .map(|x| x.decimals as u32)
            .unwrap_or(0);
        let raw = b
            .ui_token_amount
            .as_ref()
            .map(|x| x.amount.clone())
            .unwrap_or_else(|| "0".to_string());
        post_map.insert(Key { owner, mint }, (dec, raw));
    }

    // deltas per (owner, mint) but only for signer owners
    let mut per_mint: HashMap<String, MintDelta> = HashMap::new();

    for (key, (dec_p, raw_p)) in pre_map.iter() {
        if !signers.contains(&key.owner) {
            continue;
        }
        let (dec_post, raw_post) = post_map.get(key).cloned().unwrap_or((*dec_p, "0".into()));
        let pre_f = ui_amount_to_f64(raw_p, *dec_p).unwrap_or(0.0);
        let post_f = ui_amount_to_f64(&raw_post, dec_post).unwrap_or(0.0);
        let d = post_f - pre_f;
        if d.abs() < 1e-12 {
            continue;
        }
        let e = per_mint.entry(key.mint.clone()).or_default();
        if d > 0.0 {
            e.pos += d;
        } else {
            e.neg += -d;
        }
    }

    // accounts only in post (new signer ATA with positive balance)
    for (key, (dec_post, raw_post)) in post_map.iter() {
        if !signers.contains(&key.owner) || pre_map.contains_key(key) {
            continue;
        }
        let post_f = ui_amount_to_f64(raw_post, *dec_post).unwrap_or(0.0);
        if post_f > 0.0 {
            let e = per_mint.entry(key.mint.clone()).or_default();
            e.pos += post_f;
        }
    }

    // Pick biggest negative as input, biggest positive as output
    let mut best_in: Option<(String, f64)> = None;
    let mut best_out: Option<(String, f64)> = None;

    for (mint, md) in per_mint {
        if md.neg > 0.0 && best_in.as_ref().map(|x| x.1).unwrap_or(0.0) < md.neg {
            best_in = Some((mint.clone(), md.neg));
        }
        if md.pos > 0.0 && best_out.as_ref().map(|x| x.1).unwrap_or(0.0) < md.pos {
            best_out = Some((mint.clone(), md.pos));
        }
    }

    (
        best_in.as_ref().map(|x| x.0.clone()),
        best_in.map(|x| x.1),
        best_out.as_ref().map(|x| x.0.clone()),
        best_out.map(|x| x.1),
        all_mints.into_iter().collect(),
    )
}

fn normalized_from_tx(tx: &ypp::SubscribeUpdateTransaction) -> Option<NormalizedSwap> {
    let slot = tx.slot;
    let info = tx.transaction.as_ref()?;

    let sig = bs58::encode(&info.signature).into_string();
    let signers = signer_set_from_info(info);

    let (token_in, amount_in, token_out, amount_out, all_mints) = {
        let meta = info.meta.as_ref()?;
        let pre = &meta.pre_token_balances;
        let post = &meta.post_token_balances;
        infer_swap_from_balances(&signers, pre, post)
    };

    Some(NormalizedSwap {
        ts: Utc::now().timestamp(),
        slot,
        sig,
        dex: "orca".into(),
        token_in,
        amount_in,
        token_out,
        amount_out,
        signers: signers.into_iter().collect(),
        mints_in_tx: all_mints,
    })
}

// ---------- Kafka wiring ----------

fn make_consumer(cfg: &Cfg) -> Result<StreamConsumer> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.broker)
        .set("group.id", &cfg.group_id)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()?;

    consumer.subscribe(&[&cfg.in_topic])?;
    Ok(consumer)
}

fn make_producer(cfg: &Cfg) -> Result<FutureProducer> {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &cfg.broker)
        .set("linger.ms", "5")
        .set("compression.type", "lz4")
        .set("acks", "all")
        .create()?;
    Ok(producer)
}

async fn handle_msg(cfg: &Cfg, producer: &FutureProducer, m: &BorrowedMessage<'_>) -> Result<()> {
    let payload = match m.payload() {
        Some(p) => p,
        None => {
            log::warn!(
                "empty payload (offset={} partition={})",
                m.offset(),
                m.partition()
            );
            return Ok(());
        }
    };

    match decode_transaction(payload) {
        Ok(tx) => {
            if let Some(n) = normalized_from_tx(&tx) {
                let key = n.sig.clone();
                let out = serde_json::to_vec(&n)?;
                producer
                    .send(
                        FutureRecord::to(&cfg.out_topic).key(&key).payload(&out),
                        Duration::from_secs(5),
                    )
                    .await
                    .map_err(|(e, _)| anyhow::anyhow!(e))?;
                log::debug!("normalized swap -> {} key={}", &cfg.out_topic, key);
            } else {
                let info = json!({
                    "reason": "normalizer-orca:insufficient_fields",
                    "ts": Utc::now().timestamp(),
                });
                let key = format!("slot:{}:{}", m.partition(), m.offset());
                let out = serde_json::to_vec(&info)?;
                producer
                    .send(
                        FutureRecord::to(&cfg.dlq_topic).key(&key).payload(&out),
                        Duration::from_secs(5),
                    )
                    .await
                    .map_err(|(e, _)| anyhow::anyhow!(e))?;
                log::warn!("sent to dlq (missing fields) key={}", key);
            }
        }
        Err(e) => {
            // DLQ with payload/headers for later forensics
            let key = format!("decode_error:{}:{}", m.partition(), m.offset());
            let dlq_obj = json!({
                "err": format!("{e:#}"),
                "ts": Utc::now().timestamp(),
                "headers": headers_to_map(m.headers()),
                "payload_b64": general_purpose::STANDARD.encode(payload),
                "topic": m.topic(),
                "partition": m.partition(),
                "offset": m.offset(),
            });
            let out = serde_json::to_vec(&dlq_obj)?;
            producer
                .send(
                    FutureRecord::to(&cfg.dlq_topic).key(&key).payload(&out),
                    Duration::from_secs(5),
                )
                .await
                .map_err(|(e, _)| anyhow::anyhow!(e))?;
            log::error!("decode failed; sent to dlq key={}", key);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Cfg::load();

    let consumer = make_consumer(&cfg)?;
    let producer = make_producer(&cfg)?;

    log::info!(
        "normalizer-orca started: in='{}' out='{}'",
        cfg.in_topic,
        cfg.out_topic
    );

    loop {
        match consumer.recv().await {
            Err(e) => log::error!("consumer error: {:?}", e),
            Ok(m) => {
                if let Err(e) = handle_msg(&cfg, &producer, &m).await {
                    log::error!("handle_msg error: {:?}", e);
                }
                if let Err(e) = consumer.commit_message(&m, CommitMode::Async) {
                    log::warn!("commit error: {:?}", e);
                }
            }
        }
    }
}
