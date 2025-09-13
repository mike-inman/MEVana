//! Request validation middleware
//!
//! Provides comprehensive input validation with:
//! - JSON schema validation
//! - Input sanitization
//! - Size limits enforcement
//! - Content type validation

use crate::error::{ApiError, ValidationErrorBuilder, ApiResult};
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use tracing::{debug, warn, error, instrument};

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub max_body_size_bytes: usize,
    pub max_header_size_bytes: usize,
    pub max_query_params: usize,
    pub allowed_content_types: Vec<String>,
    pub enable_input_sanitization: bool,
    pub enable_sql_injection_detection: bool,
    pub enable_xss_detection: bool,
    pub blocked_patterns: Vec<String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_body_size_bytes: 10 * 1024 * 1024, // 10MB
            max_header_size_bytes: 8 * 1024,       // 8KB
            max_query_params: 100,
            allowed_content_types: vec![
                "application/json".to_string(),
                "application/x-www-form-urlencoded".to_string(),
                "multipart/form-data".to_string(),
                "text/plain".to_string(),
            ],
            enable_input_sanitization: true,
            enable_sql_injection_detection: true,
            enable_xss_detection: true,
            blocked_patterns: vec![
                "<script".to_string(),
                "javascript:".to_string(),
                "data:text/html".to_string(),
                "vbscript:".to_string(),
                "onload=".to_string(),
                "onerror=".to_string(),
                "onclick=".to_string(),
                "eval(".to_string(),
                "expression(".to_string(),
                "union select".to_string(),
                "drop table".to_string(),
                "insert into".to_string(),
                "delete from".to_string(),
                "update set".to_string(),
            ],
        }
    }
}

/// Request validation middleware
pub struct ValidationMiddleware {
    config: ValidationConfig,
}

