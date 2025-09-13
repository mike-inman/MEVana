use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shared_core::port_config::{PortConfig, get_service_port, get_health_check_url};
use std::collections::HashMap;
use std::env;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub services: Vec<ServiceConfig>,
    pub startup_timeout: Duration,
    pub health_check_interval: Duration,
    pub max_restart_attempts: u32,
    pub restart_delay: Duration,
    pub infrastructure: InfrastructureConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub tier: ServiceTier,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
    pub dependencies: Vec<String>,
    pub health_check: HealthCheckConfig,
    pub startup_delay: Duration,
    pub critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ServiceTier {
    Infrastructure = 0,
    Core = 1,
    Supporting = 2,
    HotPath = 3,
    Monitoring = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub endpoint: Option<String>,
    pub port: Option<u16>,
    pub path: Option<String>,
    pub timeout: Duration,
    pub retries: u32,
    pub custom_check: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureConfig {
    pub docker_compose_path: String,
    pub redis_url: String,
    pub kafka_bootstrap_servers: String,
    pub postgres_url: String,
    pub check_infrastructure: bool,
}

impl OrchestratorConfig {
    pub async fn load() -> Result<Self> {
        // Try to load from config file first, otherwise use default
        if let Ok(config_str) = tokio::fs::read_to_string("orchestrator-config.toml").await {
            toml::from_str(&config_str).context("Failed to parse orchestrator config")
        } else {
            Ok(Self::default_config())
        }
    }

    fn default_config() -> Self {
        // Load centralized port configuration
        let port_config = PortConfig::load().unwrap_or_else(|e| {
            log::warn!("Failed to load port config: {}, using built-in defaults", e);
            // Create a basic default configuration
            use crate::config::*;
            PortConfig {
                infrastructure: shared_core::port_config::InfrastructurePorts {
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
                trading_services: shared_core::port_config::TradingServicePorts {
                    normalizer_service: 9901,
                    edge_assembler: 9902,
                    arb_engine_api: 8085,
                    arb_engine_metrics: 9903,
                    executor_service: 9904,
                    hotpath_arb_engine: 8088,
                    hotpath_metrics: 8087,
                    api_service: 8092,
                    monitoring_service: 8091,
                },
                supporting_services: shared_core::port_config::SupportingServicePorts {
                    discovery_service: 9102,
                    token_registry_service: 9898,
                    wallet_vault: 8090,
                    system_orchestrator: 9999,
                },
                development: shared_core::port_config::DevelopmentPorts {
                    test_api: 8093,
                    test_metrics: 9105,
                    dev_hotpath: 8094,
                    debug_normalizer: 9911,
                    debug_edge: 9912,
                },
                future_services: shared_core::port_config::FutureServicePorts {
                    web_ui_dev: 3200,
                    web_ui_prod: 3100,
                    auth_gateway: 3001,
                    websocket_gateway: 8095,
                    stream_aggregator: 8096,
                    analytics_api: 8097,
                },
            }
        });

        Self {
            startup_timeout: Duration::from_secs(300), // 5 minutes total
            health_check_interval: Duration::from_secs(30),
            max_restart_attempts: 1,  // Reduce restart attempts to stop endless loops
            restart_delay: Duration::from_secs(30), // Longer delay between restarts
            infrastructure: InfrastructureConfig {
                docker_compose_path: "infrastructure/docker-compose.yml".to_string(),
                redis_url: format!("redis://localhost:{}", port_config.infrastructure.redis),
                kafka_bootstrap_servers: format!("localhost:{}", port_config.infrastructure.kafka_internal),
                postgres_url: env::var("DATABASE_URL").expect("DATABASE_URL environment variable is required - no hardcoded fallbacks allowed"),
                check_infrastructure: true,
            },
            services: vec![
                // Infrastructure Services (Tier 0)
                ServiceConfig {
                    name: "infrastructure".to_string(),
                    tier: ServiceTier::Infrastructure,
                    command: "docker".to_string(),
                    args: vec!["compose".to_string(), "-f".to_string(), "infrastructure/docker-compose.yml".to_string(), "up".to_string(), "-d".to_string()],
                    env: HashMap::new(),
                    working_dir: Some(".".to_string()),
                    dependencies: vec![],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9090/api/v1/query?query=up".to_string()),
                        port: Some(9090),
                        path: Some("/api/v1/query?query=up".to_string()),
                        timeout: Duration::from_secs(10),
                        retries: 5,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(30),
                    critical: true,  // Only infrastructure is critical
                },

                // Core Services (Tier 1)
                ServiceConfig {
                    name: "normalizer-service".to_string(),
                    tier: ServiceTier::Supporting,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "normalizer-service".to_string(), "--bin".to_string(), "normalizer-service".to_string(), "--".to_string(), "pipe".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), port_config.trading_services.normalizer_service.to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: None, // Service doesn't implement health endpoint
                        port: None,
                        path: None,
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(2),
                    critical: false,  // Optional service - available for future use
                },

                ServiceConfig {
                    name: "edge-assembler".to_string(),
                    tier: ServiceTier::Supporting,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "edge-assembler".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), port_config.trading_services.edge_assembler.to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: None, // Service doesn't implement health endpoint
                        port: None,
                        path: None,
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(1),
                    critical: false,  // Optional service - available for future use
                },

                ServiceConfig {
                    name: "arb-engine".to_string(),
                    tier: ServiceTier::Supporting,
                    command: "cargo".to_string(),
                    args: vec!["check".to_string(), "-p".to_string(), "arb-engine".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), port_config.trading_services.arb_engine_metrics.to_string()),
                        ("ARB_SIZING_MODE".to_string(), "adaptive".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: None, // Service doesn't implement health endpoint  
                        port: None,
                        path: None,
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(1),
                    critical: false,  // Optional service - available for future use
                },

                ServiceConfig {
                    name: "executor-service".to_string(),
                    tier: ServiceTier::Supporting,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "executor-service".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), port_config.trading_services.executor_service.to_string()),
                        ("DATABASE_URL".to_string(), env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://mev:mevpassword@localhost:5433/mevdb".to_string())),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: None, // Service doesn't implement health endpoint
                        port: None,
                        path: None,
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(1),
                    critical: false,  // Optional service - available for future use
                },

                // Supporting Services (Tier 2)
                ServiceConfig {
                    name: "discovery-service".to_string(),
                    tier: ServiceTier::Supporting,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "discovery-service".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("DISCOVERY_METRICS_PORT".to_string(), "9102".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9102/health".to_string()),
                        port: Some(9102),
                        path: Some("/health".to_string()),
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(1),
                    critical: false,
                },

                ServiceConfig {
                    name: "token-registry-service".to_string(),
                    tier: ServiceTier::Supporting,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "token-registry-service".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("TOKEN_REGISTRY_PORT".to_string(), "9898".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9898/health".to_string()),
                        port: Some(9898),
                        path: Some("/health".to_string()),
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(1),
                    critical: false,
                },

                // HotPath Services (Tier 3) - Started after core system is stable
                ServiceConfig {
                    name: "hotpath-arb-engine".to_string(),
                    tier: ServiceTier::Supporting,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "hotpath_arb_engine".to_string(), "--bin".to_string(), "hotpath_arb_engine".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), port_config.trading_services.hotpath_arb_engine.to_string()),
                        ("GEYSER_ENDPOINT".to_string(), std::env::var("GEYSER_ENDPOINT").unwrap_or_else(|_| "https://grpc.ny.shyft.to".to_string())),
                        ("X_TOKEN".to_string(), std::env::var("X_TOKEN").unwrap_or_else(|_| "your_token_here".to_string())),
                        ("DATABASE_URL".to_string(), env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://mev:mevpassword@localhost:5433/mevdb".to_string())),
                        ("PAPER_TRADING".to_string(), "false".to_string()),
                        ("ORDERBOOK_HISTORY_ENABLED".to_string(), "true".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some(format!("http://localhost:{}/performance", port_config.trading_services.hotpath_arb_engine)),
                        port: Some(port_config.trading_services.hotpath_arb_engine),
                        path: Some("/performance".to_string()),
                        timeout: Duration::from_secs(10),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(2),
                    critical: false,  // Optional service - available for future use
                },

                // Monitoring Services (Tier 4) - Started last
                ServiceConfig {
                    name: "monitoring-service".to_string(),
                    tier: ServiceTier::Supporting,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "monitoring-service".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), "9100".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9100/health".to_string()),
                        port: Some(9100),
                        path: Some("/health".to_string()),
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(2),
                    critical: false,
                },
            ],
        }
    }
}