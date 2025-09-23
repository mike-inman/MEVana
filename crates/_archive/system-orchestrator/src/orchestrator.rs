use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use prometheus::{Encoder, IntCounter, IntGauge};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::config::OrchestratorConfig;
use crate::service_manager::ServiceManager;
use crate::startup_sequence::StartupSequence;

pub struct SystemOrchestrator {
    config: OrchestratorConfig,
    service_manager: ServiceManager,
    startup_time: Arc<RwLock<Option<Instant>>>,
    infrastructure_was_already_healthy: bool,

    // Prometheus metrics
    services_healthy: IntGauge,
    services_unhealthy: IntGauge,
    system_restarts: IntCounter,
}

impl SystemOrchestrator {
    pub async fn new(config: OrchestratorConfig) -> Result<Self> {
        info!("🏗️  Initializing System Orchestrator");

        // Initialize Prometheus metrics
        let registry = prometheus::default_registry();

        let services_total = IntGauge::new(
            "mevana_services_total",
            "Total number of registered services",
        )?;
        registry.register(Box::new(services_total.clone()))?;

        let services_healthy =
            IntGauge::new("mevana_services_healthy", "Number of healthy services")?;
        registry.register(Box::new(services_healthy.clone()))?;

        let services_unhealthy =
            IntGauge::new("mevana_services_unhealthy", "Number of unhealthy services")?;
        registry.register(Box::new(services_unhealthy.clone()))?;

        let system_restarts = IntCounter::new(
            "mevana_system_restarts_total",
            "Total number of system restarts",
        )?;
        registry.register(Box::new(system_restarts.clone()))?;

        services_total.set(config.services.len() as i64);

        Ok(Self {
            config,
            service_manager: ServiceManager::new(),
            startup_time: Arc::new(RwLock::new(None)),
            infrastructure_was_already_healthy: false,
            services_healthy,
            services_unhealthy,
            system_restarts,
        })
    }

    pub async fn start_system(&mut self) -> Result<()> {
        let start_time = Instant::now();
        *self.startup_time.write().await = Some(start_time);

        info!("🚀 Starting MEVana Trading System");

        // Clean up any existing processes before starting
        info!("🧹 Cleaning up existing MEVana processes...");
        self.cleanup_existing_processes().await?;

        info!("📊 Total services to start: {}", self.config.services.len());

        // Start and wait for infrastructure if enabled
        if self.config.infrastructure.check_infrastructure {
            info!("🏗️  Starting infrastructure services...");

            // First check if infrastructure is already healthy
            info!("🔍 Checking if infrastructure is already healthy...");
            let infrastructure_healthy = self.check_infrastructure_health().await;

            if infrastructure_healthy {
                info!("✅ Infrastructure is already healthy - skipping startup delays!");
                self.infrastructure_was_already_healthy = true;
            } else {
                info!("⚠️  Infrastructure not healthy, starting services...");
                self.start_infrastructure().await?;

                info!("⏳ Waiting for infrastructure services to be ready...");
                self.wait_for_infrastructure().await?;
                self.infrastructure_was_already_healthy = false;
            }
        }

        // Execute startup sequence
        let startup_sequence = StartupSequence::new(
            self.config.clone(),
            self.service_manager.clone(),
            self.infrastructure_was_already_healthy,
        );

        startup_sequence.execute().await?;

        // Start background monitoring tasks
        self.start_monitoring_tasks().await;

        // Start metrics HTTP server
        self.start_metrics_server().await;

        let elapsed = start_time.elapsed();
        info!("🎉 MEVana system startup completed in {:?}", elapsed);
        info!("📈 System ready for high-frequency arbitrage trading!");

        // Keep running and monitoring
        self.run_monitoring_loop().await
    }

