//! JWT token management
//!
//! Provides secure JWT operations with:
//! - Token generation and validation
//! - Automatic expiration handling
//! - Token rotation support
//! - Blacklist management for revoked tokens

use super::{AuthConfig, AuthResult, AuthErrorType};
use crate::error::ApiError;
use chrono::{Duration, Utc, DateTime};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use tracing::{debug, warn, error, instrument};

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: Uuid,
    
    /// Issued at (timestamp)
    pub iat: i64,
    
    /// Expiration time (timestamp)
    pub exp: i64,
    
    /// Not before (timestamp)
    pub nbf: i64,
    
    /// JWT ID (unique identifier)
    pub jti: Uuid,
    
    /// Issuer
    pub iss: String,
    
    /// User email
    pub email: String,
    
    /// User permissions
    pub permissions: Vec<String>,
    
    /// User roles
    pub roles: Vec<String>,
    
    /// User tier
    pub tier: String,
    
    /// Token type
    pub token_type: TokenType,
    
    /// Session ID
    pub session_id: Uuid,
}

/// Token type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    Access,
    Refresh,
}

/// Token pair (access + refresh)
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// JWT authentication service
pub struct JwtAuthService {
    config: AuthConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    redis: ConnectionManager,
    validation: Validation,
}

impl JwtAuthService {
    /// Create new JWT service
    pub fn new(config: AuthConfig, redis: ConnectionManager) -> AuthResult<Self> {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());
        
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["MEVana-API"]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        
        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            redis,
            validation,
        })
    }
    
    /// Generate token pair for user
    #[instrument(skip(self, permissions, roles))]
    pub async fn generate_token_pair(
        &self,
        user_id: Uuid,
        email: String,
        permissions: Vec<String>,
        roles: Vec<String>,
        tier: String,
        session_id: Uuid,
    ) -> AuthResult<TokenPair> {
        debug!("Generating token pair for user: {}", user_id);
        
        let now = Utc::now();
        let access_expires_at = now + Duration::seconds(self.config.access_token_lifetime_seconds as i64);
        let refresh_expires_at = now + Duration::seconds(self.config.refresh_token_lifetime_seconds as i64);
        
        // Generate access token
        let access_claims = JwtClaims {
            sub: user_id,
            iat: now.timestamp(),
            exp: access_expires_at.timestamp(),
            nbf: now.timestamp(),
            jti: Uuid::new_v4(),
            iss: "MEVana-API".to_string(),
            email: email.clone(),
            permissions: permissions.clone(),
            roles: roles.clone(),
            tier: tier.clone(),
            token_type: TokenType::Access,
            session_id,
        };
        
        // Generate refresh token
        let refresh_claims = JwtClaims {
            sub: user_id,
            iat: now.timestamp(),
            exp: refresh_expires_at.timestamp(),
            nbf: now.timestamp(),
            jti: Uuid::new_v4(),
            iss: "MEVana-API".to_string(),
            email,
            permissions: Vec::new(), // Refresh tokens don't carry permissions
            roles: Vec::new(),       // Refresh tokens don't carry roles
            tier,
            token_type: TokenType::Refresh,
            session_id,
        };
        
        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| {
                error!("Failed to encode access token: {:?}", e);
                ApiError::InternalServerError("Token generation failed".to_string())
            })?;
            
        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| {
                error!("Failed to encode refresh token: {:?}", e);
                ApiError::InternalServerError("Token generation failed".to_string())
            })?;
        
        // Store tokens in Redis for tracking/blacklisting
        self.store_token_metadata(&access_claims).await?;
        self.store_token_metadata(&refresh_claims).await?;
        
        Ok(TokenPair {
            access_token,
            refresh_token,
            expires_in: self.config.access_token_lifetime_seconds,
            token_type: "Bearer".to_string(),
        })
    }
    
    /// Validate and decode JWT token
    #[instrument(skip(self, token))]
    pub async fn validate_token(&self, token: &str) -> AuthResult<JwtClaims> {
        // Check if token is blacklisted
        if self.is_token_blacklisted(token).await? {
            debug!("Token is blacklisted");
            return Err(ApiError::Unauthorized(AuthErrorType::TokenInvalid));
        }
        
        // Decode and validate token
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                debug!("Token validation failed: {:?}", e);
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        ApiError::Unauthorized(AuthErrorType::TokenExpired)
                    }
                    _ => ApiError::Unauthorized(AuthErrorType::TokenInvalid),
                }
            })?;
        
        Ok(token_data.claims)
    }
    
    /// Refresh token pair
    #[instrument(skip(self, refresh_token))]
    pub async fn refresh_tokens(
        &self,
        refresh_token: &str,
    ) -> AuthResult<TokenPair> {
        // Validate refresh token
        let claims = self.validate_token(refresh_token).await?;
        
        // Ensure it's a refresh token
        if claims.token_type != TokenType::Refresh {
            return Err(ApiError::Unauthorized(AuthErrorType::TokenInvalid));
        }
        
        // Get user's current permissions and roles from database
        // This is a simplified version - in production, you'd fetch from database
        let permissions = vec!["read".to_string()]; // TODO: Fetch from database
        let roles = vec!["user".to_string()];       // TODO: Fetch from database
        
        // Generate new token pair
        let new_tokens = self.generate_token_pair(
            claims.sub,
            claims.email,
            permissions,
            roles,
            claims.tier,
            claims.session_id,
        ).await?;
        
        // If token rotation is enabled, blacklist the old refresh token
        if self.config.enable_token_rotation {
            self.blacklist_token(refresh_token).await?;
        }
        
        Ok(new_tokens)
    }
    
    /// Blacklist a token (logout/revoke)
    #[instrument(skip(self, token))]
    pub async fn blacklist_token(&self, token: &str) -> AuthResult<()> {
        debug!("Blacklisting token");
        
        let claims = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|_| ApiError::Unauthorized(AuthErrorType::TokenInvalid))?;
        
        // Store token ID in blacklist with expiration
        let blacklist_key = format!("blacklist:token:{}", claims.jti);
        let ttl_seconds = (claims.exp - Utc::now().timestamp()).max(0) as u64;
        
        redis::cmd("SETEX")
            .arg(&blacklist_key)
            .arg(ttl_seconds)
            .arg("1")
            .query_async::<ConnectionManager, ()>(&mut self.redis.clone())
            .await
            .map_err(|e| {
                error!("Failed to blacklist token: {:?}", e);
                ApiError::InternalServerError("Token blacklisting failed".to_string())
            })?;
        
        debug!("Token blacklisted successfully");
        Ok(())
    }
    
    /// Check if token is blacklisted
    #[instrument(skip(self, token))]
    async fn is_token_blacklisted(&self, token: &str) -> AuthResult<bool> {
        let claims = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|_| ApiError::Unauthorized(AuthErrorType::TokenInvalid))?;
        
        let blacklist_key = format!("blacklist:token:{}", claims.jti);
        
        let is_blacklisted: bool = redis::cmd("EXISTS")
            .arg(&blacklist_key)
            .query_async::<ConnectionManager, i32>(&mut self.redis.clone())
            .await
            .map_err(|e| {
                error!("Failed to check token blacklist: {:?}", e);
                ApiError::InternalServerError("Token validation failed".to_string())
            })?
            == 1;
        
        Ok(is_blacklisted)
    }
    
    /// Store token metadata for tracking
    #[instrument(skip(self, claims))]
    async fn store_token_metadata(&self, claims: &JwtClaims) -> AuthResult<()> {
        let metadata_key = format!("token:meta:{}", claims.jti);
        let metadata = serde_json::json!({
            "user_id": claims.sub,
            "token_type": claims.token_type,
            "issued_at": claims.iat,
            "expires_at": claims.exp,
            "session_id": claims.session_id,
        });
        
        let ttl_seconds = (claims.exp - Utc::now().timestamp()).max(0) as u64;
        
        redis::cmd("SETEX")
            .arg(&metadata_key)
            .arg(ttl_seconds)
            .arg(metadata.to_string())
            .query_async::<ConnectionManager, ()>(&mut self.redis.clone())
            .await
            .map_err(|e| {
                error!("Failed to store token metadata: {:?}", e);
                ApiError::InternalServerError("Token storage failed".to_string())
            })?;
        
        Ok(())
    }
    
    /// Get user's active tokens
    #[instrument(skip(self))]
    pub async fn get_user_active_tokens(&self, user_id: Uuid) -> AuthResult<Vec<TokenInfo>> {
        let pattern = format!("token:meta:*");
        
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async::<ConnectionManager, Vec<String>>(&mut self.redis.clone())
            .await
            .map_err(|e| {
                error!("Failed to get active tokens: {:?}", e);
                ApiError::InternalServerError("Token retrieval failed".to_string())
            })?;
        
        let mut tokens = Vec::new();
        
        for key in keys {
            if let Ok(metadata_str) = redis::cmd("GET")
                .arg(&key)
                .query_async::<ConnectionManager, String>(&mut self.redis.clone())
                .await
            {
                if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_str) {
                    if let Some(token_user_id) = metadata.get("user_id") {
                        if token_user_id.as_str() == Some(&user_id.to_string()) {
                            tokens.push(TokenInfo {
                                jti: key.trim_start_matches("token:meta:").parse().unwrap_or_default(),
                                token_type: metadata.get("token_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown").to_string(),
                                issued_at: DateTime::from_timestamp(
                                    metadata.get("issued_at").and_then(|v| v.as_i64()).unwrap_or(0), 
                                    0
                                ).unwrap_or_default(),
                                expires_at: DateTime::from_timestamp(
                                    metadata.get("expires_at").and_then(|v| v.as_i64()).unwrap_or(0),
                                    0
                                ).unwrap_or_default(),
                                session_id: metadata.get("session_id")
                                    .and_then(|v| v.as_str())
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or_default(),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(tokens)
    }
    
    /// Revoke all tokens for a user
    #[instrument(skip(self))]
    pub async fn revoke_all_user_tokens(&self, user_id: Uuid) -> AuthResult<()> {
        debug!("Revoking all tokens for user: {}", user_id);
        
        let active_tokens = self.get_user_active_tokens(user_id).await?;
        
        for token in active_tokens {
            let blacklist_key = format!("blacklist:token:{}", token.jti);
            let ttl_seconds = (token.expires_at.timestamp() - Utc::now().timestamp()).max(0) as u64;
            
            let _: () = redis::cmd("SETEX")
                .arg(&blacklist_key)
                .arg(ttl_seconds)
                .arg("1")
                .query_async(&mut self.redis.clone())
                .await
                .map_err(|e| {
                    error!("Failed to blacklist token {}: {:?}", token.jti, e);
                    ApiError::InternalServerError("Token revocation failed".to_string())
                })?;
        }
        
        debug!("All tokens revoked for user: {}", user_id);
        Ok(())
    }
}

/// Token information for management
#[derive(Debug, Clone, Serialize)]
pub struct TokenInfo {
    pub jti: Uuid,
    pub token_type: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub session_id: Uuid,
}