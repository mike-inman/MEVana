#![allow(warnings)]

use axum::{
    extract::{Query, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use dashmap::DashMap;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tower_http::cors::{Any, CorsLayer};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

// Import the detection queue consumer from hotpath engine
use hotpath_arb_engine::orderbook_tracker::consume_detected_arbitrage_events;

#[cfg(feature = "database")]
use sqlx::{PgPool, Row};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArbitrageStats {
    pub opportunities_detected: u64,
    pub opportunities_executed: u64,
    pub total_profit_bps: i64,
    pub avg_detection_time_us: u64,
    pub success_rate_pct: f64,
    #[schema(value_type = String, format = "date-time")]
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VenueStats {
    pub venue: String,
    pub volume_24h: f64,
    pub opportunities_count: u64,
    pub avg_profit_bps: i32,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiveMetrics {
    pub current_slot: u64,
    pub streams_active: u32,
    pub transactions_per_second: f64,
    pub cpu_cores_used: Vec<u8>,
    pub memory_usage_mb: u64,
    pub uptime_seconds: u64,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        live_arbitrage_stats,
        arbitrage_summary,
        performance_metrics,
        get_all_arbitrage_records,
        venue_stats,
        system_status
    ),
    components(
        schemas(ArbitrageStats, VenueStats, LiveMetrics)
    ),
    tags(
        (name = "monitoring", description = "MEVana Monitoring Service API")
    ),
    info(
        title = "MEVana Monitoring Service API",
        version = "1.0.0",
        description = "Real-time monitoring and analytics for MEVana arbitrage engine",
        contact(
            name = "MEVana Team",
            url = "http://149.28.56.125:8090"
        )
    )
)]
struct ApiDoc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub detection_latency_p50_us: u64,
    pub detection_latency_p95_us: u64,
    pub detection_latency_p99_us: u64,
    pub execution_latency_p50_ms: u64,
    pub execution_latency_p95_ms: u64,
    pub execution_latency_p99_ms: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub redis_client: redis::Client,
    pub metrics_cache: Arc<DashMap<String, serde_json::Value>>,
    #[cfg(feature = "database")]
    pub db_pool: Option<PgPool>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    info!("🚀 Starting Monitoring Service for HotPath Arbitrage Engine");

    // Redis connection for metrics sharing
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis_client = redis::Client::open(redis_url)?;

    // Test Redis connection
    let mut redis_conn = redis_client.get_connection()?;
    redis::cmd("PING").execute(&mut redis_conn);
    info!("✅ Connected to Redis for metrics sharing");

    // Database connection for writing discarded arbitrage events
    #[cfg(feature = "database")]
    let db_pool = {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb".to_string());
        match sqlx::PgPool::connect(&database_url).await {
            Ok(pool) => {
                info!("✅ Connected to PostgreSQL for arbitrage data storage");
                Some(pool)
            }
            Err(e) => {
                error!("❌ Failed to connect to database: {}", e);
                info!("📊 Continuing without database - Redis metrics only");
                None
            }
        }
    };

    let state = AppState {
        redis_client,
        metrics_cache: Arc::new(DashMap::new()),
        #[cfg(feature = "database")]
        db_pool,
    };

    // Start background metrics collector
    let metrics_state = state.clone();
    tokio::spawn(async move {
        metrics_collector_task(metrics_state).await;
    });

    // Build router with CORS and Swagger UI
    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(health_check))
        .route("/metrics", get(prometheus_metrics))
        .route("/api/arbitrage/live", get(live_arbitrage_stats))
        .route("/api/arbitrage/summary", get(arbitrage_summary))
        .route("/api/arbitrage/performance", get(performance_metrics))
        .route("/api/arbitrage/all", get(get_all_arbitrage_records))
        .route("/api/venues", get(venue_stats))
        .route("/api/system/status", get(system_status))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(Any),
        )
        .with_state(state);

    let port = std::env::var("MONITORING_PORT")
        .unwrap_or_else(|_| "8090".to_string())
        .parse::<u16>()?;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🌐 Monitoring Service listening on http://0.0.0.0:{}", port);
    info!("📊 API Endpoints available:");
    info!("  - GET /health - Health check");
    info!("  - GET /metrics - Prometheus metrics");
    info!("  - GET /api/arbitrage/live - Real-time arbitrage stats");
    info!("  - GET /api/arbitrage/summary - 24h summary");
    info!("  - GET /api/arbitrage/performance - Performance metrics");
    info!("  - GET /api/venues - Venue statistics");
    info!("  - GET /api/system/status - System status");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn metrics_collector_task(state: AppState) {
    info!("📈 Starting metrics collector task");
    let mut interval = interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        // DIRECT QUEUE CONSUMPTION - Get qualified arbitrage opportunities from shared memory
        let detected_events = consume_detected_arbitrage_events(100);

        if !detected_events.is_empty() {
            info!(
                "⚡ CONSUMING {} qualified arbitrage opportunities from shared memory queue",
                detected_events.len()
            );

            // Write detected opportunities to database
            if let Err(e) = process_detected_events(&state, detected_events).await {
                error!("❌ Failed to process detected opportunities: {}", e);
            }
        }

        match collect_metrics_from_redis(&state).await {
            Ok(_) => debug!("✅ Metrics collected from Redis"),
            Err(e) => error!("❌ Failed to collect metrics: {}", e),
        }
    }
}