    async fn start_monitoring_tasks(&self) {
        info!("📊 Starting background monitoring tasks");

        // Start process monitoring
        let service_manager = self.service_manager.clone();
        tokio::spawn(async move {
            service_manager.monitor_processes().await;
        });

        // Start health monitoring
        let service_manager = self.service_manager.clone();
        let services_healthy = self.services_healthy.clone();
        let services_unhealthy = self.services_unhealthy.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let services = service_manager.list_services().await;
                let mut healthy_count = 0;
                let mut unhealthy_count = 0;

                for service in services {
                    match service_manager
                        .check_service_health(&service.config.name)
                        .await
                    {
                        Ok(crate::health_checker::HealthStatus::Healthy) => {
                            healthy_count += 1;
                        }
                        Ok(crate::health_checker::HealthStatus::Unknown) => {
                            healthy_count += 1; // Treat unknown as healthy
                        }
                        _ => {
                            unhealthy_count += 1;
                        }
                    }
                }

                services_healthy.set(healthy_count);
                services_unhealthy.set(unhealthy_count);
            }
        });
    }

    async fn start_metrics_server(&self) {
        info!("📊 Starting metrics HTTP server");

        tokio::spawn(async move {
            use warp::Filter;

            let metrics_route = warp::path("metrics").and(warp::get()).map(|| {
                let encoder = prometheus::TextEncoder::new();
                let metric_families = prometheus::gather();
                let mut buffer = Vec::new();

                match encoder.encode(&metric_families, &mut buffer) {
                    Ok(_) => warp::reply::with_status(
                        String::from_utf8_lossy(&buffer).to_string(),
                        warp::http::StatusCode::OK,
                    ),
                    Err(_) => warp::reply::with_status(
                        "Failed to encode metrics".to_string(),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ),
                }
            });

            let health_route = warp::path("health").and(warp::get()).map(|| "OK");

            let status_route = warp::path("status").and(warp::get()).map(|| {
                warp::reply::json(&serde_json::json!({
                    "status": "running",
                    "service": "system-orchestrator",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            });

            let routes = metrics_route.or(health_route).or(status_route);

            let port = std::env::var("ORCHESTRATOR_METRICS_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(9999);

            info!(
                "📊 System orchestrator metrics available at http://localhost:{}/metrics",
                port
            );
            info!(
                "🏥 System orchestrator health check at http://localhost:{}/health",
                port
            );
            info!(
                "📋 System orchestrator status at http://localhost:{}/status",
                port
            );

            warp::serve(routes).run(([0, 0, 0, 0], port)).await;
        });
    }

    async fn run_monitoring_loop(&mut self) -> Result<()> {
        info!("🔍 Starting continuous system monitoring");

        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute

        loop {
            interval.tick().await;

            // Check for critical service failures and handle them
            let services = self.service_manager.list_services().await;
            for service in services {
                if service.config.critical {
                    match self
                        .service_manager
                        .get_service_status(&service.config.name)
                        .await
                    {
                        Some(crate::service_manager::ServiceStatus::Crashed) => {
                            error!("🚨 Critical service '{}' has crashed!", service.config.name);

                            // Attempt to restart
                            info!(
                                "🔄 Attempting to restart critical service '{}'",
                                service.config.name
                            );
                            if let Err(e) = self
                                .service_manager
                                .restart_service(&service.config.name)
                                .await
                            {
                                error!(
                                    "❌ Failed to restart critical service '{}': {}",
                                    service.config.name, e
                                );

                                // If multiple critical services are down, consider system restart
                                let crashed_critical_services =
                                    self.count_crashed_critical_services().await;
                                if crashed_critical_services >= 2 {
                                    error!("🚨 Multiple critical services crashed, initiating system restart");
                                    return self.restart_system().await;
                                }
                            } else {
                                info!(
                                    "✅ Critical service '{}' restarted successfully",
                                    service.config.name
                                );
                            }
                        }
                        Some(crate::service_manager::ServiceStatus::Unhealthy) => {
                            warn!(
                                "⚠️  Critical service '{}' is unhealthy",
                                service.config.name
                            );
                        }
                        _ => {} // Service is OK
                    }
                }
            }
        }
    }

    async fn count_crashed_critical_services(&self) -> usize {
        let services = self.service_manager.list_services().await;
        services
            .iter()
            .filter(|s| s.config.critical)
            .filter(|s| matches!(s.status, crate::service_manager::ServiceStatus::Crashed))
            .count()
    }

    async fn restart_system(&mut self) -> Result<()> {
        error!("🔄 Initiating full system restart due to critical failures");
        self.system_restarts.inc();

        // Shutdown all services
        self.service_manager.shutdown_all_services().await?;

        // Wait for clean shutdown
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Restart the system - use Box::pin to avoid recursion issue
        Box::pin(self.start_system()).await
    }

    pub async fn shutdown_system(&self) -> Result<()> {
        info!("🛑 Initiating graceful system shutdown");

        // Shutdown all services
        self.service_manager.shutdown_all_services().await?;

        info!("✅ System shutdown completed successfully");
        Ok(())
    }

    /// Clean up any existing MEVana processes to ensure clean startup
    async fn cleanup_existing_processes(&self) -> Result<()> {
        info!("🔍 Checking for existing MEVana processes...");

        // Kill any existing system-orchestrator processes (except current)
        let output = Command::new("pkill")
            .args(["-f", "system-orchestrator"])
            .output()
            .await;

        match output {
            Ok(_) => info!("🧹 Cleaned up existing system-orchestrator processes"),
            Err(e) => warn!("⚠️  Failed to cleanup system-orchestrator processes: {}", e),
        }

        // Kill cargo run processes for MEVana services
        let services_to_kill = [
            "hotpath_arb_engine",
            "arb-engine",
            "executor-service",
            "normalizer-service",
            "edge-assembler",
            "discovery-service",
            "token-registry-service",
            "monitoring-service",
        ];

        for service in &services_to_kill {
            let output = Command::new("pkill")
                .arg("-f")
                .arg(format!("cargo run.*{}", service))
                .output()
                .await;

            match output {
                Ok(_) => info!("🧹 Cleaned up existing {} processes", service),
                Err(e) => warn!("⚠️  Failed to cleanup {} processes: {}", service, e),
            }
        }

        // Give processes time to terminate
        tokio::time::sleep(Duration::from_secs(3)).await;

        info!("✅ Process cleanup completed");
        Ok(())
    }

    /// Start infrastructure services using docker-compose
    async fn start_infrastructure(&self) -> Result<()> {
        info!("🚀 Starting infrastructure services with docker-compose...");

        // First, ensure docker network exists
        let _network_result = Command::new("docker")
            .args(["network", "create", "mevana_net"])
            .output()
            .await;

        // Start infrastructure using docker-compose
        let result = Command::new("docker")
            .args([
                "compose",
                "-f",
                "infrastructure/docker-compose.yml",
                "up",
                "-d",
            ])
            .current_dir(".")
            .output()
            .await
            .context("Failed to start infrastructure services")?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(anyhow::anyhow!("Infrastructure startup failed: {}", stderr));
        }

        info!("✅ Infrastructure services startup initiated");
        Ok(())
    }

    /// Wait for infrastructure services to be ready using docker-compose health checks
    async fn wait_for_infrastructure(&self) -> Result<()> {
        info!("⏳ Waiting for infrastructure services to be healthy...");

        let start_time = Instant::now();
        let timeout = Duration::from_secs(300); // 5 minutes
        let check_interval = Duration::from_secs(10);

        let mut last_status_log = Instant::now();

        loop {
            let elapsed = start_time.elapsed();
            if elapsed >= timeout {
                return Err(anyhow::anyhow!(
                    "Infrastructure readiness timeout after {:?}",
                    elapsed
                ));
            }

            // Log status every 30 seconds
            if last_status_log.elapsed() >= Duration::from_secs(30) {
                info!(
                    "⏳ Still waiting for infrastructure... ({:.0}s elapsed)",
                    elapsed.as_secs()
                );
                last_status_log = Instant::now();
            }

            let services = vec!["postgres", "kafka", "redis", "prometheus"];
            let mut all_healthy = true;
            let mut unhealthy_services = Vec::new();

            for service_name in &services {
                if !self.check_docker_service_health(service_name).await {
                    all_healthy = false;
                    unhealthy_services.push(service_name.to_string());
                }
            }

            if all_healthy {
                info!("✅ All infrastructure services are healthy!");
                return Ok(());
            }

            if !unhealthy_services.is_empty() {
                debug!(
                    "⚠️  Waiting for services: {}",
                    unhealthy_services.join(", ")
                );
            }

            tokio::time::sleep(check_interval).await;
        }
    }

    /// Check if all infrastructure services are currently healthy (quick check)
    async fn check_infrastructure_health(&self) -> bool {
        let services = vec!["postgres", "kafka", "redis", "prometheus"];

        for service_name in &services {
            if !self.check_docker_service_health(service_name).await {
                debug!("❌ Infrastructure service {} is not healthy", service_name);
                return false;
            }
        }

        true
    }

    /// Check if a docker service is healthy using docker-compose
    async fn check_docker_service_health(&self, service_name: &str) -> bool {
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            Command::new("docker")
                .args(["compose", "-f", "infrastructure/docker-compose.yml", "ps"])
                .arg(service_name)
                .current_dir(".")
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Check if the service is running and healthy
                    let healthy = stdout.lines().any(|line| {
                        line.contains(service_name)
                            && (line.contains("(healthy)") || line.contains("Up"))
                    });
                    if healthy {
                        debug!("✅ Service {} is healthy", service_name);
                    } else {
                        debug!("❌ Service {} is not healthy yet", service_name);
                        debug!("Docker ps output: {}", stdout);
                    }
                    healthy
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    debug!(
                        "Docker health check failed for {}: {}",
                        service_name, stderr
                    );
                    false
                }
            }
            Ok(Err(e)) => {
                debug!(
                    "Failed to execute docker command for {}: {}",
                    service_name, e
                );
                false
            }
            Err(_) => {
                debug!("Docker health check timed out for {}", service_name);
                false
            }
        }
    }
}
