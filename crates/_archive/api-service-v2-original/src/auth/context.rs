//! Authentication context and request handling
//!
//! Provides request context with authenticated user information
//! and security metadata for all API operations.

use super::{JwtClaims, Permission, PermissionSet};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// API context containing authenticated user information
#[derive(Debug, Clone, Serialize)]
pub struct ApiContext {
    /// Authenticated user ID
    pub user_id: Uuid,
    
    /// User email
    pub email: String,
    
    /// User permissions
    pub permissions: PermissionSet,
    
    /// User roles
    pub roles: HashSet<String>,
    
    /// User tier
    pub tier: String,
    
    /// Session ID
    pub session_id: Uuid,
    
    /// Token information
    pub token_info: TokenInfo,
    
    /// Request metadata
    pub request_info: RequestInfo,
    
    /// Authentication timestamp
    pub authenticated_at: DateTime<Utc>,
}

impl ApiContext {
    /// Create new API context from JWT claims
    pub fn from_jwt_claims(claims: JwtClaims, request_info: RequestInfo) -> Self {
        let permissions = PermissionSet::from_strings(&claims.permissions);
        let roles = claims.roles.into_iter().collect();
        
        Self {
            user_id: claims.sub,
            email: claims.email,
            permissions,
            roles,
            tier: claims.tier,
            session_id: claims.session_id,
            token_info: TokenInfo {
                jti: claims.jti,
                issued_at: DateTime::from_timestamp(claims.iat, 0).unwrap_or_default(),
                expires_at: DateTime::from_timestamp(claims.exp, 0).unwrap_or_default(),
            },
            request_info,
            authenticated_at: Utc::now(),
        }
    }
    
    /// Check if user has specific permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.permissions.has_permission(permission)
    }
    
    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        permissions.iter().any(|p| self.has_permission(*p))
    }
    
    /// Check if user has all specified permissions
    pub fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        permissions.iter().all(|p| self.has_permission(*p))
    }
    
    /// Check if user has specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }
    
    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }
    
    /// Get user tier as enum
    pub fn get_user_tier(&self) -> UserTier {
        match self.tier.as_str() {
            "Free" => UserTier::Free,
            "Premium" => UserTier::Premium,
            "Professional" => UserTier::Professional,
            "Enterprise" => UserTier::Enterprise,
            _ => UserTier::Free,
        }
    }
    
    /// Check if token is expired
    pub fn is_token_expired(&self) -> bool {
        Utc::now() > self.token_info.expires_at
    }
    
    /// Get time until token expires
    pub fn time_until_expiry(&self) -> chrono::Duration {
        self.token_info.expires_at - Utc::now()
    }
}

/// Token information embedded in context
#[derive(Debug, Clone, Serialize)]
pub struct TokenInfo {
    pub jti: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Request information for audit and security
#[derive(Debug, Clone, Serialize)]
pub struct RequestInfo {
    /// Unique request ID for tracing
    pub request_id: Uuid,
    
    /// Client IP address
    pub ip_address: Option<std::net::IpAddr>,
    
    /// User agent string
    pub user_agent: Option<String>,
    
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
    
    /// HTTP method
    pub method: String,
    
    /// Request path
    pub path: String,
    
    /// Request origin
    pub origin: Option<String>,
    
    /// Forwarded headers (for proxy/load balancer)
    pub forwarded_for: Option<String>,
}

impl RequestInfo {
    /// Create new request info
    pub fn new(
        request_id: Uuid,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<String>,
        method: String,
        path: String,
        origin: Option<String>,
        forwarded_for: Option<String>,
    ) -> Self {
        Self {
            request_id,
            ip_address,
            user_agent,
            timestamp: Utc::now(),
            method,
            path,
            origin,
            forwarded_for,
        }
    }
}

/// User tier enumeration with access level definitions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserTier {
    Free,
    Premium,
    Professional,
    Enterprise,
}

impl UserTier {
    /// Get rate limit for this tier (requests per minute)
    pub fn rate_limit(&self) -> u32 {
        match self {
            UserTier::Free => 60,
            UserTier::Premium => 300,
            UserTier::Professional => 1000,
            UserTier::Enterprise => 5000,
        }
    }
    
    /// Get maximum concurrent sessions allowed
    pub fn max_sessions(&self) -> u32 {
        match self {
            UserTier::Free => 2,
            UserTier::Premium => 5,
            UserTier::Professional => 10,
            UserTier::Enterprise => 50,
        }
    }
    
    /// Get maximum API keys allowed
    pub fn max_api_keys(&self) -> u32 {
        match self {
            UserTier::Free => 1,
            UserTier::Premium => 5,
            UserTier::Professional => 10,
            UserTier::Enterprise => 100,
        }
    }
    
    /// Check if tier has access to feature
    pub fn has_feature_access(&self, feature: &str) -> bool {
        match feature {
            "basic_trading" => true,
            "advanced_analytics" => matches!(self, UserTier::Premium | UserTier::Professional | UserTier::Enterprise),
            "custom_strategies" => matches!(self, UserTier::Professional | UserTier::Enterprise),
            "api_access" => matches!(self, UserTier::Premium | UserTier::Professional | UserTier::Enterprise),
            "priority_support" => matches!(self, UserTier::Professional | UserTier::Enterprise),
            "white_label" => matches!(self, UserTier::Enterprise),
            _ => false,
        }
    }
}

/// Authentication information for response headers
#[derive(Debug, Clone, Serialize)]
pub struct AuthenticationInfo {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub authenticated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub permissions_count: usize,
    pub roles_count: usize,
}

impl From<&ApiContext> for AuthenticationInfo {
    fn from(context: &ApiContext) -> Self {
        Self {
            user_id: context.user_id,
            session_id: context.session_id,
            authenticated_at: context.authenticated_at,
            expires_at: context.token_info.expires_at,
            permissions_count: context.permissions.count(),
            roles_count: context.roles.len(),
        }
    }
}

/// Request context builder for middleware
pub struct RequestContextBuilder {
    request_id: Option<Uuid>,
    ip_address: Option<std::net::IpAddr>,
    user_agent: Option<String>,
    method: Option<String>,
    path: Option<String>,
    origin: Option<String>,
    forwarded_for: Option<String>,
}

impl RequestContextBuilder {
    pub fn new() -> Self {
        Self {
            request_id: None,
            ip_address: None,
            user_agent: None,
            method: None,
            path: None,
            origin: None,
            forwarded_for: None,
        }
    }
    
    pub fn request_id(mut self, id: Uuid) -> Self {
        self.request_id = Some(id);
        self
    }
    
    pub fn ip_address(mut self, ip: std::net::IpAddr) -> Self {
        self.ip_address = Some(ip);
        self
    }
    
    pub fn user_agent(mut self, agent: String) -> Self {
        self.user_agent = Some(agent);
        self
    }
    
    pub fn method(mut self, method: String) -> Self {
        self.method = Some(method);
        self
    }
    
    pub fn path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }
    
    pub fn origin(mut self, origin: String) -> Self {
        self.origin = Some(origin);
        self
    }
    
    pub fn forwarded_for(mut self, forwarded: String) -> Self {
        self.forwarded_for = Some(forwarded);
        self
    }
    
    pub fn build(self) -> RequestInfo {
        RequestInfo::new(
            self.request_id.unwrap_or_else(Uuid::new_v4),
            self.ip_address,
            self.user_agent,
            self.method.unwrap_or_else(|| "UNKNOWN".to_string()),
            self.path.unwrap_or_else(|| "/".to_string()),
            self.origin,
            self.forwarded_for,
        )
    }
}

impl Default for RequestContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}