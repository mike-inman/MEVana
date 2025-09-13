//! Authentication middleware
//!
//! Provides JWT token validation and user context extraction
//! for protected API endpoints.

use super::super::{auth::{JwtAuthService, JwtClaims}, error::{ApiError, AuthErrorType}};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, warn, instrument};

/// Authentication middleware implementation
pub struct AuthMiddleware;

impl AuthMiddleware {
    /// Main authentication middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        State(auth_service): State<Arc<JwtAuthService>>,
        mut request: Request,
        next: Next,
    ) -> Result<Response, ApiError> {
        // Extract Authorization header
        let auth_header = request
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                debug!("Missing authorization header");
                ApiError::Unauthorized(AuthErrorType::MissingToken)
            })?;

        // Extract Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| {
                debug!("Invalid authorization header format");
                ApiError::Unauthorized(AuthErrorType::InvalidToken)
            })?;

        // Validate token
        let claims = auth_service.validate_token(token).await?;
        
        // Store claims in request extensions for use by handlers
        request.extensions_mut().insert(claims.clone());
        
        debug!("Authentication successful for user: {}", claims.sub);
        
        // Continue to next middleware/handler
        let mut response = next.run(request).await;
        
        // Add authentication info to response headers
        add_auth_response_headers(&mut response, &claims);
        
        Ok(response)
    }
}

/// Add authentication information to response headers
fn add_auth_response_headers(response: &mut Response, claims: &JwtClaims) {
    let headers = response.headers_mut();
    
    // Add user context headers (for debugging/monitoring)
    if let Ok(user_id) = HeaderValue::from_str(&claims.sub.to_string()) {
        headers.insert("x-authenticated-user", user_id);
    }
    
    if let Ok(session_id) = HeaderValue::from_str(&claims.session_id.to_string()) {
        headers.insert("x-session-id", session_id);
    }
    
    if let Ok(tier) = HeaderValue::from_str(&claims.tier) {
        headers.insert("x-user-tier", tier);
    }
    
    // Add token expiration info
    if let Ok(exp) = HeaderValue::from_str(&claims.exp.to_string()) {
        headers.insert("x-token-expires", exp);
    }
}

/// Permission checking middleware
pub async fn require_permission(
    permission: super::super::auth::Permission,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        let required_permission = permission;
        Box::pin(async move {
            // Extract claims from request extensions
            let claims = request
                .extensions()
                .get::<JwtClaims>()
                .ok_or_else(|| {
                    warn!("No JWT claims found in request - authentication middleware not applied?");
                    ApiError::Unauthorized(AuthErrorType::MissingToken)
                })?;

            // Create permission set from claims
            let permissions = super::super::auth::PermissionSet::from_strings(&claims.permissions);
            
            // Check if user has required permission
            if !permissions.has_permission(required_permission) {
                warn!(
                    "User {} lacks required permission: {:?}",
                    claims.sub, required_permission
                );
                return Err(ApiError::Forbidden(format!(
                    "Required permission: {}",
                    required_permission.as_str()
                )));
            }
            
            debug!(
                "Permission check passed for user {} with permission: {:?}",
                claims.sub, required_permission
            );
            
            // Permission granted, continue
            Ok(next.run(request).await)
        })
    }
}

/// Multi-permission checking (user must have ALL permissions)
pub async fn require_all_permissions(
    permissions: Vec<super::super::auth::Permission>,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        let required_permissions = permissions.clone();
        Box::pin(async move {
            let claims = request
                .extensions()
                .get::<JwtClaims>()
                .ok_or_else(|| {
                    warn!("No JWT claims found in request - authentication middleware not applied?");
                    ApiError::Unauthorized(AuthErrorType::MissingToken)
                })?;

            let user_permissions = super::super::auth::PermissionSet::from_strings(&claims.permissions);
            
            // Check all permissions
            for permission in &required_permissions {
                if !user_permissions.has_permission(*permission) {
                    warn!(
                        "User {} lacks required permission: {:?}",
                        claims.sub, permission
                    );
                    return Err(ApiError::Forbidden(format!(
                        "Required permission: {}",
                        permission.as_str()
                    )));
                }
            }
            
            debug!(
                "All permission checks passed for user {} with {} permissions",
                claims.sub, required_permissions.len()
            );
            
            Ok(next.run(request).await)
        })
    }
}