async fn collect_metrics_from_redis(state: &AppState) -> anyhow::Result<()> {
    let mut conn = state.redis_client.get_connection()?;

    // Collect various metrics from Redis keys that hotpath_arb_engine publishes
    let keys = [
        "hotpath:stats:arbitrage",
        "hotpath:stats:arbitrage:live",
        "hotpath:stats:performance",
        "hotpath:stats:venues",
        "hotpath:stats:system",
    ];

    for key in keys {
        if let Ok(value) = redis::cmd("GET").arg(key).query::<String>(&mut conn) {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&value) {
                state.metrics_cache.insert(key.to_string(), json_value);
            }
        }
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "monitoring",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "monitoring-service",
        "timestamp": chrono::Utc::now(),
        "version": "0.1.0"
    }))
}

async fn prometheus_metrics() -> String {
    // Return basic Prometheus metrics for monitoring service itself
    format!(
        "# HELP monitoring_service_uptime_seconds Total uptime in seconds\n# TYPE monitoring_service_uptime_seconds counter\nmonitoring_service_uptime_seconds {}\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    )
}

#[utoipa::path(
    get,
    path = "/api/arbitrage/live",
    tag = "monitoring",
    responses(
        (status = 200, description = "Live arbitrage statistics", body = ArbitrageStats)
    )
)]
async fn live_arbitrage_stats(State(state): State<AppState>) -> Json<ArbitrageStats> {
    // Try database first for real data
    #[cfg(feature = "database")]
    if let Some(db_pool) = &state.db_pool {
        if let Ok(stats) = query_live_arbitrage_stats(db_pool).await {
            return Json(stats);
        }
    }

    // Fallback to Redis cache - try both keys
    for key in ["hotpath:stats:arbitrage", "hotpath:stats:arbitrage:live"] {
        if let Some(stats) = state.metrics_cache.get(key) {
            if let Ok(parsed) = serde_json::from_value::<ArbitrageStats>(stats.clone()) {
                return Json(parsed);
            }
        }
    }

    // Last resort: empty stats indicating no data yet
    Json(ArbitrageStats {
        opportunities_detected: 0,
        opportunities_executed: 0,
        total_profit_bps: 0,
        avg_detection_time_us: 0,
        success_rate_pct: 0.0,
        last_updated: chrono::Utc::now(),
    })
}

