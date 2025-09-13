//! Enterprise wallet management models
//!
//! Advanced wallet models with multi-tier security, compliance, and analytics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use super::{AuditInfo, PaginatedResponse};

// ============================================================================
// CORE WALLET MODELS
// ============================================================================

/// Enterprise wallet record with advanced security features
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct WalletRecord {
    pub id: Uuid,
    pub public_key: String,
    pub name: String,
    pub wallet_type: WalletType,
    pub security_tier: SecurityTier,
    pub status: WalletStatus,
    pub balance_sol: f64,
    pub balance_usd: Option<f64>,
    pub daily_limit_sol: Option<f64>,
    pub monthly_limit_sol: Option<f64>,
    pub total_traded_sol: f64,
    pub total_profit_sol: f64,
    pub successful_trades: i32,
    pub failed_trades: i32,
    pub last_activity: Option<DateTime<Utc>>,
    pub risk_score: f64,
    pub compliance_status: ComplianceStatus,
    pub multi_sig_threshold: Option<i32>,
    pub approved_counterparties: Vec<String>,
    pub metadata: serde_json::Value,
    pub audit_info: AuditInfo,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Wallet transaction record with comprehensive tracking
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct TransactionRecord {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub transaction_hash: String,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount_sol: f64,
    pub fee_sol: f64,
    pub net_amount_sol: f64,
    pub counterparty: Option<String>,
    pub purpose: Option<String>,
    pub risk_assessment: RiskLevel,
    pub compliance_check: bool,
    pub block_height: Option<i64>,
    pub confirmation_count: Option<i32>,
    pub metadata: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Wallet performance analytics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletAnalytics {
    pub wallet_id: Uuid,
    pub time_period: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    
    // Trading Performance
    pub total_trades: i32,
    pub successful_trades: i32,
    pub success_rate: f64,
    pub total_volume_sol: f64,
    pub total_profit_sol: f64,
    pub avg_profit_per_trade: f64,
    pub max_profit_sol: f64,
    pub max_loss_sol: f64,
    
    // Risk Metrics
    pub sharpe_ratio: Option<f64>,
    pub max_drawdown_pct: f64,
    pub var_95_sol: f64, // Value at Risk 95%
    pub volatility: f64,
    pub beta: Option<f64>,
    
    // Operational Metrics
    pub avg_execution_time_ms: f64,
    pub uptime_percentage: f64,
    pub gas_efficiency_score: f64,
    pub slippage_tolerance_breaches: i32,
    
    // Compliance & Security
    pub compliance_score: f64,
    pub security_incidents: i32,
    pub suspicious_activity_alerts: i32,
    pub kyc_status: bool,
    pub aml_status: bool,
}

/// Comprehensive wallet portfolio information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletPortfolio {
    pub wallet_id: Uuid,
    pub total_value_sol: f64,
    pub total_value_usd: f64,
    pub sol_balance: f64,
    pub token_holdings: Vec<TokenHolding>,
    pub positions: Vec<PositionInfo>,
    pub asset_allocation: HashMap<String, f64>,
    pub diversification_score: f64,
    pub concentration_risk: f64,
    pub liquidity_score: f64,
    pub last_updated: DateTime<Utc>,
}

/// Individual token holding
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TokenHolding {
    pub mint_address: String,
    pub symbol: String,
    pub name: String,
    pub balance: f64,
    pub value_sol: f64,
    pub value_usd: Option<f64>,
    pub percentage_of_portfolio: f64,
    pub price_change_24h: Option<f64>,
    pub avg_cost_basis: Option<f64>,
    pub unrealized_pnl: Option<f64>,
}

/// Trading position information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PositionInfo {
    pub position_id: String,
    pub strategy_type: String,
    pub asset_pair: String,
    pub position_size_sol: f64,
    pub entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl_sol: f64,
    pub unrealized_pnl_pct: f64,
    pub duration_hours: f64,
    pub status: PositionStatus,
}

// ============================================================================
// ENUMS AND STATUS TYPES
// ============================================================================

/// Type of wallet
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "wallet_type", rename_all = "lowercase")]
pub enum WalletType {
    Hot,      // Active trading wallet
    Warm,     // Reserve wallet
    Cold,     // Long-term storage
    MultiSig, // Multi-signature wallet
    Escrow,   // Escrow/custody wallet
    Treasury, // Treasury management
}

/// Security tier classification
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "security_tier", rename_all = "lowercase")]
pub enum SecurityTier {
    Basic,      // Standard security
    Enhanced,   // Additional security measures
    Enterprise, // Full enterprise security
    Military,   // Military-grade security
}

