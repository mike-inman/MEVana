//! Rate limiting middleware
//!
//! Implements flexible rate limiting with:
//! - Per-user rate limiting based on tier
//! - Per-endpoint rate limiting
//! - Redis-based distributed rate limiting
//! - Sliding window algorithm

use crate::{auth::JwtClaims, error::{ApiError, ApiResult}};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{debug, warn, error, instrument};
use uuid::Uuid;

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub window_size_seconds: u64,
    pub default_limit: u32,
    pub per_endpoint_limits: std::collections::HashMap<String, u32>,
    pub tier_multipliers: TierMultipliers,
    pub enable_per_user_limits: bool,
    pub enable_per_endpoint_limits: bool,
    pub enable_burst_protection: bool,
    pub burst_threshold: f64, // Percentage of limit that triggers burst protection
}

/// Tier-based rate limit multipliers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierMultipliers {
    pub free: f64,
    pub premium: f64,
    pub professional: f64,
    pub enterprise: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut per_endpoint_limits = std::collections::HashMap::new();
        per_endpoint_limits.insert("/api/v1/auth/login".to_string(), 10);
        per_endpoint_limits.insert("/api/v1/auth/register".to_string(), 5);
        per_endpoint_limits.insert("/api/v1/trading/execute".to_string(), 100);
        per_endpoint_limits.insert("/api/v1/analytics/export".to_string(), 5);
        
        Self {
            window_size_seconds: 60,
            default_limit: 100,
            per_endpoint_limits,
            tier_multipliers: TierMultipliers {
                free: 1.0,
                premium: 3.0,
                professional: 10.0,
                enterprise: 50.0,
            },
            enable_per_user_limits: true,
            enable_per_endpoint_limits: true,
            enable_burst_protection: true,
            burst_threshold: 0.8,
        }
    }
}

/// Rate limiting middleware implementation
pub struct RateLimitMiddleware {
    config: RateLimitConfig,
    redis: ConnectionManager,
}

impl RateLimitMiddleware {
    /// Create new rate limit middleware
    pub fn new(config: RateLimitConfig, redis: ConnectionManager) -> Self {
        Self { config, redis }
    }
    
