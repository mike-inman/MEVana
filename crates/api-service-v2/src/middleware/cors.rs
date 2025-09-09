//! CORS (Cross-Origin Resource Sharing) middleware
//!
//! Provides configurable CORS support with:
//! - Environment-specific allowed origins
//! - Secure defaults for production
//! - Preflight request handling
//! - Credentials support

use axum::{
    extract::Request,
    http::{HeaderValue, Method},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, warn, instrument};

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age_seconds: u32,
    pub vary_origin: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),  // Development
                "http://localhost:3001".to_string(),  // Development alt
                "https://app.mevana.com".to_string(), // Production
                "https://staging.mevana.com".to_string(), // Staging
            ],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "PATCH".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "accept".to_string(),
                "accept-encoding".to_string(),
                "authorization".to_string(),
                "content-type".to_string(),
                "dnt".to_string(),
                "origin".to_string(),
                "user-agent".to_string(),
                "x-csrf-token".to_string(),
                "x-requested-with".to_string(),
                "x-api-key".to_string(),
            ],
            exposed_headers: vec![
                "x-ratelimit-limit".to_string(),
                "x-ratelimit-remaining".to_string(),
                "x-ratelimit-reset".to_string(),
                "x-request-id".to_string(),
                "x-authenticated-user".to_string(),
            ],
            allow_credentials: true,
            max_age_seconds: 86400, // 24 hours
            vary_origin: true,
        }
    }
}

/// CORS middleware implementation
pub struct CorsMiddleware {
    config: CorsConfig,
    allowed_origins_set: HashSet<String>,
}

impl CorsMiddleware {
    /// Create new CORS middleware
    pub fn new(config: CorsConfig) -> Self {
        let allowed_origins_set = config.allowed_origins.iter().cloned().collect();
        Self {
            config,
            allowed_origins_set,
        }
    }
    
    /// Main CORS middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        config: CorsConfig,
        request: Request,
        next: Next,
    ) -> Response {
        let method = request.method().clone();
        let origin = request
            .headers()
            .get("origin")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        
        debug!("CORS check - Method: {:?}, Origin: {}", method, origin);
        
        // Handle preflight requests
        if method == Method::OPTIONS {
            debug!("Handling CORS preflight request");
            return Self::handle_preflight(config, origin).await;
        }
        
        // Process the request
        let mut response = next.run(request).await;
        
        // Add CORS headers to response
        Self::add_cors_headers(&mut response, &config, origin);
        
        response
    }
    
    /// Handle CORS preflight (OPTIONS) requests
    async fn handle_preflight(config: CorsConfig, origin: &str) -> Response {
        let mut response = Response::builder()
            .status(204) // No Content
            .body(axum::body::Body::empty())
            .unwrap();
        
        Self::add_cors_headers(&mut response, &config, origin);
        
        // Add preflight-specific headers
        let headers = response.headers_mut();
        
        // Access-Control-Allow-Methods
        let methods = config.allowed_methods.join(", ");
        if let Ok(methods_value) = HeaderValue::from_str(&methods) {
            headers.insert("access-control-allow-methods", methods_value);
        }
        
        // Access-Control-Allow-Headers
        let allow_headers = config.allowed_headers.join(", ");
        if let Ok(headers_value) = HeaderValue::from_str(&allow_headers) {
            headers.insert("access-control-allow-headers", headers_value);
        }
        
        // Access-Control-Max-Age
        if let Ok(max_age_value) = HeaderValue::from_str(&config.max_age_seconds.to_string()) {
            headers.insert("access-control-max-age", max_age_value);
        }
        
        debug!("CORS preflight response sent for origin: {}", origin);
        response
    }
    
    /// Add CORS headers to response
    fn add_cors_headers(response: &mut Response, config: &CorsConfig, origin: &str) {
        let headers = response.headers_mut();
        
        // Access-Control-Allow-Origin
        if Self::is_origin_allowed(config, origin) {
            if let Ok(origin_value) = HeaderValue::from_str(origin) {
                headers.insert("access-control-allow-origin", origin_value);
            }
            
            // Include credentials if allowed and origin is specific
            if config.allow_credentials && origin != "*" {
                headers.insert("access-control-allow-credentials", HeaderValue::from_static("true"));
            }
        } else {
            warn!("Origin not allowed: {}", origin);
            // Don't set CORS headers for disallowed origins
            return;
        }
        
        // Access-Control-Expose-Headers
        if !config.exposed_headers.is_empty() {
            let exposed = config.exposed_headers.join(", ");
            if let Ok(exposed_value) = HeaderValue::from_str(&exposed) {
                headers.insert("access-control-expose-headers", exposed_value);
            }
        }
        
        // Vary header for origin-specific responses
        if config.vary_origin {
            headers.insert("vary", HeaderValue::from_static("origin"));
        }
    }
    
    /// Check if origin is allowed
    fn is_origin_allowed(config: &CorsConfig, origin: &str) -> bool {
        if origin.is_empty() {
            return false;
        }
        
        // Check exact match
        if config.allowed_origins.contains(&origin.to_string()) {
            return true;
        }
        
        // Check wildcard
        if config.allowed_origins.contains(&"*".to_string()) {
            return true;
        }
        
        // Check pattern matching (basic subdomain support)
        for allowed in &config.allowed_origins {
            if allowed.starts_with("*.") {
                let domain = &allowed[2..]; // Remove "*."
                if origin.ends_with(domain) || origin == domain {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Create development-friendly CORS config
    pub fn development() -> CorsConfig {
        CorsConfig {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "PATCH".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "*".to_string(),
            ],
            exposed_headers: vec![
                "*".to_string(),
            ],
            allow_credentials: false,
            max_age_seconds: 3600, // 1 hour
            vary_origin: false,
        }
    }
    
    /// Create production CORS config with strict security
    pub fn production(allowed_domains: Vec<String>) -> CorsConfig {
        CorsConfig {
            allowed_origins: allowed_domains,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "PATCH".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "accept".to_string(),
                "accept-encoding".to_string(),
                "authorization".to_string(),
                "content-type".to_string(),
                "origin".to_string(),
                "x-csrf-token".to_string(),
                "x-requested-with".to_string(),
            ],
            exposed_headers: vec![
                "x-ratelimit-limit".to_string(),
                "x-ratelimit-remaining".to_string(),
                "x-request-id".to_string(),
            ],
            allow_credentials: true,
            max_age_seconds: 86400, // 24 hours
            vary_origin: true,
        }
    }
}