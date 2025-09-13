//! Arbitrage opportunity models
//!
//! Data models for arbitrage opportunities, execution results, and related entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

// ============================================================================
// CORE ARBITRAGE MODELS
// ============================================================================

/// Arbitrage opportunity record from database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ArbitrageOpportunityRecord {
    pub id: Uuid,
    pub opportunity_id: String,
    pub timestamp: DateTime<Utc>,
    pub slot: Option<i64>,
    pub venue_sell: String,
    pub venue_buy: String,
    pub token_pair: String,
    pub profit_bps: i32,
    pub amount_in: f64,
    pub expected_amount_out: f64,
    pub price_impact_pct: Option<f64>,
    pub detection_time_us: Option<i64>,
    pub strategy_type: String,
    pub status: OpportunityStatus,
    pub executed: bool,
    pub execution_signature: Option<String>,
    pub actual_profit_bps: Option<i32>,
    pub execution_time_us: Option<i64>,
    pub failure_reason: Option<String>,
    pub risk_score: Option<f64>,
    pub confidence_score: Option<f64>,
    pub market_conditions: Option<serde_json::Value>,
    pub raw_opportunity: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Execution result record from database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ExecutionResultRecord {
    pub id: Uuid,
    pub opportunity_id: String,
    pub timestamp: DateTime<Utc>,
    pub executed: bool,
    pub success: Option<bool>,
    pub actual_profit_bps: Option<i32>,
    pub execution_time_us: Option<i64>,
    pub gas_cost_sol: Option<f64>,
    pub slippage_bps: Option<i32>,
    pub failure_reason: Option<String>,
    pub transaction_signature: Option<String>,
    pub bundle_id: Option<String>,
    pub execution_method: Option<String>,
    pub network_fees_sol: Option<f64>,
    pub flash_loan_fees_sol: Option<f64>,
    pub total_fees_sol: Option<f64>,
    pub net_profit_sol: Option<f64>,
    pub raw_result: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Advanced performance metrics for arbitrage strategies
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ArbitragePerformanceRecord {
    pub id: Uuid,
    pub strategy_type: String,
    pub venue_pair: String,
    pub token_pair: String,
    pub time_period: String, // hourly, daily, weekly
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub opportunities_detected: i32,
    pub opportunities_executed: i32,
    pub success_rate: f64,
    pub avg_profit_bps: f64,
    pub total_profit_sol: f64,
    pub total_volume_sol: f64,
    pub avg_execution_time_us: f64,
    pub avg_slippage_bps: f64,
    pub max_drawdown_bps: Option<f64>,
    pub sharpe_ratio: Option<f64>,
    pub win_rate: f64,
    pub avg_win_bps: f64,
    pub avg_loss_bps: f64,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// ENUMS AND STATUS TYPES
// ============================================================================

/// Status of an arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "opportunity_status", rename_all = "lowercase")]
pub enum OpportunityStatus {
    Detected,
    Analyzing,
    Approved,
    Executing,
    Executed,
    Failed,
    Rejected,
    Expired,
}

/// Execution method used for the arbitrage
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ExecutionMethod {
    Direct,
    FlashLoan,
    JitoBundle,
    CrossDex,
    Custom(String),
}

/// Risk classification for opportunities
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

/// Request to create a new arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateArbitrageOpportunityRequest {
    #[validate(length(min = 1, max = 255))]
    pub opportunity_id: String,
    
    pub slot: Option<i64>,
    
    #[validate(length(min = 1, max = 100))]
    pub venue_sell: String,
    
    #[validate(length(min = 1, max = 100))]
    pub venue_buy: String,
    
    #[validate(length(min = 1, max = 100))]
    pub token_pair: String,
    
    #[validate(range(min = 1))]
    pub profit_bps: i32,
    
    #[validate(range(min = 0.0))]
    pub amount_in: f64,
    
    #[validate(range(min = 0.0))]
    pub expected_amount_out: f64,
    
    pub price_impact_pct: Option<f64>,
    pub detection_time_us: Option<i64>,
    
    #[validate(length(min = 1, max = 100))]
    pub strategy_type: String,
    
    pub risk_score: Option<f64>,
    pub confidence_score: Option<f64>,
    pub market_conditions: Option<serde_json::Value>,
    pub raw_opportunity: serde_json::Value,
}

/// Response for arbitrage opportunity operations
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArbitrageOpportunityResponse {
    pub opportunity: ArbitrageOpportunityRecord,
    pub metadata: OpportunityMetadata,
}

/// Additional metadata for arbitrage opportunities
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpportunityMetadata {
    pub estimated_gas_cost: Option<f64>,
    pub estimated_execution_time_ms: Option<u64>,
    pub recommended_action: String,
    pub risk_factors: Vec<String>,
    pub similar_opportunities_count: u32,
    pub market_volatility: Option<f64>,
}

/// Request to update opportunity status
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateOpportunityStatusRequest {
    pub status: OpportunityStatus,
    pub reason: Option<String>,
    pub execution_signature: Option<String>,
    pub actual_profit_bps: Option<i32>,
    pub execution_time_us: Option<i64>,
    pub failure_reason: Option<String>,
}

// ============================================================================
// QUERY PARAMETERS
// ============================================================================

/// Query parameters for arbitrage opportunities
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct ArbitrageQueryParams {
    /// Number of results to return (default: 50, max: 1000)
    #[serde(default = "default_limit")]
    pub limit: i32,
    
    /// Number of results to skip (default: 0)
    #[serde(default)]
    pub offset: i32,
    
    /// Filter by venue selling the asset
    pub venue_sell: Option<String>,
    
    /// Filter by venue buying the asset  
    pub venue_buy: Option<String>,
    
    /// Filter by token pair (e.g., "SOL-USDC")
    pub token_pair: Option<String>,
    
    /// Minimum profit in basis points
    pub min_profit_bps: Option<i32>,
    
    /// Maximum profit in basis points
    pub max_profit_bps: Option<i32>,
    
    /// Filter by opportunity status
    pub status: Option<OpportunityStatus>,
    
    /// Filter by strategy type
    pub strategy_type: Option<String>,
    
    /// Only show executed opportunities
    pub executed_only: Option<bool>,
    
    /// Only show successful executions
    pub success_only: Option<bool>,
    
    /// Start date filter (ISO 8601)
    pub from_date: Option<DateTime<Utc>>,
    
    /// End date filter (ISO 8601)
    pub to_date: Option<DateTime<Utc>>,
    
    /// Sort field (timestamp, profit_bps, execution_time)
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    
    /// Sort order (asc, desc)
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
    
    /// Include raw opportunity data
    pub include_raw: Option<bool>,
}

/// Query parameters for execution results
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct ExecutionQueryParams {
    #[serde(default = "default_limit")]
    pub limit: i32,
    
    #[serde(default)]
    pub offset: i32,
    
    pub opportunity_id: Option<String>,
    pub success_only: Option<bool>,
    pub execution_method: Option<String>,
    pub min_profit_bps: Option<i32>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
}

/// Query parameters for performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct PerformanceQueryParams {
    /// Time period aggregation (hourly, daily, weekly, monthly)
    #[serde(default = "default_period")]
    pub period: String,
    
    /// Strategy type filter
    pub strategy_type: Option<String>,
    
    /// Venue pair filter
    pub venue_pair: Option<String>,
    
    /// Token pair filter
    pub token_pair: Option<String>,
    
    /// Start date for metrics
    pub from_date: Option<DateTime<Utc>>,
    
    /// End date for metrics
    pub to_date: Option<DateTime<Utc>>,
    
    /// Minimum number of opportunities to include
    pub min_opportunities: Option<i32>,
}

