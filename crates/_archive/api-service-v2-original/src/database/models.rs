//! Database models and entities
//!
//! All database models implement the Entity trait and include:
//! - Audit trail information
//! - Type-safe field definitions
//! - Serialization/deserialization
//! - Validation rules

use super::{AuditInfo, Entity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// User account entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct User {
    pub id: Option<Uuid>,
    
    #[validate(email, message = "Invalid email format")]
    pub email: String,
    
    #[validate(length(min = 2, max = 100, message = "Username must be between 2 and 100 characters"))]
    pub username: String,
    
    /// Hashed password (never serialized in responses)
    #[serde(skip_serializing)]
    pub password_hash: String,
    
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: bool,
    pub is_verified: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    
    /// User roles for authorization
    pub roles: Vec<String>,
    
    /// User tier determines access levels
    pub tier: UserTier,
    
    #[serde(flatten)]
    pub audit_info: AuditInfo,
}

impl Entity for User {
    type Id = Uuid;
    
    fn id(&self) -> Option<Self::Id> {
        self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
    
    fn table_name() -> &'static str {
        "users"
    }
}

/// User tier determines access levels and rate limits
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum UserTier {
    Free,
    Premium,
    Professional,
    Enterprise,
}

impl Default for UserTier {
    fn default() -> Self {
        UserTier::Free
    }
}

/// API key entity for programmatic access
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKey {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    
    #[validate(length(min = 1, max = 100, message = "Key name is required"))]
    pub name: String,
    
    /// Hashed API key (never serialized)
    #[serde(skip_serializing)]
    pub key_hash: String,
    
    /// Prefix shown to user (first 8 chars)
    pub key_prefix: String,
    
    pub permissions: Vec<String>,
    pub rate_limit_per_hour: Option<u32>,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    
    #[serde(flatten)]
    pub audit_info: AuditInfo,
}

impl Entity for ApiKey {
    type Id = Uuid;
    
    fn id(&self) -> Option<Self::Id> {
        self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
    
    fn table_name() -> &'static str {
        "api_keys"
    }
}

/// Trading session entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TradingSession {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    
    pub session_token: String,
    pub wallet_id: Option<Uuid>,
    pub strategy: TradingStrategy,
    pub status: SessionStatus,
    
    /// Session configuration
    pub max_position_size: Option<f64>,
    pub stop_loss_percent: Option<f32>,
    pub take_profit_percent: Option<f32>,
    
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    
    /// Session metrics
    pub trades_executed: u32,
    pub total_volume: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    
    #[serde(flatten)]
    pub audit_info: AuditInfo,
}

impl Entity for TradingSession {
    type Id = Uuid;
    
    fn id(&self) -> Option<Self::Id> {
        self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
    
    fn table_name() -> &'static str {
        "trading_sessions"
    }
}

/// Trading strategy types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub enum TradingStrategy {
    Arbitrage,
    MarketMaking,
    TrendFollowing,
    MeanReversion,
    Custom(String),
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub enum SessionStatus {
    Active,
    Paused,
    Stopped,
    Error,
}

/// Audit log entry for all system operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLog {
    pub id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<Uuid>,
    
    pub success: bool,
    pub error_message: Option<String>,
    
    pub timestamp: DateTime<Utc>,
}

impl Entity for AuditLog {
    type Id = Uuid;
    
    fn id(&self) -> Option<Self::Id> {
        self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
    
    fn table_name() -> &'static str {
        "audit_logs"
    }
}

/// Rate limiting entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEntry {
    pub key: String,
    pub count: u32,
    pub window_start: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// System configuration entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemConfig {
    pub id: Option<Uuid>,
    pub key: String,
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub is_sensitive: bool,
    
    #[serde(flatten)]
    pub audit_info: AuditInfo,
}

impl Entity for SystemConfig {
    type Id = Uuid;
    
    fn id(&self) -> Option<Self::Id> {
        self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = Some(id);
    }
    
    fn table_name() -> &'static str {
        "system_config"
    }
}