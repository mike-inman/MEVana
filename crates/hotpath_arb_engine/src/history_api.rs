use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ArbitrageOpportunityResponse {
    pub opportunity_id: String,
    pub timestamp: String,
    pub slot: i64,
    pub venue_sell: String,
    pub venue_buy: String,
    pub token_pair: String,
    pub profit_bps: i32,
    pub amount_in: Option<f64>,
    pub expected_amount_out: Option<f64>,
    pub detection_time_us: Option<i64>,
    pub strategy_type: Option<String>,
    pub executed: Option<bool>,
    pub success: Option<bool>,
    pub actual_profit_bps: Option<i32>,
    pub execution_time_us: Option<i64>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VenuePerformanceResponse {
    pub venue_pair: String,
    pub total_opportunities: i32,
    pub executed_opportunities: i32,
    pub successful_executions: i32,
    pub success_rate_pct: Option<f64>,
    pub average_profit_bps: Option<f64>,
    pub average_execution_time_us: Option<f64>,
    pub last_updated: String,
}

#[derive(Debug, Serialize)]
pub struct LiveStatsResponse {
    pub current_time: String,
    pub total_opportunities_detected: i64,
    pub total_executed: i64,
    pub total_successful: i64,
    pub overall_success_rate: f64,
    pub average_profit_bps: f64,
    pub top_venues: Vec<VenuePerformanceResponse>,
    pub recent_opportunities: Vec<ArbitrageOpportunityResponse>,
    pub active_tokens: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpportunityQuery {
    pub limit: Option<i32>,
    pub venue_sell: Option<String>,
    pub venue_buy: Option<String>,
    pub min_profit_bps: Option<i32>,
    pub success_only: Option<bool>,
    pub last_hours: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PerformanceQuery {
    pub venue_pair: Option<String>,
    pub min_opportunities: Option<i32>,
    pub sort_by: Option<String>, // "success_rate", "profit", "volume"
}

pub fn create_history_api_router(db_pool: PgPool) -> Router {
    let state = Arc::new(db_pool);

    // Configure CORS to allow all origins for development
    // In production, you should restrict this to specific origins
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route(
            "/api/arbitrage/opportunities",
            get(get_arbitrage_opportunities),
        )
        .route("/api/arbitrage/performance", get(get_venue_performance))
        .route("/api/arbitrage/live", get(get_live_stats))
        .route("/api/arbitrage/summary", get(get_summary))
        .route("/api/health", get(health_check))
        .layer(cors)
        .with_state(state)
}

/// GET /api/arbitrage/opportunities - Get historical arbitrage opportunities
pub async fn get_arbitrage_opportunities(
    State(db): State<Arc<PgPool>>,
    Query(params): Query<OpportunityQuery>,
) -> Result<Json<ApiResponse<Vec<ArbitrageOpportunityResponse>>>, StatusCode> {
    let limit = params.limit.unwrap_or(100).min(1000); // Max 1000 for performance
    let last_hours = params.last_hours.unwrap_or(24);

    let mut query_str = r#"
        SELECT 
            ao.opportunity_id,
            ao.timestamp,
            ao.slot,
            ao.venue_sell,
            ao.venue_buy,
            ao.token_pair,
            ao.profit_bps,
            ao.amount_in,
            ao.expected_amount_out,
            ao.detection_time_us,
            ao.strategy_type,
            er.executed,
            er.success,
            er.actual_profit_bps,
            er.execution_time_us,
            er.failure_reason
        FROM arbitrage_opportunities ao
        LEFT JOIN execution_results er ON ao.opportunity_id = er.opportunity_id
        WHERE ao.timestamp >= NOW() - INTERVAL '%d hours'
    "#
    .to_string();

    let mut conditions = Vec::new();

    if let Some(venue_sell) = &params.venue_sell {
        conditions.push(format!("AND ao.venue_sell = '{}'", venue_sell));
    }

    if let Some(venue_buy) = &params.venue_buy {
        conditions.push(format!("AND ao.venue_buy = '{}'", venue_buy));
    }

    if let Some(min_profit) = params.min_profit_bps {
        conditions.push(format!("AND ao.profit_bps >= {}", min_profit));
    }

    if params.success_only == Some(true) {
        conditions.push("AND er.success = true".to_string());
    }

    query_str = format!(
        "{} {} ORDER BY ao.timestamp DESC LIMIT {}",
        query_str,
        conditions.join(" "),
        limit
    );

    let formatted_query = query_str.replace("%d", &last_hours.to_string());

    match sqlx::query_as::<
        _,
        (
            String,
            chrono::DateTime<chrono::Utc>,
            i64,
            String,
            String,
            String,
            i32,
            Option<f64>,
            Option<f64>,
            Option<i64>,
            Option<String>,
            Option<bool>,
            Option<bool>,
            Option<i32>,
            Option<i64>,
            Option<String>,
        ),
    >(&formatted_query)
    .fetch_all(db.as_ref())
    .await
    {
        Ok(rows) => {
            let opportunities: Vec<ArbitrageOpportunityResponse> = rows
                .into_iter()
                .map(
                    |(
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
                        strategy_type,
                        executed,
                        success,
                        actual_profit_bps,
                        execution_time_us,
                        failure_reason,
                    )| {
                        ArbitrageOpportunityResponse {
                            opportunity_id,
                            timestamp: timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                            slot,
                            venue_sell,
                            venue_buy,
                            token_pair,
                            profit_bps,
                            amount_in,
                            expected_amount_out,
                            detection_time_us,
                            strategy_type,
                            executed,
                            success,
                            actual_profit_bps,
                            execution_time_us,
                            failure_reason,
                        }
                    },
                )
                .collect();

            Ok(Json(ApiResponse::success(opportunities)))
        }
        Err(e) => {
            error!("Failed to fetch arbitrage opportunities: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/arbitrage/performance - Get venue performance statistics
pub async fn get_venue_performance(
    State(db): State<Arc<PgPool>>,
    Query(params): Query<PerformanceQuery>,
) -> Result<Json<ApiResponse<Vec<VenuePerformanceResponse>>>, StatusCode> {
    let mut query_str = r#"
        SELECT 
            CONCAT(venue_sell, '->', venue_buy) as venue_pair,
            COUNT(*) as total_opportunities,
            COUNT(CASE WHEN er.executed = true THEN 1 END) as executed_opportunities,
            COUNT(CASE WHEN er.success = true THEN 1 END) as successful_executions,
            AVG(CASE WHEN er.success = true THEN ao.profit_bps END) as average_profit_bps,
            AVG(CASE WHEN er.executed = true THEN er.execution_time_us END) as average_execution_time_us,
            MAX(ao.timestamp) as last_updated
        FROM arbitrage_opportunities ao
        LEFT JOIN execution_results er ON ao.opportunity_id = er.opportunity_id
        WHERE ao.timestamp >= NOW() - INTERVAL '24 hours'
    "#.to_string();

    if let Some(venue_pair) = &params.venue_pair {
        query_str.push_str(&format!(
            " AND CONCAT(venue_sell, '->', venue_buy) = '{}'",
            venue_pair
        ));
    }

    query_str.push_str(" GROUP BY venue_sell, venue_buy");

    if let Some(min_ops) = params.min_opportunities {
        query_str.push_str(&format!(" HAVING COUNT(*) >= {}", min_ops));
    }

    let sort_clause = match params.sort_by.as_deref() {
        Some("success_rate") => " ORDER BY (COUNT(CASE WHEN er.success = true THEN 1 END)::float / NULLIF(COUNT(CASE WHEN er.executed = true THEN 1 END), 0)) DESC",
        Some("profit") => " ORDER BY AVG(CASE WHEN er.success = true THEN ao.profit_bps END) DESC",
        Some("volume") | _ => " ORDER BY COUNT(*) DESC",
    };

    query_str.push_str(sort_clause);

    match sqlx::query_as::<
        _,
        (
            String,
            i64,
            i64,
            i64,
            Option<f64>,
            Option<f64>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(&query_str)
    .fetch_all(db.as_ref())
    .await
    {
        Ok(rows) => {
            let performance: Vec<VenuePerformanceResponse> = rows
                .into_iter()
                .map(
                    |(
                        venue_pair,
                        total_opportunities,
                        executed_opportunities,
                        successful_executions,
                        average_profit_bps,
                        average_execution_time_us,
                        last_updated,
                    )| {
                        let success_rate_pct = if executed_opportunities > 0 {
                            Some(
                                (successful_executions as f64 / executed_opportunities as f64)
                                    * 100.0,
                            )
                        } else {
                            None
                        };

                        VenuePerformanceResponse {
                            venue_pair,
                            total_opportunities: total_opportunities as i32,
                            executed_opportunities: executed_opportunities as i32,
                            successful_executions: successful_executions as i32,
                            success_rate_pct,
                            average_profit_bps,
                            average_execution_time_us,
                            last_updated: last_updated
                                .map(|ts| ts.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                .unwrap_or_default(),
                        }
                    },
                )
                .collect();

            Ok(Json(ApiResponse::success(performance)))
        }
        Err(e) => {
            error!("Failed to fetch venue performance: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/arbitrage/live - Get real-time statistics and recent activity
pub async fn get_live_stats(
    State(db): State<Arc<PgPool>>,
) -> Result<Json<ApiResponse<LiveStatsResponse>>, StatusCode> {
    // Get overall stats for last 24 hours
    let stats_query = r#"
        SELECT 
            COUNT(DISTINCT ao.opportunity_id) as total_opportunities,
            COUNT(CASE WHEN er.executed = true THEN 1 END) as total_executed,
            COUNT(CASE WHEN er.success = true THEN 1 END) as total_successful,
            AVG(CASE WHEN er.success = true THEN ao.profit_bps END) as avg_profit
        FROM arbitrage_opportunities ao
        LEFT JOIN execution_results er ON ao.opportunity_id = er.opportunity_id
        WHERE ao.timestamp >= NOW() - INTERVAL '24 hours'
    "#;

    let (total_opportunities, total_executed, total_successful, avg_profit) =
        match sqlx::query_as::<_, (i64, i64, i64, Option<f64>)>(stats_query)
            .fetch_one(db.as_ref())
            .await
        {
            Ok(row) => row,
            Err(e) => {
                error!("Failed to fetch live stats: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

    let overall_success_rate = if total_executed > 0 {
        (total_successful as f64 / total_executed as f64) * 100.0
    } else {
        0.0
    };

    // Get recent opportunities (last 50)
    let recent_query = r#"
        SELECT 
            ao.opportunity_id,
            ao.timestamp,
            ao.slot,
            ao.venue_sell,
            ao.venue_buy,
            ao.token_pair,
            ao.profit_bps,
            ao.amount_in,
            ao.expected_amount_out,
            ao.detection_time_us,
            ao.strategy_type,
            er.executed,
            er.success,
            er.actual_profit_bps,
            er.execution_time_us,
            er.failure_reason
        FROM arbitrage_opportunities ao
        LEFT JOIN execution_results er ON ao.opportunity_id = er.opportunity_id
        WHERE ao.timestamp >= NOW() - INTERVAL '1 hour'
        ORDER BY ao.timestamp DESC 
        LIMIT 50
    "#;

    let recent_opportunities = match sqlx::query_as::<
        _,
        (
            String,
            chrono::DateTime<chrono::Utc>,
            i64,
            String,
            String,
            String,
            i32,
            Option<f64>,
            Option<f64>,
            Option<i64>,
            Option<String>,
            Option<bool>,
            Option<bool>,
            Option<i32>,
            Option<i64>,
            Option<String>,
        ),
    >(recent_query)
    .fetch_all(db.as_ref())
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(
                |(
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
                    strategy_type,
                    executed,
                    success,
                    actual_profit_bps,
                    execution_time_us,
                    failure_reason,
                )| {
                    ArbitrageOpportunityResponse {
                        opportunity_id,
                        timestamp: timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                        slot,
                        venue_sell,
                        venue_buy,
                        token_pair,
                        profit_bps,
                        amount_in,
                        expected_amount_out,
                        detection_time_us,
                        strategy_type,
                        executed,
                        success,
                        actual_profit_bps,
                        execution_time_us,
                        failure_reason,
                    }
                },
            )
            .collect(),
        Err(e) => {
            error!("Failed to fetch recent opportunities: {}", e);
            Vec::new()
        }
    };

    // Get top venues by volume
    let top_venues_query = r#"
        SELECT 
            CONCAT(venue_sell, '->', venue_buy) as venue_pair,
            COUNT(*) as total_opportunities,
            COUNT(CASE WHEN er.executed = true THEN 1 END) as executed_opportunities,
            COUNT(CASE WHEN er.success = true THEN 1 END) as successful_executions,
            AVG(CASE WHEN er.success = true THEN ao.profit_bps END) as average_profit_bps,
            AVG(CASE WHEN er.executed = true THEN er.execution_time_us END) as average_execution_time_us,
            MAX(ao.timestamp) as last_updated
        FROM arbitrage_opportunities ao
        LEFT JOIN execution_results er ON ao.opportunity_id = er.opportunity_id
        WHERE ao.timestamp >= NOW() - INTERVAL '24 hours'
        GROUP BY venue_sell, venue_buy
        ORDER BY COUNT(*) DESC
        LIMIT 10
    "#;

    let top_venues = match sqlx::query_as::<
        _,
        (
            String,
            i64,
            i64,
            i64,
            Option<f64>,
            Option<f64>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(top_venues_query)
    .fetch_all(db.as_ref())
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(
                |(
                    venue_pair,
                    total_opportunities,
                    executed_opportunities,
                    successful_executions,
                    average_profit_bps,
                    average_execution_time_us,
                    last_updated,
                )| {
                    let success_rate_pct = if executed_opportunities > 0 {
                        Some((successful_executions as f64 / executed_opportunities as f64) * 100.0)
                    } else {
                        None
                    };

                    VenuePerformanceResponse {
                        venue_pair,
                        total_opportunities: total_opportunities as i32,
                        executed_opportunities: executed_opportunities as i32,
                        successful_executions: successful_executions as i32,
                        success_rate_pct,
                        average_profit_bps,
                        average_execution_time_us,
                        last_updated: last_updated
                            .map(|ts| ts.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_default(),
                    }
                },
            )
            .collect(),
        Err(_) => Vec::new(),
    };

    // Get active tokens
    let tokens_query = r#"
        SELECT DISTINCT token_pair 
        FROM arbitrage_opportunities 
        WHERE timestamp >= NOW() - INTERVAL '1 hour'
        ORDER BY token_pair
        LIMIT 100
    "#;

    let active_tokens = match sqlx::query_as::<_, (String,)>(tokens_query)
        .fetch_all(db.as_ref())
        .await
    {
        Ok(rows) => rows.into_iter().map(|(token_pair,)| token_pair).collect(),
        Err(_) => Vec::new(),
    };

    let response = LiveStatsResponse {
        current_time: chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
        total_opportunities_detected: total_opportunities,
        total_executed,
        total_successful,
        overall_success_rate,
        average_profit_bps: avg_profit.unwrap_or(0.0),
        top_venues,
        recent_opportunities,
        active_tokens,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// GET /api/arbitrage/summary - Quick summary stats
pub async fn get_summary(
    State(db): State<Arc<PgPool>>,
) -> Result<Json<ApiResponse<HashMap<String, serde_json::Value>>>, StatusCode> {
    let query = r#"
        SELECT 
            COUNT(DISTINCT ao.opportunity_id) as total_opportunities_24h,
            COUNT(CASE WHEN ao.timestamp >= NOW() - INTERVAL '1 hour' THEN 1 END) as opportunities_last_hour,
            COUNT(CASE WHEN er.success = true THEN 1 END) as successful_executions_24h,
            AVG(CASE WHEN er.success = true THEN ao.profit_bps END) as avg_profit_bps,
            MAX(ao.profit_bps) as max_profit_bps,
            COUNT(DISTINCT CONCAT(ao.venue_sell, '->', ao.venue_buy)) as active_venue_pairs,
            COUNT(DISTINCT ao.token_pair) as active_token_pairs
        FROM arbitrage_opportunities ao
        LEFT JOIN execution_results er ON ao.opportunity_id = er.opportunity_id
        WHERE ao.timestamp >= NOW() - INTERVAL '24 hours'
    "#;

    match sqlx::query_as::<_, (i64, i64, i64, Option<f64>, Option<i32>, i64, i64)>(query)
        .fetch_one(db.as_ref())
        .await
    {
        Ok((
            total_24h,
            last_hour,
            successful,
            avg_profit,
            max_profit,
            venue_pairs,
            token_pairs,
        )) => {
            let mut summary = HashMap::new();
            summary.insert(
                "total_opportunities_24h".to_string(),
                serde_json::json!(total_24h),
            );
            summary.insert(
                "opportunities_last_hour".to_string(),
                serde_json::json!(last_hour),
            );
            summary.insert(
                "successful_executions_24h".to_string(),
                serde_json::json!(successful),
            );
            summary.insert(
                "average_profit_bps".to_string(),
                serde_json::json!(avg_profit.unwrap_or(0.0)),
            );
            summary.insert(
                "max_profit_bps".to_string(),
                serde_json::json!(max_profit.unwrap_or(0)),
            );
            summary.insert(
                "active_venue_pairs".to_string(),
                serde_json::json!(venue_pairs),
            );
            summary.insert(
                "active_token_pairs".to_string(),
                serde_json::json!(token_pairs),
            );
            summary.insert(
                "system_status".to_string(),
                serde_json::json!("MAD_MAX_OPERATIONAL"),
            );

            Ok(Json(ApiResponse::success(summary)))
        }
        Err(e) => {
            error!("Failed to fetch summary: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/health - Health check endpoint
pub async fn health_check(
    State(db): State<Arc<PgPool>>,
) -> Result<Json<ApiResponse<HashMap<String, serde_json::Value>>>, StatusCode> {
    // Test database connection
    match sqlx::query("SELECT 1").fetch_one(db.as_ref()).await {
        Ok(_) => {
            let mut health = HashMap::new();
            health.insert("status".to_string(), serde_json::json!("healthy"));
            health.insert("database".to_string(), serde_json::json!("connected"));
            health.insert(
                "timestamp".to_string(),
                serde_json::json!(chrono::Utc::now().timestamp()),
            );

            Ok(Json(ApiResponse::success(health)))
        }
        Err(e) => {
            error!("Database health check failed: {}", e);
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}
