// crates/normalizer-service/src/bin/tap_orca_decode.rs

// Pull in the generated registry (built by build.rs via normalizer-codegen)
mod generated {
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

use anyhow::{Context, Result};
use bs58;
use dotenv::dotenv;
use env_logger;
use log::{error, info, warn};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use rdkafka::Message as _;
use std::time::Duration;
use yellowstone_grpc_proto::prelude as ypp;
use yellowstone_grpc_proto::prost::Message as _;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    let in_topic = std::env::var("IN_TOPIC").unwrap_or_else(|_| "orca-swaps".into());
    let group_id = std::env::var("GROUP_ID").unwrap_or_else(|_| "tap-orca-decode".into());

    info!("tap_orca_decode: broker={broker} topic={in_topic} group_id={group_id}");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", &group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()
        .context("create consumer")?;

    consumer.subscribe(&[&in_topic]).context("subscribe")?;

    // Let’s print what decoders are registered (program_id → module)
    {
        // the generated::registry() returns extractors (we won’t use them),
        // but calling it ensures codegen is wired; we also list decoder keys:
        let mut count = 0usize;
        let pids = known_program_ids();
        for pid in &pids {
            count += 1;
            info!("registered decoder for program_id={pid}");
        }
        if count == 0 {
            warn!("No registered decoders! Check your idls/ + profiles/ and build.rs.");
        }
    }

    loop {
        match consumer.recv().await {
            Err(e) => warn!("kafka recv error: {e:?}"),
            Ok(m) => {
                if let Some(bytes) = m.payload() {
                    if let Err(e) = handle(bytes).await {
                        error!("handle error: {e:#}");
                    }
                } else {
                    warn!("empty payload");
                }
                let _ = consumer.commit_message(&m, rdkafka::consumer::CommitMode::Async);
            }
        }
    }
}

// Collect the PIDs we can decode (from generated::decoder_for match table)
fn known_program_ids() -> Vec<&'static str> {
    // There's no public iterator; we probe a small set of candidate PIDs
    // If you want, hardcode your Whirlpool PID in profiles/orca_whirlpool.yml.
    // For visibility, we brute-force a few: but simplest is to keep a static list:
    // In practice, you know this from your profile.
    // To avoid guessing, we just return a couple of well-known Anchor PIDs if present,
    // but if none match in runtime, it's harmless.
    // You can replace this entirely with a hard-coded vec![ "<your_whirlpool_pid>" ].
    vec![
        // replace this with your whirlpool program id (from your IDL metadata/address or profile YAML)
        // "whirLbMi...":  // placeholder
    ]
}

async fn handle(payload: &[u8]) -> Result<()> {
    // Decode the Yellowstone SubscribeUpdateTransaction
    let update = ypp::SubscribeUpdateTransaction::decode(payload)
        .context("prost decode SubscribeUpdateTransaction")?;

    let info = match update.transaction.as_ref() {
        Some(i) => i,
        None => return Ok(()), // nothing to do
    };

    // Build b58 account key list + iterate compiled instructions
    if let Some(msg) = info.transaction.as_ref().and_then(|t| t.message.as_ref()) {
        let keys_b58: Vec<String> = msg
            .account_keys
            .iter()
            .map(|k| bs58::encode(k).into_string())
            .collect();

        // The compiled instructions live under `message.instructions`
        // (prost name in v4.2.x is `instructions`)
        for (idx, ci) in msg.instructions.iter().enumerate() {
            let pid_index = ci.program_id_index as usize;
            if pid_index >= keys_b58.len() {
                continue;
            }
            let pid = &keys_b58[pid_index];

            if let Some(dec) = generated::decoder_for(pid) {
                // Try to decode this instruction using our generated Anchor discriminator
                if let Some((name, roles)) = dec(&ci.data, &ci.accounts, &keys_b58) {
                    // Pull some commonly interesting roles if present
                    let pool = roles
                        .get("whirlpool")
                        .or_else(|| roles.get("pool"))
                        .cloned()
                        .unwrap_or_default();
                    let user = roles
                        .get("authority")
                        .or_else(|| roles.get("userAuthority"))
                        .cloned()
                        .unwrap_or_default();

                    let sig = bs58::encode(&info.signature).into_string();

                    info!(
                        "DECODED: sig={} ix_index={} program_id={} name={} pool={} user={}",
                        sig, idx, pid, name, pool, user
                    );
                }
            }
        }
    }

    Ok(())
}
