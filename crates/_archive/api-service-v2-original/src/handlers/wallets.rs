//! Enterprise wallet management handlers
//!
//! Advanced wallet management with multi-tier security, compliance, and analytics

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::{ApiContext, Permission},
    error::{ApiError, ApiResult},
    models::wallets::*,
    services::ApiServices,
};

/// Create a new wallet with advanced security features
#[utoipa::path(
    post,
    path = "/api/v1/wallets",
    request_body = CreateWalletRequest,
    responses(
        (status = 201, description = "Wallet created successfully", body = WalletResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Insufficient permissions", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:create"])
    )
)]
pub async fn create_wallet(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Json(request): Json<CreateWalletRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::CreateWallet)?;
    
    // Validate request
    request.validate()?;
    
    // Create wallet with security controls
    let wallet = services.wallets
        .create_wallet(request, &ctx)
        .await?;
    
    Ok((StatusCode::CREATED, Json(wallet)))
}

/// Get wallets with advanced filtering and analytics
#[utoipa::path(
    get,
    path = "/api/v1/wallets",
    params(WalletQueryParams),
    responses(
        (status = 200, description = "List of wallets with analytics", body = PaginatedResponse<WalletRecord>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:read"])
    )
)]
pub async fn get_wallets(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Query(params): Query<WalletQueryParams>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadWallets)?;
    
    // Get wallets with analytics
    let response = services.wallets
        .get_wallets_with_analytics(params, &ctx)
        .await?;
    
    Ok(Json(response))
}

/// Get wallet by ID with comprehensive details
#[utoipa::path(
    get,
    path = "/api/v1/wallets/{wallet_id}",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID")
    ),
    responses(
        (status = 200, description = "Wallet details with analytics", body = WalletDetailResponse),
        (status = 404, description = "Wallet not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Access denied", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:read"])
    )
)]
pub async fn get_wallet_by_id(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(wallet_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadWallets)?;
    
    // Get detailed wallet information
    let wallet = services.wallets
        .get_wallet_details(wallet_id, &ctx)
        .await?;
    
    Ok(Json(wallet))
}

/// Update wallet configuration and security settings
#[utoipa::path(
    patch,
    path = "/api/v1/wallets/{wallet_id}",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID")
    ),
    request_body = UpdateWalletRequest,
    responses(
        (status = 200, description = "Wallet updated successfully", body = WalletResponse),
        (status = 404, description = "Wallet not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Insufficient permissions", body = ApiError),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:update"])
    )
)]
pub async fn update_wallet(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(wallet_id): Path<Uuid>,
    Json(request): Json<UpdateWalletRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::UpdateWallet)?;
    
    // Validate request
    request.validate()?;
    
    // Update wallet
    let wallet = services.wallets
        .update_wallet(wallet_id, request, &ctx)
        .await?;
    
    Ok(Json(wallet))
}

/// Get wallet performance analytics
#[utoipa::path(
    get,
    path = "/api/v1/wallets/{wallet_id}/analytics",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID"),
        WalletAnalyticsQuery
    ),
    responses(
        (status = 200, description = "Wallet performance analytics", body = WalletAnalytics),
        (status = 404, description = "Wallet not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:read"])
    )
)]
pub async fn get_wallet_analytics(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(wallet_id): Path<Uuid>,
    Query(params): Query<WalletAnalyticsQuery>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadWallets)?;
    
    // Get analytics
    let analytics = services.wallets
        .get_wallet_analytics(wallet_id, params, &ctx)
        .await?;
    
    Ok(Json(analytics))
}

/// Get wallet transaction history with advanced filtering
#[utoipa::path(
    get,
    path = "/api/v1/wallets/{wallet_id}/transactions",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID"),
        TransactionQueryParams
    ),
    responses(
        (status = 200, description = "Wallet transaction history", body = PaginatedResponse<TransactionRecord>),
        (status = 404, description = "Wallet not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:read"])
    )
)]
pub async fn get_wallet_transactions(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(wallet_id): Path<Uuid>,
    Query(params): Query<TransactionQueryParams>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadWallets)?;
    
    // Get transaction history
    let transactions = services.wallets
        .get_wallet_transactions(wallet_id, params, &ctx)
        .await?;
    
    Ok(Json(transactions))
}

