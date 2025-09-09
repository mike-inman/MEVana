use anyhow::Result;
use log::{error, info, warn};
use prometheus::{Encoder, IntCounter, IntGauge};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::config::OrchestratorConfig;
use crate::health_checker::HealthChecker;
use crate::service_manager::ServiceManager;
use crate::startup_sequence::StartupSequence;

pub struct SystemOrchestrator {
    config: OrchestratorConfig,
    service_manager: ServiceManager,
    health_checker: HealthChecker,
    startup_time: Arc<RwLock<Option<Instant>>>,
    
    // Prometheus metrics
    services_total: IntGauge,
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
            "Total number of registered services"
        )?;
        registry.register(Box::new(services_total.clone()))?;
        
        let services_healthy = IntGauge::new(
            "mevana_services_healthy",
            "Number of healthy services"
        )?;
        registry.register(Box::new(services_healthy.clone()))?;
        
        let services_unhealthy = IntGauge::new(
            "mevana_services_unhealthy", 
            "Number of unhealthy services"
        )?;
        registry.register(Box::new(services_unhealthy.clone()))?;
        
        let system_restarts = IntCounter::new(
            "mevana_system_restarts_total",
            "Total number of system restarts"
        )?;
        registry.register(Box::new(system_restarts.clone()))?;

        services_total.set(config.services.len() as i64);

        Ok(Self {
            config,
            service_manager: ServiceManager::new(),
            health_checker: HealthChecker::new(),
            startup_time: Arc::new(RwLock::new(None)),
            services_total,
            services_healthy,
            services_unhealthy,
            system_restarts,
        })
    }

    pub async fn start_system(&mut self) -> Result<()> {
        let start_time = Instant::now();
        *self.startup_time.write().await = Some(start_time);

        info!("🚀 Starting MEVana Trading System");
        info!("📊 Total services to start: {}", self.config.services.len());

        // Check infrastructure if enabled
        if self.config.infrastructure.check_infrastructure {
            info!("🔍 Checking infrastructure services before startup");
            match self.health_checker.check_infrastructure_services().await {
                Ok(_) => info!("✅ Infrastructure services are healthy"),
                Err(e) => {
                    warn!("⚠️  Infrastructure health check failed: {}", e);
                    info!("🏗️  Starting infrastructure services...");
                }
            }
        }

        // Execute startup sequence
        let startup_sequence = StartupSequence::new(
            self.config.clone(), 
            self.service_manager.clone()
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
        let health_checker = self.health_checker.clone();
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
                    match service_manager.check_service_health(&service.config.name).await {
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
            
            let metrics_route = warp::path("metrics")
                .and(warp::get())
                .map(|| {
                    let encoder = prometheus::TextEncoder::new();
                    let metric_families = prometheus::gather();
                    let mut buffer = Vec::new();
                    
                    match encoder.encode(&metric_families, &mut buffer) {
                        Ok(_) => {
                            warp::reply::with_status(
                                String::from_utf8_lossy(&buffer).to_string(),
                                warp::http::StatusCode::OK,
                            )
                        }
                        Err(_) => {
                            warp::reply::with_status(
                                "Failed to encode metrics".to_string(),
                                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                            )
                        }
                    }
                });

            let health_route = warp::path("health")
                .and(warp::get())
                .map(|| "OK");

            let status_route = warp::path("status")
                .and(warp::get())
                .map(|| {
                    warp::reply::json(&serde_json::json!({
                        "status": "running",
                        "service": "system-orchestrator",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }))
                });

            let routes = metrics_route
                .or(health_route)
                .or(status_route);

            let port = std::env::var("ORCHESTRATOR_METRICS_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(9999);

            info!("📊 System orchestrator metrics available at http://localhost:{}/metrics", port);
            info!("🏥 System orchestrator health check at http://localhost:{}/health", port);
            info!("📋 System orchestrator status at http://localhost:{}/status", port);
            
            warp::serve(routes)
                .run(([0, 0, 0, 0], port))
                .await;
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
                    match self.service_manager.get_service_status(&service.config.name).await {
                        Some(crate::service_manager::ServiceStatus::Crashed) => {
                            error!("🚨 Critical service '{}' has crashed!", service.config.name);
                            
                            // Attempt to restart
                            info!("🔄 Attempting to restart critical service '{}'", service.config.name);
                            if let Err(e) = self.service_manager.restart_service(&service.config.name).await {
                                error!("❌ Failed to restart critical service '{}': {}", service.config.name, e);
                                
                                // If multiple critical services are down, consider system restart
                                let crashed_critical_services = self.count_crashed_critical_services().await;
                                if crashed_critical_services >= 2 {
                                    error!("🚨 Multiple critical services crashed, initiating system restart");
                                    return self.restart_system().await;
                                }
                            } else {
                                info!("✅ Critical service '{}' restarted successfully", service.config.name);
                            }
                        }
                        Some(crate::service_manager::ServiceStatus::Unhealthy) => {
                            warn!("⚠️  Critical service '{}' is unhealthy", service.config.name);
                        }
                        _ => {} // Service is OK
                    }
                }
            }
        }
    }

    async fn count_crashed_critical_services(&self) -> usize {
        let services = self.service_manager.list_services().await;
        services.iter()
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
}