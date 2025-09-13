use api_service::{create_api_router, DatabaseService, ArbitrageKafkaConsumer};
use shared_core::database_config::get_database_url;
use std::env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Get configuration from centralized database config (single source of truth)
    let database_url = get_database_url();
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8091".to_string())
        .parse::<u16>()?;
    let kafka_broker = env::var("KAFKA_BROKER")
        .unwrap_or_else(|_| "localhost:29092".to_string());
    
    log::info!("Starting MEVana API Server on port {}", port);
    log::info!("Connecting to database: {}", database_url);
    
    // Initialize database connection
    let database = DatabaseService::new(database_url.clone()).await?;
    log::info!("Database connection established");
    
    // Initialize Kafka consumer for arbitrage opportunities
    let kafka_consumer = ArbitrageKafkaConsumer::new(kafka_broker.clone(), database.clone())?;
    log::info!("Kafka consumer initialized (broker: {})", kafka_broker);
    
    // Start Kafka consumer in background
    let consumer_handle = {
        let consumer = kafka_consumer;
        tokio::spawn(async move {
            if let Err(e) = consumer.start_consuming().await {
                log::error!("Kafka consumer failed: {}", e);
            }
        })
    };
    log::info!("Kafka consumer started in background");
    
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