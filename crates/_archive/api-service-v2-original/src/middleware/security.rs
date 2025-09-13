//! Security headers and CSRF protection middleware
//!
//! Provides comprehensive security protections with:
//! - Security headers (HSTS, CSP, etc.)
//! - CSRF token validation
//! - Clickjacking protection
//! - Content type sniffing prevention

use crate::error::{ApiError, ApiResult};
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};
use tracing::{debug, warn, error, instrument};
use uuid::Uuid;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_hsts: bool,
    pub hsts_max_age_seconds: u32,
    pub enable_csp: bool,
    pub csp_policy: String,
    pub enable_csrf_protection: bool,
    pub csrf_token_lifetime_seconds: u32,
    pub csrf_header_name: String,
    pub enable_frame_options: bool,
    pub frame_options: String,
    pub enable_content_type_options: bool,
    pub enable_referrer_policy: bool,
    pub referrer_policy: String,
    pub csrf_secret: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age_seconds: 31536000, // 1 year
            enable_csp: true,
            csp_policy: "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' https:; connect-src 'self' wss: https:".to_string(),
            enable_csrf_protection: true,
            csrf_token_lifetime_seconds: 3600, // 1 hour
            csrf_header_name: "x-csrf-token".to_string(),
            enable_frame_options: true,
            frame_options: "DENY".to_string(),
            enable_content_type_options: true,
            enable_referrer_policy: true,
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            csrf_secret: "default-secret-change-in-production".to_string(),
        }
    }
}

/// Security headers middleware
pub struct SecurityHeadersMiddleware;

impl SecurityHeadersMiddleware {
    /// Main security headers middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        request: Request,
        next: Next,
    ) -> Result<Response, ApiError> {
        let mut response = next.run(request).await;
        
        Self::add_security_headers(&mut response);
        
        Ok(response)
    }
    
    /// Add security headers to response
    fn add_security_headers(response: &mut Response) {
        let headers = response.headers_mut();
        
        // Strict Transport Security (HSTS)
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains; preload")
        );
        
        // Content Security Policy (CSP)
        headers.insert(
            "content-security-policy",
            HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' https:; connect-src 'self' wss: https:")
        );
        
        // X-Frame-Options (Clickjacking protection)
        headers.insert(
            "x-frame-options",
            HeaderValue::from_static("DENY")
        );
        
        // X-Content-Type-Options (MIME type sniffing prevention)
        headers.insert(
            "x-content-type-options",
            HeaderValue::from_static("nosniff")
        );
        
        // X-XSS-Protection (deprecated but still used by some browsers)
        headers.insert(
            "x-xss-protection",
            HeaderValue::from_static("1; mode=block")
        );
        
        // Referrer Policy
        headers.insert(
            "referrer-policy",
            HeaderValue::from_static("strict-origin-when-cross-origin")
        );
        
        // Permissions Policy (Feature Policy)
        headers.insert(
            "permissions-policy",
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()")
        );
        
        // Cross-Origin Embedder Policy
        headers.insert(
            "cross-origin-embedder-policy",
            HeaderValue::from_static("require-corp")
        );
        
        // Cross-Origin Opener Policy
        headers.insert(
            "cross-origin-opener-policy",
            HeaderValue::from_static("same-origin")
        );
        
        // Cross-Origin Resource Policy
        headers.insert(
            "cross-origin-resource-policy",
            HeaderValue::from_static("same-origin")
        );
    }
}

/// CSRF protection middleware
pub struct CsrfMiddleware {
    config: SecurityConfig,
}

