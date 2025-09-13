//! User model and related types

use crate::{auth::UserTier, database::Entity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub tier: UserTier,
    pub is_active: bool,
    pub is_verified: bool,
    pub failed_login_attempts: i32,
    pub locked_until: Option<chrono::DateTime<chrono::Utc>>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Entity for User {
    type Id = Uuid;
    
    fn id(&self) -> &Self::Id {
        &self.id
    }
    
    fn set_id(&mut self, id: Self::Id) {
        self.id = id;
    }
}

impl User {
    /// Check if user account is locked
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            chrono::Utc::now() < locked_until
        } else {
            false
        }
    }
    
    /// Get display name
    pub fn display_name(&self) -> String {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => self.email.clone(),
        }
    }
}

/// User creation request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct UserCreate {
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    
    #[validate(length(max = 100))]
    pub first_name: Option<String>,
    
    #[validate(length(max = 100))]
    pub last_name: Option<String>,
    
    pub tier: Option<UserTier>,
}

/// User update request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct UserUpdate {
    #[validate(email)]
    pub email: Option<String>,
    
    #[validate(length(max = 100))]
    pub first_name: Option<String>,
    
    #[validate(length(max = 100))]
    pub last_name: Option<String>,
    
    pub tier: Option<UserTier>,
    
    pub is_active: Option<bool>,
}

/// User response (excludes sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub tier: UserTier,
    pub is_active: bool,
    pub is_verified: bool,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            tier: user.tier,
            is_active: user.is_active,
            is_verified: user.is_verified,
            last_login_at: user.last_login_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

/// User profile update request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct UserProfileUpdate {
    #[validate(length(max = 100))]
    pub first_name: Option<String>,
    
    #[validate(length(max = 100))]
    pub last_name: Option<String>,
}

/// Password change request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct PasswordChangeRequest {
    pub current_password: String,
    
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}

/// User statistics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserStats {
    pub total_users: i64,
    pub active_users: i64,
    pub verified_users: i64,
    pub users_by_tier: std::collections::HashMap<String, i64>,
    pub new_users_last_30_days: i64,
    pub login_activity_last_24_hours: i64,
}