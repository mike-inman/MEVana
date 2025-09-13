//! Middleware pipeline for security and request processing
//!
//! Provides comprehensive security middleware including:
//! - Authentication and authorization
//! - Rate limiting per user and endpoint
//! - Request validation and sanitization
//! - Audit logging and monitoring
//! - Circuit breaker integration

pub mod auth;
pub mod rate_limit;
pub mod cors;
pub mod logging;
pub mod validation;
pub mod security;
pub mod metrics;

pub use auth::AuthMiddleware;
pub use rate_limit::{RateLimitMiddleware, RateLimitConfig};
pub use cors::CorsMiddleware;
pub use logging::{RequestLoggingMiddleware, AuditMiddleware};
pub use validation::ValidationMiddleware;
pub use security::{SecurityHeadersMiddleware, CsrfMiddleware};
pub use metrics::MetricsMiddleware;

use axum::{middleware::from_fn, Router};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use std::time::Duration;

/// Security middleware configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub enable_csrf_protection: bool,
    pub enable_rate_limiting: bool,
    pub enable_request_logging: bool,
    pub enable_audit_logging: bool,
    pub request_timeout_seconds: u64,
    pub max_request_size_bytes: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_csrf_protection: true,
            enable_rate_limiting: true,
            enable_request_logging: true,
            enable_audit_logging: true,
            request_timeout_seconds: 30,
            max_request_size_bytes: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Build the complete middleware stack
pub fn build_middleware_stack(
    security_config: SecurityConfig,
    rate_limit_config: RateLimitConfig,
) -> ServiceBuilder<
    impl tower::Layer<
        tower::util::Either<
            axum::routing::Route,
            axum::routing::MethodRouter,
        >,
    > + Clone,
> {
    ServiceBuilder::new()
        // Request ID generation and propagation
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        
        // Request timeout
        .layer(TimeoutLayer::new(Duration::from_secs(
            security_config.request_timeout_seconds
        )))
        
        // Compression
        .layer(CompressionLayer::new())
        
        // Distributed tracing
        .layer(TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| {
                tracing::info_span!(
                    "request",
                    method = %request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                )
            })
            .on_request(|_request: &axum::http::Request<_>, _span: &tracing::Span| {
                tracing::info!("started processing request")
            })
            .on_response(|response: &axum::http::Response<_>, latency: Duration, _span: &tracing::Span| {
                tracing::info!("finished processing request in {:?}, status: {}", latency, response.status())
            })
            .on_failure(|error: tower_http::classify::ServerErrorsFailureClass, latency: Duration, _span: &tracing::Span| {
                tracing::error!("request failed after {:?}: {}", latency, error)
            })
        )
}

/// Apply authentication middleware to routes
pub fn with_auth<S>(router: Router<S>) -> Router<S> 
where 
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(AuthMiddleware::middleware))
}

/// Apply rate limiting middleware
pub fn with_rate_limit<S>(router: Router<S>, config: RateLimitConfig) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(move |req, next| {
        RateLimitMiddleware::new(config.clone()).middleware(req, next)
    }))
}

/// Apply security headers middleware
pub fn with_security_headers<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(SecurityHeadersMiddleware::middleware))
}

/// Apply request logging middleware
pub fn with_request_logging<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(RequestLoggingMiddleware::middleware))
}

/// Apply audit logging middleware
pub fn with_audit_logging<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(AuditMiddleware::middleware))
}

/// Apply metrics collection middleware
pub fn with_metrics<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(MetricsMiddleware::middleware))
}

/// Apply CSRF protection middleware
pub fn with_csrf_protection<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(CsrfMiddleware::middleware))
}

/// Apply comprehensive security middleware stack
pub fn with_full_security<S>(
    router: Router<S>,
    security_config: SecurityConfig,
    rate_limit_config: RateLimitConfig,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let mut secured_router = router;
    
    // Apply security headers
    secured_router = with_security_headers(secured_router);
    
    // Apply CSRF protection if enabled
    if security_config.enable_csrf_protection {
        secured_router = with_csrf_protection(secured_router);
    }
    
    // Apply rate limiting if enabled
    if security_config.enable_rate_limiting {
        secured_router = with_rate_limit(secured_router, rate_limit_config);
    }
    
    // Apply request logging if enabled
    if security_config.enable_request_logging {
        secured_router = with_request_logging(secured_router);
    }
    
    // Apply audit logging if enabled
    if security_config.enable_audit_logging {
        secured_router = with_audit_logging(secured_router);
    }
    
    // Apply metrics collection
    secured_router = with_metrics(secured_router);
    
    // Apply authentication (always required for API routes)
    secured_router = with_auth(secured_router);
    
    secured_router
}