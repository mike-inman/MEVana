//! Metrics collection middleware
//!
//! Provides comprehensive metrics collection with:
//! - Request/response metrics
//! - Performance monitoring  
//! - Error tracking
//! - User activity metrics

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use prometheus::{
    Counter, Histogram, IntCounter, IntCounterVec, IntGauge, HistogramVec, 
    Registry, Encoder, TextEncoder, register_counter, register_histogram,
    register_int_counter, register_int_counter_vec, register_int_gauge,
    register_histogram_vec,
};
use std::{
    sync::OnceLock,
    time::Instant,
};
use tracing::{debug, error, instrument};
use uuid::Uuid;

/// Global metrics registry
static METRICS: OnceLock<ApiMetrics> = OnceLock::new();

/// API metrics collection
pub struct ApiMetrics {
    // Request metrics
    pub requests_total: IntCounterVec,
    pub request_duration: HistogramVec,
    pub request_size_bytes: Histogram,
    pub response_size_bytes: Histogram,
    
    // Error metrics
    pub errors_total: IntCounterVec,
    pub auth_failures_total: IntCounter,
    pub rate_limit_hits_total: IntCounter,
    
    // Performance metrics
    pub active_connections: IntGauge,
    pub database_query_duration: Histogram,
    pub cache_hits_total: IntCounter,
    pub cache_misses_total: IntCounter,
    
    // Business metrics
    pub user_registrations_total: IntCounter,
    pub trading_operations_total: IntCounterVec,
    pub wallet_operations_total: IntCounterVec,
    pub api_keys_created_total: IntCounter,
}

impl ApiMetrics {
    /// Initialize metrics
    pub fn new() -> Self {
        Self {
            requests_total: register_int_counter_vec!(
                "api_requests_total",
                "Total number of API requests",
                &["method", "path", "status"]
            ).unwrap(),
            
            request_duration: register_histogram_vec!(
                "api_request_duration_seconds",
                "Request duration in seconds",
                &["method", "path", "status"],
                vec![0.001, 0.01, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0]
            ).unwrap(),
            
            request_size_bytes: register_histogram!(
                "api_request_size_bytes",
                "Request size in bytes",
                vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0]
            ).unwrap(),
            
            response_size_bytes: register_histogram!(
                "api_response_size_bytes", 
                "Response size in bytes",
                vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0]
            ).unwrap(),
            
            errors_total: register_int_counter_vec!(
                "api_errors_total",
                "Total number of API errors",
                &["error_type", "path", "method"]
            ).unwrap(),
            
            auth_failures_total: register_int_counter!(
                "api_auth_failures_total",
                "Total number of authentication failures"
            ).unwrap(),
            
            rate_limit_hits_total: register_int_counter!(
                "api_rate_limit_hits_total",
                "Total number of rate limit hits"
            ).unwrap(),
            
            active_connections: register_int_gauge!(
                "api_active_connections",
                "Current number of active connections"
            ).unwrap(),
            
            database_query_duration: register_histogram!(
                "api_database_query_duration_seconds",
                "Database query duration in seconds",
                vec![0.001, 0.01, 0.1, 0.5, 1.0, 2.5, 5.0]
            ).unwrap(),
            
            cache_hits_total: register_int_counter!(
                "api_cache_hits_total",
                "Total number of cache hits"
            ).unwrap(),
            
            cache_misses_total: register_int_counter!(
                "api_cache_misses_total",
                "Total number of cache misses"
            ).unwrap(),
            
            user_registrations_total: register_int_counter!(
                "api_user_registrations_total",
                "Total number of user registrations"
            ).unwrap(),
            
            trading_operations_total: register_int_counter_vec!(
                "api_trading_operations_total",
                "Total number of trading operations",
                &["operation_type", "status"]
            ).unwrap(),
            
            wallet_operations_total: register_int_counter_vec!(
                "api_wallet_operations_total", 
                "Total number of wallet operations",
                &["operation_type", "status"]
            ).unwrap(),
            
