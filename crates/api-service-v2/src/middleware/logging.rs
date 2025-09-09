//! Request logging and audit middleware
//!
//! Provides comprehensive logging with:
//! - Structured request/response logging
//! - Performance metrics
//! - Security audit trails
//! - User action tracking

use crate::{auth::JwtClaims, error::ApiError};
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, time::Instant};
use tracing::{debug, info, warn, error, instrument, Span};
use uuid::Uuid;

/// Request logging middleware
pub struct RequestLoggingMiddleware;

impl RequestLoggingMiddleware {
    /// Main request logging middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        request: Request,
        next: Next,
    ) -> Result<Response, ApiError> {
        let start_time = Instant::now();
        
        // Extract request information
        let method = request.method().to_string();
        let path = request.uri().path().to_string();
        let query = request.uri().query().unwrap_or("").to_string();
        let version = format!("{:?}", request.version());
        
        // Extract request ID
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<Uuid>().ok())
            .unwrap_or_else(Uuid::new_v4);
        
        // Extract client information
        let user_agent = request
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        
        let client_ip = extract_client_ip(&request);
        
        // Extract user information if authenticated
        let user_context = request
            .extensions()
            .get::<JwtClaims>()
            .map(|claims| UserContext {
                user_id: claims.sub,
                email: claims.email.clone(),
                session_id: claims.session_id,
                tier: claims.tier.clone(),
            });
        
        // Log request
        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            query = %query,
            version = %version,
            user_agent = %user_agent,
            client_ip = ?client_ip,
            user_id = ?user_context.as_ref().map(|u| u.user_id),
            session_id = ?user_context.as_ref().map(|u| u.session_id),
            "Request started"
        );
        
        // Process request
        let response = next.run(request).await;
        
        // Calculate duration
        let duration = start_time.elapsed();
        
        // Log response
        let status = response.status();
        let status_class = classify_status(status);
        
        match status_class {
            StatusClass::Success => {
                info!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    duration_ms = duration.as_millis(),
                    user_id = ?user_context.as_ref().map(|u| u.user_id),
                    "Request completed successfully"
                );
            }
            StatusClass::ClientError => {
                warn!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    duration_ms = duration.as_millis(),
                    user_id = ?user_context.as_ref().map(|u| u.user_id),
                    client_ip = ?client_ip,
                    "Request failed - client error"
                );
            }
            StatusClass::ServerError => {
                error!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    duration_ms = duration.as_millis(),
                    user_id = ?user_context.as_ref().map(|u| u.user_id),
                    client_ip = ?client_ip,
                    "Request failed - server error"
                );
            }
            StatusClass::Redirection => {
                debug!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    duration_ms = duration.as_millis(),
                    "Request redirected"
                );
            }
        }
        
        Ok(response)
    }
}

/// Audit logging middleware for security-sensitive operations
pub struct AuditMiddleware;

