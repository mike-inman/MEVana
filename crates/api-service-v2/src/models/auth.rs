//! Authentication-related models

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Login request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 1))]
    pub password: String,
    
    pub remember_me: Option<bool>,
}

/// Login response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub user: super::UserResponse,
}

/// User registration request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    
    #[validate(length(max = 100))]
    pub first_name: Option<String>,
    
    #[validate(length(max = 100))]
    pub last_name: Option<String>,
    
    pub accept_terms: bool,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct TokenRefreshRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

/// Token refresh response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TokenRefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// Password reset request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct PasswordResetRequest {
    #[validate(email)]
    pub email: String,
}

/// Password reset confirmation
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct PasswordResetConfirmation {
    pub token: String,
    
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Active sessions response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActiveSessionsResponse {
    pub sessions: Vec<SessionInfo>,
    pub current_session_id: Uuid,
}

/// Logout request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LogoutRequest {
    pub all_sessions: Option<bool>,
}

/// Email verification request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct EmailVerificationRequest {
    pub token: String,
}

/// Two-factor authentication setup request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TwoFactorSetupRequest {
    pub enable: bool,
}

/// Two-factor authentication setup response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TwoFactorSetupResponse {
    pub qr_code: Option<String>,
    pub backup_codes: Option<Vec<String>>,
}

/// Two-factor authentication verification
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct TwoFactorVerification {
    #[validate(length(min = 6, max = 6))]
    pub code: String,
}