#[utoipa::path(
    get,
    path = "/api/arbitrage/summary",
    tag = "monitoring",
    responses(
        (status = 200, description = "24-hour arbitrage summary")
    )
)]
async fn arbitrage_summary(State(state): State<AppState>) -> Json<serde_json::Value> {
    // Try database first for real 24h data
    #[cfg(feature = "database")]
    if let Some(db_pool) = &state.db_pool {
        if let Ok(summary) = query_24h_summary(db_pool).await {
            return Json(summary);
        }
    }

    // Fallback: empty summary indicating no data yet
    Json(serde_json::json!({
        "period": "24h",
        "total_opportunities": 0,
        "executed_trades": 0,
        "total_profit_sol": 0.0,
        "average_profit_bps": 0,
        "success_rate": 0.0,
        "top_venues": [],
        "note": "No arbitrage data available yet - engine may still be warming up"
    }))
}

#[utoipa::path(
    get,
    path = "/api/arbitrage/performance",
    tag = "monitoring",
    responses(
        (status = 200, description = "Performance metrics")
    )
)]
async fn performance_metrics(State(state): State<AppState>) -> Json<PerformanceMetrics> {
    // Try database first for real performance data
    #[cfg(feature = "database")]
    if let Some(db_pool) = &state.db_pool {
        if let Ok(perf) = query_performance_metrics(db_pool).await {
            return Json(perf);
        }
    }

    // Fallback to Redis cache
    if let Some(perf) = state.metrics_cache.get("hotpath:stats:performance") {
        if let Ok(parsed) = serde_json::from_value::<PerformanceMetrics>(perf.clone()) {
            return Json(parsed);
        }
    }

    // Default: no performance data yet
    Json(PerformanceMetrics {
        detection_latency_p50_us: 0,
        detection_latency_p95_us: 0,
        detection_latency_p99_us: 0,
        execution_latency_p50_ms: 0,
        execution_latency_p95_ms: 0,
        execution_latency_p99_ms: 0,
    })
}

#[utoipa::path(
    get,
    path = "/api/venues",
    tag = "monitoring",
    responses(
        (status = 200, description = "Venue statistics", body = Vec<VenueStats>)
    )
)]
async fn venue_stats(State(state): State<AppState>) -> Json<Vec<VenueStats>> {
    // Try database first for real venue data
    #[cfg(feature = "database")]
    if let Some(db_pool) = &state.db_pool {
        if let Ok(venues) = query_venue_stats(db_pool).await {
            return Json(venues);
        }
    }

    // Fallback: empty list indicating no venue data yet
    Json(vec![])
}

