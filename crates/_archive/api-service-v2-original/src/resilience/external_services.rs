//! External services resilience patterns
//!
//! Provides resilience for external API calls with:
//! - HTTP client resilience patterns
//! - Service discovery integration
//! - Load balancing and failover
//! - External service health monitoring

use crate::error::{ApiError, ApiResult};
use super::ResilienceService;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration, collections::HashMap};
use tracing::{debug, warn, error, instrument};
use uuid::Uuid;

/// Resilient HTTP client
pub struct ResilientHttpClient {
    client: Client,
    resilience_service: Arc<ResilienceService>,
    service_endpoints: HashMap<String, ServiceEndpoint>,
}

impl ResilientHttpClient {
    /// Create new resilient HTTP client
    pub fn new(
        resilience_service: Arc<ResilienceService>,
        service_endpoints: HashMap<String, ServiceEndpoint>,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .user_agent("MEVana-API/2.0")
            .build()
            .unwrap_or_else(|_| Client::new());
        
        Self {
            client,
            resilience_service,
            service_endpoints,
        }
    }
    
    /// Make GET request with resilience
    #[instrument(skip(self))]
    pub async fn get(&self, service_name: &str, path: &str) -> ApiResult<Response> {
        let url = self.build_url(service_name, path)?;
        
        self.resilience_service
            .execute_external_service_call(service_name, || {
                let client = self.client.clone();
                let url = url.clone();
                async move {
                    client.get(&url).send().await
                }
            })
            .await
    }
    
    /// Make POST request with resilience
    #[instrument(skip(self, body))]
    pub async fn post<T: Serialize>(
        &self,
        service_name: &str,
        path: &str,
        body: &T,
    ) -> ApiResult<Response> {
        let url = self.build_url(service_name, path)?;
        
        self.resilience_service
            .execute_external_service_call(service_name, || {
                let client = self.client.clone();
                let url = url.clone();
                let json_body = serde_json::to_string(body).unwrap_or_default();
                async move {
                    client
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .body(json_body)
                        .send()
                        .await
                }
            })
            .await
    }
    
    /// Make PUT request with resilience
    #[instrument(skip(self, body))]
    pub async fn put<T: Serialize>(
        &self,
        service_name: &str,
        path: &str,
        body: &T,
    ) -> ApiResult<Response> {
        let url = self.build_url(service_name, path)?;
        
        self.resilience_service
            .execute_external_service_call(service_name, || {
                let client = self.client.clone();
                let url = url.clone();
                let json_body = serde_json::to_string(body).unwrap_or_default();
                async move {
                    client
                        .put(&url)
                        .header("Content-Type", "application/json")
                        .body(json_body)
                        .send()
                        .await
                }
            })
            .await
    }
    
    /// Make DELETE request with resilience
    #[instrument(skip(self))]
    pub async fn delete(&self, service_name: &str, path: &str) -> ApiResult<Response> {
        let url = self.build_url(service_name, path)?;
        
        self.resilience_service
            .execute_external_service_call(service_name, || {
                let client = self.client.clone();
                let url = url.clone();
                async move {
                    client.delete(&url).send().await
                }
            })
            .await
    }
    
    /// Build full URL for service endpoint
    fn build_url(&self, service_name: &str, path: &str) -> ApiResult<String> {
        let endpoint = self.service_endpoints
            .get(service_name)
            .ok_or_else(|| ApiError::ConfigurationError(format!("Unknown service: {}", service_name)))?;
        
        let base_url = endpoint.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        
        Ok(format!("{}/{}", base_url, path))
    }
    
