//! Arbitrage opportunity handlers
//!
//! HTTP request handlers for arbitrage opportunities management

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::{ApiContext, Permission},
    error::{ApiError, ApiResult},
    models::arbitrage::*,
    services::ApiServices,
};

/// Create a new arbitrage opportunity
#[utoipa::path(
    post,
    path = "/api/v1/arbitrage/opportunities",
    request_body = CreateArbitrageOpportunityRequest,
    responses(
        (status = 201, description = "Arbitrage opportunity created", body = ArbitrageOpportunityResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "arbitrage",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_arbitrage_opportunity(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Json(request): Json<CreateArbitrageOpportunityRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions - use existing permission
    if !ctx.has_permission(Permission::ArbitrageExecute) {
        return Err(ApiError::Forbidden("Insufficient permissions".to_string()));
    }
    
    // Validate request
    request.validate()?;
    
    // Create opportunity - temporary implementation
    let opportunity_id = uuid::Uuid::new_v4();
    let opportunity = ArbitrageOpportunityResponse {
        opportunity: ArbitrageOpportunityRecord {
            id: opportunity_id,
            opportunity_id: request.opportunity_id.clone(),
            timestamp: chrono::Utc::now(),
            slot: request.slot,
            venue_sell: request.venue_sell.clone(),
            venue_buy: request.venue_buy.clone(),
            token_pair: request.token_pair.clone(),
            profit_bps: request.profit_bps,
            amount_in: request.amount_in,
            expected_amount_out: request.expected_amount_out,
            price_impact_pct: request.price_impact_pct,
            detection_time_us: request.detection_time_us,
            strategy_type: request.strategy_type.clone(),
            status: OpportunityStatus::Detected,
            executed: false,
            execution_signature: None,
            actual_profit_bps: None,
            execution_time_us: None,
            failure_reason: None,
            risk_score: request.risk_score,
            confidence_score: request.confidence_score,
            market_conditions: request.market_conditions.clone(),
            raw_opportunity: request.raw_opportunity.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        metadata: OpportunityMetadata {
            estimated_gas_cost: Some(0.005),
            estimated_execution_time_ms: Some(500),
            recommended_action: "EXECUTE".to_string(),
            risk_factors: vec!["Market volatility".to_string()],
            similar_opportunities_count: 3,
            market_volatility: Some(0.15),
        },
    };
    
    Ok((StatusCode::CREATED, Json(opportunity)))
}

/// Get arbitrage opportunities with filtering
#[utoipa::path(
    get,
    path = "/api/v1/arbitrage/opportunities",
    params(ArbitrageQueryParams),
    responses(
        (status = 200, description = "List of arbitrage opportunities", body = PaginatedResponse<ArbitrageOpportunityRecord>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "arbitrage",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_arbitrage_opportunities(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Query(params): Query<ArbitrageQueryParams>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadArbitrageOpportunities)?;
    
    // Get opportunities
    let opportunities = services.arbitrage
        .get_opportunities(params)
        .await?;
    
    Ok(Json(opportunities))
}

/// Get arbitrage opportunity by ID
#[utoipa::path(
    get,
    path = "/api/v1/arbitrage/opportunities/{opportunity_id}",
    params(
        ("opportunity_id" = String, Path, description = "Opportunity ID")
    ),
    responses(
        (status = 200, description = "Arbitrage opportunity details", body = ArbitrageOpportunityRecord),
        (status = 404, description = "Opportunity not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "arbitrage",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_arbitrage_opportunity_by_id(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(opportunity_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadArbitrageOpportunities)?;
    
    // Get opportunity
    let opportunity = services.arbitrage
        .get_opportunity_by_id(opportunity_id)
        .await?;
    
    Ok(Json(opportunity))
}

/// Get execution results with filtering
#[utoipa::path(
    get,
    path = "/api/v1/arbitrage/executions",
    params(ExecutionQueryParams),
    responses(
        (status = 200, description = "List of execution results", body = PaginatedResponse<ExecutionResultRecord>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "arbitrage",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_execution_results(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Query(params): Query<ExecutionQueryParams>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadExecutionResults)?;
    
    // Get results
    let results = services.arbitrage
        .get_execution_results(params)
        .await?;
    
    Ok(Json(results))
}

/// Update arbitrage opportunity status
#[utoipa::path(
    patch,
    path = "/api/v1/arbitrage/opportunities/{opportunity_id}/status",
    params(
        ("opportunity_id" = String, Path, description = "Opportunity ID")
    ),
    request_body = UpdateOpportunityStatusRequest,
    responses(
        (status = 200, description = "Opportunity status updated", body = ArbitrageOpportunityRecord),
        (status = 404, description = "Opportunity not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "arbitrage",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_opportunity_status(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(opportunity_id): Path<String>,
    Json(request): Json<UpdateOpportunityStatusRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::UpdateArbitrageOpportunities)?;
    
    // Validate request
    request.validate()?;
    
    // Update status
    let opportunity = services.arbitrage
        .update_opportunity_status(opportunity_id, request)
        .await?;
    
    Ok(Json(opportunity))
}