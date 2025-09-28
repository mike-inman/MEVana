use std::env;

/// Configuration values required for the forthcoming gateway feature set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    pub gateway_port: u16,
    pub kafka_broker: String,
    pub prices_topic: String,
    pub swaps_topic: String,
    pub swaps_v2_topic: String,
    pub opportunities_topic: String,
    pub execution_results_topic: String,
    pub bundles_topic: String,
    pub fills_topic: String,
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
            opportunities_topic: env::var("GATEWAY_OPPORTUNITIES_TOPIC")
                .unwrap_or_else(|_| DEFAULT_OPPORTUNITIES_TOPIC.to_string()),
            execution_results_topic: env::var("GATEWAY_EXECUTION_RESULTS_TOPIC")
                .unwrap_or_else(|_| DEFAULT_EXECUTION_RESULTS_TOPIC.to_string()),
            bundles_topic: env::var("GATEWAY_BUNDLES_TOPIC")
                .unwrap_or_else(|_| DEFAULT_BUNDLES_TOPIC.to_string()),
            fills_topic: env::var("GATEWAY_FILLS_TOPIC")
                .unwrap_or_else(|_| DEFAULT_FILLS_TOPIC.to_string()),
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string()),
        }
    }
}

pub const DEFAULT_GATEWAY_PORT: u16 = 8095;
pub const DEFAULT_KAFKA_BROKER: &str = "localhost:29092";
pub const DEFAULT_PRICES_TOPIC: &str = "prices.latest";
pub const DEFAULT_SWAPS_TOPIC: &str = "orca-swaps.parsed";
pub const DEFAULT_SWAPS_V2_TOPIC: &str = "orca-swaps.parsed.v2";
pub const DEFAULT_OPPORTUNITIES_TOPIC: &str = "alerts.arbitrage.opportunities";
pub const DEFAULT_EXECUTION_RESULTS_TOPIC: &str = "execution.results.v1";
pub const DEFAULT_BUNDLES_TOPIC: &str = DEFAULT_EXECUTION_RESULTS_TOPIC;
pub const DEFAULT_FILLS_TOPIC: &str = DEFAULT_EXECUTION_RESULTS_TOPIC;
pub const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379/0";

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static ENV_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn from_env_uses_defaults() {
        let _guard = ENV_GUARD.lock().unwrap();
        let _env = test_support::TempEnv::new(vec![
            ("MEVANA_WEBSOCKET_GATEWAY_PORT", None),
            ("KAFKA_BROKER", None),
            ("GATEWAY_PRICES_TOPIC", None),
            ("GATEWAY_SWAPS_TOPIC", None),
            ("GATEWAY_SWAPS_V2_TOPIC", None),
            ("GATEWAY_OPPORTUNITIES_TOPIC", None),
            ("GATEWAY_EXECUTION_RESULTS_TOPIC", None),
            ("GATEWAY_BUNDLES_TOPIC", None),
            ("GATEWAY_FILLS_TOPIC", None),
            ("REDIS_URL", None),
        ]);

        let config = GatewayConfig::from_env();
        assert_eq!(config.gateway_port, DEFAULT_GATEWAY_PORT);
        assert_eq!(config.kafka_broker, DEFAULT_KAFKA_BROKER);
        assert_eq!(config.prices_topic, DEFAULT_PRICES_TOPIC);
        assert_eq!(config.swaps_topic, DEFAULT_SWAPS_TOPIC);
        assert_eq!(config.swaps_v2_topic, DEFAULT_SWAPS_V2_TOPIC);
        assert_eq!(config.opportunities_topic, DEFAULT_OPPORTUNITIES_TOPIC);
        assert_eq!(
            config.execution_results_topic,
            DEFAULT_EXECUTION_RESULTS_TOPIC
        );
        assert_eq!(config.bundles_topic, DEFAULT_BUNDLES_TOPIC);
        assert_eq!(config.fills_topic, DEFAULT_FILLS_TOPIC);
        assert_eq!(config.redis_url, DEFAULT_REDIS_URL);
    }

    #[test]
    fn from_env_overrides_values() {
        let _guard = ENV_GUARD.lock().unwrap();
        let _env = test_support::TempEnv::new(vec![
            ("MEVANA_WEBSOCKET_GATEWAY_PORT", Some("9000")),
            ("KAFKA_BROKER", Some("kafka.internal:29092")),
            ("GATEWAY_PRICES_TOPIC", Some("custom.prices")),
            ("GATEWAY_SWAPS_TOPIC", Some("custom.swaps")),
            ("GATEWAY_SWAPS_V2_TOPIC", Some("custom.swaps.v2")),
            ("GATEWAY_OPPORTUNITIES_TOPIC", Some("custom.opps")),
            ("GATEWAY_EXECUTION_RESULTS_TOPIC", Some("custom.exec")),
            ("GATEWAY_BUNDLES_TOPIC", Some("custom.bundles")),
            ("GATEWAY_FILLS_TOPIC", Some("custom.fills")),
            ("REDIS_URL", Some("redis://example:6379/1")),
        ]);

        let config = GatewayConfig::from_env();
        assert_eq!(config.gateway_port, 9000);
        assert_eq!(config.kafka_broker, "kafka.internal:29092");
        assert_eq!(config.prices_topic, "custom.prices");
        assert_eq!(config.swaps_topic, "custom.swaps");
        assert_eq!(config.swaps_v2_topic, "custom.swaps.v2");
        assert_eq!(config.opportunities_topic, "custom.opps");
        assert_eq!(config.execution_results_topic, "custom.exec");
        assert_eq!(config.bundles_topic, "custom.bundles");
        assert_eq!(config.fills_topic, "custom.fills");
        assert_eq!(config.redis_url, "redis://example:6379/1");
    }

    mod test_support {
        use std::env;

        pub struct TempEnv {
            original: Vec<(String, Option<String>)>,
        }

        impl TempEnv {
            pub fn new(pairs: Vec<(&'static str, Option<&'static str>)>) -> Self {
                let mut original = Vec::with_capacity(pairs.len());
                for (key, value) in pairs {
                    original.push((key.to_string(), env::var(key).ok()));
                    match value {
                        Some(val) => env::set_var(key, val),
                        None => env::remove_var(key),
                    }
                }
                Self { original }
            }
        }

        impl Drop for TempEnv {
            fn drop(&mut self) {
                for (key, value) in self.original.drain(..) {
                    if let Some(value) = value {
                        env::set_var(key, value);
                    } else {
                        env::remove_var(key);
                    }
                }
            }
        }
    }
}