/// Current wallet status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "wallet_status", rename_all = "lowercase")]
pub enum WalletStatus {
    Active,
    Inactive,
    Suspended,
    Locked,
    EmergencyLock,
    UnderReview,
    Compromised,
}

/// Compliance status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "compliance_status", rename_all = "lowercase")]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    UnderReview,
    RequiresAction,
    Suspended,
}

/// Transaction type classification
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "transaction_type", rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Trade,
    Transfer,
    Swap,
    Arbitrage,
    FlashLoan,
    Staking,
    Governance,
    Fee,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "transaction_status", rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
    Reverted,
}

/// Risk level classification
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "risk_level", rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Position status
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum PositionStatus {
    Open,
    Closed,
    Partial,
    Liquidated,
}

// ============================================================================
// REQUEST/RESPONSE MODELS
// ============================================================================

/// Request to create a new wallet
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateWalletRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    
    pub wallet_type: WalletType,
    pub security_tier: SecurityTier,
    
    #[validate(range(min = 0.0))]
    pub daily_limit_sol: Option<f64>,
    
    #[validate(range(min = 0.0))]
    pub monthly_limit_sol: Option<f64>,
    
    #[validate(range(min = 1, max = 10))]
    pub multi_sig_threshold: Option<i32>,
    
    pub approved_counterparties: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    
    /// Generate new keypair or import existing
    pub generate_new: bool,
    
    /// Private key for import (if generate_new = false)
    pub private_key: Option<String>,
    
    /// Additional security options
    pub enable_2fa: Option<bool>,
    pub enable_biometric: Option<bool>,
    pub enable_hardware_security: Option<bool>,
}

/// Request to update wallet settings
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateWalletRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    
    pub status: Option<WalletStatus>,
    
    #[validate(range(min = 0.0))]
    pub daily_limit_sol: Option<f64>,
    
    #[validate(range(min = 0.0))]
    pub monthly_limit_sol: Option<f64>,
    
    pub approved_counterparties: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    
    /// Security settings updates
    pub enable_2fa: Option<bool>,
    pub enable_biometric: Option<bool>,
    pub notification_preferences: Option<serde_json::Value>,
}

/// Emergency wallet lock request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct EmergencyLockRequest {
    #[validate(length(min = 1, max = 500))]
    pub reason: String,
    
    pub lock_duration_hours: Option<i32>,
    pub notify_admin: bool,
    pub create_incident: bool,
    
    /// Additional context
    pub threat_level: RiskLevel,
    pub evidence: Option<serde_json::Value>,
}

/// Comprehensive wallet response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletResponse {
    pub wallet: WalletRecord,
    pub current_balance: WalletBalance,
    pub security_status: SecurityStatus,
    pub compliance_info: ComplianceInfo,
    pub recent_activity: Vec<ActivitySummary>,
}

/// Detailed wallet information response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletDetailResponse {
    pub wallet: WalletRecord,
    pub analytics: WalletAnalytics,
    pub portfolio: WalletPortfolio,
    pub risk_assessment: WalletRiskAssessment,
    pub security_status: SecurityStatus,
    pub compliance_info: ComplianceInfo,
    pub transaction_summary: TransactionSummary,
}

/// Current wallet balance information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletBalance {
    pub sol_balance: f64,
    pub usd_value: Option<f64>,
    pub available_balance: f64,
    pub reserved_balance: f64,
    pub pending_incoming: f64,
    pub pending_outgoing: f64,
    pub last_updated: DateTime<Utc>,
}

/// Wallet security status
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SecurityStatus {
    pub overall_score: f64,
    pub two_factor_enabled: bool,
    pub biometric_enabled: bool,
    pub hardware_security_enabled: bool,
    pub last_security_audit: Option<DateTime<Utc>>,
    pub active_threats: u32,
    pub security_incidents_30d: u32,
    pub recommended_actions: Vec<String>,
}

/// Compliance information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ComplianceInfo {
    pub kyc_status: bool,
    pub aml_status: bool,
    pub sanctions_check: bool,
    pub tax_reporting_enabled: bool,
    pub jurisdiction: Option<String>,
    pub compliance_score: f64,
    pub last_compliance_check: Option<DateTime<Utc>>,
    pub pending_requirements: Vec<String>,
}

/// Recent activity summary
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActivitySummary {
    pub activity_type: String,
    pub amount_sol: Option<f64>,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub status: String,
    pub risk_level: RiskLevel,
}