/// Execute wallet security audit
#[utoipa::path(
    post,
    path = "/api/v1/wallets/{wallet_id}/audit",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID")
    ),
    responses(
        (status = 200, description = "Security audit completed", body = SecurityAuditReport),
        (status = 404, description = "Wallet not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Insufficient permissions", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:audit"])
    )
)]
pub async fn audit_wallet_security(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(wallet_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::AuditWallet)?;
    
    // Execute security audit
    let audit_report = services.wallets
        .execute_security_audit(wallet_id, &ctx)
        .await?;
    
    Ok(Json(audit_report))
}

/// Emergency wallet lockdown
#[utoipa::path(
    post,
    path = "/api/v1/wallets/{wallet_id}/emergency-lock",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID")
    ),
    request_body = EmergencyLockRequest,
    responses(
        (status = 200, description = "Wallet locked successfully", body = WalletResponse),
        (status = 404, description = "Wallet not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Insufficient permissions", body = ApiError),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:emergency"])
    )
)]
pub async fn emergency_lock_wallet(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(wallet_id): Path<Uuid>,
    Json(request): Json<EmergencyLockRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check emergency permissions
    ctx.require_permission(Permission::EmergencyWalletControl)?;
    
    // Validate request
    request.validate()?;
    
    // Execute emergency lock
    let wallet = services.wallets
        .emergency_lock(wallet_id, request, &ctx)
        .await?;
    
    Ok(Json(wallet))
}

/// Get wallet portfolio composition and value
#[utoipa::path(
    get,
    path = "/api/v1/wallets/{wallet_id}/portfolio",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID")
    ),
    responses(
        (status = 200, description = "Wallet portfolio details", body = WalletPortfolio),
        (status = 404, description = "Wallet not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:read"])
    )
)]
pub async fn get_wallet_portfolio(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(wallet_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadWallets)?;
    
    // Get portfolio composition
    let portfolio = services.wallets
        .get_wallet_portfolio(wallet_id, &ctx)
        .await?;
    
    Ok(Json(portfolio))
}

/// Get real-time wallet risk assessment
#[utoipa::path(
    get,
    path = "/api/v1/wallets/{wallet_id}/risk-assessment",
    params(
        ("wallet_id" = Uuid, Path, description = "Wallet UUID")
    ),
    responses(
        (status = 200, description = "Wallet risk assessment", body = WalletRiskAssessment),
        (status = 404, description = "Wallet not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:read"])
    )
)]
pub async fn get_wallet_risk_assessment(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Path(wallet_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    // Check permissions
    ctx.require_permission(Permission::ReadWallets)?;
    
    // Get risk assessment
    let risk_assessment = services.wallets
        .get_risk_assessment(wallet_id, &ctx)
        .await?;
    
    Ok(Json(risk_assessment))
}

/// Bulk wallet operations for institutional users
#[utoipa::path(
    post,
    path = "/api/v1/wallets/bulk-operations",
    request_body = BulkWalletOperationRequest,
    responses(
        (status = 200, description = "Bulk operation completed", body = BulkOperationResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Insufficient permissions", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "wallets",
    security(
        ("bearer_auth" = ["wallet:bulk"])
    )
)]
pub async fn bulk_wallet_operations(
    State(services): State<Arc<ApiServices>>,
    ctx: ApiContext,
    Json(request): Json<BulkWalletOperationRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check bulk operation permissions
    ctx.require_permission(Permission::BulkWalletOperations)?;
    
    // Validate request
    request.validate()?;
    
    // Execute bulk operations
    let response = services.wallets
        .execute_bulk_operations(request, &ctx)
        .await?;
    
    Ok(Json(response))
}