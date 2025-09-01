use crate::ai_hooks::{add_advice, list_advice, list_recent, MAX_ADVICE, MAX_RECENT};
use crate::edge_integration::{edge_threshold_snapshot, set_edge_thresholds};
use crate::hotpath::SignalQueue;
use crate::quoting::update_depth_impact_bps;
use crate::sizing::adaptive_state_snapshot;
use crate::stats::compute_readiness;
use crate::stats::reload_stats_config;
use crate::stats::{CIRCUIT_OPEN, EDGE_DECODE_ERRORS, EDGE_PROCESS_ERRORS, LAST_EDGE_UPDATE_NS};
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use axum::{
    routing::{get, post},
    Json, Router,
};
use metrics_exporter_prometheus::PrometheusHandle;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct ListParams {
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct AdviceIn {
    tag: Option<String>,
    message: String,
}

#[derive(Deserialize)]
struct ConfigUpdate {
    min_profit_bps: Option<u32>,
    max_slippage_bps: Option<u32>,
    depth_impact_bps: Option<u32>,
}

#[derive(Serialize)]
struct ConfigSnapshot {
    min_profit_bps: u32,
    max_slippage_bps: u32,
    depth_impact_bps: u32,
}

fn auth_ok(headers: &axum::http::HeaderMap) -> bool {
    if let Ok(token) = std::env::var("AI_API_TOKEN") {
        if token.is_empty() {
            return true;
        }
        if let Some(provided) = headers.get("X-AI-Token") {
            if let Ok(p) = provided.to_str() {
                return p == token;
            }
        }
        return false;
    }
    true // if no token set, allow
}

async fn ws_handler(ws: WebSocketUpgrade, queue: Arc<SignalQueue>) -> axum::response::Response {
    ws.on_upgrade(move |socket| ws_session(socket, queue))
}

async fn ws_session(mut socket: WebSocket, queue: Arc<SignalQueue>) {
    let rx = queue.receiver();
    loop {
        let sig = rx.recv_async().await; // returns ArbSignal directly
        if let Ok(txt) = serde_json::to_string(&sig) {
            if socket.send(WsMessage::Text(txt)).await.is_err() {
                break;
            }
        }
    }
}

pub fn build_http_router(prom: PrometheusHandle, queue: Option<Arc<SignalQueue>>) -> Router {
    use crate::stats::render_all;
    let mut app = Router::new()
        .route(
            "/metrics",
            get({
                let prom = prom.clone();
                move || async move {
                    let base = prom.render();
                    let custom = render_all();
                    format!("{}\n{}", base, custom)
                }
            }),
        )
        // Liveness: process is up; minimal checks
        .route(
            "/live",
            get(|| async move {
                axum::Json(serde_json::json!({
                    "status": "alive"
                }))
            })
        )
        // Readiness: recent edges, acceptable error ratio, circuit closed
        .route(
            "/ready",
            get(|| async move {
                let (ready, err_ratio, staleness_ms, total, errs, circuit, half_open, max_stale, err_thresh, last) = compute_readiness();
                use std::sync::atomic::Ordering;
                let decode_err = EDGE_DECODE_ERRORS.load(Ordering::Relaxed);
                let proc_err = EDGE_PROCESS_ERRORS.load(Ordering::Relaxed);
                debug_assert_eq!(errs, decode_err + proc_err);
                let phase = if circuit { "circuit_open" } else if last == 0 { "starting" } else if staleness_ms > max_stale { "stale" } else if err_ratio > err_thresh { "high_error_ratio" } else { "ready" };
                let status_code = if ready { axum::http::StatusCode::OK } else { axum::http::StatusCode::SERVICE_UNAVAILABLE };
                let body = serde_json::json!({
                    "status": phase,
                    "ready": ready,
                    "last_edge_update_ns": last,
                    "staleness_ms": if staleness_ms == u64::MAX { serde_json::Value::Null } else { serde_json::Value::from(staleness_ms) },
                    "edge_total": total,
                    "edge_decode_errors": decode_err,
                    "edge_process_errors": proc_err,
                    "error_ratio": err_ratio,
                    "circuit_open": circuit,
                    "circuit_half_open": half_open,
                    "max_staleness_ms": max_stale,
                    "max_error_ratio": err_thresh
                });
                (status_code, axum::Json(body))
            })
        )
        .route(
            "/circuit/reset",
            post(|headers: axum::http::HeaderMap| async move {
                if !auth_ok(&headers) { return (axum::http::StatusCode::UNAUTHORIZED, axum::Json(serde_json::json!({"error":"unauthorized"}))); }
                use std::sync::atomic::Ordering;
                CIRCUIT_OPEN.store(false, Ordering::Relaxed);
                EDGE_DECODE_ERRORS.store(0, Ordering::Relaxed);
                EDGE_PROCESS_ERRORS.store(0, Ordering::Relaxed);
                tracing::warn!("circuit_manually_reset");
                (axum::http::StatusCode::OK, axum::Json(serde_json::json!({"status":"ok","circuit_open":false,"reset":true})))
            })
        )
        // Retain /health as backwards-compatible alias (simple wrapper around /ready data subset)
        .route(
            "/health",
            get(|| async move {
                use std::sync::atomic::Ordering;
                let last = LAST_EDGE_UPDATE_NS.load(Ordering::Relaxed);
                let now_ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
                let staleness_ms = if last > 0 && now_ns > last { Some((now_ns - last) / 1_000_000) } else { None };
                axum::Json(serde_json::json!({
                    "last_edge_update_ns": last,
                    "staleness_ms": staleness_ms
                }))
            })
        )
        .route(
            "/ai/recent",
            get(|Query(p): Query<ListParams>| async move {
                let lim = p.limit.unwrap_or(50);
                if lim == 0 || lim > MAX_RECENT {
                    return Err::<Json<_>, _>(axum::http::StatusCode::BAD_REQUEST);
                }
                let res = list_recent(lim);
                Ok(Json(res))
            }),
        )
        .route(
            "/ai/advice",
            get(|Query(p): Query<ListParams>| async move {
                let lim = p.limit.unwrap_or(50);
                if lim == 0 || lim > MAX_ADVICE {
                    return Err::<Json<_>, _>(axum::http::StatusCode::BAD_REQUEST);
                }
                let res = list_advice(lim);
                Ok(Json(res))
            }),
        )
        .route(
            "/ai/advice",
            post(
                |headers: axum::http::HeaderMap, Json(inp): Json<AdviceIn>| async move {
                    if !auth_ok(&headers) {
                        return Err::<Json<_>, _>(axum::http::StatusCode::UNAUTHORIZED);
                    }
                    let entry =
                        add_advice(inp.tag.unwrap_or_else(|| "general".into()), inp.message);
                    Ok(Json(entry))
                },
            ),
        )
        .route(
            "/ai/config",
            get(|| async move {
                let (minp, maxs) = edge_threshold_snapshot();
                let depth = std::env::var("ARB_DEPTH_PRICE_IMPACT_BPS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
                Json(ConfigSnapshot { min_profit_bps: minp, max_slippage_bps: maxs, depth_impact_bps: depth })
            }),
        )
        .route(
            "/ai/config/reload",
            post(|| async move {
                reload_stats_config();
                Json(serde_json::json!({"status":"ok","reloaded":true}))
            }),
        )
        .route(
            "/ai/config/update",
            post(
                |headers: axum::http::HeaderMap, Json(upd): Json<ConfigUpdate>| async move {
                    if !auth_ok(&headers) {
                        return Err::<Json<_>, _>(axum::http::StatusCode::UNAUTHORIZED);
                    }
                    // Persist to env vars for now (process scope) & return snapshot (note: Edge detector reads its own fields until restarted)
                    set_edge_thresholds(upd.min_profit_bps, upd.max_slippage_bps);
                    if let Some(d) = upd.depth_impact_bps { update_depth_impact_bps(d); std::env::set_var("ARB_DEPTH_PRICE_IMPACT_BPS", d.to_string()); }
                    let (minp, maxs) = edge_threshold_snapshot();
                    let depth = std::env::var("ARB_DEPTH_PRICE_IMPACT_BPS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
                    Ok(Json(ConfigSnapshot { min_profit_bps: minp, max_slippage_bps: maxs, depth_impact_bps: depth }))
                },
            ),
        );
    // Sizing snapshot
    app = app.route(
        "/ai/sizing",
        get(|| async move { Json(serde_json::to_value(adaptive_state_snapshot()).unwrap()) }),
    );
    if let Some(q) = queue {
        app = app.route("/ws/arbs", get(move |ws| ws_handler(ws, q.clone())));
    }
    app
}

/// Start a lightweight HTTP server exposing /metrics and optional /ws/arbs
pub async fn start_http_control(
    addr: SocketAddr,
    prom: PrometheusHandle,
    queue: Option<Arc<SignalQueue>>,
) -> anyhow::Result<()> {
    let app = build_http_router(prom, queue);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "http_control_listening");
    axum::serve(listener, app).await?;
    Ok(())
}
