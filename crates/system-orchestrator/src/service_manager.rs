use anyhow::{Context, Result};
use chrono;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use uuid::Uuid;

use crate::config::ServiceConfig;
use crate::health_checker::{HealthChecker, HealthStatus};

#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub id: Uuid,
    pub config: ServiceConfig,
    pub status: ServiceStatus,
    pub start_time: Option<Instant>,
    pub restart_count: u32,
    pub last_restart: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    NotStarted,
    Starting,
    Running,
    Healthy,
    Unhealthy,
    Crashed,
    Stopping,
    Stopped,
}

#[derive(Clone)]
pub struct ServiceManager {
    services: Arc<RwLock<HashMap<String, ServiceInstance>>>,
    processes: Arc<RwLock<HashMap<String, Child>>>,
    health_checker: HealthChecker,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
            health_checker: HealthChecker::new(),
        }
    }

    pub async fn register_service(&self, config: ServiceConfig) -> Result<()> {
        let service = ServiceInstance {
            id: Uuid::new_v4(),
            config: config.clone(),
            status: ServiceStatus::NotStarted,
            start_time: None,
            restart_count: 0,
            last_restart: None,
        };

        let mut services = self.services.write().await;
        services.insert(config.name.clone(), service);

        info!(
            "📝 Registered service: {} (tier: {:?})",
            config.name, config.tier
        );
        Ok(())
    }

    pub async fn start_service(&self, service_name: &str) -> Result<()> {
        self.start_service_with_options(service_name, false).await
    }

    pub async fn start_service_without_delay(&self, service_name: &str) -> Result<()> {
        self.start_service_with_options(service_name, true).await
    }

    async fn start_service_with_options(&self, service_name: &str, skip_delay: bool) -> Result<()> {
        info!("🚀 Starting service: {}", service_name);

        // Update service status to Starting
        {
            let mut services = self.services.write().await;
            if let Some(service) = services.get_mut(service_name) {
                service.status = ServiceStatus::Starting;
                service.start_time = Some(Instant::now());
            } else {
                return Err(anyhow::anyhow!("Service '{}' not found", service_name));
            }
        }

        // Get service config
        let config = {
            let services = self.services.read().await;
            services
                .get(service_name)
                .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", service_name))?
                .config
                .clone()
        };

        // Apply startup delay (unless skipped)
        if !skip_delay && !config.startup_delay.is_zero() {
            info!(
                "⏳ Applying startup delay for '{}': {:?}",
                service_name, config.startup_delay
            );
            tokio::time::sleep(config.startup_delay).await;
        } else if skip_delay && !config.startup_delay.is_zero() {
            info!(
                "⚡ Skipping startup delay for '{}' (infrastructure already healthy)",
                service_name
            );
        }

        // Start the process
        match self.spawn_service_process(&config).await {
            Ok(child) => {
                // Store the child process
                let mut processes = self.processes.write().await;
                processes.insert(service_name.to_string(), child);

                // Update service status
                let mut services = self.services.write().await;
                if let Some(service) = services.get_mut(service_name) {
                    service.status = ServiceStatus::Running;
                }

                info!("✅ Service '{}' started successfully", service_name);
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to start service '{}': {}", service_name, e);

                // Update service status to Crashed
                let mut services = self.services.write().await;
                if let Some(service) = services.get_mut(service_name) {
                    service.status = ServiceStatus::Crashed;
                }

                Err(e)
            }
        }
    }

    async fn spawn_service_process(&self, config: &ServiceConfig) -> Result<Child> {
        let mut cmd = Command::new(&config.command);

        // Set arguments
        cmd.args(&config.args);

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        // Configure stdio
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("🔧 Spawning process: {} {:?}", config.command, config.args);

        cmd.spawn()
            .with_context(|| format!("Failed to spawn process for service '{}'", config.name))
    }

    pub async fn stop_service(&self, service_name: &str) -> Result<()> {
        info!("🛑 Stopping service: {}", service_name);

        // Update service status
        {
            let mut services = self.services.write().await;
            if let Some(service) = services.get_mut(service_name) {
                service.status = ServiceStatus::Stopping;
            }
        }

        // Kill the process
        let mut processes = self.processes.write().await;
        if let Some(mut child) = processes.remove(service_name) {
            match child.kill().await {
                Ok(_) => info!("✅ Service '{}' stopped successfully", service_name),
                Err(e) => warn!("⚠️  Error stopping service '{}': {}", service_name, e),
            }
        } else {
            warn!(
                "⚠️  No running process found for service '{}'",
                service_name
            );
        }

        // Update service status
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_name) {
            service.status = ServiceStatus::Stopped;
        }

        Ok(())
    }

    pub async fn restart_service(&self, service_name: &str) -> Result<()> {
        info!("🔄 Restarting service: {}", service_name);

        // Check restart limits
        let should_restart = {
            let mut services = self.services.write().await;
            if let Some(service) = services.get_mut(service_name) {
                service.restart_count += 1;
                service.last_restart = Some(Instant::now());

                if service.restart_count > 3 {
                    // max_restart_attempts from config
                    error!(
                        "❌ Service '{}' has exceeded maximum restart attempts ({})",
                        service_name, service.restart_count
                    );
                    false
                } else {
                    true
                }
            } else {
                false
            }
        };

        if !should_restart {
            return Err(anyhow::anyhow!(
                "Service '{}' restart limit exceeded",
                service_name
            ));
        }

        // Stop then start the service
        self.stop_service(service_name).await?;

        // Wait a bit before restarting
        tokio::time::sleep(Duration::from_secs(5)).await;

        self.start_service(service_name).await?;

        info!("✅ Service '{}' restarted successfully", service_name);
        Ok(())
    }

    pub async fn check_service_health(&self, service_name: &str) -> Result<HealthStatus> {
        let config = {
            let services = self.services.read().await;
            services
                .get(service_name)
                .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", service_name))?
                .config
                .clone()
        };

        let health_status = self.health_checker.check_service_health(&config).await;

        // Update service status based on health check
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_name) {
            service.status = match health_status {
                HealthStatus::Healthy => ServiceStatus::Healthy,
                HealthStatus::Unknown => ServiceStatus::Running, // Assume running if unknown
                _ => ServiceStatus::Unhealthy,
            };
        }

        Ok(health_status)
    }

    pub async fn wait_for_service_healthy(
        &self,
        service_name: &str,
        max_wait: Duration,
    ) -> Result<()> {
        let config = {
            let services = self.services.read().await;
            services
                .get(service_name)
                .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", service_name))?
                .config
                .clone()
        };

        self.health_checker
            .wait_for_service_healthy(&config, max_wait)
            .await
    }

    pub async fn get_service_status(&self, service_name: &str) -> Option<ServiceStatus> {
        let services = self.services.read().await;
        services.get(service_name).map(|s| s.status.clone())
    }

    pub async fn list_services(&self) -> Vec<ServiceInstance> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }

    pub async fn monitor_processes(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));

        loop {
            interval.tick().await;

            let service_names: Vec<String> = {
                let processes = self.processes.read().await;
                processes.keys().cloned().collect()
            };

            for service_name in service_names {
                // Check if process is still running
                let is_running = {
                    let mut processes = self.processes.write().await;
                    if let Some(child) = processes.get_mut(&service_name) {
                        match child.try_wait() {
                            Ok(Some(_)) => {
                                // Process has exited
                                processes.remove(&service_name);
                                false
                            }
                            Ok(None) => true, // Still running
                            Err(e) => {
                                warn!(
                                    "⚠️  Error checking process status for '{}': {}",
                                    service_name, e
                                );
                                false
                            }
                        }
                    } else {
                        false
                    }
                };

                if !is_running {
                    warn!(
                        "⚠️  Process for service '{}' has stopped unexpectedly",
                        service_name
                    );

                    // Update service status
                    let mut services = self.services.write().await;
                    if let Some(service) = services.get_mut(&service_name) {
                        service.status = ServiceStatus::Crashed;

                        // Log the crash to central error log
                        self.log_service_crash(
                            &service_name,
                            &format!("Service process terminated unexpectedly"),
                        )
                        .await;

                        if service.config.critical {
                            error!("❌ Critical service '{}' has crashed", service_name);

                            // Attempt to restart critical services
                            drop(services); // Release the lock
                            if let Err(e) = self.restart_service(&service_name).await {
                                error!(
                                    "❌ Failed to restart critical service '{}': {}",
                                    service_name, e
                                );
                            }
                        } else {
                            service.status = ServiceStatus::Stopped;
                        }
                    }
                }
            }
        }
    }

    pub async fn shutdown_all_services(&self) -> Result<()> {
        info!("🛑 Shutting down all services...");

        let service_names: Vec<String> = {
            let services = self.services.read().await;
            services.keys().cloned().collect()
        };

        // Stop services in reverse tier order (monitoring -> hotpath -> supporting -> core -> infrastructure)
        let mut services_by_tier: Vec<_> = {
            let services = self.services.read().await;
            services.values().cloned().collect()
        };

        // Sort by tier in reverse order for shutdown
        services_by_tier.sort_by(|a, b| b.config.tier.cmp(&a.config.tier));

        for service in services_by_tier {
            if let Err(e) = self.stop_service(&service.config.name).await {
                error!("❌ Error stopping service '{}': {}", service.config.name, e);
            }
        }

        info!("✅ All services shutdown completed");
        Ok(())
    }

    /// Log service crash to central error log for monitoring
    async fn log_service_crash(&self, service_name: &str, error_msg: &str) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let log_entry = format!(
            "[{}] SERVICE_CRASH: {} - {}\n",
            timestamp, service_name, error_msg
        );

        // Write to central error log file
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open("mevana_service_errors.log")
            .await
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(log_entry.as_bytes()).await {
                    error!("Failed to write to error log: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to open error log file: {}", e);
            }
        }

        // Also log to console
        warn!("⚠️ SERVICE CRASH: {} - {}", service_name, error_msg);
    }
}
