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
use serde_json;
use std::collections::HashMap;
use yellowstone_grpc_proto::prelude as ypp;
use yellowstone_grpc_proto::prost::Message as _;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    let in_topic = std::env::var("IN_TOPIC").unwrap_or_else(|_| "orca-swaps".into());
    let out_topic = std::env::var("OUT_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed.v2".into());
    let group_id = std::env::var("GROUP_ID").unwrap_or_else(|_| "tap-orca-extract".into());

    info!("tap_orca_extract: broker={broker} in={in_topic} out={out_topic} group_id={group_id}");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", &group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()
        .context("create consumer")?;

    consumer.subscribe(&[&in_topic]).context("subscribe")?;

    // Producer for v2 output
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("compression.type", "lz4")
        .set("linger.ms", "5")
        .set("acks", "all")
        .create()
        .context("create producer")?;

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

    loop {
        match consumer.recv().await {
            Err(e) => warn!("kafka recv error: {e:?}"),
            Ok(m) => {
                if let Some(bytes) = m.payload() {
                    if let Err(e) = handle(bytes, &producer, &out_topic).await {
                        error!("handle error: {e:#}");
                    }
                } else {
                    warn!("empty payload");
                }
                let _ = consumer.commit_message(&m, CommitMode::Async);
            }
        }
    }
}

async fn handle(payload: &[u8], producer: &FutureProducer, out_topic: &str) -> Result<()> {
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
                // Emit to Kafka as JSON with key = signature
                let key = ctx.signature.clone();
                let payload = serde_json::to_vec(&n)?;
                let _ = producer
                    .send(
                        FutureRecord::to(out_topic).key(&key).payload(&payload),
                        std::time::Duration::from_secs(5),
                    )
                    .await;
                // Also echo to stdout for quick inspection
                println!("{}", serde_json::to_string(&n)?);
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
