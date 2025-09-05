use crate::ai_hooks::{add_advice, list_advice, list_recent, MAX_ADVICE, MAX_RECENT};
use crate::edge_integration::{
    edge_dashboard_json, edge_threshold_snapshot, edge_verifier_snapshot_json, set_edge_thresholds,
    EdgeArbitrageDetector,
};
use crate::hotpath::SignalQueue;
use crate::quoting::update_depth_impact_bps;
use crate::sizing::adaptive_state_snapshot;
use crate::stats::compute_readiness;
use crate::stats::reload_stats_config;
use crate::stats::{CIRCUIT_OPEN, EDGE_DECODE_ERRORS, EDGE_PROCESS_ERRORS, LAST_EDGE_UPDATE_NS};
use crate::token_meta::{
    build_enrichment, decimals_snapshot, enrich_signal_json, fetch_token_now, logo_snapshot,
    prime_tokens,
};
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
    let enrich_ws = std::env::var("ENRICH_WS_SIGNALS").ok().as_deref() == Some("1");
    loop {
        let sig = rx.recv_async().await; // returns ArbSignal directly
        if enrich_ws {
            // Build enrichment without blocking on network
            let base = serde_json::to_value(&sig)
                .unwrap_or_else(|_| serde_json::json!({"error":"serialize"}));
            let mut obj = match base {
                serde_json::Value::Object(map) => map,
                _ => serde_json::Map::new(),
            };
            let extra = build_enrichment(&sig);
            if let serde_json::Value::Object(extra_map) = extra {
                for (k, v) in extra_map {
                    obj.insert(k, v);
                }
            }
            // boundary_size_in & simple liquidity hint from current pool snapshot (best-effort)
            if let Some(det) = EDGE_DET.get() {
                // choose sell_pool as primary
                if let Some(ps) = det.pool_snapshot_json(&sig.sell_pool_id) {
                    if let Some(boundary) = ps
                        .get("edges")
                        .and_then(|e| e.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.get("size_to_next_boundary_in"))
                        .and_then(|v| v.as_str())
                    {
                        obj.insert(
                            "boundary_size_in".into(),
                            serde_json::Value::String(boundary.to_string()),
                        );
                    }
                    // Derive a liquidity hint for CPMM & CLMM based on state
                    if let Some(state) = ps.get("state") {
                        if let Some(kind) = state.get("kind").and_then(|v| v.as_str()) {
                            match kind {
                                "cpmm" => {
                                    if let (Some(rin), Some(rout)) =
                                        (state.get("reserve_in"), state.get("reserve_out"))
                                    {
                                        if rin.is_number() && rout.is_number() {
                                            obj.insert("pool_liquidity_hint".into(), serde_json::json!({"reserve_in": rin, "reserve_out": rout}));
                                        }
                                    }
                                }
                                "clmm" => {
                                    if let Some(liq) = state.get("liquidity_total") {
                                        if liq.is_number() {
                                            obj.insert(
                                                "pool_liquidity_hint".into(),
                                                serde_json::json!({"liquidity": liq}),
                                            );
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            let merged = serde_json::Value::Object(obj);
            if let Ok(txt) = serde_json::to_string(&merged) {
                if socket.send(WsMessage::Text(txt)).await.is_err() {
                    break;
                }
            }
        } else if let Ok(txt) = serde_json::to_string(&sig) {
            if socket.send(WsMessage::Text(txt)).await.is_err() {
                break;
            }
        }
    }
}

static EDGE_DET: once_cell::sync::OnceCell<Arc<EdgeArbitrageDetector>> =
    once_cell::sync::OnceCell::new();

pub fn register_edge_detector(det: Arc<EdgeArbitrageDetector>) {
    let _ = EDGE_DET.set(det); // ignore if already set
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
        .route(
            "/ai/token/fresh/:mint",
            get(|axum::extract::Path(mint): axum::extract::Path<String>| async move {
                // If not cached or missing decimals, force fetch now; otherwise return cached marker and trigger background prime.
                let need_force = decimals_snapshot(&mint).is_none();
                if need_force {
                    if let Some(rec) = fetch_token_now(&mint).await {
                        return axum::Json(serde_json::to_value(rec).unwrap());
                    }
                }
                // Optionally always force when env set
                if std::env::var("TOKEN_META_FORCE_ALWAYS").ok().as_deref() == Some("1") {
                    if let Some(rec) = fetch_token_now(&mint).await {
                        return axum::Json(serde_json::to_value(rec).unwrap());
                    }
                }
                prime_tokens([mint.as_str()]);
                axum::Json(serde_json::json!({"mint": mint, "cached": true}))
            })
        )
        // Pool snapshot (depth / latest edges) - best-effort
        .route(
            "/pool/:id",
            get(|axum::extract::Path(id): axum::extract::Path<String>| async move {
                if let Some(det) = EDGE_DET.get() {
                    if let Some(js) = det.pool_snapshot_json(&id) {
                        return axum::Json(js);
                    }
                }
                axum::Json(serde_json::json!({"error":"not_found"}))
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
            "/ai/recent_enriched",
            get(|Query(p): Query<ListParams>| async move {
                let lim = p.limit.unwrap_or(50);
                if lim == 0 || lim > MAX_RECENT {
                    return Err::<Json<_>, _>(axum::http::StatusCode::BAD_REQUEST);
                }
                let res = list_recent(lim);
                // Prime token metadata fetches concurrently (fire & forget)
                // Prime metadata (hit/miss metrics inside)
                for sig in &res { prime_tokens([sig.pair.a.as_str(), sig.pair.b.as_str()]); }
                let enriched: Vec<_> = res
                    .into_iter()
                    .map(|s| {
                        let mut base = serde_json::to_value(&s).unwrap_or_else(|_| serde_json::json!({"error":"serialize"}));
                        base = enrich_signal_json(base); // legacy fields
                        if let serde_json::Value::Object(mut map) = base {
                            if let serde_json::Value::Object(extra) = build_enrichment(&s) { for (k,v) in extra { map.insert(k,v); } }
                            // derive detection_latency_ms mirror for dashboard convenience
                            if let Some(lat_ns) = map.get("detection_latency_ns").and_then(|v| v.as_u64()) { map.insert("detection_latency_ms".into(), serde_json::json!((lat_ns as f64)/1_000_000.0)); }
                            serde_json::Value::Object(map)
                        } else { base }
                    })
                    .collect();
                Ok(Json(enriched))
            }),
        )
        .route(
            "/ai/pairs",
            get(|Query(p): Query<ListParams>| async move {
                let lim = p.limit.unwrap_or(500);
                if let Some(det) = EDGE_DET.get() {
                    let js = det.list_pairs_json(Some(lim.min(10_000))); // safety cap
                    return Ok(Json(js));
                }
                Err::<Json<_>, _>(axum::http::StatusCode::SERVICE_UNAVAILABLE)
            })
        )
        .route(
            "/ai/pairs/full",
            get(|Query(p): Query<ListParams>| async move {
                let lim = p.limit.unwrap_or(200);
                if let Some(det) = EDGE_DET.get() {
                    let base = det.list_pairs_json(Some(lim.min(2000)));
                    let mut out = Vec::new();
                    if let Some(arr) = base.as_array() {
                        for v in arr.iter() {
                            if let Some(obj) = v.as_object() {
                                let mut m = obj.clone();
                                // Extract token mints first (cloning to owned Strings) to end immutable borrows before mutation
                                let token_a = m.get("token_a").and_then(|x| x.as_str()).map(|s| s.to_string());
                                let token_b = m.get("token_b").and_then(|x| x.as_str()).map(|s| s.to_string());
                                if let (Some(a), Some(b)) = (token_a.as_deref(), token_b.as_deref()) {
                                    if let Some(dec_a) = decimals_snapshot(a) { m.insert("decimals_a".into(), serde_json::json!(dec_a)); }
                                    if let Some(dec_b) = decimals_snapshot(b) { m.insert("decimals_b".into(), serde_json::json!(dec_b)); }
                                    if let Some(logo_a) = logo_snapshot(a) { m.insert("logo_a".into(), serde_json::json!(logo_a)); }
                                    if let Some(logo_b) = logo_snapshot(b) { m.insert("logo_b".into(), serde_json::json!(logo_b)); }
                                }
                                // fallback: if pair_symbol missing but both symbols present
                                if !m.contains_key("pair_symbol") {
                                    if let (Some(sa), Some(sb)) = (m.get("symbol_a").and_then(|x| x.as_str()), m.get("symbol_b").and_then(|x| x.as_str())) {
                                        m.insert("pair_symbol".into(), serde_json::json!(format!("{}/{}", sa, sb)));
                                    }
                                }
                                out.push(serde_json::Value::Object(m));
                            }
                        }
                    }
                    return Ok(Json(out));
                }
                Err::<Json<_>, _>(axum::http::StatusCode::SERVICE_UNAVAILABLE)
            })
        )
        .route(
            "/ai/pairs/stats",
            get(|Query(p): Query<ListParams>| async move {
                let lim = p.limit.unwrap_or(200);
                if let Some(det) = EDGE_DET.get() {
                    let js = det.list_pair_stats_json(Some(lim.min(2000)));
                    return Ok(Json(js));
                }
                Err::<Json<_>, _>(axum::http::StatusCode::SERVICE_UNAVAILABLE)
            })
        )
        .route(
            "/ai/usdc/:mint",
            get(|axum::extract::Path(mint): axum::extract::Path<String>| async move {
                if let Some(det) = EDGE_DET.get() {
                    let js = det.usdc_prices_json(&mint);
                    return axum::Json(js);
                }
                axum::Json(serde_json::json!({"error":"not_ready"}))
            })
        )
        .route(
            "/ai/price/:base/:quote",
            get(|axum::extract::Path((base, quote)): axum::extract::Path<(String,String)>| async move {
                if let Some(det) = EDGE_DET.get() {
                    let js = det.pair_prices_json(&base, &quote);
                    return axum::Json(js);
                }
                axum::Json(serde_json::json!({"error":"not_ready"}))
            })
        )
        .route(
            "/ai/engine/status",
            get(|| async move {
                use std::sync::atomic::Ordering;
                if let Some(det) = EDGE_DET.get() {
                    let (pairs, stat_pairs) = det.stats_counts();
                    let last = LAST_EDGE_UPDATE_NS.load(Ordering::Relaxed);
                    let staleness_ms = if last>0 {
                        let now_ns = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
                        if now_ns>last { Some((now_ns-last)/1_000_000) } else { Some(0) }
                    } else { None };
                    return axum::Json(serde_json::json!({
                        "pairs_tracked": pairs,
                        "pairs_with_stats": stat_pairs,
                        "last_edge_update_ns": last,
                        "staleness_ms": staleness_ms
                    }));
                }
                axum::Json(serde_json::json!({"error":"not_ready"}))
            })
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
    // Edge verifier snapshot (lightweight JSON)
    app = app.route(
        "/edge_stats",
        get(|| async move { axum::Json(edge_verifier_snapshot_json()) }),
    );
    // Lean dashboard endpoint (no synthetic fallback; purely what has been observed)
    app = app.route(
        "/edge_dashboard",
        get(|| async move { axum::Json(edge_dashboard_json()) }),
    );
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