// ============================================================================
// ADVANCED ANALYTICS MODELS
// ============================================================================

/// Real-time arbitrage analytics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArbitrageAnalytics {
    pub current_opportunities: u32,
    pub opportunities_last_hour: u32,
    pub success_rate_24h: f64,
    pub avg_profit_24h: f64,
    pub total_volume_24h: f64,
    pub top_venues: Vec<VenuePerformance>,
    pub top_strategies: Vec<StrategyPerformance>,
    pub market_conditions: MarketConditions,
    pub risk_metrics: RiskMetrics,
}

/// Performance metrics by venue
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VenuePerformance {
    pub venue: String,
    pub opportunities_count: u32,
    pub success_rate: f64,
    pub avg_profit_bps: f64,
    pub total_volume_sol: f64,
    pub avg_execution_time_ms: f64,
}

/// Performance metrics by strategy
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StrategyPerformance {
    pub strategy_type: String,
    pub opportunities_count: u32,
    pub success_rate: f64,
    pub avg_profit_bps: f64,
    pub total_volume_sol: f64,
    pub sharpe_ratio: Option<f64>,
}

/// Current market conditions affecting arbitrage
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MarketConditions {
    pub volatility_index: f64,
    pub liquidity_index: f64,
    pub gas_price_gwei: f64,
    pub network_congestion: f64,
    pub active_bots_estimate: u32,
    pub competition_level: String,
}

/// Risk metrics for the arbitrage system
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RiskMetrics {
    pub current_exposure_sol: f64,
    pub var_95_24h: f64, // Value at Risk
    pub max_drawdown_7d: f64,
    pub concentration_risk: f64,
    pub liquidity_risk: f64,
    pub counterparty_risk: HashMap<String, f64>,
}

// ============================================================================
// PAGINATION AND RESPONSES
// ============================================================================

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
    pub metadata: Option<serde_json::Value>,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginationInfo {
    pub offset: i32,
    pub limit: i32,
    pub total: Option<i64>,
    pub has_more: bool,
    pub next_offset: Option<i32>,
}

// ============================================================================
// DEFAULT VALUES
// ============================================================================

fn default_limit() -> i32 { 50 }
fn default_sort_by() -> String { "timestamp".to_string() }
fn default_sort_order() -> String { "desc".to_string() }
fn default_period() -> String { "daily".to_string() }