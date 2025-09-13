//! Health check handlers

use crate::{
    error::{ApiError, ApiResult},
    models::{ApiResponse, HealthResponse, HealthStatus},
};
use axum::Json;

/// Basic health check endpoint
pub async fn health_check() -> ApiResult<Json<ApiResponse<HealthResponse>>> {
    let health_response = HealthResponse {
        status: HealthStatus::Healthy,
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // Would calculate actual uptime
        components: std::collections::HashMap::new(),
    };
    
    Ok(Json(ApiResponse::success(health_response)))
}