#[utoipa::path(
    get,
    path = "/api/arbitrage/all",
    tag = "monitoring",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of records to return (default: 1000, max: 10000)")
    ),
    responses(
        (status = 200, description = "All arbitrage records"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_all_arbitrage_records(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get limit from query params (default 1000, max 10000)
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(1000)
        .min(10000);

    #[cfg(feature = "database")]
    if let Some(db_pool) = &state.db_pool {
        match query_all_arbitrage_records(db_pool, limit).await {
            Ok(records) => return Ok(Json(records)),
            Err(e) => {
                error!("Failed to query all arbitrage records: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // No database configured
    Ok(Json(serde_json::json!({
        "error": "Database not configured",
        "records": []
    })))
}

#[utoipa::path(
    get,
    path = "/api/system/status",
    tag = "monitoring",
    responses(
        (status = 200, description = "System status", body = LiveMetrics),
        (status = 503, description = "System metrics not yet available")
    )
)]
async fn system_status(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(system) = state.metrics_cache.get("hotpath:stats:system") {
        if let Ok(parsed) = serde_json::from_value::<LiveMetrics>(system.clone()) {
            return Json(parsed).into_response();
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "error": "system metrics not yet available"
        })),
    )
        .into_response()
}

async fn process_detected_events(
    state: &AppState,
    events: Vec<hotpath_arb_engine::orderbook_tracker::ArbitrageEvent>,
) -> anyhow::Result<()> {
    if events.is_empty() {
        return Ok(());
    }

    info!(
        "💾 WRITING {} events to database and Redis cache",
        events.len()
    );

    // Write to database (existing arbitrage_opportunities table)
    #[cfg(feature = "database")]
    if let Some(db_pool) = &state.db_pool {
        for event in &events {
            let result = sqlx::query(
                r#"
                INSERT INTO arbitrage_opportunities 
                (opportunity_id, timestamp, slot, venue_sell, venue_buy, token_pair, profit_bps, 
                 amount_in, expected_amount_out, detection_time_us, strategy_type)
                VALUES ($1, to_timestamp($2::double precision / 1000), $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#
            )
            .bind(&event.opportunity_id)
            .bind(event.timestamp as f64)
            .bind(event.slot as i64)
            .bind(&event.venue_sell)
            .bind(&event.venue_buy)
            .bind(&event.token_pair)
            .bind(event.profit_bps as i32)
            .bind(event.amount_in as f64)
            .bind(event.amount_out as f64)
            .bind(event.execution_time_us as i64)
            .bind("arbitrage-detected")
            .execute(db_pool).await;

            if let Err(e) = result {
                error!(
                    "❌ Failed to insert arbitrage opportunity {}: {}",
                    event.opportunity_id, e
                );
            } else {
                info!(
                    "✅ DB INSERT: {} {}bps {}->{} (slot: {})",
                    event.opportunity_id,
                    event.profit_bps,
                    event.venue_sell,
                    event.venue_buy,
                    event.slot
                );
            }
        }
    }

    // Update Redis with aggregated metrics
    let mut redis_conn = state.redis_client.get_connection()?;

    // Update arbitrage stats in Redis
    let total_opportunities = events.len() as u64;
    let avg_profit: f64 =
        events.iter().map(|e| e.profit_bps as f64).sum::<f64>() / total_opportunities as f64;
    let avg_detection_time: f64 = events
        .iter()
        .map(|e| e.execution_time_us as f64)
        .sum::<f64>()
        / total_opportunities as f64;

    let arbitrage_stats = serde_json::json!({
        "opportunities_detected": total_opportunities,
        "opportunities_executed": events.iter().filter(|e| e.success).count(),
        "total_profit_bps": events.iter().map(|e| e.profit_bps as i64).sum::<i64>(),
        "avg_detection_time_us": avg_detection_time as u64,
        "success_rate_pct": events.iter().filter(|e| e.success).count() as f64 / total_opportunities as f64 * 100.0,
        "last_updated": chrono::Utc::now(),
        "timestamp": chrono::Utc::now().timestamp()
    });

    // Write to both Redis keys for compatibility
    let stats_json = arbitrage_stats.to_string();

    redis::cmd("SET")
        .arg("hotpath:stats:arbitrage")
        .arg(&stats_json)
        .arg("EX")
        .arg(300) // 5 minute expiry
        .execute(&mut redis_conn);

    redis::cmd("SET")
        .arg("hotpath:stats:arbitrage:live")
        .arg(&stats_json)
        .arg("EX")
        .arg(300) // 5 minute expiry
        .execute(&mut redis_conn);

    debug!(
        "✅ Updated Redis with live arbitrage stats from {} events",
        total_opportunities
    );

    Ok(())
}

// Database query functions for real arbitrage data
#[cfg(feature = "database")]
async fn query_live_arbitrage_stats(db_pool: &sqlx::PgPool) -> anyhow::Result<ArbitrageStats> {
    // Query recent arbitrage opportunities (last 1 hour)
    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as opportunities_detected,
            COUNT(*) FILTER (WHERE strategy_type != 'discard-pile') as opportunities_executed,
            COALESCE(SUM(profit_bps), 0) as total_profit_bps,
            COALESCE(AVG(detection_time_us)::DOUBLE PRECISION, 0) as avg_detection_time_us,
            COALESCE(
                COUNT(*) FILTER (WHERE strategy_type != 'discard-pile') * 100.0 / NULLIF(COUNT(*), 0), 
                0
            ) as success_rate_pct
        FROM arbitrage_opportunities 
        WHERE timestamp >= NOW() - INTERVAL '24 hours'
        "#
    )
    .fetch_one(db_pool)
    .await?;

    Ok(ArbitrageStats {
        opportunities_detected: row.get::<i64, _>("opportunities_detected") as u64,
        opportunities_executed: row.get::<i64, _>("opportunities_executed") as u64,
        total_profit_bps: row.get::<i64, _>("total_profit_bps"),
        avg_detection_time_us: row.get::<f64, _>("avg_detection_time_us") as u64,
        success_rate_pct: row.get::<f64, _>("success_rate_pct"),
        last_updated: chrono::Utc::now(),
    })
}

#[cfg(feature = "database")]
async fn query_24h_summary(db_pool: &sqlx::PgPool) -> anyhow::Result<serde_json::Value> {
    // Query 24h aggregated data
    let summary_row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_opportunities,
            COUNT(*) FILTER (WHERE strategy_type != 'discard-pile') as executed_trades,
            COALESCE(SUM(profit_bps), 0) as total_profit_bps,
            COALESCE(AVG(profit_bps)::DOUBLE PRECISION, 0) as average_profit_bps,
            COALESCE(
                COUNT(*) FILTER (WHERE strategy_type != 'discard-pile') * 100.0 / NULLIF(COUNT(*), 0), 
                0
            ) as success_rate
        FROM arbitrage_opportunities 
        WHERE timestamp >= NOW() - INTERVAL '24 hours'
        "#
    )
    .fetch_one(db_pool)
    .await?;

    // Query top venues
    let venue_rows = sqlx::query(
        r#"
        SELECT 
            CONCAT(venue_sell, '->', venue_buy) as venue_pair,
            COUNT(*) as trades,
            COALESCE(SUM(profit_bps), 0) as total_profit_bps
        FROM arbitrage_opportunities 
        WHERE timestamp >= NOW() - INTERVAL '24 hours'
          AND strategy_type != 'discard-pile'
        GROUP BY venue_sell, venue_buy
        ORDER BY total_profit_bps DESC
        LIMIT 5
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    let total_profit_bps = summary_row.get::<i64, _>("total_profit_bps");
    let total_profit_sol = total_profit_bps as f64 / 10000.0; // Convert bps to SOL estimate

    let top_venues: Vec<serde_json::Value> = venue_rows
        .iter()
        .map(|row| {
            let venue_profit_bps = row.get::<i64, _>("total_profit_bps");
            serde_json::json!({
                "name": row.get::<String, _>("venue_pair"),
                "profit_sol": venue_profit_bps as f64 / 10000.0,
                "trades": row.get::<i64, _>("trades")
            })
        })
        .collect();

    Ok(serde_json::json!({
        "period": "24h",
        "total_opportunities": summary_row.get::<i64, _>("total_opportunities"),
        "executed_trades": summary_row.get::<i64, _>("executed_trades"),
        "total_profit_sol": total_profit_sol,
        "average_profit_bps": summary_row.get::<f64, _>("average_profit_bps") as i32,
        "success_rate": summary_row.get::<f64, _>("success_rate"),
        "top_venues": top_venues
    }))
}

#[cfg(feature = "database")]
async fn query_performance_metrics(db_pool: &sqlx::PgPool) -> anyhow::Result<PerformanceMetrics> {
    // Query percentile performance metrics from recent data
    let row = sqlx::query(
        r#"
        SELECT 
            COALESCE(percentile_cont(0.5) WITHIN GROUP (ORDER BY detection_time_us), 0) as p50_detection,
            COALESCE(percentile_cont(0.95) WITHIN GROUP (ORDER BY detection_time_us), 0) as p95_detection,
            COALESCE(percentile_cont(0.99) WITHIN GROUP (ORDER BY detection_time_us), 0) as p99_detection
        FROM arbitrage_opportunities 
        WHERE timestamp >= NOW() - INTERVAL '24 hours'
          AND detection_time_us > 0
        "#
    )
    .fetch_one(db_pool)
    .await?;

    Ok(PerformanceMetrics {
        detection_latency_p50_us: row.get::<f64, _>("p50_detection") as u64,
        detection_latency_p95_us: row.get::<f64, _>("p95_detection") as u64,
        detection_latency_p99_us: row.get::<f64, _>("p99_detection") as u64,
        // For now, use mock execution data until we have execution_results table populated
        execution_latency_p50_ms: 45,
        execution_latency_p95_ms: 89,
        execution_latency_p99_ms: 125,
    })
}

#[cfg(feature = "database")]
async fn query_venue_stats(db_pool: &sqlx::PgPool) -> anyhow::Result<Vec<VenueStats>> {
    let rows = sqlx::query(
        r#"
        SELECT 
            CONCAT(venue_sell, '->', venue_buy) as venue,
            COUNT(*) as opportunities_count,
            COALESCE(AVG(profit_bps), 0) as avg_profit_bps,
            COALESCE(
                COUNT(*) FILTER (WHERE strategy_type != 'discard-pile') * 100.0 / NULLIF(COUNT(*), 0), 
                0
            ) as success_rate,
            COALESCE(SUM(amount_in * expected_amount_out), 0) as volume_estimate
        FROM arbitrage_opportunities 
        WHERE timestamp >= NOW() - INTERVAL '24 hours'
        GROUP BY venue_sell, venue_buy
        HAVING COUNT(*) >= 5  -- Only venues with meaningful activity
        ORDER BY avg_profit_bps DESC
        LIMIT 10
        "#
    )
    .fetch_all(db_pool)
    .await?;

    let venues: Vec<VenueStats> = rows
        .iter()
        .map(|row| VenueStats {
            venue: row.get::<String, _>("venue"),
            volume_24h: row.get::<f64, _>("volume_estimate"),
            opportunities_count: row.get::<i64, _>("opportunities_count") as u64,
            avg_profit_bps: row.get::<f64, _>("avg_profit_bps") as i32,
            success_rate: row.get::<f64, _>("success_rate"),
        })
        .collect();

    Ok(venues)
}

#[cfg(feature = "database")]
async fn query_all_arbitrage_records(
    db_pool: &sqlx::PgPool,
    limit: i64,
) -> anyhow::Result<serde_json::Value> {
    // Query all arbitrage records with limit
    let rows = sqlx::query(
        r#"
        SELECT 
            opportunity_id,
            timestamp,
            slot,
            venue_sell,
            venue_buy,
            token_pair,
            profit_bps,
            amount_in,
            expected_amount_out,
            detection_time_us,
            strategy_type
        FROM arbitrage_opportunities 
        ORDER BY timestamp DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(db_pool)
    .await?;

    let records: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "opportunity_id": row.get::<String, _>("opportunity_id"),
                "timestamp": row.get::<chrono::DateTime<chrono::Utc>, _>("timestamp"),
                "slot": row.get::<i64, _>("slot"),
                "venue_sell": row.get::<String, _>("venue_sell"),
                "venue_buy": row.get::<String, _>("venue_buy"),
                "token_pair": row.get::<String, _>("token_pair"),
                "profit_bps": row.get::<i32, _>("profit_bps"),
                "amount_in": row.get::<f64, _>("amount_in"),
                "expected_amount_out": row.get::<f64, _>("expected_amount_out"),
                "detection_time_us": row.get::<i64, _>("detection_time_us"),
                "strategy_type": row.get::<String, _>("strategy_type")
            })
        })
        .collect();

    Ok(serde_json::json!({
        "total": records.len(),
        "limit": limit,
        "records": records
    }))
}