impl CsrfMiddleware {
    /// Create new CSRF middleware
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }
    
    /// Main CSRF middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        config: SecurityConfig,
        request: Request,
        next: Next,
    ) -> Result<Response, ApiError> {
        let method = request.method();
        let path = request.uri().path();
        
        // Skip CSRF protection for safe methods and specific endpoints
        if Self::should_skip_csrf_check(method, path) {
            return Ok(next.run(request).await);
        }
        
        // Validate CSRF token
        Self::validate_csrf_token(&config, &request).await?;
        
        debug!("CSRF token validation passed");
        
        let mut response = next.run(request).await;
        
        // Add new CSRF token to response
        Self::add_csrf_token_header(&mut response, &config).await;
        
        Ok(response)
    }
    
    /// Check if CSRF validation should be skipped
    fn should_skip_csrf_check(method: &Method, path: &str) -> bool {
        // Skip for safe HTTP methods
        if matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS) {
            return true;
        }
        
        // Skip for authentication endpoints (they have their own protection)
        if path.starts_with("/api/v1/auth/login") || 
           path.starts_with("/api/v1/auth/register") ||
           path.starts_with("/api/v1/health") {
            return true;
        }
        
        false
    }
    
    /// Validate CSRF token from request
    async fn validate_csrf_token(
        config: &SecurityConfig,
        request: &Request,
    ) -> ApiResult<()> {
        // Extract CSRF token from header
        let csrf_token = request
            .headers()
            .get(&config.csrf_header_name)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                warn!("Missing CSRF token");
                ApiError::BadRequest("Missing CSRF token".to_string())
            })?;
        
        // Validate token format and signature
        Self::verify_csrf_token(config, csrf_token)?;
        
        Ok(())
    }
    
    /// Verify CSRF token
    fn verify_csrf_token(config: &SecurityConfig, token: &str) -> ApiResult<()> {
        // Parse token components (timestamp.random.signature)
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            warn!("Invalid CSRF token format");
            return Err(ApiError::BadRequest("Invalid CSRF token format".to_string()));
        }
        
        let timestamp_str = parts[0];
        let random_str = parts[1];
        let signature = parts[2];
        
        // Parse timestamp
        let timestamp = timestamp_str.parse::<u64>().map_err(|_| {
            warn!("Invalid CSRF token timestamp");
            ApiError::BadRequest("Invalid CSRF token".to_string())
        })?;
        
        // Check if token is expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now - timestamp > config.csrf_token_lifetime_seconds as u64 {
            warn!("CSRF token expired");
            return Err(ApiError::BadRequest("CSRF token expired".to_string()));
        }
        
        // Verify signature
        let expected_signature = Self::generate_csrf_signature(
            config, 
            timestamp_str, 
            random_str
        )?;
        
        if signature != expected_signature {
            warn!("Invalid CSRF token signature");
            return Err(ApiError::BadRequest("Invalid CSRF token".to_string()));
        }
        
        debug!("CSRF token validation successful");
        Ok(())
    }
    
    /// Generate CSRF token
    fn generate_csrf_token(config: &SecurityConfig) -> ApiResult<String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let random: u64 = rand::thread_rng().gen();
        let timestamp_str = timestamp.to_string();
        let random_str = random.to_string();
        
        let signature = Self::generate_csrf_signature(config, &timestamp_str, &random_str)?;
        
        Ok(format!("{}.{}.{}", timestamp_str, random_str, signature))
    }
    
    /// Generate CSRF token signature
    fn generate_csrf_signature(
        config: &SecurityConfig,
        timestamp: &str,
        random: &str,
    ) -> ApiResult<String> {
        let mut mac = Hmac::<Sha256>::new_from_slice(config.csrf_secret.as_bytes())
            .map_err(|e| {
                error!("Failed to create HMAC: {:?}", e);
                ApiError::InternalServerError("CSRF token generation failed".to_string())
            })?;
        
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(random.as_bytes());
        
        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());
        
        Ok(signature)
    }
    
    /// Add CSRF token to response headers
    async fn add_csrf_token_header(response: &mut Response, config: &SecurityConfig) {
        if let Ok(token) = Self::generate_csrf_token(config) {
            if let Ok(token_value) = HeaderValue::from_str(&token) {
                response.headers_mut().insert(&config.csrf_header_name, token_value);
            }
        }
    }
    
    /// Generate CSRF token for client (endpoint handler)
    pub fn generate_token_for_client(config: &SecurityConfig) -> ApiResult<String> {
        Self::generate_csrf_token(config)
    }
}

/// Content Security Policy builder
pub struct CspBuilder {
    directives: HashMap<String, Vec<String>>,
}

impl CspBuilder {
    /// Create new CSP builder
    pub fn new() -> Self {
        Self {
            directives: HashMap::new(),
        }
    }
    
    /// Add directive to CSP
    pub fn directive(mut self, name: &str, sources: &[&str]) -> Self {
        self.directives.insert(
            name.to_string(),
            sources.iter().map(|s| s.to_string()).collect(),
        );
        self
    }
    
    /// Build CSP header value
    pub fn build(self) -> String {
        let mut policy_parts = Vec::new();
        
        for (directive, sources) in self.directives {
            let sources_str = sources.join(" ");
            policy_parts.push(format!("{} {}", directive, sources_str));
        }
        
        policy_parts.join("; ")
    }
    
    /// Create secure default CSP
    pub fn secure_default() -> Self {
        Self::new()
            .directive("default-src", &["'self'"])
            .directive("script-src", &["'self'"])
            .directive("style-src", &["'self'", "'unsafe-inline'"])
            .directive("img-src", &["'self'", "data:", "https:"])
            .directive("font-src", &["'self'", "https:"])
            .directive("connect-src", &["'self'", "https:", "wss:"])
            .directive("frame-src", &["'none'"])
            .directive("object-src", &["'none'"])
            .directive("base-uri", &["'self'"])
            .directive("form-action", &["'self'"])
    }
}