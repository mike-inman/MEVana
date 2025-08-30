// crates/normalizer-service/src/kafka.rs
use anyhow::Result;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::consumer::StreamConsumer; // (no need to import `Consumer` if unused)
use serde::Serialize;
use std::time::Duration;

pub fn make_producer(broker: &str) -> Result<FutureProducer> {
    Ok(ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("linger.ms", "0")
        .set("compression.type", "lz4")
        .set("enable.idempotence", "true")
        .create()?)
}

pub async fn publish_json<T: Serialize>(
    producer: &FutureProducer,
    topic: &str,
    key: &str,
    value: &T,
) -> Result<()> {
    let payload = serde_json::to_vec(value)?;
    producer
        .send(FutureRecord::to(topic).key(key).payload(&payload), Duration::from_millis(0))
        .await
        .map_err(|(e, _)| anyhow::anyhow!(e))?;
    Ok(())
}

/// Fire-and-forget (hot path). Serialize **before** spawning so the future is Send+'static.
pub fn publish_json_ff<T>(producer: FutureProducer, topic: String, key: String, value: T)
where
    T: Serialize + Send + 'static,
{
    // Own the bytes; no references captured across the spawn.
    let payload = match serde_json::to_vec(&value) {
        Ok(b) => b,
        Err(_) => return, // swallow in FF path (or log)
    };

    tokio::spawn(async move {
        // Best-effort send; ignore error in FF path (or add tracing)
        let _ = producer
            .send(
                FutureRecord::to(&topic).key(&key).payload(&payload),
                Duration::from_millis(0),
            )
            .await;
    });
}

pub fn make_consumer(broker: &str, group: &str) -> Result<StreamConsumer> {
    Ok(ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("group.id", group)
        .set("enable.auto.commit", "false")
        .set("socket.keepalive.enable", "true")
        .set("fetch.wait.max.ms", "0")
        .set("queued.min.messages", "1")
        .set("queued.max.messages.kbytes", "1024")
        .set("isolation.level", "read_committed")
        .create()?)
}
