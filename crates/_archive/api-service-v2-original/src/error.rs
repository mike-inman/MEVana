//! Error handling for the API service
//!
//! Provides comprehensive error types with:
//! - HTTP status code mapping
//! - User-friendly error messages
//! - Detailed error context
//! - Security-aware error responses

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Main API result type
pub type ApiResult<T> = Result<T, ApiError>;

/// Comprehensive API error types
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    // Authentication and authorization errors
    #[error("Authentication required")]
    Unauthorized(super::auth::AuthErrorType),
    
    #[error("Access denied: {0}")]
    Forbidden(String),
    
    // Validation errors
    #[error("Validation failed: {0}")]
    ValidationError(String),
    
    #[error("Invalid request: {0}")]
    BadRequest(String),
    
    // Resource errors
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Resource already exists: {0}")]
    Conflict(String),
    
    // Database errors
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Database connection failed")]
    DatabaseConnectionError,
    
    // External service errors
    #[error("External service error: {0}")]
    ExternalServiceError(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    // Configuration errors
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    // Rate limiting
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    // Generic errors
    #[error("Internal server error: {0}")]
    InternalServerError(String),
    
    #[error("Request timeout")]
    RequestTimeout,
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl ApiError {
    /// Get HTTP status code for error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::ValidationError(_) | ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            ApiError::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::DatabaseConnectionError => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            ApiError::DatabaseError(_) 
            | ApiError::ConfigurationError(_) 
            | ApiError::InternalServerError(_) 
            | ApiError::SerializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    
    /// Get error type for client
    pub fn error_type(&self) -> &'static str {
        match self {
            ApiError::Unauthorized(_) => "UNAUTHORIZED",
            ApiError::Forbidden(_) => "FORBIDDEN",
            ApiError::ValidationError(_) => "VALIDATION_ERROR",
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::Conflict(_) => "CONFLICT",
            ApiError::DatabaseError(_) => "DATABASE_ERROR",
            ApiError::DatabaseConnectionError => "DATABASE_CONNECTION_ERROR",
            ApiError::ExternalServiceError(_) => "EXTERNAL_SERVICE_ERROR",
            ApiError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            ApiError::ConfigurationError(_) => "CONFIGURATION_ERROR",
            ApiError::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            ApiError::InternalServerError(_) => "INTERNAL_SERVER_ERROR",
            ApiError::RequestTimeout => "REQUEST_TIMEOUT",
            ApiError::SerializationError(_) => "SERIALIZATION_ERROR",
        }
    }
    
    /// Get user-friendly error message (sanitized for security)
    pub fn user_message(&self) -> String {
        match self {
            ApiError::Unauthorized(_) => "Authentication required. Please log in.".to_string(),
            ApiError::Forbidden(_) => "You don't have permission to access this resource.".to_string(),
            ApiError::ValidationError(msg) => format!("Validation failed: {}", msg),
            ApiError::BadRequest(msg) => format!("Invalid request: {}", msg),
            ApiError::NotFound(_) => "The requested resource was not found.".to_string(),
            ApiError::Conflict(_) => "A conflict occurred. The resource may already exist.".to_string(),
            ApiError::RateLimitExceeded => "Too many requests. Please try again later.".to_string(),
            ApiError::RequestTimeout => "Request timed out. Please try again.".to_string(),
            ApiError::ServiceUnavailable(_) => "Service temporarily unavailable. Please try again later.".to_string(),
            
            // For security, don't expose internal details
            ApiError::DatabaseError(_) 
            | ApiError::DatabaseConnectionError
            | ApiError::ExternalServiceError(_)
            | ApiError::ConfigurationError(_)
            | ApiError::InternalServerError(_)
            | ApiError::SerializationError(_) => {
                "An internal error occurred. Please try again later.".to_string()
            }
        }
    }
    
    /// Check if error should be logged (internal errors should be logged)
    pub fn should_log(&self) -> bool {
        match self {
            ApiError::Unauthorized(_) 
            | ApiError::Forbidden(_)
            | ApiError::ValidationError(_)
            | ApiError::BadRequest(_)
            | ApiError::NotFound(_)
            | ApiError::Conflict(_)
            | ApiError::RateLimitExceeded => false,
            
            _ => true,
        }
    }
    
    /// Get log level for error
    pub fn log_level(&self) -> tracing::Level {
        match self {
            ApiError::RequestTimeout 
            | ApiError::ServiceUnavailable(_) => tracing::Level::WARN,
            
            ApiError::DatabaseError(_)
            | ApiError::DatabaseConnectionError
            | ApiError::ExternalServiceError(_)
            | ApiError::ConfigurationError(_)
            | ApiError::InternalServerError(_)
            | ApiError::SerializationError(_) => tracing::Level::ERROR,
            
            _ => tracing::Level::DEBUG,
        }
    }
}

