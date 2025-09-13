//! Comprehensive testing framework
//!
//! Provides testing utilities and integration tests for:
//! - Authentication and authorization
//! - Security middleware  
//! - Database operations with resilience
//! - API endpoint testing
//! - Performance benchmarks

#[cfg(test)]
pub mod auth_tests;

#[cfg(test)] 
pub mod database_tests;

#[cfg(test)]
pub mod middleware_tests;

#[cfg(test)]
pub mod resilience_tests;

#[cfg(test)]
pub mod integration_tests;

use crate::{
    config::ApiConfig,
    services::ApiServices,
};
use std::sync::Arc;

/// Test utilities and fixtures
pub struct TestFixtures;

impl TestFixtures {
    /// Create test configuration
    pub fn test_config() -> ApiConfig {
        ApiConfig::development()
    }
    
    /// Create test services (mocked)
    pub async fn test_services() -> Arc<ApiServices> {
        let config = Self::test_config();
        // In a real implementation, this would use test databases and mocked services
        Arc::new(ApiServices::new(&config).await.expect("Failed to create test services"))
    }
}

/// Integration test macros and utilities would go here
#[macro_export]
macro_rules! test_endpoint {
    ($name:ident, $method:expr, $path:expr, $body:expr, $expected_status:expr) => {
        #[tokio::test]
        async fn $name() {
            let app = crate::create_app(crate::tests::TestFixtures::test_services().await.into());
            
            let response = axum_test::TestServer::new(app)
                .unwrap()
                .$method($path)
                .json(&$body)
                .await;
            
            assert_eq!(response.status_code(), $expected_status);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fixtures_creation() {
        let config = TestFixtures::test_config();
        assert_eq!(config.server.bind_address, "127.0.0.1:3000");
        
        // Test services creation would require actual implementation
        // let services = TestFixtures::test_services().await;
        // assert!(services.is_ok());
    }
}