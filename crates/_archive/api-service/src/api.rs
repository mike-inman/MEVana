use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

use crate::database::DatabaseService;
use crate::handlers::*;
use crate::models::*;
use crate::middleware::request_id::RequestIdLayer;

#[derive(OpenApi)]
#[openapi(
    paths(
        get_wallets,
        get_wallet_by_id,
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
    ),
    components(schemas(
        WalletRecord,
        WalletQueryParams,
        ArbitrageOpportunityRecord,
        ArbitrageQueryParams,
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
        ErrorResponse,
    )),
    tags(
        (name = "wallets", description = "Wallet management and information"),
        (name = "arbitrage", description = "Arbitrage opportunities and execution results"),
        (name = "metrics", description = "Performance metrics and system health"),
        (name = "tokens", description = "Token registry and metadata"),
        (name = "statistics", description = "Trading statistics and analytics"),
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
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
        )
        
        // Health check endpoint
        .route("/health", get(health_check))
        
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
        .route("/arbitrage/opportunities", get(get_arbitrage_opportunities))
        .route("/arbitrage/opportunities/:opportunity_id", get(get_arbitrage_opportunity_by_id))
        .route("/arbitrage/executions", get(get_execution_results))
        
        // Metrics routes
        .route("/metrics/performance", get(get_performance_metrics))
        .route("/metrics/health", get(get_system_health))
        .route("/metrics/trading-performance", get(get_trading_performance_summary))
        
        // Token routes
        .route("/tokens", get(get_tokens))
        .route("/tokens/:mint", get(get_token_by_mint))
        
        // Statistics routes
        .route("/statistics/venues", get(get_venue_statistics))
        .route("/statistics/token-pairs", get(get_token_pair_statistics))
        .route("/statistics/wallet-performance", get(get_wallet_performance_statistics))
}

async fn health_check() -> &'static str {
    "OK"
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
        let database = DatabaseService::new("postgresql://test".to_string()).await.unwrap();
        let app = create_api_router(database);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}