/// Error response structure for API clients
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorDetail,
    pub request_id: Option<Uuid>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Detailed error information
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ErrorDetail {
    pub error_type: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(error: ApiError, request_id: Option<Uuid>) -> Self {
        Self {
            success: false,
            error: ErrorDetail {
                error_type: error.error_type().to_string(),
                message: error.user_message(),
                details: None,
            },
            request_id,
            timestamp: chrono::Utc::now(),
        }
    }
    
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.error.details = Some(details);
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        
        // Log error if appropriate
        if self.should_log() {
            match self.log_level() {
                tracing::Level::ERROR => tracing::error!("API Error: {:?}", self),
                tracing::Level::WARN => tracing::warn!("API Warning: {:?}", self),
                tracing::Level::DEBUG => tracing::debug!("API Debug: {:?}", self),
                _ => {}
            }
        }
        
        let error_response = ErrorResponse::new(self, None);
        
        (status_code, Json(error_response)).into_response()
    }
}

/// Validation error builder for detailed field-level errors
#[derive(Debug, Default)]
pub struct ValidationErrorBuilder {
    errors: Vec<FieldError>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FieldError {
    pub field: String,
    pub message: String,
    pub code: String,
}

impl ValidationErrorBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn add_error(mut self, field: &str, message: &str, code: &str) -> Self {
        self.errors.push(FieldError {
            field: field.to_string(),
            message: message.to_string(),
            code: code.to_string(),
        });
        self
    }
    
    pub fn build(self) -> ApiError {
        if self.errors.is_empty() {
            ApiError::ValidationError("Unknown validation error".to_string())
        } else {
            let summary = format!("Validation failed for {} field(s)", self.errors.len());
            ApiError::ValidationError(summary)
        }
    }
    
    pub fn into_response_with_details(self) -> Response {
        let error = self.build();
        let status_code = error.status_code();
        
        let error_response = ErrorResponse::new(error, None)
            .with_details(serde_json::json!({
                "field_errors": self.errors
            }));
        
        (status_code, Json(error_response)).into_response()
    }
}

// Implement conversions from common error types
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {:?}", err);
        
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".to_string()),
            sqlx::Error::Database(db_err) => {
                if let Some(constraint) = db_err.constraint() {
                    if constraint.contains("unique") || constraint.contains("pk") {
                        return ApiError::Conflict("Resource already exists".to_string());
                    }
                }
                ApiError::DatabaseError("Database operation failed".to_string())
            }
            sqlx::Error::PoolTimedOut => ApiError::ServiceUnavailable("Database pool exhausted".to_string()),
            sqlx::Error::Io(_) => ApiError::DatabaseConnectionError,
            _ => ApiError::DatabaseError("Database operation failed".to_string()),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!("Serialization error: {:?}", err);
        ApiError::SerializationError(err.to_string())
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        tracing::error!("Redis error: {:?}", err);
        ApiError::ServiceUnavailable("Cache service unavailable".to_string())
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let mut builder = ValidationErrorBuilder::new();
        
        for (field, field_errors) in errors.field_errors() {
            for error in field_errors {
                let message = error.message.as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| format!("Invalid value for field '{}'", field));
                
                let code = error.code.to_string();
                builder = builder.add_error(field, &message, &code);
            }
        }
        
        builder.build()
    }
}

/// Helper macro for creating API errors
#[macro_export]
macro_rules! api_error {
    (unauthorized) => {
        $crate::error::ApiError::Unauthorized($crate::auth::AuthErrorType::InvalidCredentials)
    };
    (forbidden, $msg:expr) => {
        $crate::error::ApiError::Forbidden($msg.to_string())
    };
    (not_found, $msg:expr) => {
        $crate::error::ApiError::NotFound($msg.to_string())
    };
    (bad_request, $msg:expr) => {
        $crate::error::ApiError::BadRequest($msg.to_string())
    };
    (conflict, $msg:expr) => {
        $crate::error::ApiError::Conflict($msg.to_string())
    };
    (internal, $msg:expr) => {
        $crate::error::ApiError::InternalServerError($msg.to_string())
    };
}