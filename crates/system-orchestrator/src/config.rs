use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        Self {
            startup_timeout: Duration::from_secs(300), // 5 minutes total
            health_check_interval: Duration::from_secs(30),
            max_restart_attempts: 3,
            restart_delay: Duration::from_secs(10),
            infrastructure: InfrastructureConfig {
                docker_compose_path: "infrastructure/docker-compose.yml".to_string(),
                redis_url: "redis://localhost:6379".to_string(),
                kafka_bootstrap_servers: "localhost:9092".to_string(),
                postgres_url: "postgresql://mevana:mevana_pass@localhost:5432/mevana_db".to_string(),
                check_infrastructure: true,
            },
            services: vec![
                // Infrastructure Services (Tier 0)
                ServiceConfig {
                    name: "infrastructure".to_string(),
                    tier: ServiceTier::Infrastructure,
                    command: "docker-compose".to_string(),
                    args: vec!["-f".to_string(), "infrastructure/docker-compose.yml".to_string(), "up".to_string(), "-d".to_string()],
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
                    critical: true,
                },

                // Core Services (Tier 1)
                ServiceConfig {
                    name: "normalizer-service".to_string(),
                    tier: ServiceTier::Core,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "normalizer-service".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), "9901".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["infrastructure".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9901/health".to_string()),
                        port: Some(9901),
                        path: Some("/health".to_string()),
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(10),
                    critical: true,
                },

                ServiceConfig {
                    name: "edge-assembler".to_string(),
                    tier: ServiceTier::Core,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "edge-assembler".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), "9902".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["normalizer-service".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9902/health".to_string()),
                        port: Some(9902),
                        path: Some("/health".to_string()),
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(5),
                    critical: true,
                },

                ServiceConfig {
                    name: "arb-engine".to_string(),
                    tier: ServiceTier::Core,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "arb-engine".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), "9903".to_string()),
                        ("ARB_SIZING_MODE".to_string(), "adaptive".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["edge-assembler".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9903/health".to_string()),
                        port: Some(9903),
                        path: Some("/health".to_string()),
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(5),
                    critical: true,
                },

                ServiceConfig {
                    name: "executor-service".to_string(),
                    tier: ServiceTier::Core,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "executor-service".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), "9904".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["arb-engine".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9904/health".to_string()),
                        port: Some(9904),
                        path: Some("/health".to_string()),
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(5),
                    critical: true,
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
                    startup_delay: Duration::from_secs(5),
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
                    startup_delay: Duration::from_secs(5),
                    critical: false,
                },

                // HotPath Services (Tier 3) - Started after core system is stable
                ServiceConfig {
                    name: "hotpath-arb-engine".to_string(),
                    tier: ServiceTier::HotPath,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "hotpath_arb_engine".to_string(), "--bin".to_string(), "hotpath_arb_engine".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), "8088".to_string()),
                        ("GEYSER_ENDPOINT".to_string(), std::env::var("GEYSER_ENDPOINT").unwrap_or_else(|_| "https://grpc.ny.shyft.to".to_string())),
                        ("X_TOKEN".to_string(), std::env::var("X_TOKEN").unwrap_or_else(|_| "your_token_here".to_string())),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["executor-service".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:8088/performance".to_string()),
                        port: Some(8088),
                        path: Some("/performance".to_string()),
                        timeout: Duration::from_secs(10),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(15),
                    critical: true,
                },

                // Monitoring Services (Tier 4) - Started last
                ServiceConfig {
                    name: "monitoring-service".to_string(),
                    tier: ServiceTier::Monitoring,
                    command: "cargo".to_string(),
                    args: vec!["run".to_string(), "-p".to_string(), "monitoring-service".to_string()],
                    env: HashMap::from([
                        ("RUST_LOG".to_string(), "info".to_string()),
                        ("METRICS_PORT".to_string(), "9100".to_string()),
                    ]),
                    working_dir: Some(".".to_string()),
                    dependencies: vec!["hotpath-arb-engine".to_string()],
                    health_check: HealthCheckConfig {
                        endpoint: Some("http://localhost:9100/health".to_string()),
                        port: Some(9100),
                        path: Some("/health".to_string()),
                        timeout: Duration::from_secs(5),
                        retries: 3,
                        custom_check: None,
                    },
                    startup_delay: Duration::from_secs(10),
                    critical: false,
                },
            ],
        }
    }
}