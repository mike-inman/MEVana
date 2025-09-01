use arb_engine::circuit::circuit_on_message;
use arb_engine::edge_integration::decode_and_validate_edge;
use arb_engine::engine_fast::FastEngine;
use arb_engine::hotpath::SignalQueue;
use arb_engine::metrics_http::start_http_control;
use arb_engine::stats::{
    record_ingest_latency_ns, CIRCUIT_OPEN, EDGE_DECODE_ERRORS, EDGE_PROCESS_ERRORS, EDGE_TOTAL,
};
use metrics::counter;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

// WebSocket + metrics server logic moved into metrics_http.rs helper

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("arb_engine=info".parse()?))
        .init();

    // Metrics recorder
    let handle: PrometheusHandle = PrometheusBuilder::new().install_recorder()?;

    // Ring buffer & engine
    let q = Arc::new(SignalQueue::new(1024));
    let tx = q.sender();
    let engine = Arc::new(FastEngine::new(tx.clone()));

    // Edge consumer (Kafka prost edge deltas)
    let broker = std::env::var("KAFKA_BROKER").unwrap_or("localhost:29092".into());
    let edge_topic = std::env::var("EDGE_DELTA_TOPIC").unwrap_or("edge.delta.v1".into());

    // Build Kafka consumer manually here to decode and feed engine
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "arb-engine.edge")
        .set("bootstrap.servers", &broker)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()?;
    consumer.subscribe(&[&edge_topic])?;

    // Spawn task to read edge deltas
    let engine_clone = engine.clone();
    // Simple ingestion counter incremented per accepted edge (rate derivable externally).

    tokio::spawn(async move {
        use std::time::Instant;
        loop {
            match consumer.recv().await {
                Ok(msg) => {
                    let ingest_start = Instant::now();
                    if let Some(payload) = msg.payload() {
                        counter!("edge_messages_total");
                        let mut msg_error = false;
                        match decode_and_validate_edge(payload) {
                            Ok(delta) => {
                                if let Err(e) = engine_clone.process_edge_update(delta) {
                                    tracing::error!(error=?e, "edge_process_failed");
                                    counter!("edge_rejected_total", "reason" => "process_error");
                                    EDGE_PROCESS_ERRORS
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    msg_error = true;
                                }
                                EDGE_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                            Err(reason) => {
                                counter!("edge_decode_errors_total", "reason" => reason);
                                EDGE_DECODE_ERRORS
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                EDGE_TOTAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                msg_error = true;
                            }
                        }
                        // record ingest latency (receive -> processed or rejected)
                        let ns = ingest_start.elapsed().as_nanos() as u64;
                        record_ingest_latency_ns(ns);
                        let now_ns = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as u64;
                        circuit_on_message(msg_error, now_ns);
                    }
                    let _ = consumer.commit_message(&msg, rdkafka::consumer::CommitMode::Async);
                }
                Err(e) => {
                    tracing::warn!(error=?e, "kafka_recv_error");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
        }
    });

    // Start HTTP control plane (/metrics + /ws/arbs)
    let addr: SocketAddr = std::env::var("WS_BIND")
        .unwrap_or("0.0.0.0:8080".into())
        .parse()?;
    let http_q = if std::env::var("DISABLE_WS").ok().as_deref() == Some("1") {
        None
    } else {
        Some(q.clone())
    };
    tokio::spawn(async move {
        if let Err(e) = start_http_control(addr, handle.clone(), http_q).await {
            tracing::error!(error=?e, "http_control_failed");
        }
    });
    tracing::info!(%addr, "http_control_started");

    #[cfg(feature = "diagnostics")]
    {
        use std::time::Duration;
        let engine_metrics = engine.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                engine_metrics.refresh_structural_gauges();
            }
        });
    }
    Ok(())
}
