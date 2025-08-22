use anyhow::Result;
use axum::{
    extract::ws::{Message as WsMessage, WebSocketUpgrade},
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message as _};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    prices: broadcast::Sender<String>,
    swaps: broadcast::Sender<String>,
    swaps_v2: broadcast::Sender<String>,
    redis_client: redis::Client,
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
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379/0".into());
    let redis_client = redis::Client::open(redis_url.clone())?;
    let state = AppState {
        prices: prices_tx.clone(),
        swaps: swaps_tx.clone(),
        swaps_v2: swaps_v2_tx.clone(),
        redis_client,
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
        .route("/pool/:id", get(http_pool))
        .route("/depth/:pool", get(http_depth))
        .route("/ws/ping", get(ws_ping))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(Arc::new(state));

    let bind_addr = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8081".to_string());
    let listener = TcpListener::bind(&bind_addr).await?;
    info!("ui-gateway listening on {bind_addr}");
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

async fn http_depth(Path(pool): Path<String>, State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let key = format!("dex:pool_depth:{}", pool);
    let mut map = serde_json::Map::new();
    match s.redis_client.get_multiplexed_tokio_connection().await {
        Ok(mut conn) => {
            let resp: redis::Value = redis::cmd("HGETALL")
                .arg(&key)
                .query_async(&mut conn)
                .await
                .unwrap_or(redis::Value::Nil);
            if let redis::Value::Array(kvs) = resp {
                let mut it = kvs.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    if let (redis::Value::BulkString(kb), v) = (k, v) {
                        let name = String::from_utf8_lossy(&kb).to_string();
                        let val = match v {
                            redis::Value::BulkString(vb) => {
                                json!(String::from_utf8_lossy(&vb).to_string())
                            }
                            redis::Value::Int(i) => json!(i),
                            redis::Value::Nil => serde_json::Value::Null,
                            _ => serde_json::Value::Null,
                        };
                        map.insert(name, val);
                    }
                }
            }
        }
        Err(_) => {}
    }
    Json(json!({
        "pool": pool,
        "key": key,
        "data": map,
    }))
}

async fn http_pool(Path(pool): Path<String>, State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let key = format!("dex:pool:{}", pool);
    let mut map = serde_json::Map::new();
    match s.redis_client.get_multiplexed_tokio_connection().await {
        Ok(mut conn) => {
            let resp: redis::Value = redis::cmd("HGETALL")
                .arg(&key)
                .query_async(&mut conn)
                .await
                .unwrap_or(redis::Value::Nil);
            if let redis::Value::Array(kvs) = resp {
                let mut it = kvs.into_iter();
                while let (Some(k), Some(v)) = (it.next(), it.next()) {
                    if let (redis::Value::BulkString(kb), v) = (k, v) {
                        let name = String::from_utf8_lossy(&kb).to_string();
                        let val = match v {
                            redis::Value::BulkString(vb) => {
                                json!(String::from_utf8_lossy(&vb).to_string())
                            }
                            redis::Value::Int(i) => json!(i),
                            redis::Value::Nil => serde_json::Value::Null,
                            _ => serde_json::Value::Null,
                        };
                        map.insert(name, val);
                    }
                }
            }
        }
        Err(_) => {}
    }
    Json(json!({
        "pool": pool,
        "key": key,
        "data": map,
    }))
}