            api_keys_created_total: register_int_counter!(
                "api_keys_created_total",
                "Total number of API keys created"
            ).unwrap(),
        }
    }
    
    /// Get global metrics instance
    pub fn global() -> &'static ApiMetrics {
        METRICS.get_or_init(Self::new)
    }
    
    /// Record request metrics
    pub fn record_request(
        &self,
        method: &str,
        path: &str,
        status: StatusCode,
        duration: std::time::Duration,
        request_size: Option<usize>,
        response_size: Option<usize>,
    ) {
        let status_str = status.as_u16().to_string();
        
        // Increment request counter
        self.requests_total
            .with_label_values(&[method, path, &status_str])
            .inc();
        
        // Record request duration
        self.request_duration
            .with_label_values(&[method, path, &status_str])
            .observe(duration.as_secs_f64());
        
        // Record request size
        if let Some(size) = request_size {
            self.request_size_bytes.observe(size as f64);
        }
        
        // Record response size
        if let Some(size) = response_size {
            self.response_size_bytes.observe(size as f64);
        }
    }
    
    /// Record error metrics
    pub fn record_error(&self, error_type: &str, path: &str, method: &str) {
        self.errors_total
            .with_label_values(&[error_type, path, method])
            .inc();
    }
    
    /// Record authentication failure
    pub fn record_auth_failure(&self) {
        self.auth_failures_total.inc();
    }
    
    /// Record rate limit hit
    pub fn record_rate_limit_hit(&self) {
        self.rate_limit_hits_total.inc();
    }
    
    /// Set active connections count
    pub fn set_active_connections(&self, count: i64) {
        self.active_connections.set(count);
    }
    
    /// Record database query duration
    pub fn record_database_query(&self, duration: std::time::Duration) {
        self.database_query_duration.observe(duration.as_secs_f64());
    }
    
    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits_total.inc();
    }
    
    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses_total.inc();
    }
    
    /// Record user registration
    pub fn record_user_registration(&self) {
        self.user_registrations_total.inc();
    }
    
    /// Record trading operation
    pub fn record_trading_operation(&self, operation_type: &str, success: bool) {
        let status = if success { "success" } else { "failure" };
        self.trading_operations_total
            .with_label_values(&[operation_type, status])
            .inc();
    }
    
    /// Record wallet operation
    pub fn record_wallet_operation(&self, operation_type: &str, success: bool) {
        let status = if success { "success" } else { "failure" };
        self.wallet_operations_total
            .with_label_values(&[operation_type, status])
            .inc();
    }
    
    /// Record API key creation
    pub fn record_api_key_created(&self) {
        self.api_keys_created_total.inc();
    }
    
    /// Get metrics as Prometheus text format
    pub fn export_metrics() -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

/// Metrics middleware implementation
pub struct MetricsMiddleware;

impl MetricsMiddleware {
    /// Main metrics middleware function
    #[instrument(skip_all)]
    pub async fn middleware(
        request: Request,
        next: Next,
    ) -> Response {
        let start_time = Instant::now();
        let method = request.method().to_string();
        let path = Self::sanitize_path(request.uri().path());
        
        // Increment active connections
        let metrics = ApiMetrics::global();
        metrics.active_connections.inc();
        
        // Estimate request size (headers + potential body)
        let request_size = Self::estimate_request_size(&request);
        
        // Process request
        let response = next.run(request).await;
        
        // Calculate duration and get response info
        let duration = start_time.elapsed();
        let status = response.status();
        let response_size = Self::estimate_response_size(&response);
        
        // Record metrics
        metrics.record_request(
            &method,
            &path,
            status,
            duration,
            request_size,
            response_size,
        );
        
        // Record errors if applicable
        if status.is_client_error() || status.is_server_error() {
            let error_type = match status {
                StatusCode::UNAUTHORIZED => "unauthorized",
                StatusCode::FORBIDDEN => "forbidden",
                StatusCode::NOT_FOUND => "not_found",
                StatusCode::TOO_MANY_REQUESTS => "rate_limited",
                StatusCode::INTERNAL_SERVER_ERROR => "internal_error",
                StatusCode::SERVICE_UNAVAILABLE => "service_unavailable",
                _ if status.is_client_error() => "client_error",
                _ if status.is_server_error() => "server_error",
                _ => "unknown_error",
            };
            
            metrics.record_error(error_type, &path, &method);
            
            // Record specific error types
            match status {
                StatusCode::UNAUTHORIZED => metrics.record_auth_failure(),
                StatusCode::TOO_MANY_REQUESTS => metrics.record_rate_limit_hit(),
                _ => {}
            }
        }
        
        // Decrement active connections
        metrics.active_connections.dec();
        
        debug!(
            "Request metrics recorded - Method: {}, Path: {}, Status: {}, Duration: {:?}",
            method, path, status.as_u16(), duration
        );
        
        response
    }
    
    /// Sanitize path for metrics (remove dynamic segments)
    fn sanitize_path(path: &str) -> String {
        // Replace UUIDs with placeholder
        let uuid_regex = regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
        let sanitized = uuid_regex.replace_all(path, ":id");
        
        // Replace other numeric IDs
        let id_regex = regex::Regex::new(r"/\d+(/|$)").unwrap();
        let sanitized = id_regex.replace_all(&sanitized, "/:id$1");
        
        sanitized.to_string()
    }
    
    /// Estimate request size (simplified)
    fn estimate_request_size(request: &Request) -> Option<usize> {
        let mut size = 0;
        
        // Add headers size
        for (name, value) in request.headers() {
            size += name.as_str().len() + value.len() + 4; // +4 for ": \r\n"
        }
        
        // Add method, path, version size
        size += request.method().as_str().len();
        size += request.uri().to_string().len();
        size += 10; // HTTP version and spaces
        
        Some(size)
    }
    
    /// Estimate response size (simplified)
    fn estimate_response_size(response: &Response) -> Option<usize> {
        let mut size = 0;
        
        // Add headers size
        for (name, value) in response.headers() {
            size += name.as_str().len() + value.len() + 4; // +4 for ": \r\n"
        }
        
        // Add status line size
        size += response.status().as_str().len() + 15; // "HTTP/1.1 " + status + "\r\n"
        
        // Note: Body size would require more complex handling in a real implementation
        
        Some(size)
    }
}