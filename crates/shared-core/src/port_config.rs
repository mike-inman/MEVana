use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;

/// Centralized port configuration manager
/// Automatically reads from config/ports.toml and applies environment overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    pub infrastructure: InfrastructurePorts,
    pub trading_services: TradingServicePorts,
    pub supporting_services: SupportingServicePorts,
    pub development: DevelopmentPorts,
    pub future_services: FutureServicePorts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructurePorts {
    pub zookeeper: u16,
    pub kafka_internal: u16,
    pub kafka_external: u16,
    pub redis: u16,
    pub postgres_internal: u16,
    pub postgres_external: u16,
    pub prometheus: u16,
    pub grafana: u16,
    pub kafka_ui: u16,
    pub redis_ui_internal: u16,
    pub redis_ui_external: u16,
    pub adminer: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingServicePorts {
    pub normalizer_service: u16,
    pub edge_assembler: u16,
    pub arb_engine_api: u16,
    pub arb_engine_metrics: u16,
    pub executor_service: u16,
    pub hotpath_arb_engine: u16,
    pub hotpath_metrics: u16,
    pub api_service: u16,
    pub monitoring_service: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportingServicePorts {
    pub discovery_service: u16,
    pub token_registry_service: u16,
    pub wallet_vault: u16,
    pub system_orchestrator: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentPorts {
    pub test_api: u16,
    pub test_metrics: u16,
    pub dev_hotpath: u16,
    pub debug_normalizer: u16,
    pub debug_edge: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureServicePorts {
    pub web_ui_dev: u16,
    pub web_ui_prod: u16,
    pub auth_gateway: u16,
    pub websocket_gateway: u16,
    pub stream_aggregator: u16,
    pub analytics_api: u16,
}

impl PortConfig {
    /// Load port configuration from config/ports.toml with environment overrides
    pub fn load() -> Result<Self> {
        // Try to load from config file
        let config_path = Path::new("config/ports.toml");
        let mut config = if config_path.exists() {
            let config_str =
                std::fs::read_to_string(config_path).context("Failed to read config/ports.toml")?;
            toml::from_str::<PortConfig>(&config_str)
                .context("Failed to parse config/ports.toml")?
        } else {
            // Fallback to default configuration
            Self::default_config()
        };

        // Apply environment variable overrides
        config.apply_environment_overrides()?;

        Ok(config)
    }

    fn default_config() -> Self {
        Self {
            infrastructure: InfrastructurePorts {
                zookeeper: 2181,
                kafka_internal: 9092,
                kafka_external: 29092,
                redis: 6379,
                postgres_internal: 5432,
                postgres_external: 5433,
                prometheus: 9090,
                grafana: 3000,
                kafka_ui: 8080,
                redis_ui_internal: 8081,
                redis_ui_external: 8083,
                adminer: 8082,
            },
            trading_services: TradingServicePorts {
                normalizer_service: 9901,
                edge_assembler: 9902,
                arb_engine_api: 8085,
                arb_engine_metrics: 9903,
                executor_service: 9904,
                hotpath_arb_engine: 8088,
                hotpath_metrics: 8087, // Fixed: Different port than hotpath_arb_engine
                api_service: 8092,
                monitoring_service: 8091,
            },
            supporting_services: SupportingServicePorts {
                discovery_service: 9102,
                token_registry_service: 9898,
                wallet_vault: 8090,
                system_orchestrator: 9999,
            },
            development: DevelopmentPorts {
                test_api: 8093,
                test_metrics: 9105,
                dev_hotpath: 8094,
                debug_normalizer: 9911,
                debug_edge: 9912,
            },
            future_services: FutureServicePorts {
                web_ui_dev: 3200,
                web_ui_prod: 3100,
                auth_gateway: 3001,
                websocket_gateway: 8095,
                stream_aggregator: 8096,
                analytics_api: 8097,
            },
        }
    }

    fn apply_environment_overrides(&mut self) -> Result<()> {
        // Standard MEVANA_* environment variables
        if let Ok(port) = env::var("MEVANA_HOTPATH_METRICS_PORT") {
            self.trading_services.hotpath_arb_engine = port
                .parse()
                .context("Invalid MEVANA_HOTPATH_METRICS_PORT")?;
        }

        if let Ok(port) = env::var("MEVANA_ARB_ENGINE_API_PORT") {
            self.trading_services.arb_engine_api =
                port.parse().context("Invalid MEVANA_ARB_ENGINE_API_PORT")?;
        }

        if let Ok(port) = env::var("MEVANA_API_SERVICE_HTTP_PORT") {
            self.trading_services.api_service = port
                .parse()
                .context("Invalid MEVANA_API_SERVICE_HTTP_PORT")?;
        }

        if let Ok(port) = env::var("MEVANA_NORMALIZER_METRICS_PORT") {
            self.trading_services.normalizer_service = port
                .parse()
                .context("Invalid MEVANA_NORMALIZER_METRICS_PORT")?;
        }

        if let Ok(port) = env::var("MEVANA_EDGE_ASSEMBLER_METRICS_PORT") {
            self.trading_services.edge_assembler = port
                .parse()
                .context("Invalid MEVANA_EDGE_ASSEMBLER_METRICS_PORT")?;
        }

        if let Ok(port) = env::var("MEVANA_EXECUTOR_METRICS_PORT") {
            self.trading_services.executor_service = port
                .parse()
                .context("Invalid MEVANA_EXECUTOR_METRICS_PORT")?;
        }

        // Legacy environment variables for backwards compatibility
        if let Ok(port) = env::var("METRICS_PORT") {
            // Used by hotpath_arb_engine - only override if MEVANA_* not set
            if env::var("MEVANA_HOTPATH_METRICS_PORT").is_err() {
                self.trading_services.hotpath_arb_engine =
                    port.parse().context("Invalid METRICS_PORT")?;
            }
        }

        Ok(())
    }

    /// Get port for a specific service by name
    pub fn get_port(&self, service_name: &str) -> Option<u16> {
        match service_name {
            // Trading services
            "hotpath_arb_engine" => Some(self.trading_services.hotpath_arb_engine),
            "arb_engine" => Some(self.trading_services.arb_engine_api),
            "api_service" => Some(self.trading_services.api_service),
            "normalizer_service" => Some(self.trading_services.normalizer_service),
            "edge_assembler" => Some(self.trading_services.edge_assembler),
            "executor_service" => Some(self.trading_services.executor_service),
            "monitoring_service" => Some(self.trading_services.monitoring_service),

            // Supporting services
            "discovery_service" => Some(self.supporting_services.discovery_service),
            "token_registry_service" => Some(self.supporting_services.token_registry_service),
            "wallet_vault" => Some(self.supporting_services.wallet_vault),
            "system_orchestrator" => Some(self.supporting_services.system_orchestrator),

            // Infrastructure
            "kafka" => Some(self.infrastructure.kafka_internal),
            "kafka_external" => Some(self.infrastructure.kafka_external),
            "redis" => Some(self.infrastructure.redis),
            "postgres" => Some(self.infrastructure.postgres_external),
            "prometheus" => Some(self.infrastructure.prometheus),
            "grafana" => Some(self.infrastructure.grafana),

            _ => None,
        }
    }

    /// Get health check URL for a service
    pub fn get_health_check_url(&self, service_name: &str) -> Option<String> {
        let port = self.get_port(service_name)?;

        let path = match service_name {
            "hotpath_arb_engine" => "/performance",
            "arb_engine" => "/metrics",
            "prometheus" => "/-/healthy",
            "grafana" => "/api/health",
            _ => "/health",
        };

        Some(format!("http://localhost:{}{}", port, path))
    }

    /// Validate that no ports conflict
    pub fn validate_no_conflicts(&self) -> Result<()> {
        let mut used_ports = HashMap::new();
        let mut conflicts = Vec::new();

        // Collect all ports with their service names
        let ports_to_check = vec![
            ("zookeeper", self.infrastructure.zookeeper),
            ("kafka_internal", self.infrastructure.kafka_internal),
            ("kafka_external", self.infrastructure.kafka_external),
            ("redis", self.infrastructure.redis),
            ("postgres_external", self.infrastructure.postgres_external),
            ("prometheus", self.infrastructure.prometheus),
            ("grafana", self.infrastructure.grafana),
            ("kafka_ui", self.infrastructure.kafka_ui),
            ("redis_ui_external", self.infrastructure.redis_ui_external),
            ("adminer", self.infrastructure.adminer),
            (
                "normalizer_service",
                self.trading_services.normalizer_service,
            ),
            ("edge_assembler", self.trading_services.edge_assembler),
            ("arb_engine_api", self.trading_services.arb_engine_api),
            ("executor_service", self.trading_services.executor_service),
            (
                "hotpath_arb_engine",
                self.trading_services.hotpath_arb_engine,
            ),
            ("api_service", self.trading_services.api_service),
            (
                "monitoring_service",
                self.trading_services.monitoring_service,
            ),
            (
                "discovery_service",
                self.supporting_services.discovery_service,
            ),
            (
                "token_registry_service",
                self.supporting_services.token_registry_service,
            ),
            ("wallet_vault", self.supporting_services.wallet_vault),
        ];

        for (service, port) in ports_to_check {
            if let Some(existing_service) = used_ports.get(&port) {
                conflicts.push(format!(
                    "Port {} used by both {} and {}",
                    port, existing_service, service
                ));
            } else {
                used_ports.insert(port, service);
            }
        }

        if !conflicts.is_empty() {
            return Err(anyhow::anyhow!(
                "Port conflicts detected:\n{}",
                conflicts.join("\n")
            ));
        }

        Ok(())
    }

    /// Generate environment variables for docker-compose
    pub fn to_environment_map(&self) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();

        // Infrastructure services
        env_vars.insert(
            "KAFKA_PORT".to_string(),
            self.infrastructure.kafka_internal.to_string(),
        );
        env_vars.insert(
            "REDIS_PORT".to_string(),
            self.infrastructure.redis.to_string(),
        );
        env_vars.insert(
            "POSTGRES_PORT".to_string(),
            self.infrastructure.postgres_external.to_string(),
        );

        // Trading services
        env_vars.insert(
            "HOTPATH_METRICS_PORT".to_string(),
            self.trading_services.hotpath_arb_engine.to_string(),
        );
        env_vars.insert(
            "ARB_ENGINE_PORT".to_string(),
            self.trading_services.arb_engine_api.to_string(),
        );
        env_vars.insert(
            "API_SERVICE_PORT".to_string(),
            self.trading_services.api_service.to_string(),
        );
        env_vars.insert(
            "NORMALIZER_PORT".to_string(),
            self.trading_services.normalizer_service.to_string(),
        );
        env_vars.insert(
            "EDGE_ASSEMBLER_PORT".to_string(),
            self.trading_services.edge_assembler.to_string(),
        );
        env_vars.insert(
            "EXECUTOR_PORT".to_string(),
            self.trading_services.executor_service.to_string(),
        );

        env_vars
    }

    /// Generate .env file content
    pub fn to_env_file(&self) -> String {
        let env_map = self.to_environment_map();
        let mut content = String::new();
        content.push_str("# MEVana Port Configuration - Auto-generated\n");
        content.push_str(&format!("# Generated at: {}\n\n", chrono::Utc::now()));

        for (key, value) in env_map.iter() {
            content.push_str(&format!("{}={}\n", key, value));
        }

        content
    }
}

// Global port config instance
lazy_static::lazy_static! {
    pub static ref PORT_CONFIG: PortConfig = PortConfig::load()
        .expect("Failed to load port configuration");
}

/// Convenience function to get a port for a service
pub fn get_service_port(service_name: &str) -> Option<u16> {
    PORT_CONFIG.get_port(service_name)
}

/// Convenience function to get a health check URL
pub fn get_health_check_url(service_name: &str) -> Option<String> {
    PORT_CONFIG.get_health_check_url(service_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_config_loading() {
        let config = PortConfig::load().expect("Should load config");
        assert_eq!(config.trading_services.hotpath_arb_engine, 8088);
        assert_eq!(config.infrastructure.kafka_internal, 9092);
    }

    #[test]
    fn test_no_port_conflicts() {
        let config = PortConfig::load().expect("Should load config");
        config
            .validate_no_conflicts()
            .expect("Should have no conflicts");
    }

    #[test]
    fn test_service_port_lookup() {
        let config = PortConfig::load().expect("Should load config");
        assert_eq!(config.get_port("hotpath_arb_engine"), Some(8088));
        assert_eq!(config.get_port("nonexistent"), None);
    }

    #[test]
    fn test_health_check_urls() {
        let config = PortConfig::load().expect("Should load config");
        assert_eq!(
            config.get_health_check_url("hotpath_arb_engine"),
            Some("http://localhost:8088/performance".to_string())
        );
        assert_eq!(
            config.get_health_check_url("api_service"),
            Some("http://localhost:8092/health".to_string())
        );
    }
}
