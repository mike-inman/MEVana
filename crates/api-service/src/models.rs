use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use uuid::Uuid;

// ============================================================================
// WALLET MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletRecord {
    pub id: Uuid,
    pub pubkey: String,
    pub tier: String,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub access_count: u64,
    pub is_active: bool,
    pub daily_transactions: u32,
    pub total_volume_lamports: u64,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct WalletQueryParams {
    pub tier: Option<String>,
    pub is_active: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================================================
// ARBITRAGE MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArbitrageOpportunityRecord {
    pub id: u64,
    pub opportunity_id: String,
    pub timestamp: DateTime<Utc>,
    pub slot: u64,
    pub venue_sell: String,
    pub venue_buy: String,
    pub token_pair: String,
    pub profit_bps: i32,
    pub amount_in: f64,
    pub expected_amount_out: f64,
    pub price_impact_pct: Option<f64>,
    pub detection_time_us: u64,
    pub strategy_type: String,
    pub raw_opportunity: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ArbitrageQueryParams {
    pub venue: Option<String>,
    pub token_pair: Option<String>,
    pub min_profit_bps: Option<i32>,
    pub strategy_type: Option<String>,
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateArbitrageRequest {
    pub opportunity_id: String,
    pub slot: Option<u64>,
    pub venue_sell: String,
    pub venue_buy: String,
    pub token_pair: String,
    pub profit_bps: i32,
    pub amount_in: f64,
    pub expected_amount_out: f64,
    pub price_impact_pct: Option<f64>,
    pub detection_time_us: Option<u64>,
    pub strategy_type: String,
    pub raw_opportunity: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionResultRecord {
    pub id: u64,
    pub opportunity_id: String,
    pub timestamp: DateTime<Utc>,
    pub executed: bool,
    pub success: Option<bool>,
    pub actual_profit_bps: Option<i32>,
    pub execution_time_us: Option<u64>,
    pub gas_cost_sol: Option<f64>,
    pub slippage_bps: Option<i32>,
    pub failure_reason: Option<String>,
    pub transaction_signature: Option<String>,
    pub bundle_id: Option<String>,
    pub wallet_id: Option<Uuid>,
    pub raw_result: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ExecutionQueryParams {
    pub opportunity_id: Option<String>,
    pub wallet_id: Option<Uuid>,
    pub success: Option<bool>,
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================================================
// PERFORMANCE METRICS MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PerformanceMetricRecord {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub processing_time_p50_us: Option<u64>,
    pub processing_time_p95_us: Option<u64>,
    pub processing_time_p99_us: Option<u64>,
    pub opportunities_per_minute: Option<i32>,
    pub execution_success_rate: Option<f64>,
    pub total_profit_captured_bps: Option<i64>,
    pub memory_usage_mb: Option<f64>,
    pub cpu_utilization: Option<f64>,
    pub active_wallets: Option<i32>,
    pub metrics_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct MetricsQueryParams {
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================================================
// TOKEN MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenRecord {
    pub id: u32,
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: i32,
    pub logo_uri: Option<String>,
    pub coingecko_id: Option<String>,
    pub is_verified: bool,
    pub market_cap_usd: Option<u64>,
    pub volume_24h_usd: Option<u64>,
    pub price_usd: Option<f64>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct TokenQueryParams {
    pub symbol: Option<String>,
    pub is_verified: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================================================
// SYSTEM HEALTH MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct SystemHealthRecord {
    pub opportunities_last_hour: u64,
    pub successful_executions_last_hour: u64,
    pub active_wallets: u64,
    pub avg_execution_time_ms: Option<f64>,
    pub success_rate_last_hour: Option<f64>,
    pub total_profit_bps_last_hour: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TradingPerformanceRecord {
    pub trading_date: NaiveDate,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub avg_profit_bps: Option<f64>,
    pub total_profit_bps: Option<i64>,
    pub avg_execution_time_ms: Option<f64>,
    pub active_wallets: u64,
}

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct TradingPerformanceQueryParams {
    pub days: Option<i32>,
}

// ============================================================================
// FLASH LOAN MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlashLoanRecord {
    pub id: u64,
    pub opportunity_id: Option<String>,
    pub provider: String,
    pub amount: u64,
    pub fee_lamports: u64,
    pub success: bool,
    pub repayment_amount: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub transaction_signature: Option<String>,
    pub wallet_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FlashLoanQueryParams {
    pub provider: Option<String>,
    pub success: Option<bool>,
    pub wallet_id: Option<Uuid>,
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================================================
// AUDIT LOG MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogRecord {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub user_id: Option<String>,
    pub wallet_id: Option<Uuid>,
    pub action: String,
    pub resource: Option<String>,
    pub resource_id: Option<String>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AuditLogQueryParams {
    pub event_type: Option<String>,
    pub user_id: Option<String>,
    pub wallet_id: Option<Uuid>,
    pub success: Option<bool>,
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    pub offset: i32,
    pub limit: i32,
    pub total: Option<u64>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }
    
    pub fn error(message: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}

// ============================================================================
// STATISTICS MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VenueStatistics {
    pub venue_name: String,
    pub total_opportunities: u64,
    pub successful_executions: u64,
    pub avg_profit_bps: Option<f64>,
    pub total_profit_bps: Option<i64>,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenPairStatistics {
    pub token_pair: String,
    pub total_opportunities: u64,
    pub avg_profit_bps: Option<f64>,
    pub max_profit_bps: Option<i32>,
    pub avg_volume_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletPerformanceStatistics {
    pub wallet_id: Uuid,
    pub wallet_tier: String,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub total_profit_bps: Option<i64>,
    pub avg_execution_time_ms: Option<f64>,
    pub success_rate: f64,
}

// ============================================================================
// SMART FILTER MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SmartFilterConfig {
    pub min_profit_bps: f64,
    pub max_profit_bps: f64,
    pub min_net_profit_sol: f64,
    pub min_venue_success_rate: f64,
    pub max_gas_budget_pct: f64,
    pub max_concurrent_opportunities: f64,
    pub competition_lookback_ms: f64,
    pub learning_rate: f64,
    pub min_samples_for_learning: f64,
    pub last_updated: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSmartFilterConfigRequest {
    pub parameter: String,
    pub value: f64,
    pub applied_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SmartFilterConfigUpdateResponse {
    pub parameter: String,
    pub old_value: Option<f64>,
    pub new_value: f64,
    pub config_change_id: i32,
    pub timestamp: DateTime<Utc>,
    pub applied_by: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SmartFilterPerformanceSnapshot {
    pub id: i32,
    pub timestamp: DateTime<Utc>,
    pub opportunities_detected: i32,
    pub opportunities_executed: i32,
    pub total_profit_captured_bps: i64,
    pub success_rate: f64,
    pub avg_processing_time_us: i32,
    pub total_opportunities: i32,
    pub passed_profit_filter: i32,
    pub passed_gas_filter: i32,
    pub passed_competition_filter: i32,
    pub rejected_low_profit: i32,
    pub rejected_high_gas: i32,
    pub rejected_suspicious: i32,
    pub rejected_low_success_rate: i32,
    pub rejected_competition: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SmartFilterConfigHistoryRecord {
    pub id: i32,
    pub timestamp: DateTime<Utc>,
    pub parameter_name: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: serde_json::Value,
    pub applied_by: String,
    pub performance_before: Option<serde_json::Value>,
    pub performance_after: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct SmartFilterPerformanceQueryParams {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct SmartFilterConfigHistoryQueryParams {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub parameter_name: Option<String>,
    pub applied_by: Option<String>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
}

// ============================================================================
// ERROR MODELS
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl ErrorResponse {
    pub fn new(code: &str, message: &str) -> Self {
        ErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            timestamp: Utc::now(),
        }
    }
    
    pub fn with_details(code: &str, message: &str, details: serde_json::Value) -> Self {
        ErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            details: Some(details),
            timestamp: Utc::now(),
        }
    }
}