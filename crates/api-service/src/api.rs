use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::database::DatabaseService;
use crate::handlers::*;
use crate::middleware::request_id::RequestIdLayer;
use crate::models::*;

#[derive(OpenApi)]
#[openapi(
    paths(
        get_wallets,
        get_wallet_by_id,
        create_arbitrage_opportunity,
        get_arbitrage_opportunities,
        get_arbitrage_opportunity_by_id,
        get_execution_results,
        get_performance_metrics,
        get_system_health,
        get_trading_performance_summary,
        get_tokens,
        get_token_by_mint,
        get_venue_statistics,
        get_token_pair_statistics,
        get_wallet_performance_statistics,
        get_smart_filter_config,
        update_smart_filter_config,
        get_smart_filter_performance,
        get_smart_filter_config_history,
    ),
    components(schemas(
        WalletRecord,
        WalletQueryParams,
        ArbitrageOpportunityRecord,
        ArbitrageQueryParams,
        CreateArbitrageRequest,
        ExecutionResultRecord,
        ExecutionQueryParams,
        PerformanceMetricRecord,
        MetricsQueryParams,
        SystemHealthRecord,
        TradingPerformanceRecord,
        TradingPerformanceQueryParams,
        TokenRecord,
        TokenQueryParams,
        FlashLoanRecord,
        FlashLoanQueryParams,
        AuditLogRecord,
        AuditLogQueryParams,
        PaginatedResponse<WalletRecord>,
        PaginationInfo,
        ApiResponse<WalletRecord>,
        VenueStatistics,
        TokenPairStatistics,
        WalletPerformanceStatistics,
        SmartFilterConfig,
        UpdateSmartFilterConfigRequest,
        SmartFilterConfigUpdateResponse,
        SmartFilterPerformanceSnapshot,
        SmartFilterConfigHistoryRecord,
        SmartFilterPerformanceQueryParams,
        SmartFilterConfigHistoryQueryParams,
        ErrorResponse,
    )),
    tags(
        (name = "wallets", description = "Wallet management and information"),
        (name = "arbitrage", description = "Arbitrage opportunities and execution results"),
        (name = "metrics", description = "Performance metrics and system health"),
        (name = "tokens", description = "Token registry and metadata"),
        (name = "statistics", description = "Trading statistics and analytics"),
        (name = "smart-filter", description = "Smart Filter configuration and performance"),
    ),
    info(
        title = "MEVana API",
        description = "Comprehensive API for the MEVana arbitrage trading system",
        version = "1.0.0",
        contact(
            name = "MEVana Team",
            email = "api@mevana.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        ),
    ),
    servers(
        (url = "http://localhost:8080", description = "Development server"),
        (url = "https://api.mevana.com", description = "Production server"),
    )
)]
pub struct ApiDoc;

pub fn create_api_router(database: DatabaseService) -> Router {
    let state = Arc::new(database);

    Router::new()
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Health check endpoint
        .route("/health", get(health_check))
        .route("/health/detailed", get(detailed_health_check))
        // API v1 routes
        .nest("/api/v1", create_v1_routes())
        // Add middleware
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(RequestIdLayer::new())
        // Add state
        .with_state(state)
}

fn create_v1_routes() -> Router<Arc<DatabaseService>> {
    Router::new()
        // Wallet routes
        .route("/wallets", get(get_wallets))
        .route("/wallets/:wallet_id", get(get_wallet_by_id))
        // Arbitrage routes
        .route(
            "/arbitrage/opportunities",
            post(create_arbitrage_opportunity).get(get_arbitrage_opportunities),
        )
        .route(
            "/arbitrage/opportunities/:opportunity_id",
            get(get_arbitrage_opportunity_by_id),
        )
        .route("/arbitrage/executions", get(get_execution_results))
        // Metrics routes
        .route("/metrics/performance", get(get_performance_metrics))
        .route("/metrics/health", get(get_system_health))
        .route(
            "/metrics/trading-performance",
            get(get_trading_performance_summary),
        )
        // Token routes
        .route("/tokens", get(get_tokens))
        .route("/tokens/:mint", get(get_token_by_mint))
        // Statistics routes
        .route("/statistics/venues", get(get_venue_statistics))
        .route("/statistics/token-pairs", get(get_token_pair_statistics))
        .route(
            "/statistics/wallet-performance",
            get(get_wallet_performance_statistics),
        )
        // Smart Filter routes
        .route(
            "/smart-filter/config",
            get(get_smart_filter_config).post(update_smart_filter_config),
        )
        .route(
            "/smart-filter/performance",
            get(get_smart_filter_performance),
        )
        .route(
            "/smart-filter/config/history",
            get(get_smart_filter_config_history),
        )
}

async fn health_check() -> &'static str {
    "OK"
}

async fn detailed_health_check(
    State(db): State<Arc<DatabaseService>>,
) -> impl axum::response::IntoResponse {
    use axum::Json;
    use chrono::Utc;
    use serde_json::json;

    let mut status = "healthy";
    let mut checks = std::collections::HashMap::new();
    let start_time = std::time::Instant::now();

    // Check database connectivity
    match db.test_connection().await {
        Ok(_) => {
            checks.insert(
                "database",
                json!({
                    "status": "healthy",
                    "message": "Database connection successful"
                }),
            );
        }
        Err(e) => {
            checks.insert(
                "database",
                json!({
                    "status": "unhealthy",
                    "message": format!("Database connection failed: {}", e)
                }),
            );
            status = "unhealthy";
        }
    };

    // Check system resources (basic)
    checks.insert(
        "system",
        json!({
            "status": "healthy",
            "uptime_seconds": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "memory_usage": "available" // Could be enhanced with actual memory stats
        }),
    );

    // Check API response time
    let response_time_ms = start_time.elapsed().as_millis();
    checks.insert(
        "response_time",
        json!({
            "status": if response_time_ms < 1000 { "healthy" } else { "slow" },
            "response_time_ms": response_time_ms
        }),
    );

    let health_response = json!({
        "status": status,
        "timestamp": Utc::now(),
        "version": "1.0.0",
        "service": "mevana-api",
        "checks": checks
    });

    let status_code = if status == "healthy" {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(health_response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let database = DatabaseService::new("postgresql://test".to_string())
            .await
            .unwrap();
        let app = create_api_router(database);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
