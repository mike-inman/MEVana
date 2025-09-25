use std::{fmt::Debug, sync::Arc, time::Duration};

use log::{debug, error, info, warn};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::BorrowedMessage,
    Message,
};
use tokio::{
    sync::broadcast,
    task::{self, JoinHandle},
};

use super::config::GatewayConfig;

const BROADCAST_CAPACITY: usize = 256;
const CONSUMER_SESSION_TIMEOUT_MS: &str = "6000";
const CONSUMER_POLL_BACKOFF: Duration = Duration::from_millis(250);

/// Wrapper around the spawned Kafka consumer tasks so callers can manage
/// lifecycle if needed.
#[derive(Debug)]
pub struct GatewayConsumerHandles {
    handles: Vec<JoinHandle<()>>,
}

impl GatewayConsumerHandles {
    pub fn new(handles: Vec<JoinHandle<()>>) -> Self {
        Self { handles }
    }

    /// Abort all running consumer tasks.
    pub fn abort(self) {
        for handle in self.handles {
            handle.abort();
        }
    }
}

#[derive(Debug, Clone)]
pub struct GatewayWebsocketManager {
    config: Arc<GatewayConfig>,
    prices_tx: broadcast::Sender<String>,
    swaps_tx: broadcast::Sender<String>,
    swaps_v2_tx: broadcast::Sender<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn websocket_manager_is_send_sync() {
        assert_send_sync::<GatewayWebsocketManager>();
    }
}

impl GatewayWebsocketManager {
    pub fn new(config: Arc<GatewayConfig>) -> Self {
        let (prices_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (swaps_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (swaps_v2_tx, _) = broadcast::channel(BROADCAST_CAPACITY);

        Self {
            config,
            prices_tx,
            swaps_tx,
            swaps_v2_tx,
        }
    }

    /// Spawn background Kafka consumers that publish payloads into broadcast
    /// channels for WebSocket handlers to consume.
    pub fn spawn_kafka_tasks(&self) -> GatewayConsumerHandles {
        let mut handles = Vec::with_capacity(3);

        handles.push(self.spawn_consumer_task(
            self.config.prices_topic.clone(),
            self.prices_tx.clone(),
            "prices",
        ));

        handles.push(self.spawn_consumer_task(
            self.config.swaps_topic.clone(),
            self.swaps_tx.clone(),
            "swaps",
        ));

        handles.push(self.spawn_consumer_task(
            self.config.swaps_v2_topic.clone(),
            self.swaps_v2_tx.clone(),
            "swaps_v2",
        ));

        GatewayConsumerHandles::new(handles)
    }

    fn spawn_consumer_task(
        &self,
        topic: String,
        sender: broadcast::Sender<String>,
        label: &'static str,
    ) -> JoinHandle<()> {
        let broker = self.config.kafka_broker.clone();

        task::spawn(async move {
            match Self::build_consumer(&broker, label) {
                Ok(consumer) => {
                    if let Err(e) = consumer.subscribe(&[&topic]) {
                        error!("gateway kafka subscribe failed topic={} err={}", topic, e);
                        return;
                    }

                    info!(
                        "gateway kafka consumer started label={} topic={} broker={}",
                        label, topic, broker
                    );

                    loop {
                        match consumer.recv().await {
                            Ok(message) => {
                                if let Some(payload) = Self::message_to_string(&message) {
                                    if sender.send(payload).is_err() {
                                        debug!("gateway kafka no active receivers label={}", label);
                                    }
                                }
                            }
                            Err(err) => {
                                warn!(
                                    "gateway kafka recv error label={} err={} -- backing off",
                                    label, err
                                );
                                tokio::time::sleep(CONSUMER_POLL_BACKOFF).await;
                            }
                        }
                    }
                }
                Err(err) => {
                    error!(
                        "gateway kafka consumer init failed label={} err={}",
                        label, err
                    );
                }
            }
        })
    }

    fn build_consumer(
        broker: &str,
        label: &str,
    ) -> Result<StreamConsumer, rdkafka::error::KafkaError> {
        ClientConfig::new()
            .set("bootstrap.servers", broker)
            .set("group.id", format!("gateway-{}-group", label))
            .set("session.timeout.ms", CONSUMER_SESSION_TIMEOUT_MS)
            .set("enable.auto.commit", "true")
            .set("enable.partition.eof", "false")
            .set("auto.offset.reset", "latest")
            .create()
    }

    fn message_to_string(message: &BorrowedMessage<'_>) -> Option<String> {
        match message.payload_view::<str>() {
            Some(Ok(text)) => Some(text.to_owned()),
            Some(Err(err)) => {
                warn!("gateway kafka payload decode error err={}", err);
                None
            }
            None => None,
        }
    }

    pub fn subscribe_prices(&self) -> broadcast::Receiver<String> {
        self.prices_tx.subscribe()
    }

    pub fn subscribe_swaps(&self) -> broadcast::Receiver<String> {
        self.swaps_tx.subscribe()
    }

    pub fn subscribe_swaps_v2(&self) -> broadcast::Receiver<String> {
        self.swaps_v2_tx.subscribe()
    }

    pub fn readiness_status(&self) -> String {
        format!(
            "websocket:port={} prices_rx={} swaps_rx={} swaps_v2_rx={}",
            self.config.gateway_port,
            self.prices_tx.receiver_count(),
            self.swaps_tx.receiver_count(),
            self.swaps_v2_tx.receiver_count()
        )
    }
}