/// Any permission checking (user must have AT LEAST ONE permission)
pub async fn require_any_permission(
    permissions: Vec<super::super::auth::Permission>,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        let required_permissions = permissions.clone();
        Box::pin(async move {
            let claims = request
                .extensions()
                .get::<JwtClaims>()
                .ok_or_else(|| {
                    warn!("No JWT claims found in request - authentication middleware not applied?");
                    ApiError::Unauthorized(AuthErrorType::MissingToken)
                })?;

            let user_permissions = super::super::auth::PermissionSet::from_strings(&claims.permissions);
            
            // Check if user has any of the required permissions
            let has_permission = required_permissions
                .iter()
                .any(|p| user_permissions.has_permission(*p));
            
            if !has_permission {
                warn!(
                    "User {} lacks any of the required permissions: {:?}",
                    claims.sub, required_permissions
                );
                let permission_strings: Vec<String> = required_permissions
                    .iter()
                    .map(|p| p.as_str().to_string())
                    .collect();
                return Err(ApiError::Forbidden(format!(
                    "Required one of permissions: {}",
                    permission_strings.join(", ")
                )));
            }
            
            debug!(
                "Permission check passed for user {} with one of {} permissions",
                claims.sub, required_permissions.len()
            );
            
            Ok(next.run(request).await)
        })
    }
}

/// Role-based access control middleware
pub async fn require_role(
    role: String,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        let required_role = role.clone();
        Box::pin(async move {
            let claims = request
                .extensions()
                .get::<JwtClaims>()
                .ok_or_else(|| {
                    warn!("No JWT claims found in request - authentication middleware not applied?");
                    ApiError::Unauthorized(AuthErrorType::MissingToken)
                })?;

            // Check if user has required role
            if !claims.roles.contains(&required_role) {
                warn!(
                    "User {} lacks required role: {}",
                    claims.sub, required_role
                );
                return Err(ApiError::Forbidden(format!(
                    "Required role: {}",
                    required_role
                )));
            }
            
            debug!(
                "Role check passed for user {} with role: {}",
                claims.sub, required_role
            );
            
            Ok(next.run(request).await)
        })
    }
}

/// User tier checking middleware
pub async fn require_tier(
    min_tier: super::super::auth::UserTier,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        let required_tier = min_tier.clone();
        Box::pin(async move {
            let claims = request
                .extensions()
                .get::<JwtClaims>()
                .ok_or_else(|| {
                    warn!("No JWT claims found in request - authentication middleware not applied?");
                    ApiError::Unauthorized(AuthErrorType::MissingToken)
                })?;

            // Parse user tier
            let user_tier = match claims.tier.as_str() {
                "Free" => super::super::auth::UserTier::Free,
                "Premium" => super::super::auth::UserTier::Premium,
                "Professional" => super::super::auth::UserTier::Professional,
                "Enterprise" => super::super::auth::UserTier::Enterprise,
                _ => super::super::auth::UserTier::Free,
            };
            
            // Check tier hierarchy (Enterprise > Professional > Premium > Free)
            let tier_level = |tier: &super::super::auth::UserTier| match tier {
                super::super::auth::UserTier::Free => 0,
                super::super::auth::UserTier::Premium => 1,
                super::super::auth::UserTier::Professional => 2,
                super::super::auth::UserTier::Enterprise => 3,
            };
            
            if tier_level(&user_tier) < tier_level(&required_tier) {
                warn!(
                    "User {} has insufficient tier: {:?}, required: {:?}",
                    claims.sub, user_tier, required_tier
                );
                return Err(ApiError::Forbidden(format!(
                    "Required minimum tier: {:?}",
                    required_tier
                )));
            }
            
            debug!(
                "Tier check passed for user {} with tier: {:?}",
                claims.sub, user_tier
            );
            
            Ok(next.run(request).await)
        })
    }
}

/// Extract JWT claims from request for use in handlers
pub fn extract_claims(request: &Request) -> Result<&JwtClaims, ApiError> {
    request
        .extensions()
        .get::<JwtClaims>()
        .ok_or_else(|| {
            warn!("No JWT claims found in request - authentication middleware not applied?");
            ApiError::Unauthorized(AuthErrorType::MissingToken)
        })
}

/// Extract user ID from request
pub fn extract_user_id(request: &Request) -> Result<Uuid, ApiError> {
    Ok(extract_claims(request)?.sub)
}

/// Extract session ID from request
pub fn extract_session_id(request: &Request) -> Result<Uuid, ApiError> {
    Ok(extract_claims(request)?.session_id)
}