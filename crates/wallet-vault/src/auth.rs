use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub exp: usize,  // Expiration time
    pub iat: usize,  // Issued at
}

#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub required: bool, // Allow bypassing auth for development
}

impl AuthConfig {
    pub fn new(jwt_secret: String, required: bool) -> Self {
        Self { jwt_secret, required }
    }
}

// Middleware function that validates JWT tokens
pub async fn auth_middleware(
    State(auth_config): State<Arc<AuthConfig>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth if not required (development mode)
    if !auth_config.required {
        return Ok(next.run(request).await);
    }

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| {
            if header.starts_with("Bearer ") {
                Some(header.trim_start_matches("Bearer "))
            } else {
                None
            }
        });

    let token = match auth_header {
        Some(token) => token,
        None => {
            log::warn!("Missing or invalid Authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Validate JWT token
    let validation = Validation::new(Algorithm::HS256);
    let decoding_key = DecodingKey::from_secret(auth_config.jwt_secret.as_bytes());
    
    match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            // Add user ID to request extensions for use in handlers
            request.extensions_mut().insert(UserContext {
                user_id: token_data.claims.sub,
            });
            Ok(next.run(request).await)
        }
        Err(err) => {
            log::warn!("JWT validation failed: {:?}", err);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
}

// Alternative auth for API keys (for service-to-service)
pub async fn api_key_middleware(
    State(auth_config): State<Arc<AuthConfig>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth if not required
    if !auth_config.required {
        return Ok(next.run(request).await);
    }

    // Check for API key in X-User-ID header (from Next.js API)
    let user_id_header = request
        .headers()
        .get("X-User-ID")
        .and_then(|header| header.to_str().ok())
        .map(|s| s.to_string());

    let api_key = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| {
            if header.starts_with("Bearer ") {
                Some(header.trim_start_matches("Bearer "))
            } else {
                None
            }
        })
        .map(|s| s.to_string());

    // Validate API key (simple secret for service-to-service)
    let expected_key = std::env::var("WALLET_VAULT_SECRET").unwrap_or_else(|_| "dev-secret".to_string());
    
    match (user_id_header, api_key) {
        (Some(user_id), Some(key)) if key == expected_key => {
            // Add user context from header
            request.extensions_mut().insert(UserContext {
                user_id,
            });
            Ok(next.run(request).await)
        }
        _ => {
            log::warn!("Invalid API key or missing user ID");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}