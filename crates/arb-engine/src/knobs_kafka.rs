use crate::config::{set_knobs, Knobs};
use rdkafka::Message;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig,
};

pub async fn run_knobs_consumer(broker: &str, topic: &str) -> anyhow::Result<()> {
    let c: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("group.id", "mevana.knobs")
        .set("auto.offset.reset", "latest")
        .create()?;
    c.subscribe(&[topic])?;

    loop {
        let m = c.recv().await?;
        if let Some(Ok(payload)) = m.payload_view::<str>() {
            if let Ok(k) = serde_json::from_str::<Knobs>(payload) {
                set_knobs(k);
                tracing::info!("🔧 updated knobs from Kafka");
            }
        }
    }
}
