//! Data models for the API
//!
//! Provides strongly-typed models with:
//! - Comprehensive validation
//! - Serialization/deserialization
//! - Database entity traits
//! - API request/response types

pub mod user;
pub mod auth;
pub mod wallet;
pub mod trading;
pub mod analytics;
pub mod health;

// Re-export commonly used types
pub use user::{User, UserCreate, UserUpdate, UserResponse};
pub use auth::{LoginRequest, LoginResponse, RegisterRequest, TokenRefreshRequest};
pub use wallet::{Wallet, WalletCreate, WalletUpdate, WalletResponse};
pub use trading::{Trade, TradeRequest, TradeResponse, TradeHistory};
pub use analytics::{AnalyticsQuery, AnalyticsResponse};
pub use health::{HealthResponse, HealthStatus};

use crate::database::Entity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Common pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    
    #[serde(default)]
    pub offset: i64,
    
    #[serde(default)]
    pub sort_by: Option<String>,
    
    #[serde(default)]
    pub sort_direction: Option<SortDirection>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
            sort_by: None,
            sort_direction: None,
        }
    }
}

fn default_limit() -> i64 {
    20
}

/// Sort direction enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

/// Generic paginated response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginationInfo {
    pub total_count: i64,
    pub page_count: i64,
    pub current_page: i64,
    pub per_page: i64,
    pub has_next: bool,
    pub has_previous: bool,
}

impl PaginationInfo {
    pub fn new(total_count: i64, limit: i64, offset: i64) -> Self {
        let per_page = limit;
        let current_page = (offset / limit) + 1;
        let page_count = (total_count + limit - 1) / limit; // Ceiling division
        let has_next = current_page < page_count;
        let has_previous = current_page > 1;
        
        Self {
            total_count,
            page_count,
            current_page,
            per_page,
            has_next,
            has_previous,
        }
    }
}

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub request_id: Option<Uuid>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    /// Create successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            request_id: None,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Create successful response with message
    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
            request_id: None,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Create error response
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
            request_id: None,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Set request ID
    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

/// Audit log entry for tracking changes
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLog {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: String,
    pub action: AuditAction,
    pub changes: Option<serde_json::Value>,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Audit action enumeration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum AuditAction {
    Create,
    Read,
    Update,
    Delete,
    Login,
    Logout,
    PasswordChange,
    PermissionChange,
}

impl Entity for AuditLog {
    type Id = Uuid;
    
    fn id(&self) -> &Self::Id {
        &self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = id;
    }
}