impl AuditMiddleware {
    /// Main audit logging middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        request: Request,
        next: Next,
    ) -> Result<Response, ApiError> {
        let path = request.uri().path().to_string();
        let method = request.method().to_string();
        
        // Check if this endpoint requires audit logging
        if !should_audit(&path, &method) {
            return Ok(next.run(request).await);
        }
        
        let start_time = Instant::now();
        
        // Extract audit context
        let audit_context = AuditContext::from_request(&request);
        
        // Log audit start
        info!(
            audit_event = "request_start",
            request_id = %audit_context.request_id,
            user_id = ?audit_context.user_id,
            session_id = ?audit_context.session_id,
            method = %method,
            path = %path,
            client_ip = ?audit_context.client_ip,
            user_agent = %audit_context.user_agent,
            timestamp = %chrono::Utc::now().to_rfc3339(),
            "Audit: Request initiated"
        );
        
        // Process request
        let response = next.run(request).await;
        
        // Calculate duration and outcome
        let duration = start_time.elapsed();
        let status = response.status();
        let outcome = if status.is_success() { "success" } else { "failure" };
        
        // Log audit completion
        info!(
            audit_event = "request_complete",
            request_id = %audit_context.request_id,
            user_id = ?audit_context.user_id,
            session_id = ?audit_context.session_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            outcome = %outcome,
            duration_ms = duration.as_millis(),
            timestamp = %chrono::Utc::now().to_rfc3339(),
            "Audit: Request completed"
        );
        
        // Log specific security events
        if is_security_sensitive(&path) {
            Self::log_security_event(&audit_context, &method, &path, status).await;
        }
        
        Ok(response)
    }
    
    /// Log security-specific events
    async fn log_security_event(
        context: &AuditContext,
        method: &str,
        path: &str,
        status: StatusCode,
    ) {
        let event_type = match path {
            p if p.contains("/auth/login") => "authentication_attempt",
            p if p.contains("/auth/logout") => "authentication_logout",
            p if p.contains("/auth/register") => "user_registration",
            p if p.contains("/wallet") => "wallet_operation",
            p if p.contains("/trading") => "trading_operation",
            p if p.contains("/admin") => "admin_operation",
            _ => "security_operation",
        };
        
        warn!(
            audit_event = "security_event",
            event_type = %event_type,
            request_id = %context.request_id,
            user_id = ?context.user_id,
            session_id = ?context.session_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            client_ip = ?context.client_ip,
            user_agent = %context.user_agent,
            success = %status.is_success(),
            timestamp = %chrono::Utc::now().to_rfc3339(),
            "Security event logged"
        );
    }
}

/// User context for logging
#[derive(Debug, Clone)]
struct UserContext {
    user_id: Uuid,
    email: String,
    session_id: Uuid,
    tier: String,
}

/// Audit context for security logging
#[derive(Debug, Clone)]
struct AuditContext {
    request_id: Uuid,
    user_id: Option<Uuid>,
    session_id: Option<Uuid>,
    client_ip: Option<IpAddr>,
    user_agent: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl AuditContext {
    /// Create audit context from request
    fn from_request(request: &Request) -> Self {
        let user_context = request
            .extensions()
            .get::<JwtClaims>()
            .map(|claims| (claims.sub, claims.session_id));
        
        Self {
            request_id: request
                .headers()
                .get("x-request-id")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<Uuid>().ok())
                .unwrap_or_else(Uuid::new_v4),
            user_id: user_context.map(|(id, _)| id),
            session_id: user_context.map(|(_, session)| session),
            client_ip: extract_client_ip(request),
            user_agent: request
                .headers()
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Status code classification
#[derive(Debug, Clone, PartialEq, Eq)]
enum StatusClass {
    Success,
    Redirection,
    ClientError,
    ServerError,
}

/// Classify HTTP status code
fn classify_status(status: StatusCode) -> StatusClass {
    match status.as_u16() {
        200..=299 => StatusClass::Success,
        300..=399 => StatusClass::Redirection,
        400..=499 => StatusClass::ClientError,
        500..=599 => StatusClass::ServerError,
        _ => StatusClass::ServerError,
    }
}

/// Extract client IP from request
fn extract_client_ip(request: &Request) -> Option<IpAddr> {
    // Check X-Forwarded-For header first (for reverse proxies)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }
    
    // Check X-Real-IP header (nginx)
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }
    
    // Note: In a real implementation, you might get the remote address from the connection
    // This would require access to the underlying connection info
    None
}

/// Check if endpoint should be audited
fn should_audit(path: &str, method: &str) -> bool {
    // Always audit authentication endpoints
    if path.contains("/auth/") {
        return true;
    }
    
    // Always audit admin endpoints
    if path.contains("/admin/") {
        return true;
    }
    
    // Audit wallet operations
    if path.contains("/wallet/") {
        return true;
    }
    
    // Audit trading operations
    if path.contains("/trading/") {
        return true;
    }
    
    // Audit write operations on sensitive endpoints
    if matches!(method, "POST" | "PUT" | "PATCH" | "DELETE") {
        return path.contains("/api/v1/");
    }
    
    false
}

/// Check if endpoint is security sensitive
fn is_security_sensitive(path: &str) -> bool {
    path.contains("/auth/") ||
    path.contains("/admin/") ||
    path.contains("/wallet/") ||
    path.contains("/trading/") ||
    path.contains("/user/")
}