/// Transaction summary statistics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TransactionSummary {
    pub total_transactions: i32,
    pub transactions_30d: i32,
    pub total_volume_sol: f64,
    pub volume_30d_sol: f64,
    pub avg_transaction_size: f64,
    pub largest_transaction_sol: f64,
    pub most_frequent_counterparty: Option<String>,
    pub transaction_types: HashMap<String, i32>,
}

// ============================================================================
// QUERY PARAMETERS
// ============================================================================

/// Query parameters for wallet listing
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct WalletQueryParams {
    #[serde(default = "default_limit")]
    pub limit: i32,
    
    #[serde(default)]
    pub offset: i32,
    
    pub wallet_type: Option<WalletType>,
    pub status: Option<WalletStatus>,
    pub security_tier: Option<SecurityTier>,
    pub compliance_status: Option<ComplianceStatus>,
    
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
    
    pub include_analytics: Option<bool>,
    pub include_portfolio: Option<bool>,
}

/// Query parameters for wallet analytics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct WalletAnalyticsQuery {
    #[serde(default = "default_analytics_period")]
    pub period: String, // hourly, daily, weekly, monthly
    
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub include_risk_metrics: Option<bool>,
    pub include_benchmarks: Option<bool>,
}

/// Query parameters for transaction history
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct TransactionQueryParams {
    #[serde(default = "default_limit")]
    pub limit: i32,
    
    #[serde(default)]
    pub offset: i32,
    
    pub transaction_type: Option<TransactionType>,
    pub status: Option<TransactionStatus>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub counterparty: Option<String>,
    
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
}

// ============================================================================
// ADVANCED FEATURES
// ============================================================================

/// Real-time wallet risk assessment
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalletRiskAssessment {
    pub overall_risk_score: f64,
    pub market_risk: f64,
    pub liquidity_risk: f64,
    pub counterparty_risk: f64,
    pub operational_risk: f64,
    pub compliance_risk: f64,
    pub concentration_risk: f64,
    pub risk_factors: Vec<RiskFactor>,
    pub recommendations: Vec<String>,
    pub last_assessment: DateTime<Utc>,
}

/// Individual risk factor
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RiskFactor {
    pub factor_type: String,
    pub description: String,
    pub severity: RiskLevel,
    pub impact_score: f64,
    pub mitigation_suggestions: Vec<String>,
}

/// Security audit report
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SecurityAuditReport {
    pub audit_id: Uuid,
    pub wallet_id: Uuid,
    pub audit_timestamp: DateTime<Utc>,
    pub overall_score: f64,
    pub security_checks: Vec<SecurityCheck>,
    pub vulnerabilities: Vec<SecurityVulnerability>,
    pub recommendations: Vec<SecurityRecommendation>,
    pub compliance_status: ComplianceStatus,
    pub next_audit_due: DateTime<Utc>,
}

/// Individual security check
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SecurityCheck {
    pub check_name: String,
    pub status: String,
    pub score: f64,
    pub details: String,
    pub last_checked: DateTime<Utc>,
}

/// Security vulnerability
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SecurityVulnerability {
    pub vulnerability_id: String,
    pub severity: RiskLevel,
    pub description: String,
    pub impact: String,
    pub remediation: String,
    pub discovered_at: DateTime<Utc>,
}

/// Security recommendation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SecurityRecommendation {
    pub recommendation_id: String,
    pub priority: String,
    pub category: String,
    pub description: String,
    pub implementation_effort: String,
    pub expected_impact: String,
}

/// Bulk wallet operation request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct BulkWalletOperationRequest {
    pub operation_type: BulkOperationType,
    pub wallet_ids: Vec<Uuid>,
    pub parameters: serde_json::Value,
    pub dry_run: bool,
}

/// Types of bulk operations
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum BulkOperationType {
    UpdateLimits,
    ChangeStatus,
    SecurityAudit,
    ComplianceCheck,
    RiskAssessment,
    BackupKeys,
}

/// Bulk operation response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkOperationResponse {
    pub operation_id: Uuid,
    pub total_wallets: u32,
    pub successful_operations: u32,
    pub failed_operations: u32,
    pub results: Vec<BulkOperationResult>,
    pub summary: String,
}

/// Individual bulk operation result
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkOperationResult {
    pub wallet_id: Uuid,
    pub success: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

// ============================================================================
// DEFAULT VALUES
// ============================================================================

fn default_limit() -> i32 { 50 }
fn default_sort_by() -> String { "created_at".to_string() }
fn default_sort_order() -> String { "desc".to_string() }
fn default_analytics_period() -> String { "daily".to_string() }