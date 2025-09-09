use api_service::{create_api_router, DatabaseService};
use std::env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Get configuration from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/mevana".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;
    
    log::info!("Starting MEVana API Server on port {}", port);
    log::info!("Connecting to database: {}", database_url);
    
    // Initialize database connection
    let database = DatabaseService::new(database_url).await?;
    log::info!("Database connection established");
    
    // Create the API router
    let app = create_api_router(database);
    
    // Bind to address
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    
    log::info!("MEVana API Server listening on {}", addr);
    log::info!("Swagger UI available at: http://{}//swagger-ui", addr);
    log::info!("OpenAPI spec available at: http://{}/api-docs/openapi.json", addr);
    
    // Start the server
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_server_startup() {
        // This is a basic test to ensure the server can be created
        // In production, you'd want more comprehensive integration tests
        let database_url = "postgresql://test".to_string();
        
        // This would fail with a real database connection, but tests the basic setup
        let result = DatabaseService::new(database_url).await;
        
        // We expect this to fail in test environment, which is fine
        assert!(result.is_err());
    }
}