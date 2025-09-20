use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
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

/// Create a new arbitrage opportunity
#[utoipa::path(
    post,
    path = "/api/v1/arbitrage/opportunities",
    request_body = CreateArbitrageRequest,
    responses(
        (status = 201, description = "Arbitrage opportunity created", body = ApiResponse<ArbitrageOpportunityRecord>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "arbitrage"
)]
pub async fn create_arbitrage_opportunity(
    State(db): State<AppState>,
    Json(request): Json<CreateArbitrageRequest>,
) -> impl IntoResponse {
    // Data validation before database insertion
    if let Err(validation_error) = validate_arbitrage_request(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("VALIDATION_ERROR", &validation_error)),
        ));
    }

    match db.create_arbitrage_opportunity(request).await {
        Ok(created_opportunity) => Ok((
            StatusCode::CREATED,
            Json(ApiResponse::success(created_opportunity)),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

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
    let opportunity_id = match opportunity_id.parse::<u64>() {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_ID",
                    "Invalid opportunity ID format",
                )),
            ))
        }
    };

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
pub async fn get_venue_statistics(
    State(_db): State<AppState>,
) -> Result<Json<ApiResponse<Vec<VenueStatistics>>>, StatusCode> {
    // Placeholder implementation
    let venues: Vec<VenueStatistics> = vec![];
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
pub async fn get_token_pair_statistics(
    State(_db): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TokenPairStatistics>>>, StatusCode> {
    // Placeholder implementation
    let pairs: Vec<TokenPairStatistics> = vec![];
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
pub async fn get_wallet_performance_statistics(
    State(_db): State<AppState>,
) -> Result<Json<ApiResponse<Vec<WalletPerformanceStatistics>>>, StatusCode> {
    // Placeholder implementation
    let stats: Vec<WalletPerformanceStatistics> = vec![];
    Ok(Json(ApiResponse::success(stats)))
}

// ============================================================================
// SMART FILTER HANDLERS
// ============================================================================

/// Get current Smart Filter configuration
#[utoipa::path(
    get,
    path = "/api/v1/smart-filter/config",
    responses(
        (status = 200, description = "Current Smart Filter configuration", body = ApiResponse<SmartFilterConfig>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "smart-filter"
)]
pub async fn get_smart_filter_config(State(db): State<AppState>) -> impl IntoResponse {
    match db.get_smart_filter_config().await {
        Ok(config) => Ok(Json(ApiResponse::success(config))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

/// Update Smart Filter configuration parameter
#[utoipa::path(
    post,
    path = "/api/v1/smart-filter/config",
    request_body = UpdateSmartFilterConfigRequest,
    responses(
        (status = 200, description = "Configuration updated successfully", body = ApiResponse<SmartFilterConfigUpdateResponse>),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "smart-filter"
)]
pub async fn update_smart_filter_config(
    State(db): State<AppState>,
    Json(request): Json<UpdateSmartFilterConfigRequest>,
) -> impl IntoResponse {
    // Validate the parameter and value
    if let Err(validation_error) = validate_smart_filter_config(&request) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("VALIDATION_ERROR", &validation_error)),
        ));
    }

    match db.update_smart_filter_config(request).await {
        Ok(update_response) => Ok(Json(ApiResponse::success(update_response))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", &e.to_string())),
        )),
    }
}

