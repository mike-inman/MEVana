// crates/normalizer-service/src/bin/tap_orca_decode.rs

use normalizer_generated::registry as generated;

use anyhow::{Context, Result};
use bs58;
use dotenv::dotenv;
use env_logger;
use log::{error, info, warn};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use yellowstone_grpc_proto::prelude as ypp;
use yellowstone_grpc_proto::prost::Message as _;

// NEW: Configuration structure with validation
#[derive(Debug, Clone)]
struct ServiceConfig {
    broker: String,
    in_topic: String,
    group_id: String,
}

impl ServiceConfig {
    fn from_env() -> Result<Self> {
        // Load environment variables with validation
        let broker = std::env::var("KAFKA_BROKER")
            .map_err(|_| anyhow::anyhow!("KAFKA_BROKER environment variable is required"))?;

        let in_topic = std::env::var("IN_TOPIC").unwrap_or_else(|_| "orca-swaps".to_string());

        let group_id = std::env::var("GROUP_ID").unwrap_or_else(|_| "tap-orca-decode".to_string());

        Ok(Self {
            broker,
            in_topic,
            group_id,
        })
    }

    fn log_config(&self) {
        info!("=== TAP ORCA DECODE SERVICE CONFIGURATION ===");
        info!("Kafka Broker: {}", self.broker);
        info!("Input Topic: {}", self.in_topic);
        info!("Group ID: {}", self.group_id);
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

    info!("tap_orca_decode: Starting decode service");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &config.broker)
        .set("group.id", &config.group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()
        .context("create consumer")?;

    if let Err(e) = consumer.subscribe(&[&config.in_topic]) {
        error!("Failed to subscribe to topic: {}", e);
        return Err(anyhow::anyhow!("Failed to subscribe: {}", e));
    }

    info!("Successfully subscribed to topic: {}", config.in_topic);

    // Let's print what decoders are registered (program_id → module)
    {
        // the generated::registry() returns extractors (we won't use them),
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

    info!("Starting message processing loop");

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