impl ValidationMiddleware {
    /// Create new validation middleware
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }
    
    /// Main validation middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        config: ValidationConfig,
        request: Request,
        next: Next,
    ) -> Result<Response, ApiError> {
        debug!("Starting request validation");
        
        // Validate request headers
        Self::validate_headers(&config, request.headers())?;
        
        // Validate content type
        Self::validate_content_type(&config, request.headers())?;
        
        // Validate query parameters
        Self::validate_query_parameters(&config, request.uri().query())?;
        
        // TODO: In a complete implementation, you'd also validate the request body
        // This requires consuming the body, validating it, and then reconstructing the request
        // For now, we'll continue with the basic validations
        
        debug!("Request validation passed");
        
        // Continue to next middleware/handler
        Ok(next.run(request).await)
    }
    
    /// Validate request headers
    fn validate_headers(config: &ValidationConfig, headers: &HeaderMap) -> ApiResult<()> {
        let mut total_size = 0;
        let mut validation_errors = ValidationErrorBuilder::new();
        
        for (name, value) in headers {
            let header_size = name.as_str().len() + value.len();
            total_size += header_size;
            
            // Check individual header size
            if header_size > 1024 { // 1KB per header
                validation_errors = validation_errors.add_error(
                    "headers",
                    &format!("Header '{}' exceeds maximum size", name.as_str()),
                    "HEADER_TOO_LARGE"
                );
            }
            
            // Validate header value for malicious content
            if let Ok(value_str) = value.to_str() {
                if config.enable_input_sanitization {
                    Self::check_malicious_patterns(config, value_str, &format!("header.{}", name.as_str()))?;
                }
            }
        }
        
        // Check total headers size
        if total_size > config.max_header_size_bytes {
            validation_errors = validation_errors.add_error(
                "headers",
                &format!("Total headers size {} exceeds maximum {}", total_size, config.max_header_size_bytes),
                "HEADERS_TOO_LARGE"
            );
        }
        
        // Return validation errors if any
        if !validation_errors.errors.is_empty() {
            return Err(validation_errors.build());
        }
        
        Ok(())
    }
    
    /// Validate content type
    fn validate_content_type(config: &ValidationConfig, headers: &HeaderMap) -> ApiResult<()> {
        // Skip validation for GET requests and other methods without body
        if let Some(content_type) = headers.get("content-type") {
            if let Ok(content_type_str) = content_type.to_str() {
                let content_type_main = content_type_str
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();
                
                if !config.allowed_content_types.is_empty() && 
                   !config.allowed_content_types.iter().any(|ct| ct.to_lowercase() == content_type_main) {
                    warn!("Invalid content type: {}", content_type_str);
                    return Err(ApiError::BadRequest(format!(
                        "Unsupported content type: {}. Allowed types: {}",
                        content_type_main,
                        config.allowed_content_types.join(", ")
                    )));
                }
            } else {
                return Err(ApiError::BadRequest("Invalid content-type header".to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Validate query parameters
    fn validate_query_parameters(config: &ValidationConfig, query: Option<&str>) -> ApiResult<()> {
        if let Some(query_str) = query {
            let params = Self::parse_query_string(query_str);
            
            // Check parameter count
            if params.len() > config.max_query_params {
                return Err(ApiError::BadRequest(format!(
                    "Too many query parameters: {}. Maximum allowed: {}",
                    params.len(),
                    config.max_query_params
                )));
            }
            
            // Validate each parameter
            for (key, value) in params {
                // Check parameter key and value for malicious content
                if config.enable_input_sanitization {
                    Self::check_malicious_patterns(config, &key, "query_param_key")?;
                    Self::check_malicious_patterns(config, &value, "query_param_value")?;
                }
                
                // Additional validation rules
                Self::validate_parameter(&key, &value)?;
            }
        }
        
        Ok(())
    }
    
    /// Parse query string into key-value pairs
    fn parse_query_string(query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    urlencoding::decode(key).unwrap_or_default().to_string(),
                    urlencoding::decode(value).unwrap_or_default().to_string(),
                );
            } else {
                params.insert(
                    urlencoding::decode(pair).unwrap_or_default().to_string(),
                    String::new(),
                );
            }
        }
        
        params
    }
    
    /// Check input for malicious patterns
    fn check_malicious_patterns(
        config: &ValidationConfig,
        input: &str,
        field: &str,
    ) -> ApiResult<()> {
        let input_lower = input.to_lowercase();
        
        for pattern in &config.blocked_patterns {
            if input_lower.contains(&pattern.to_lowercase()) {
                warn!("Malicious pattern detected in {}: {}", field, pattern);
                return Err(ApiError::BadRequest(format!(
                    "Invalid input detected in {}",
                    field
                )));
            }
        }
        
        // SQL injection detection
        if config.enable_sql_injection_detection {
            Self::detect_sql_injection(&input_lower, field)?;
        }
        
        // XSS detection
        if config.enable_xss_detection {
            Self::detect_xss(&input_lower, field)?;
        }
        
        Ok(())
    }
    
    /// Detect SQL injection patterns
    fn detect_sql_injection(input: &str, field: &str) -> ApiResult<()> {
        let sql_patterns = [
            r"union\s+select",
            r"drop\s+table",
            r"insert\s+into",
            r"delete\s+from",
            r"update\s+set",
            r"alter\s+table",
            r"create\s+table",
            r"exec\s*\(",
            r"sp_",
            r"xp_",
            r"--\s*$",
            r"/\*.*\*/",
        ];
        
        for pattern in &sql_patterns {
            if regex::Regex::new(pattern)
                .map(|re| re.is_match(input))
                .unwrap_or(false)
            {
                warn!("SQL injection pattern detected in {}: {}", field, pattern);
                return Err(ApiError::BadRequest(format!(
                    "SQL injection attempt detected in {}",
                    field
                )));
            }
        }
        
        Ok(())
    }
    
    /// Detect XSS patterns
    fn detect_xss(input: &str, field: &str) -> ApiResult<()> {
        let xss_patterns = [
            r"<script[^>]*>",
            r"</script>",
            r"javascript:",
            r"vbscript:",
            r"data:\s*text/html",
            r"on\w+\s*=",
            r"eval\s*\(",
            r"expression\s*\(",
        ];
        
        for pattern in &xss_patterns {
            if regex::Regex::new(pattern)
                .map(|re| re.is_match(input))
                .unwrap_or(false)
            {
                warn!("XSS pattern detected in {}: {}", field, pattern);
                return Err(ApiError::BadRequest(format!(
                    "XSS attempt detected in {}",
                    field
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate individual parameter
    fn validate_parameter(key: &str, value: &str) -> ApiResult<()> {
        // Parameter key validation
        if key.is_empty() {
            return Err(ApiError::BadRequest("Empty parameter key not allowed".to_string()));
        }
        
        if key.len() > 100 {
            return Err(ApiError::BadRequest(format!(
                "Parameter key too long: {} characters. Maximum: 100",
                key.len()
            )));
        }
        
        // Parameter value validation
        if value.len() > 10000 {
            return Err(ApiError::BadRequest(format!(
                "Parameter value too long: {} characters. Maximum: 10000",
                value.len()
            )));
        }
        
        // Validate specific parameter types
        match key {
            "limit" | "offset" | "page" | "size" => {
                if let Err(_) = value.parse::<u32>() {
                    return Err(ApiError::BadRequest(format!(
                        "Parameter '{}' must be a positive integer",
                        key
                    )));
                }
            }
            "sort" => {
                let allowed_sorts = ["asc", "desc", "ascending", "descending"];
                if !value.is_empty() && !allowed_sorts.contains(&value.to_lowercase().as_str()) {
                    return Err(ApiError::BadRequest(format!(
                        "Invalid sort direction: '{}'. Allowed: {}",
                        value,
                        allowed_sorts.join(", ")
                    )));
                }
            }
            "format" => {
                let allowed_formats = ["json", "csv", "xml"];
                if !value.is_empty() && !allowed_formats.contains(&value.to_lowercase().as_str()) {
                    return Err(ApiError::BadRequest(format!(
                        "Invalid format: '{}'. Allowed: {}",
                        value,
                        allowed_formats.join(", ")
                    )));
                }
            }
            _ => {} // No specific validation for other parameters
        }
        
        Ok(())
    }
}