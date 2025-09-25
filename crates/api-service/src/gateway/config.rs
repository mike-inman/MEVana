use std::env;

/// Configuration values required for the forthcoming gateway feature set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    pub gateway_port: u16,
    pub kafka_broker: String,
    pub prices_topic: String,
    pub swaps_topic: String,
    pub swaps_v2_topic: String,
    pub redis_url: String,
}

impl GatewayConfig {
    /// Create a configuration using environment variables with sensible defaults.
    pub fn from_env() -> Self {
        Self {
            gateway_port: env::var("MEVANA_WEBSOCKET_GATEWAY_PORT")
                .ok()
                .and_then(|port| port.parse().ok())
                .unwrap_or(DEFAULT_GATEWAY_PORT),
            kafka_broker: env::var("KAFKA_BROKER")
                .unwrap_or_else(|_| DEFAULT_KAFKA_BROKER.to_string()),
            prices_topic: env::var("GATEWAY_PRICES_TOPIC")
                .unwrap_or_else(|_| DEFAULT_PRICES_TOPIC.to_string()),
            swaps_topic: env::var("GATEWAY_SWAPS_TOPIC")
                .unwrap_or_else(|_| DEFAULT_SWAPS_TOPIC.to_string()),
            swaps_v2_topic: env::var("GATEWAY_SWAPS_V2_TOPIC")
                .unwrap_or_else(|_| DEFAULT_SWAPS_V2_TOPIC.to_string()),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string()),
        }
    }
}

pub const DEFAULT_GATEWAY_PORT: u16 = 8095;
pub const DEFAULT_KAFKA_BROKER: &str = "localhost:29092";
pub const DEFAULT_PRICES_TOPIC: &str = "prices.latest";
pub const DEFAULT_SWAPS_TOPIC: &str = "orca-swaps.parsed";
pub const DEFAULT_SWAPS_V2_TOPIC: &str = "orca-swaps.parsed.v2";
pub const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379/0";