/// Get Smart Filter performance snapshots
#[utoipa::path(
    get,
    path = "/api/v1/smart-filter/performance",
    params(SmartFilterPerformanceQueryParams),
    responses(
        (status = 200, description = "Smart Filter performance data", body = PaginatedResponse<SmartFilterPerformanceSnapshot>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "smart-filter"
)]
pub async fn get_smart_filter_performance(
    State(db): State<AppState>,
    Query(params): Query<SmartFilterPerformanceQueryParams>,
) -> impl IntoResponse {
    match db.get_smart_filter_performance(params).await {
        Ok(snapshots) => {
            let pagination = PaginationInfo {
                offset: snapshots.len() as i32,
                limit: 100,
                total: None,
                has_more: snapshots.len() == 100,
            };

            let response = PaginatedResponse {
                data: snapshots,
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

/// Get Smart Filter configuration change history
#[utoipa::path(
    get,
    path = "/api/v1/smart-filter/config/history",
    params(SmartFilterConfigHistoryQueryParams),
    responses(
        (status = 200, description = "Configuration change history", body = PaginatedResponse<SmartFilterConfigHistoryRecord>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "smart-filter"
)]
pub async fn get_smart_filter_config_history(
    State(db): State<AppState>,
    Query(params): Query<SmartFilterConfigHistoryQueryParams>,
) -> impl IntoResponse {
    match db.get_smart_filter_config_history(params).await {
        Ok(history) => {
            let pagination = PaginationInfo {
                offset: history.len() as i32,
                limit: 100,
                total: None,
                has_more: history.len() == 100,
            };

            let response = PaginatedResponse {
                data: history,
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
// VALIDATION FUNCTIONS
// ============================================================================

/// Validates arbitrage opportunity request data before database insertion
fn validate_arbitrage_request(request: &CreateArbitrageRequest) -> Result<(), String> {
    // Validate opportunity_id (should be non-empty)
    if request.opportunity_id.trim().is_empty() {
        return Err("opportunity_id cannot be empty".to_string());
    }

    // Validate venue names (should be non-empty)
    if request.venue_sell.trim().is_empty() {
        return Err("venue_sell cannot be empty".to_string());
    }
    if request.venue_buy.trim().is_empty() {
        return Err("venue_buy cannot be empty".to_string());
    }

    // Validate token_pair (should be non-empty)
    if request.token_pair.trim().is_empty() {
        return Err("token_pair cannot be empty".to_string());
    }

    // Validate strategy_type (should be non-empty)
    if request.strategy_type.trim().is_empty() {
        return Err("strategy_type cannot be empty".to_string());
    }

    // Validate numeric fields
    if request.amount_in <= 0.0 {
        return Err("amount_in must be positive".to_string());
    }
    if !request.amount_in.is_finite() {
        return Err("amount_in must be a finite number".to_string());
    }

    if request.expected_amount_out < 0.0 {
        return Err("expected_amount_out cannot be negative".to_string());
    }
    if !request.expected_amount_out.is_finite() {
        return Err("expected_amount_out must be a finite number".to_string());
    }

    // Validate optional price_impact_pct if present
    if let Some(price_impact) = request.price_impact_pct {
        if !price_impact.is_finite() {
            return Err("price_impact_pct must be a finite number".to_string());
        }
        if price_impact < -100.0 || price_impact > 100.0 {
            return Err("price_impact_pct must be between -100% and 100%".to_string());
        }
    }

    // Validate slot if present (should be reasonable)
    if let Some(slot) = request.slot {
        if slot == 0 {
            return Err("slot should not be zero".to_string());
        }
    }

    // Validate detection_time_us if present (should be reasonable)
    if let Some(detection_time) = request.detection_time_us {
        if detection_time > 10_000_000 {
            // 10 seconds in microseconds
            return Err("detection_time_us seems unreasonably high (>10s)".to_string());
        }
    }

    // Validate profit_bps (should be reasonable)
    if request.profit_bps < -10000 || request.profit_bps > 10000 {
        return Err(
            "profit_bps should be between -10000 and 10000 (i.e., -100% to 100%)".to_string(),
        );
    }

    Ok(())
}

/// Validates Smart Filter configuration update request
fn validate_smart_filter_config(request: &UpdateSmartFilterConfigRequest) -> Result<(), String> {
    // Validate parameter name
    let valid_parameters = [
        "min_profit_bps",
        "max_profit_bps",
        "min_net_profit_sol",
        "min_venue_success_rate",
        "max_gas_budget_pct",
        "max_concurrent_opportunities",
        "competition_lookback_ms",
        "learning_rate",
        "min_samples_for_learning",
    ];

    if !valid_parameters.contains(&request.parameter.as_str()) {
        return Err(format!("Invalid parameter name: {}", request.parameter));
    }

    // Validate parameter-specific values
    match request.parameter.as_str() {
        "min_profit_bps" => {
            if request.value < 0.0 || request.value > 10000.0 {
                return Err("min_profit_bps must be between 0 and 10000".to_string());
            }
        }
        "max_profit_bps" => {
            if request.value < 0.0 || request.value > 10000.0 {
                return Err("max_profit_bps must be between 0 and 10000".to_string());
            }
        }
        "min_net_profit_sol" => {
            if request.value < 0.0 || request.value > 10.0 {
                return Err("min_net_profit_sol must be between 0 and 10 SOL".to_string());
            }
        }
        "min_venue_success_rate" => {
            if request.value < 0.0 || request.value > 100.0 {
                return Err("min_venue_success_rate must be between 0 and 100".to_string());
            }
        }
        "max_gas_budget_pct" => {
            if request.value < 0.0 || request.value > 100.0 {
                return Err("max_gas_budget_pct must be between 0 and 100".to_string());
            }
        }
        "max_concurrent_opportunities" => {
            if request.value < 1.0 || request.value > 1000.0 || request.value.fract() != 0.0 {
                return Err(
                    "max_concurrent_opportunities must be an integer between 1 and 1000"
                        .to_string(),
                );
            }
        }
        "competition_lookback_ms" => {
            if request.value < 100.0 || request.value > 60000.0 || request.value.fract() != 0.0 {
                return Err(
                    "competition_lookback_ms must be an integer between 100 and 60000".to_string(),
                );
            }
        }
        "learning_rate" => {
            if request.value < 0.001 || request.value > 1.0 {
                return Err("learning_rate must be between 0.001 and 1.0".to_string());
            }
        }
        "min_samples_for_learning" => {
            if request.value < 1.0 || request.value > 10000.0 || request.value.fract() != 0.0 {
                return Err(
                    "min_samples_for_learning must be an integer between 1 and 10000".to_string(),
                );
            }
        }
        _ => return Err("Unknown parameter".to_string()),
    }

    // Validate applied_by field
    if request.applied_by.trim().is_empty() {
        return Err("applied_by cannot be empty".to_string());
    }

    Ok(())
}
