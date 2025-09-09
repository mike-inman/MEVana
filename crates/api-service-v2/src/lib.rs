//! # MEVana API Service v2
//!
//! Enterprise-grade API service with JWT authentication, circuit breakers,
//! and extensible architecture designed for high-frequency trading operations.
//!
//! ## Architecture Overview
//!
//! - **Security-First**: Every endpoint secured with JWT authentication
//! - **Resilient**: Circuit breakers and graceful degradation
//! - **Extensible**: Plugin-based architecture for easy feature addition
//! - **Observable**: Comprehensive metrics and distributed tracing
//! - **Testable**: Complete test coverage with benchmarks

pub mod auth;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod resilience;
pub mod routes;
pub mod services;
pub mod utils;

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
pub use auth::{ApiContext, JwtAuthService, Permission};
pub use config::ApiConfig;
pub use database::{DatabaseRepository, SqlxRepository};
pub use error::{ApiError, ApiResult};
pub use services::ApiServices;

use axum::{Extension, Router};
use std::sync::Arc;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
    request_id::RequestIdLayer,
    timeout::TimeoutLayer,
};
use std::time::Duration;
use tracing::info;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub services: Arc<ApiServices>,
    pub config: Arc<ApiConfig>,
}

/// Build the complete application router with all middleware
pub fn create_app(state: AppState) -> Router {
    let api_router = routes::create_api_routes()
        .layer(Extension(state.services.clone()))
        .layer(Extension(state.config.clone()));

    Router::new()
        // Health check endpoint (no auth required)
        .route("/health", axum::routing::get(handlers::health::health_check))
        
        // API routes (all require authentication)
        .nest("/api/v1", api_router)
        
        // OpenAPI documentation
        .merge(routes::create_docs_routes())
        
        // Global middleware stack (applied in reverse order)
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive()) // TODO: Configure for production
        .layer(RequestIdLayer)
        .layer(TraceLayer::new_for_http())
}

/// Initialize and start the API service
pub async fn run_service(config: ApiConfig) -> ApiResult<()> {
    info!("🚀 Initializing MEVana API Service v2");
    
    // Initialize services
    let services = Arc::new(ApiServices::new(&config).await?);
    
    let state = AppState {
        services,
        config: Arc::new(config.clone()),
    };
    
    // Build application
    let app = create_app(state);
    
    // Start server
    let listener = tokio::net::TcpListener::bind(&config.server.bind_address).await?;
    
    info!(
        "🌟 MEVana API Service v2 running on {}",
        config.server.bind_address
    );
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    
    Ok(())
}

/// Handle graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("🛑 Graceful shutdown initiated");
}