    /// Health check for external service
    #[instrument(skip(self))]
    pub async fn health_check(&self, service_name: &str) -> ExternalServiceHealthInfo {
        let start_time = std::time::Instant::now();
        
        match self.service_endpoints.get(service_name) {
            Some(endpoint) => {
                let health_url = format!("{}/{}", 
                    endpoint.base_url.trim_end_matches('/'),
                    endpoint.health_path.trim_start_matches('/')
                );
                
                match self.client
                    .get(&health_url)
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await
                {
                    Ok(response) => {
                        let duration = start_time.elapsed();
                        let status = if response.status().is_success() {
                            resilience_framework::health::ServiceHealth::Healthy
                        } else {
                            resilience_framework::health::ServiceHealth::Degraded
                        };
                        
                        ExternalServiceHealthInfo {
                            service_name: service_name.to_string(),
                            status,
                            response_time: duration,
                            http_status: Some(response.status().as_u16()),
                            endpoint_url: health_url,
                            last_check: chrono::Utc::now(),
                            error_message: None,
                        }
                    }
                    Err(e) => {
                        let duration = start_time.elapsed();
                        
                        ExternalServiceHealthInfo {
                            service_name: service_name.to_string(),
                            status: resilience_framework::health::ServiceHealth::Unhealthy,
                            response_time: duration,
                            http_status: None,
                            endpoint_url: health_url,
                            last_check: chrono::Utc::now(),
                            error_message: Some(e.to_string()),
                        }
                    }
                }
            }
            None => {
                ExternalServiceHealthInfo {
                    service_name: service_name.to_string(),
                    status: resilience_framework::health::ServiceHealth::Unhealthy,
                    response_time: Duration::from_secs(0),
                    http_status: None,
                    endpoint_url: "unknown".to_string(),
                    last_check: chrono::Utc::now(),
                    error_message: Some("Service endpoint not configured".to_string()),
                }
            }
        }
    }
    
    /// Check health of all configured services
    pub async fn check_all_services(&self) -> HashMap<String, ExternalServiceHealthInfo> {
        let mut results = HashMap::new();
        
        for service_name in self.service_endpoints.keys() {
            let health = self.health_check(service_name).await;
            results.insert(service_name.clone(), health);
        }
        
        results
    }
}

/// External service endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub name: String,
    pub base_url: String,
    pub health_path: String,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

impl ServiceEndpoint {
    /// Create new service endpoint
    pub fn new(name: String, base_url: String) -> Self {
        Self {
            name,
            base_url,
            health_path: "/health".to_string(),
            timeout_seconds: 30,
            retry_attempts: 3,
        }
    }
    
    /// Set health check path
    pub fn with_health_path(mut self, path: String) -> Self {
        self.health_path = path;
        self
    }
    
    /// Set timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
    
    /// Set retry attempts
    pub fn with_retry_attempts(mut self, attempts: u32) -> Self {
        self.retry_attempts = attempts;
        self
    }
}

/// External service health information
#[derive(Debug, Clone, Serialize)]
pub struct ExternalServiceHealthInfo {
    pub service_name: String,
    pub status: resilience_framework::health::ServiceHealth,
    pub response_time: Duration,
    pub http_status: Option<u16>,
    pub endpoint_url: String,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub error_message: Option<String>,
}

/// Service registry for managing external service endpoints
pub struct ServiceRegistry {
    services: HashMap<String, ServiceEndpoint>,
}

impl ServiceRegistry {
    /// Create new service registry
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }
    
    /// Register a service endpoint
    pub fn register_service(&mut self, endpoint: ServiceEndpoint) {
        self.services.insert(endpoint.name.clone(), endpoint);
    }
    
    /// Get service endpoint
    pub fn get_service(&self, name: &str) -> Option<&ServiceEndpoint> {
        self.services.get(name)
    }
    
    /// Get all services
    pub fn get_all_services(&self) -> &HashMap<String, ServiceEndpoint> {
        &self.services
    }
    
    /// Remove service
    pub fn remove_service(&mut self, name: &str) -> Option<ServiceEndpoint> {
        self.services.remove(name)
    }
    
    /// Create registry with common MEVana services
    pub fn with_mevana_services() -> Self {
        let mut registry = Self::new();
        
        // Register common external services
        registry.register_service(
            ServiceEndpoint::new(
                "solana-rpc".to_string(),
                "https://api.mainnet-beta.solana.com".to_string(),
            ).with_health_path("/health".to_string())
        );
        
        registry.register_service(
            ServiceEndpoint::new(
                "jupiter-api".to_string(),
                "https://quote-api.jup.ag".to_string(),
            ).with_health_path("/v6/health".to_string())
        );
        
        registry.register_service(
            ServiceEndpoint::new(
                "coingecko-api".to_string(),
                "https://api.coingecko.com".to_string(),
            ).with_health_path("/api/v3/ping".to_string())
        );
        
        registry.register_service(
            ServiceEndpoint::new(
                "helius-api".to_string(),
                "https://api.helius.xyz".to_string(),
            ).with_health_path("/v0/health".to_string())
        );
        
        registry
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}