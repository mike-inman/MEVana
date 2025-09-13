//! Health monitoring models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub components: HashMap<String, ComponentHealth>,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub response_time_ms: Option<u64>,
    pub details: Option<serde_json::Value>,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub active_connections: u32,
    pub requests_per_second: f64,
}