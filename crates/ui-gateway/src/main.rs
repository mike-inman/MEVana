use anyhow::Result;
use axum::{
    extract::ws::{Message as WsMessage, WebSocketUpgrade},
    extract::State,
    routing::get,
    Router,
};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    prices: broadcast::Sender<String>,
    swaps: broadcast::Sender<String>,
    swaps_v2: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    let prices_topic = std::env::var("PRICES_TOPIC").unwrap_or_else(|_| "prices.latest".into());
    let swaps_topic = std::env::var("SWAPS_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed".into());
    let swaps_v2_topic =
        std::env::var("SWAPS_V2_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed.v2".into());

    let (prices_tx, _) = broadcast::channel::<String>(2048);
    let (swaps_tx, _) = broadcast::channel::<String>(2048);
    let (swaps_v2_tx, _) = broadcast::channel::<String>(2048);
    let state = AppState {
        prices: prices_tx.clone(),
        swaps: swaps_tx.clone(),
        swaps_v2: swaps_v2_tx.clone(),
    };

    // Kafka consumers
    tokio::spawn(consume_loop(
        broker.clone(),
        prices_topic.clone(),
        prices_tx.clone(),
        "prices",
    ));
    tokio::spawn(consume_loop(
        broker.clone(),
        swaps_topic.clone(),
        swaps_tx.clone(),
        "swaps",
    ));
    tokio::spawn(consume_loop(
        broker.clone(),
        swaps_v2_topic.clone(),
        swaps_v2_tx.clone(),
        "swaps_v2",
    ));

    // HTTP/WS
    let app = Router::new()
        .route("/ws/prices", get(ws_prices))
        .route("/ws/swaps", get(ws_swaps))
        .route("/ws/swaps.v2", get(ws_swaps_v2))
        .route("/ws/ping", get(ws_ping))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(Arc::new(state));

    let addr = "0.0.0.0:8081";
    let listener = TcpListener::bind(addr).await?;
    info!("ui-gateway listening on {addr}");
    info!("consuming topic={}", prices_topic);
    info!("consuming topic={}", swaps_topic);
    info!("consuming topic={}", swaps_v2_topic);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn consume_loop(
    broker: String,
    topic: String,
    tx: broadcast::Sender<String>,
    label: &'static str,
) {
    let c: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", format!("ui-{}-group", label))
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "latest")
        .create()
        .expect("consumer");
    c.subscribe(&[&topic]).expect("subscribe");

    loop {
        match c.recv().await {
            Ok(m) => {
                if let Some(p) = m.payload_view::<str>().and_then(|r| r.ok()) {
                    let _ = tx.send(p.to_string());
                }
            }
            Err(e) => warn!(?e, "kafka recv error"),
        }
    }
}

async fn ws_prices(
    ws: WebSocketUpgrade,
    State(s): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        let mut rx = s.prices.subscribe();
        tracing::info!("WS /prices connected");
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if socket.send(WsMessage::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        tracing::info!("WS /prices disconnected");
    })
}

async fn ws_swaps(
    ws: WebSocketUpgrade,
    State(s): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        let mut rx = s.swaps.subscribe();
        tracing::info!("WS /swaps connected");
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if socket.send(WsMessage::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        tracing::info!("WS /swaps disconnected");
    })
}

async fn ws_swaps_v2(
    ws: WebSocketUpgrade,
    State(s): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        let mut rx = s.swaps_v2.subscribe();
        tracing::info!("WS /swaps.v2 connected");
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if socket.send(WsMessage::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        tracing::info!("WS /swaps.v2 disconnected");
    })
}

async fn ws_ping(ws: WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        tracing::info!("WS /ping connected");
        // keep-alive pings every 3s so the UI can show liveness reliably
        let mut iv = tokio::time::interval(std::time::Duration::from_secs(3));
        loop {
            iv.tick().await;
            if socket.send(WsMessage::Text("ok".into())).await.is_err() {
                break;
            }
        }
        tracing::info!("WS /ping disconnected");
    })
}
