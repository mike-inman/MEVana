use anyhow::Result;
use rdkafka::{
    consumer::{StreamConsumer, Consumer},
    config::ClientConfig,
    message::Message,
};
use futures::StreamExt;
use log::{info, error, warn};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let kafka_broker = std::env::var("KAFKA_BROKER")
        .unwrap_or_else(|_| "localhost:29092".to_string());
    
    info!("🚀 Starting Kafka test consumer");
    info!("📡 Connecting to Kafka broker: {}", kafka_broker);
    
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_broker)
        .set("group.id", "api-service-test-consumer")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("auto.offset.reset", "latest") // Start from latest
        .set("enable.auto.commit", "true")
        .set("auto.commit.interval.ms", "1000")
        .create()?;

    consumer.subscribe(&["alerts.arbitrage.opportunities"])?;
    info!("✅ Subscribed to alerts.arbitrage.opportunities topic");

    let mut stream = consumer.stream();
    let mut message_count = 0;
    
    info!("🔍 Listening for arbitrage opportunities from hotpath engine...");
    
    while let Some(message) = stream.next().await {
        match message {
            Ok(borrowed_message) => {
                message_count += 1;
                if let Some(payload) = borrowed_message.payload() {
                    match std::str::from_utf8(payload) {
                        Ok(message_str) => {
                            info!("📨 Received arbitrage opportunity #{}: {}", message_count, message_str);
                            
                            // Try to parse as JSON for better formatting
                            match serde_json::from_str::<serde_json::Value>(message_str) {
                                Ok(json) => {
                                    if let Some(signature) = json.get("signature") {
                                        if let Some(profit_bps) = json.get("profit_bps") {
                                            if let Some(confidence) = json.get("confidence_score") {
                                                info!("💰 Opportunity: sig={}, profit={}bps, confidence={}", 
                                                      signature, profit_bps, confidence);
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    info!("⚠️  Could not parse as JSON: {}", message_str);
                                }
                            }
                        }
                        Err(e) => {
                            error!("❌ Failed to decode message as UTF-8: {}", e);
                        }
                    }
                } else {
                    warn!("⚠️  Received empty message payload");
                }
            }
            Err(e) => {
                error!("❌ Kafka consumer error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }

    warn!("🔄 Kafka consumer stream ended unexpectedly");
    Ok(())
}