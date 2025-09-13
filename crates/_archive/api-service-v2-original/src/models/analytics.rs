//! Analytics and reporting models

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Analytics query request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::IntoParams)]
pub struct AnalyticsQuery {
    pub metric: AnalyticsMetric,
    pub time_range: TimeRange,
    pub granularity: Option<TimeGranularity>,
    pub filters: Option<serde_json::Value>,
}

/// Analytics metrics enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum AnalyticsMetric {
    TradingVolume,
    ProfitLoss,
    TradeCount,
    SuccessRate,
    ArbitrageOpportunities,
    WalletBalance,
    UserActivity,
}

/// Time range for analytics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TimeRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Time granularity for data aggregation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TimeGranularity {
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

/// Analytics response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsResponse {
    pub metric: AnalyticsMetric,
    pub data: Vec<DataPoint>,
    pub summary: AnalyticsSummary,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Data point for time series analytics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DataPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
    pub metadata: Option<serde_json::Value>,
}

/// Analytics summary
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnalyticsSummary {
    pub total: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
    pub trend: TrendDirection,
    pub change_percentage: f64,
}

/// Trend direction enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}