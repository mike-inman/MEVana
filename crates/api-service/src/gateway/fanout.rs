use std::{sync::Arc, time::Duration};

use chrono::Utc;
use log::{debug, error, info, warn};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::BorrowedMessage,
    Message,
};
use serde_json::Value;
use tokio::{
    sync::broadcast,
    task::{self, JoinHandle},
};

use super::config::GatewayConfig;

const BROADCAST_CAPACITY: usize = 512;
const CONSUMER_SESSION_TIMEOUT_MS: &str = "6000";
const CONSUMER_POLL_BACKOFF: Duration = Duration::from_millis(250);
const SOL_LAMPORTS: f64 = 1_000_000_000.0;
const SOL_USD_FALLBACK: f64 = 150.0;

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

#[derive(Clone)]
pub struct GatewayFanout {
    config: Arc<GatewayConfig>,
    prices_tx: broadcast::Sender<String>,
    swaps_tx: broadcast::Sender<String>,
    swaps_v2_tx: broadcast::Sender<String>,
    opportunities_tx: broadcast::Sender<String>,
    bundles_tx: broadcast::Sender<String>,
    fills_tx: broadcast::Sender<String>,
}

impl GatewayFanout {
    pub fn new(config: Arc<GatewayConfig>) -> Self {
        let (prices_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (swaps_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (swaps_v2_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (opportunities_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (bundles_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (fills_tx, _) = broadcast::channel(BROADCAST_CAPACITY);

        Self {
            config,
            prices_tx,
            swaps_tx,
            swaps_v2_tx,
            opportunities_tx,
            bundles_tx,
            fills_tx,
        }
    }

    /// Spawn background Kafka consumers that publish payloads into broadcast
    /// channels for downstream WebSocket and SSE handlers.
    pub fn spawn_kafka_tasks(&self) -> GatewayConsumerHandles {
        let mut handles = Vec::with_capacity(5);

        handles.push(self.spawn_passthrough_consumer(
            self.config.prices_topic.clone(),
            self.prices_tx.clone(),
            "prices",
        ));

        handles.push(self.spawn_passthrough_consumer(
            self.config.swaps_topic.clone(),
            self.swaps_tx.clone(),
            "swaps",
        ));

        handles.push(self.spawn_passthrough_consumer(
            self.config.swaps_v2_topic.clone(),
            self.swaps_v2_tx.clone(),
            "swaps_v2",
        ));

        handles.push(self.spawn_opportunities_consumer());
        handles.push(self.spawn_execution_consumer());

        GatewayConsumerHandles::new(handles)
    }

    fn spawn_passthrough_consumer(
        &self,
        topic: String,
        sender: broadcast::Sender<String>,
        label: &'static str,
    ) -> JoinHandle<()> {
        let broker = self.config.kafka_broker.clone();

        task::spawn(async move {
            match build_consumer(&broker, label) {
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
                                if let Some(payload) = message_to_string(&message) {
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

    fn spawn_opportunities_consumer(&self) -> JoinHandle<()> {
        let broker = self.config.kafka_broker.clone();
        let topic = self.config.opportunities_topic.clone();
        let sender = self.opportunities_tx.clone();

        task::spawn(async move {
            match build_consumer(&broker, "opportunities") {
                Ok(consumer) => {
                    if let Err(e) = consumer.subscribe(&[&topic]) {
                        error!("gateway kafka subscribe failed topic={} err={}", topic, e);
                        return;
                    }

                    info!(
                        "gateway kafka consumer started label=opportunities topic={} broker={}",
                        topic, broker
                    );

                    loop {
                        match consumer.recv().await {
                            Ok(message) => {
                                if let Some(raw) = message_to_string(&message) {
                                    if let Some(transformed) = transform_opportunity(&raw) {
                                        if sender.send(transformed).is_err() {
                                            debug!(
                                                "gateway kafka no receivers label=opportunities"
                                            );
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                warn!(
                                    "gateway kafka recv error label=opportunities err={} -- backing off",
                                    err
                                );
                                tokio::time::sleep(CONSUMER_POLL_BACKOFF).await;
                            }
                        }
                    }
                }
                Err(err) => error!(
                    "gateway kafka consumer init failed label=opportunities err={}",
                    err
                ),
            }
        })
    }

    fn spawn_execution_consumer(&self) -> JoinHandle<()> {
        let broker = self.config.kafka_broker.clone();
        let topic = self.config.execution_results_topic.clone();
        let bundles_tx = self.bundles_tx.clone();
        let fills_tx = self.fills_tx.clone();
        let bundles_fallback_topic = self.config.bundles_topic.clone();
        let fills_fallback_topic = self.config.fills_topic.clone();

        task::spawn(async move {
            match build_consumer(&broker, "execution") {
                Ok(consumer) => {
                    let topics = if topic == bundles_fallback_topic && topic == fills_fallback_topic
                    {
                        vec![topic]
                    } else {
                        let mut list = vec![topic.clone()];
                        if bundles_fallback_topic != topic {
                            list.push(bundles_fallback_topic.clone());
                        }
                        if fills_fallback_topic != topic
                            && fills_fallback_topic != bundles_fallback_topic
                        {
                            list.push(fills_fallback_topic.clone());
                        }
                        list
                    };

                    let topic_refs: Vec<&str> = topics.iter().map(String::as_str).collect();
                    if let Err(e) = consumer.subscribe(&topic_refs) {
                        error!(
                            "gateway kafka subscribe failed topics={:?} err={}",
                            topic_refs, e
                        );
                        return;
                    }

                    info!(
                        "gateway kafka consumer started label=execution topics={:?} broker={}",
                        topic_refs, broker
                    );

                    loop {
                        match consumer.recv().await {
                            Ok(message) => {
                                if let Some(raw) = message_to_string(&message) {
                                    if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                                        if let Some(bundle_payload) =
                                            transform_execution_for_bundle(&value)
                                        {
                                            if bundles_tx.send(bundle_payload).is_err() {
                                                debug!("gateway kafka no receivers label=bundles");
                                            }
                                        }

                                        if let Some(fill_payload) =
                                            transform_execution_for_fill(&value)
                                        {
                                            if fills_tx.send(fill_payload).is_err() {
                                                debug!("gateway kafka no receivers label=fills");
                                            }
                                        }
                                    } else {
                                        warn!(
                                            "gateway execution payload parse error -- discarding"
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                warn!(
                                    "gateway kafka recv error label=execution err={} -- backing off",
                                    err
                                );
                                tokio::time::sleep(CONSUMER_POLL_BACKOFF).await;
                            }
                        }
                    }
                }
                Err(err) => error!(
                    "gateway kafka consumer init failed label=execution err={}",
                    err
                ),
            }
        })
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

    pub fn subscribe_opportunities(&self) -> broadcast::Receiver<String> {
        self.opportunities_tx.subscribe()
    }

    pub fn subscribe_bundles(&self) -> broadcast::Receiver<String> {
        self.bundles_tx.subscribe()
    }

    pub fn subscribe_fills(&self) -> broadcast::Receiver<String> {
        self.fills_tx.subscribe()
    }

    pub fn readiness_status(&self) -> String {
        format!(
            "fanout:port={} prices_rx={} swaps_rx={} swaps_v2_rx={} opps_rx={} bundles_rx={} fills_rx={}",
            self.config.gateway_port,
            self.prices_tx.receiver_count(),
            self.swaps_tx.receiver_count(),
            self.swaps_v2_tx.receiver_count(),
            self.opportunities_tx.receiver_count(),
            self.bundles_tx.receiver_count(),
            self.fills_tx.receiver_count()
        )
    }
}

fn build_consumer(broker: &str, label: &str) -> Result<StreamConsumer, rdkafka::error::KafkaError> {
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

fn transform_opportunity(raw: &str) -> Option<String> {
    let value: Value = serde_json::from_str(raw).ok()?;

    let signature = value.get("signature").and_then(Value::as_str)?.to_owned();
    let token_pair = value
        .get("token_pair")
        .and_then(Value::as_str)
        .map(|s| s.to_owned())
        .or_else(|| {
            let token_a = value
                .get("token_a")
                .and_then(Value::as_str)
                .unwrap_or("TOKEN_A");
            let token_b = value
                .get("token_b")
                .and_then(Value::as_str)
                .unwrap_or("TOKEN_B");
            Some(format!("{}/{}", token_a, token_b))
        })
        .unwrap_or_else(|| "UNKNOWN/UNKNOWN".to_string());

    let venues = value
        .get("venues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let venue_a = venues.get(0).and_then(Value::as_str).unwrap_or("VenueA");
    let venue_b = venues.get(1).and_then(Value::as_str).unwrap_or(venue_a);

    let profit_bps = value.get("profit_bps").and_then(as_f64).unwrap_or(0.0);

    let estimated_profit_sol = value
        .get("estimated_profit_sol")
        .and_then(as_f64)
        .unwrap_or(0.0);

    let details = value.get("details");
    let price_buy = details
        .and_then(|d| d.get("price_buy"))
        .and_then(as_f64)
        .unwrap_or(0.0);
    let price_sell = details
        .and_then(|d| d.get("price_sell"))
        .and_then(as_f64)
        .unwrap_or(0.0);
    let size_in = details
        .and_then(|d| d.get("size_in"))
        .and_then(Value::as_str)
        .unwrap_or("0");

    let ev_usd = details
        .and_then(|d| d.get("ev_usd"))
        .and_then(as_f64)
        .unwrap_or(estimated_profit_sol * SOL_USD_FALLBACK);

    let payload = serde_json::json!({
        "id": signature,
        "ts_ms": Utc::now().timestamp_millis(),
        "pair": token_pair,
        "buy_on": venue_b,
        "sell_on": venue_a,
        "price_buy": price_buy,
        "price_sell": price_sell,
        "edge_bps": profit_bps,
        "ev_usd": ev_usd,
        "size_in": size_in,
    });

    serde_json::to_string(&payload).ok()
}

fn transform_execution_for_bundle(value: &Value) -> Option<String> {
    let execution_id = value
        .get("execution_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if execution_id.is_empty() {
        return None;
    }

    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("Received");

    let status = map_bundle_status(status)?;

    let bundle_payload = serde_json::json!({
        "id": execution_id,
        "status": status,
        "cu_limit": value
            .get("gas_used")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        "tip_lamports": value
            .get("estimated_priority_fee_lamports")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        "tx_count": value
            .get("transaction_signatures")
            .and_then(Value::as_array)
            .map(|arr| arr.len())
            .unwrap_or_default(),
        "expiry_slot": 0,
        "bundle_id": value.get("bundle_id").and_then(Value::as_str),
        "ts_ms": value
            .get("executed_at_ns")
            .and_then(Value::as_i64)
            .map(|ns| ns / 1_000_000)
            .unwrap_or_else(|| Utc::now().timestamp_millis()),
        "error": value.get("error_message").and_then(Value::as_str),
    });

    serde_json::to_string(&bundle_payload).ok()
}

fn transform_execution_for_fill(value: &Value) -> Option<String> {
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("Received");

    if !matches!(status, "Confirmed" | "Finalized") {
        return None;
    }

    let sig = value
        .get("transaction_signatures")
        .and_then(Value::as_array)
        .and_then(|arr| arr.get(0))
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN");

    let executed_at_ms = value
        .get("executed_at_ns")
        .and_then(Value::as_i64)
        .map(|ns| ns / 1_000_000)
        .unwrap_or_else(|| Utc::now().timestamp_millis());

    let est_profit_lamports = value
        .get("estimated_profit_lamports")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let actual_profit_lamports = value
        .get("actual_profit_lamports")
        .and_then(Value::as_i64)
        .unwrap_or(est_profit_lamports);

    let payload = serde_json::json!({
        "sig": sig,
        "pair": "UNKNOWN/UNKNOWN",
        "venue": value
            .get("flash_loan_provider")
            .and_then(Value::as_str)
            .unwrap_or("N/A"),
        "amount_in": "0",
        "amount_out": "0",
        "fee_lamports": value
            .get("flash_loan_fee_lamports")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        "ev_estimate_usd": lamports_to_usd(est_profit_lamports),
        "ev_real_usd": lamports_to_usd(actual_profit_lamports),
        "slippage_bps": 0,
        "latency_ms": value
            .get("execution_time_ms")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        "slot": 0,
        "ok": !matches!(status, "Failed" | "Rejected" | "Unlanded"),
        "ts_ms": executed_at_ms,
    });

    serde_json::to_string(&payload).ok()
}

fn map_bundle_status(status: &str) -> Option<&'static str> {
    match status {
        "Received" | "Validated" | "FlashLoanQuoted" | "BundleBuilt" | "Simulated" => {
            Some("queued")
        }
        "Submitted" => Some("submitted"),
        "LandedPendingProfit" | "Confirmed" | "Finalized" => Some("landed"),
        "Failed" | "Rejected" => Some("failed"),
        "Unlanded" => Some("expired"),
        _ => Some("queued"),
    }
}

fn as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(num) => num.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn lamports_to_usd(lamports: i64) -> f64 {
    (lamports as f64 / SOL_LAMPORTS) * SOL_USD_FALLBACK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opportunity_transform_populates_defaults() {
        let raw = serde_json::json!({
            "signature": "abc",
            "venues": ["Raydium", "Meteora"],
            "profit_bps": 25,
            "token_a": "SOL",
            "token_b": "USDC",
            "estimated_profit_sol": 0.01,
            "details": {"size_in": "1000000"}
        })
        .to_string();

        let transformed = transform_opportunity(&raw).unwrap();
        let value: Value = serde_json::from_str(&transformed).unwrap();
        assert_eq!(value.get("id").unwrap(), "abc");
        assert_eq!(value.get("buy_on").unwrap(), "Meteora");
        assert_eq!(value.get("sell_on").unwrap(), "Raydium");
        assert_eq!(value.get("pair").unwrap(), "SOL/USDC");
        assert!(value.get("ev_usd").unwrap().as_f64().unwrap() > 0.0);
    }

    #[test]
    fn bundle_status_mapping_handles_known_states() {
        let payload = serde_json::json!({
            "execution_id": "123",
            "status": "Submitted",
            "gas_used": 150000,
            "transaction_signatures": ["sig1"],
            "executed_at_ns": 1_700_000_000_000_000_000_i64,
        });
        let bundle = transform_execution_for_bundle(&payload).unwrap();
        let json: Value = serde_json::from_str(&bundle).unwrap();
        assert_eq!(json.get("status").unwrap(), "submitted");
        assert_eq!(json.get("tx_count").unwrap(), 1);
    }

    #[test]
    fn fill_transform_only_for_confirmed() {
        let payload = serde_json::json!({
            "execution_id": "123",
            "status": "Confirmed",
            "transaction_signatures": ["sig1"],
            "estimated_profit_lamports": 1_000_000_i64,
            "execution_time_ms": 42,
            "executed_at_ns": 1_700_000_000_000_000_000_i64
        });
        assert!(transform_execution_for_fill(&payload).is_some());

        let pending = serde_json::json!({
            "execution_id": "123",
            "status": "Submitted"
        });
        assert!(transform_execution_for_fill(&pending).is_none());
    }

    #[test]
    fn fanout_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GatewayFanout>();
    }
}
