use std::{sync::Arc, time::Duration};

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Path,
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use log::{info, warn};
use serde::Serialize;
use tokio::time::interval;

use super::{
    config::GatewayConfig,
    fanout::{GatewayConsumerHandles, GatewayFanout},
    proxy::DownstreamProxy,
    sse,
};
use redis::Value;

/// Shared state for the gateway router.
#[derive(Clone)]
pub struct GatewayContext {
    pub config: Arc<GatewayConfig>,
    pub proxy: DownstreamProxy,
    pub fanout: GatewayFanout,
}

impl GatewayContext {
    pub fn new(config: GatewayConfig) -> Self {
        let config = Arc::new(config);
        let proxy = DownstreamProxy::new(config.clone());
        let fanout = GatewayFanout::new(config.clone());

        Self {
            config,
            proxy,
            fanout,
        }
    }

    pub fn start_kafka_consumers(&self) -> GatewayConsumerHandles {
        self.fanout.spawn_kafka_tasks()
    }
}

#[allow(dead_code)]
fn assert_gateway_context_send_sync() {
    fn assert<T: Clone + Send + Sync + 'static>() {}
    assert::<GatewayContext>();
}

/// Build a router containing gateway routes.
pub fn build_gateway_router(context: GatewayContext) -> Router {
    let shared_state = Arc::new(context);

    Router::new()
        .route("/gateway/health", get(gateway_health))
        .route("/gateway/ws/prices", get(ws_prices))
        .route("/gateway/ws/swaps", get(ws_swaps))
        .route("/gateway/ws/swaps.v2", get(ws_swaps_v2))
        .route("/gateway/ws/ping", get(ws_ping))
        .route("/gateway/sse/opportunities", get(sse_opportunities))
        .route("/gateway/sse/bundles", get(sse_bundles))
        .route("/gateway/sse/fills", get(sse_fills))
        .route("/gateway/depth/:pool", get(http_depth))
        .layer(Extension(shared_state))
}

#[derive(Debug, Serialize)]
struct GatewayHealthResponse {
    status: &'static str,
    gateway_port: u16,
    proxy: String,
    fanout: String,
}

async fn gateway_health(Extension(ctx): Extension<Arc<GatewayContext>>) -> impl IntoResponse {
    Json(GatewayHealthResponse {
        status: "readying",
        gateway_port: ctx.config.gateway_port,
        proxy: ctx.proxy.readiness_status(),
        fanout: ctx.fanout.readiness_status(),
    })
}

async fn ws_prices(
    ws: WebSocketUpgrade,
    Extension(ctx): Extension<Arc<GatewayContext>>,
) -> impl IntoResponse {
    let rx = ctx.fanout.subscribe_prices();
    upgrade_stream(ws, rx, "prices")
}

async fn ws_swaps(
    ws: WebSocketUpgrade,
    Extension(ctx): Extension<Arc<GatewayContext>>,
) -> impl IntoResponse {
    let rx = ctx.fanout.subscribe_swaps();
    upgrade_stream(ws, rx, "swaps")
}

async fn ws_swaps_v2(
    ws: WebSocketUpgrade,
    Extension(ctx): Extension<Arc<GatewayContext>>,
) -> impl IntoResponse {
    let rx = ctx.fanout.subscribe_swaps_v2();
    upgrade_stream(ws, rx, "swaps.v2")
}

async fn sse_opportunities(Extension(ctx): Extension<Arc<GatewayContext>>) -> impl IntoResponse {
    sse::into_sse(ctx.fanout.subscribe_opportunities())
}

async fn sse_bundles(Extension(ctx): Extension<Arc<GatewayContext>>) -> impl IntoResponse {
    sse::into_sse(ctx.fanout.subscribe_bundles())
}

async fn sse_fills(Extension(ctx): Extension<Arc<GatewayContext>>) -> impl IntoResponse {
    sse::into_sse(ctx.fanout.subscribe_fills())
}

async fn ws_ping(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move { run_ping_channel(socket).await })
}

async fn http_depth(
    Path(pool): Path<String>,
    Extension(ctx): Extension<Arc<GatewayContext>>,
) -> impl IntoResponse {
    let key = format!("dex:pool_depth:{}", pool);
    let mut payload = serde_json::Map::new();

    match redis::Client::open(ctx.config.redis_url.as_str()) {
        Ok(client) => match client.get_multiplexed_tokio_connection().await {
            Ok(mut conn) => match redis::cmd("HGETALL")
                .arg(&key)
                .query_async::<_, Value>(&mut conn)
                .await
            {
                Ok(Value::Bulk(values)) => {
                    let mut iter = values.into_iter();
                    while let (Some(field), Some(value)) = (iter.next(), iter.next()) {
                        if let Some(name) = extract_string(field) {
                            let json_value = match value {
                                Value::Data(bytes) => serde_json::Value::String(
                                    String::from_utf8_lossy(&bytes).to_string(),
                                ),
                                Value::Status(status) => serde_json::Value::String(status),
                                Value::Int(i) => serde_json::Value::Number(i.into()),
                                Value::Nil => serde_json::Value::Null,
                                other => serde_json::Value::String(format!("{:?}", other)),
                            };
                            payload.insert(name, json_value);
                        }
                    }
                }
                Ok(Value::Nil) => {}
                Ok(other) => warn!(
                    "gateway depth unexpected response key={} value={:?}",
                    key, other
                ),
                Err(err) => warn!("gateway depth fetch error key={} err={}", key, err),
            },
            Err(err) => warn!("gateway redis connection error err={}", err),
        },
        Err(err) => warn!("gateway redis client error err={}", err),
    }

    Json(serde_json::json!({
        "pool": pool,
        "redis_key": key,
        "data": payload,
    }))
}

fn upgrade_stream(
    ws: WebSocketUpgrade,
    mut rx: tokio::sync::broadcast::Receiver<String>,
    label: &'static str,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move { forward_broadcast(socket, &mut rx, label).await })
}

async fn forward_broadcast(
    mut socket: WebSocket,
    rx: &mut tokio::sync::broadcast::Receiver<String>,
    label: &'static str,
) {
    info!("gateway websocket connected label={label}");
    loop {
        match rx.recv().await {
            Ok(payload) => {
                if socket.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(
                    "gateway websocket lagged label={} skipped={} messages",
                    label, skipped
                );
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                break;
            }
        }
    }
    info!("gateway websocket disconnected label={label}");
}

async fn run_ping_channel(mut socket: WebSocket) {
    info!("gateway websocket connected label=ping");
    let mut ticker = interval(Duration::from_secs(3));
    loop {
        ticker.tick().await;
        if socket.send(Message::Text("ok".to_string())).await.is_err() {
            break;
        }
    }
    info!("gateway websocket disconnected label=ping");
}

fn extract_string(value: Value) -> Option<String> {
    match value {
        Value::Data(bytes) => Some(String::from_utf8_lossy(&bytes).to_string()),
        Value::Status(status) => Some(status),
        Value::Int(i) => Some(i.to_string()),
        Value::Nil => None,
        _ => None,
    }
}
