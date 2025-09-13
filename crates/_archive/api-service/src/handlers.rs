use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::database::DatabaseService;
use crate::models::*;

pub type AppState = Arc<DatabaseService>;

// ============================================================================
// WALLET HANDLERS
// ============================================================================

/// Get list of wallets with pagination
#[utoipa::path(
    get,
    path = "/api/v1/wallets",
    params(WalletQueryParams),
    responses(
        (status = 200, description = "List of wallets", body = PaginatedResponse<WalletRecord>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "wallets"
)]
pub async fn get_wallets(
    State(db): State<AppState>,
    Query(params): Query<WalletQueryParams>,
) -> impl IntoResponse {
    let limit = params.limit;
    let offset = params.offset;

    match db.get_wallets(limit, offset).await {
        Ok(wallets) => {
            let pagination = PaginationInfo {
                offset: offset.unwrap_or(0),
                limit: limit.unwrap_or(50),
                total: None,
                has_more: wallets.len() == limit.unwrap_or(50) as usize,
            };

            let response = PaginatedResponse {
                data: wallets,
                pagination,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

/// Get wallet by ID
#[utoipa::path(
    get,
    path = "/api/v1/wallets/{wallet_id}",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID")
    ),
    responses(
        (status = 200, description = "Wallet details", body = ApiResponse<WalletRecord>),
        (status = 404, description = "Wallet not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "wallets"
)]
pub async fn get_wallet_by_id(
    State(db): State<AppState>,
    Path(wallet_id): Path<Uuid>,
) -> impl IntoResponse {
    match db.get_wallet_by_id(wallet_id).await {
        Ok(Some(wallet)) => Ok(Json(ApiResponse::success(wallet))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Wallet not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

// ============================================================================
// ARBITRAGE HANDLERS
// ============================================================================

/// Get arbitrage opportunities with filtering
#[utoipa::path(
    get,
    path = "/api/v1/arbitrage/opportunities",
    params(ArbitrageQueryParams),
    responses(
        (status = 200, description = "List of arbitrage opportunities", body = PaginatedResponse<ArbitrageOpportunityRecord>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "arbitrage"
)]
pub async fn get_arbitrage_opportunities(
    State(db): State<AppState>,
    Query(params): Query<ArbitrageQueryParams>,
) -> impl IntoResponse {
    match db.get_arbitrage_opportunities(params).await {
        Ok(opportunities) => {
            let pagination = PaginationInfo {
                offset: opportunities.len() as i32,
                limit: 100,
                total: None,
                has_more: opportunities.len() == 100,
            };

            let response = PaginatedResponse {
                data: opportunities,
                pagination,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

/// Get arbitrage opportunity by ID
#[utoipa::path(
    get,
    path = "/api/v1/arbitrage/opportunities/{opportunity_id}",
    params(
        ("opportunity_id" = String, Path, description = "Opportunity ID")
    ),
    responses(
        (status = 200, description = "Arbitrage opportunity details", body = ApiResponse<ArbitrageOpportunityRecord>),
        (status = 404, description = "Opportunity not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "arbitrage"
)]
pub async fn get_arbitrage_opportunity_by_id(
    State(db): State<AppState>,
    Path(opportunity_id): Path<String>,
) -> impl IntoResponse {
    match db.get_arbitrage_opportunity_by_id(opportunity_id).await {
        Ok(Some(opportunity)) => Ok(Json(ApiResponse::success(opportunity))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Opportunity not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

/// Get execution results with filtering
#[utoipa::path(
    get,
    path = "/api/v1/arbitrage/executions",
    params(ExecutionQueryParams),
    responses(
        (status = 200, description = "List of execution results", body = PaginatedResponse<ExecutionResultRecord>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "arbitrage"
)]
pub async fn get_execution_results(
    State(db): State<AppState>,
    Query(params): Query<ExecutionQueryParams>,
) -> impl IntoResponse {
    match db.get_execution_results(params).await {
        Ok(results) => {
            let pagination = PaginationInfo {
                offset: results.len() as i32,
                limit: 100,
                total: None,
                has_more: results.len() == 100,
            };

            let response = PaginatedResponse {
                data: results,
                pagination,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

// ============================================================================
// PERFORMANCE METRICS HANDLERS
// ============================================================================

/// Get performance metrics
#[utoipa::path(
    get,
    path = "/api/v1/metrics/performance",
    params(MetricsQueryParams),
    responses(
        (status = 200, description = "Performance metrics", body = PaginatedResponse<PerformanceMetricRecord>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "metrics"
)]
pub async fn get_performance_metrics(
    State(db): State<AppState>,
    Query(params): Query<MetricsQueryParams>,
) -> impl IntoResponse {
    match db.get_performance_metrics(params).await {
        Ok(metrics) => {
            let pagination = PaginationInfo {
                offset: metrics.len() as i32,
                limit: 100,
                total: None,
                has_more: metrics.len() == 100,
            };

            let response = PaginatedResponse {
                data: metrics,
                pagination,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

/// Get system health status
#[utoipa::path(
    get,
    path = "/api/v1/metrics/health",
    responses(
        (status = 200, description = "System health status", body = ApiResponse<SystemHealthRecord>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "metrics"
)]
pub async fn get_system_health(State(db): State<AppState>) -> impl IntoResponse {
    match db.get_system_health().await {
        Ok(health) => Ok(Json(ApiResponse::success(health))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

/// Get trading performance summary
#[utoipa::path(
    get,
    path = "/api/v1/metrics/trading-performance",
    params(TradingPerformanceQueryParams),
    responses(
        (status = 200, description = "Trading performance summary", body = ApiResponse<Vec<TradingPerformanceRecord>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "metrics"
)]
pub async fn get_trading_performance_summary(
    State(db): State<AppState>,
    Query(params): Query<TradingPerformanceQueryParams>,
) -> impl IntoResponse {
    match db.get_trading_performance_summary(params.days).await {
        Ok(performance) => Ok(Json(ApiResponse::success(performance))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

// ============================================================================
// TOKEN HANDLERS
// ============================================================================

/// Get token registry
#[utoipa::path(
    get,
    path = "/api/v1/tokens",
    params(TokenQueryParams),
    responses(
        (status = 200, description = "List of tokens", body = PaginatedResponse<TokenRecord>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "tokens"
)]
pub async fn get_tokens(
    State(db): State<AppState>,
    Query(params): Query<TokenQueryParams>,
) -> impl IntoResponse {
    match db.get_tokens(params).await {
        Ok(tokens) => {
            let pagination = PaginationInfo {
                offset: tokens.len() as i32,
                limit: 100,
                total: None,
                has_more: tokens.len() == 100,
            };

            let response = PaginatedResponse {
                data: tokens,
                pagination,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

/// Get token by mint address
#[utoipa::path(
    get,
    path = "/api/v1/tokens/{mint}",
    params(
        ("mint" = String, Path, description = "Token mint address")
    ),
    responses(
        (status = 200, description = "Token details", body = ApiResponse<TokenRecord>),
        (status = 404, description = "Token not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "tokens"
)]
pub async fn get_token_by_mint(
    State(db): State<AppState>,
    Path(mint): Path<String>,
) -> impl IntoResponse {
    match db.get_token_by_mint(mint).await {
        Ok(Some(token)) => Ok(Json(ApiResponse::success(token))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Token not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

// ============================================================================
// STATISTICS HANDLERS
// ============================================================================

/// Get venue statistics
#[utoipa::path(
    get,
    path = "/api/v1/statistics/venues",
    responses(
        (status = 200, description = "Venue statistics", body = ApiResponse<Vec<VenueStatistics>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "statistics"
)]
pub async fn get_venue_statistics(State(_db): State<AppState>) -> impl IntoResponse {
    // Placeholder implementation
    let venues = vec![];
    Ok(Json(ApiResponse::success(venues)))
}

/// Get token pair statistics
#[utoipa::path(
    get,
    path = "/api/v1/statistics/token-pairs",
    responses(
        (status = 200, description = "Token pair statistics", body = ApiResponse<Vec<TokenPairStatistics>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "statistics"
)]
pub async fn get_token_pair_statistics(State(_db): State<AppState>) -> impl IntoResponse {
    // Placeholder implementation
    let pairs = vec![];
    Ok(Json(ApiResponse::success(pairs)))
}

/// Get wallet performance statistics
#[utoipa::path(
    get,
    path = "/api/v1/statistics/wallet-performance",
    responses(
        (status = 200, description = "Wallet performance statistics", body = ApiResponse<Vec<WalletPerformanceStatistics>>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "statistics"
)]
pub async fn get_wallet_performance_statistics(State(_db): State<AppState>) -> impl IntoResponse {
    // Placeholder implementation
    let stats = vec![];
    Ok(Json(ApiResponse::success(stats)))
}