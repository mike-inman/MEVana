use api_service::{
    build_gateway_router, create_api_router, ArbitrageKafkaConsumer, DatabaseService,
    GatewayConfig, GatewayConsumerHandles, GatewayContext,
};
use shared_core::database_config::get_database_url;
use std::{env, net::SocketAddr};
use tokio::{net::TcpListener, task::JoinHandle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Get configuration from centralized database config (single source of truth)
    let database_url = get_database_url();
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8091".to_string())
        .parse::<u16>()?;
    let kafka_broker = env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:29092".to_string());

    log::info!("Starting MEVana API Server on port {}", port);
    log::info!("Connecting to database: {}", database_url);

    // Initialize database connection
    let database = DatabaseService::new(database_url.clone()).await?;
    log::info!("Database connection established");

    // Initialize Kafka consumer for arbitrage opportunities
    let kafka_consumer = ArbitrageKafkaConsumer::new(kafka_broker.clone(), database.clone())?;
    log::info!("Kafka consumer initialized (broker: {})", kafka_broker);

    // Start Kafka consumer in background
    let _consumer_handle = {
        let consumer = kafka_consumer;
        tokio::spawn(async move {
            if let Err(e) = consumer.start_consuming().await {
                log::error!("Kafka consumer failed: {}", e);
            }
        })
    };
    log::info!("Kafka consumer started in background");

    let mut gateway_runtime: Option<GatewayRuntime> = None;
    let gateway_enabled = env_flag("MEVANA_GATEWAY_ENABLE", false);

    if gateway_enabled {
        let gateway_config = GatewayConfig::from_env();
        let gateway_context = GatewayContext::new(gateway_config.clone());
        fn assert_state<T: Clone + Send + Sync + 'static>() {}
        assert_state::<GatewayContext>();
        let gateway_router = build_gateway_router(gateway_context.clone());
        let gateway_addr: SocketAddr =
            format!("0.0.0.0:{}", gateway_config.gateway_port).parse()?;
        let gateway_listener = TcpListener::bind(gateway_addr).await?;
        let bound_addr = gateway_listener.local_addr()?;
        log::info!("Gateway server listening on {}", bound_addr);

        let gateway_consumers = gateway_context.start_kafka_consumers();
        let gateway_service = gateway_router.into_make_service_with_connect_info::<SocketAddr>();
        let gateway_task = tokio::spawn(async move {
            if let Err(err) = axum::serve(gateway_listener, gateway_service).await {
                log::error!("Gateway server terminated with error: {}", err);
            }
        });

        gateway_runtime = Some(GatewayRuntime::new(gateway_task, gateway_consumers));
    } else {
        log::info!("Gateway server disabled (set MEVANA_GATEWAY_ENABLE=true to enable)");
    }

    // Create the API router
    let app = create_api_router(database);

    // Bind to address
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    log::info!("MEVana API Server listening on {}", addr);
    log::info!("Swagger UI available at: http://{}//swagger-ui", addr);
    log::info!(
        "OpenAPI spec available at: http://{}/api-docs/openapi.json",
        addr
    );

    // Start the server
    let api_result = axum::serve(listener, app).await;

    if let Some(runtime) = gateway_runtime {
        runtime.shutdown().await;
    }

    api_result?;
    Ok(())
}

struct GatewayRuntime {
    task: JoinHandle<()>,
    consumers: GatewayConsumerHandles,
}

impl GatewayRuntime {
    fn new(task: JoinHandle<()>, consumers: GatewayConsumerHandles) -> Self {
        Self { task, consumers }
    }

    async fn shutdown(self) {
        self.task.abort();
        let _ = self.task.await;
        self.consumers.abort();
    }
}

fn env_flag(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
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
