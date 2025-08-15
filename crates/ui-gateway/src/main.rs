use anyhow::Result;
use axum::extract::ws::{Message as WsMessage, WebSocketUpgrade};
use axum::{extract::State, routing::get, Router};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use rdkafka::message::Message as _; // enables m.payload(), m.key(), etc.

#[derive(Clone)]
struct AppState {
    prices_tx: broadcast::Sender<String>,
    swaps_tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let broker = std::env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".into());
    let prices_topic = std::env::var("PRICES_TOPIC").unwrap_or_else(|_| "prices.latest".into());
    let swaps_topic = std::env::var("SWAPS_TOPIC").unwrap_or_else(|_| "orca-swaps.parsed".into());

    let (prices_tx, _) = broadcast::channel::<String>(1024);
    let (swaps_tx, _) = broadcast::channel::<String>(1024);
    let state = AppState {
        prices_tx: prices_tx.clone(),
        swaps_tx: swaps_tx.clone(),
    };

    // spawn Kafka consumers
    tokio::spawn(consumer_task(
        broker.clone(),
        prices_topic,
        prices_tx,
        "prices",
    ));
    tokio::spawn(consumer_task(
        broker.clone(),
        swaps_topic,
        swaps_tx,
        "swaps",
    ));

    let app = Router::new()
        .route("/ws/prices", get(ws_prices))
        .route("/ws/swaps", get(ws_swaps))
        .route("/ws/ping", get(ws_ping))
        .with_state(state);

    let addr = "0.0.0.0:8081";
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("ui-gateway listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn consumer_task(
    broker: String,
    topic: String,
    tx: broadcast::Sender<String>,
    label: &'static str,
) {
    let c: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("group.id", format!("ui-{}", label))
        .set("auto.offset.reset", "latest")
        .create()
        .expect("consumer");
    c.subscribe(&[&topic]).expect("subscribe");
    tracing::info!("consuming topic={}", topic);

    loop {
        match c.recv().await {
            Ok(m) => {
                if let Some(p) = m.payload() {
                    if let Ok(s) = std::str::from_utf8(p) {
                        let _ = tx.send(s.to_string());
                    }
                }
            }
            Err(e) => tracing::warn!(?e, "kafka recv error"),
        }
    }
}

async fn ws_prices(ws: WebSocketUpgrade, State(s): State<AppState>) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        let mut rx = s.prices_tx.subscribe();
        while let Ok(msg) = rx.recv().await {
            let _ = socket.send(WsMessage::Text(msg)).await;
        }
    })
}

async fn ws_swaps(ws: WebSocketUpgrade, State(s): State<AppState>) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        let mut rx = s.swaps_tx.subscribe();
        while let Ok(msg) = rx.recv().await {
            let _ = socket.send(WsMessage::Text(msg)).await;
        }
    })
}

async fn ws_ping(ws: WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        let _ = socket.send(WsMessage::Text("ok".into())).await;
    })
}