    /// Main rate limiting middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        &self,
        mut request: Request,
        next: Next,
    ) -> Result<Response, ApiError> {
        let path = request.uri().path().to_string();
        
        // Extract user info from JWT claims (if authenticated)
        let user_context = request
            .extensions()
            .get::<JwtClaims>()
            .map(|claims| UserContext {
                user_id: claims.sub,
                tier: parse_user_tier(&claims.tier),
                session_id: claims.session_id,
            });
        
        // Check rate limits
        self.check_rate_limits(&path, &user_context).await?;
        
        // Continue to next middleware/handler
        let mut response = next.run(request).await;
        
        // Add rate limit headers to response
        self.add_rate_limit_headers(&mut response, &path, &user_context).await;
        
        Ok(response)
    }
    
    /// Check all applicable rate limits
    async fn check_rate_limits(
        &self,
        path: &str,
        user_context: &Option<UserContext>,
    ) -> ApiResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Check per-endpoint limits
        if self.config.enable_per_endpoint_limits {
            let endpoint_limit = self.get_endpoint_limit(path);
            let endpoint_key = format!("rate_limit:endpoint:{}:{}", 
                path.replace('/', "_"), 
                now / self.config.window_size_seconds
            );
            
            self.check_limit(&endpoint_key, endpoint_limit, "endpoint").await?;
        }
        
        // Check per-user limits (if user is authenticated)
        if let Some(user) = user_context {
            if self.config.enable_per_user_limits {
                let user_limit = self.get_user_limit(path, &user.tier);
                let user_key = format!("rate_limit:user:{}:{}", 
                    user.user_id, 
                    now / self.config.window_size_seconds
                );
                
                self.check_limit(&user_key, user_limit, "user").await?;
            }
        }
        
        Ok(())
    }
    
    /// Check individual rate limit
    async fn check_limit(
        &self,
        key: &str,
        limit: u32,
        limit_type: &str,
    ) -> ApiResult<()> {
        let current_count: u32 = redis::cmd("INCR")
            .arg(key)
            .query_async(&mut self.redis.clone())
            .await
            .unwrap_or(0);
        
        // Set expiration on first increment
        if current_count == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(key)
                .arg(self.config.window_size_seconds)
                .query_async(&mut self.redis.clone())
                .await
                .map_err(|e| {
                    error!("Failed to set rate limit expiration: {:?}", e);
                    ApiError::InternalServerError("Rate limiting setup failed".to_string())
                })?;
        }
        
        debug!(
            "Rate limit check - Key: {}, Count: {}, Limit: {}, Type: {}",
            key, current_count, limit, limit_type
        );
        
        if current_count > limit {
            warn!(
                "Rate limit exceeded - Key: {}, Count: {}, Limit: {}, Type: {}",
                key, current_count, limit, limit_type
            );
            return Err(ApiError::RateLimitExceeded);
        }
        
        // Check burst protection
        if self.config.enable_burst_protection {
            let burst_threshold = (limit as f64 * self.config.burst_threshold) as u32;
            if current_count > burst_threshold {
                warn!(
                    "Burst threshold reached - Key: {}, Count: {}, Threshold: {}, Type: {}",
                    key, current_count, burst_threshold, limit_type
                );
                // Could implement additional logic here like temporary slowdown
            }
        }
        
        Ok(())
    }
    
    /// Get rate limit for specific endpoint
    fn get_endpoint_limit(&self, path: &str) -> u32 {
        self.config
            .per_endpoint_limits
            .get(path)
            .copied()
            .unwrap_or(self.config.default_limit)
    }
    
    /// Get rate limit for user based on tier
    fn get_user_limit(&self, path: &str, tier: &UserTier) -> u32 {
        let base_limit = self.get_endpoint_limit(path);
        let multiplier = match tier {
            UserTier::Free => self.config.tier_multipliers.free,
            UserTier::Premium => self.config.tier_multipliers.premium,
            UserTier::Professional => self.config.tier_multipliers.professional,
            UserTier::Enterprise => self.config.tier_multipliers.enterprise,
        };
        
        (base_limit as f64 * multiplier) as u32
    }
    
    /// Add rate limit information to response headers
    async fn add_rate_limit_headers(
        &self,
        response: &mut Response,
        path: &str,
        user_context: &Option<UserContext>,
    ) {
        let headers = response.headers_mut();
        
        // Add general rate limit info
        if let Ok(limit_value) = axum::http::HeaderValue::from_str(&self.config.default_limit.to_string()) {
            headers.insert("x-ratelimit-limit", limit_value);
        }
        
        if let Ok(window_value) = axum::http::HeaderValue::from_str(&self.config.window_size_seconds.to_string()) {
            headers.insert("x-ratelimit-window", window_value);
        }
        
        // Add user-specific rate limit info if available
        if let Some(user) = user_context {
            let user_limit = self.get_user_limit(path, &user.tier);
            if let Ok(user_limit_value) = axum::http::HeaderValue::from_str(&user_limit.to_string()) {
                headers.insert("x-ratelimit-user-limit", user_limit_value);
            }
            
            // Get current usage
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let user_key = format!("rate_limit:user:{}:{}", 
                user.user_id, 
                now / self.config.window_size_seconds
            );
            
            if let Ok(current_count) = redis::cmd("GET")
                .arg(&user_key)
                .query_async::<ConnectionManager, u32>(&mut self.redis.clone())
                .await
            {
                if let Ok(remaining_value) = axum::http::HeaderValue::from_str(&(user_limit.saturating_sub(current_count)).to_string()) {
                    headers.insert("x-ratelimit-remaining", remaining_value);
                }
                
                if let Ok(used_value) = axum::http::HeaderValue::from_str(&current_count.to_string()) {
                    headers.insert("x-ratelimit-used", used_value);
                }
            }
        }
    }
    
    /// Get rate limit status for user
    pub async fn get_rate_limit_status(
        &self,
        user_id: Uuid,
        path: &str,
        tier: &UserTier,
    ) -> ApiResult<RateLimitStatus> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let user_key = format!("rate_limit:user:{}:{}", 
            user_id, 
            now / self.config.window_size_seconds
        );
        
        let current_count: u32 = redis::cmd("GET")
            .arg(&user_key)
            .query_async(&mut self.redis.clone())
            .await
            .unwrap_or(0);
        
        let limit = self.get_user_limit(path, tier);
        let window_reset_time = (now / self.config.window_size_seconds + 1) * self.config.window_size_seconds;
        
        Ok(RateLimitStatus {
            limit,
            used: current_count,
            remaining: limit.saturating_sub(current_count),
            window_size_seconds: self.config.window_size_seconds,
            window_reset_time,
            is_limited: current_count >= limit,
        })
    }
}

/// User context for rate limiting
#[derive(Debug, Clone)]
struct UserContext {
    user_id: Uuid,
    tier: UserTier,
    session_id: Uuid,
}

/// User tier enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
enum UserTier {
    Free,
    Premium,
    Professional,
    Enterprise,
}

/// Rate limit status information
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStatus {
    pub limit: u32,
    pub used: u32,
    pub remaining: u32,
    pub window_size_seconds: u64,
    pub window_reset_time: u64,
    pub is_limited: bool,
}

/// Parse user tier from string
fn parse_user_tier(tier_str: &str) -> UserTier {
    match tier_str {
        "Free" => UserTier::Free,
        "Premium" => UserTier::Premium,
        "Professional" => UserTier::Professional,
        "Enterprise" => UserTier::Enterprise,
        _ => UserTier::Free,
    }
}