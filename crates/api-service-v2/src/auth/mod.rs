//! Authentication and authorization module
//!
//! Provides enterprise-grade security with:
//! - JWT-based authentication
//! - Role-based access control (RBAC)
//! - Permission-based authorization
//! - Token rotation and blacklisting
//! - Rate limiting per user/endpoint

pub mod jwt;
pub mod permissions;
pub mod context;
pub mod password;
pub mod session;

pub use jwt::{JwtAuthService, JwtClaims, TokenPair, TokenType};
pub use permissions::{Permission, PermissionSet, Role, RoleManager};
pub use context::{ApiContext, AuthenticationInfo, RequestContext};
pub use password::{PasswordService, PasswordStrength};
pub use session::{SessionManager, SessionInfo, SessionStatus};

use crate::error::{ApiError, ApiResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT signing secret
    pub jwt_secret: String,
    
    /// Access token lifetime in seconds
    pub access_token_lifetime_seconds: u64,
    
    /// Refresh token lifetime in seconds  
    pub refresh_token_lifetime_seconds: u64,
    
    /// Maximum concurrent sessions per user
    pub max_sessions_per_user: u32,
    
    /// Enable token rotation on refresh
    pub enable_token_rotation: bool,
    
    /// Password requirements
    pub password_min_length: usize,
    pub password_require_uppercase: bool,
    pub password_require_lowercase: bool,
    pub password_require_numbers: bool,
    pub password_require_special_chars: bool,
    
    /// Rate limiting
    pub rate_limit_requests_per_minute: u32,
    pub rate_limit_window_minutes: u32,
    
    /// Session security
    pub require_secure_cookies: bool,
    pub session_timeout_minutes: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "your-256-bit-secret".to_string(), // Should be from environment
            access_token_lifetime_seconds: 15 * 60,        // 15 minutes
            refresh_token_lifetime_seconds: 7 * 24 * 3600, // 7 days
            max_sessions_per_user: 5,
            enable_token_rotation: true,
            password_min_length: 12,
            password_require_uppercase: true,
            password_require_lowercase: true,
            password_require_numbers: true,
            password_require_special_chars: true,
            rate_limit_requests_per_minute: 100,
            rate_limit_window_minutes: 1,
            require_secure_cookies: true,
            session_timeout_minutes: 30,
        }
    }
}

/// Authentication request
#[derive(Debug, Deserialize, utoipa::ToSchema, validator::Validate)]
pub struct LoginRequest {
    #[validate(email, message = "Invalid email format")]
    pub email: String,
    
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
    
    /// Optional 2FA code
    pub totp_code: Option<String>,
    
    /// Remember this session for longer
    pub remember_me: Option<bool>,
}

/// Authentication response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfo,
    pub permissions: Vec<String>,
}

/// User information returned in auth responses
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UserInfo {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub tier: String,
    pub roles: Vec<String>,
    pub is_verified: bool,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// Token refresh request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Password change request
#[derive(Debug, Deserialize, utoipa::ToSchema, validator::Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, message = "Current password is required"))]
    pub current_password: String,
    
    #[validate(length(min = 12, message = "New password must be at least 12 characters"))]
    pub new_password: String,
}

/// User registration request
#[derive(Debug, Deserialize, utoipa::ToSchema, validator::Validate)]
pub struct RegisterRequest {
    #[validate(email, message = "Invalid email format")]
    pub email: String,
    
    #[validate(length(min = 3, max = 50, message = "Username must be between 3 and 50 characters"))]
    #[validate(regex = "^[a-zA-Z0-9_-]+$", message = "Username can only contain letters, numbers, underscores and hyphens")]
    pub username: String,
    
    #[validate(length(min = 12, message = "Password must be at least 12 characters"))]
    pub password: String,
    
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Authentication error types
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub enum AuthErrorType {
    InvalidCredentials,
    TokenExpired,
    TokenInvalid,
    InsufficientPermissions,
    AccountLocked,
    AccountNotVerified,
    TwoFactorRequired,
    RateLimitExceeded,
    SessionExpired,
}

/// Authentication result type
pub type AuthResult<T> = Result<T, ApiError>;

/// Extract bearer token from Authorization header
pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    auth_header
        .strip_prefix("Bearer ")
        .map(|token| token.trim())
        .filter(|token| !token.is_empty())
}

/// Validate email format
pub fn is_valid_email(email: &str) -> bool {
    use regex::Regex;
    
    let email_regex = Regex::new(
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    ).unwrap();
    
    email_regex.is_match(email)
}

/// Generate secure random string
pub fn generate_secure_random(length: usize) -> String {
    use rand::{distributions::Alphanumeric, Rng};
    
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Calculate password strength score
pub fn calculate_password_strength(password: &str) -> u8 {
    let mut score = 0u8;
    
    // Length bonus
    match password.len() {
        0..=7 => score += 0,
        8..=11 => score += 10,
        12..=15 => score += 20,
        16..=19 => score += 25,
        _ => score += 30,
    }
    
    // Character diversity
    if password.chars().any(|c| c.is_lowercase()) { score += 10; }
    if password.chars().any(|c| c.is_uppercase()) { score += 10; }
    if password.chars().any(|c| c.is_numeric()) { score += 10; }
    if password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)) { score += 15; }
    
    // Complexity patterns
    if password.len() >= 12 && password.chars().collect::<HashSet<_>>().len() as f32 / password.len() as f32 > 0.7 {
        score += 15;
    }
    
    // Cap at 100
    std::cmp::min(score, 100)
}