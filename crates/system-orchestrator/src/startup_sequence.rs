use anyhow::Result;
use log::{error, info, warn};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::config::{OrchestratorConfig, ServiceConfig, ServiceTier};
use crate::service_manager::ServiceManager;

pub struct StartupSequence {
    config: OrchestratorConfig,
    service_manager: ServiceManager,
}

impl StartupSequence {
    pub fn new(config: OrchestratorConfig, service_manager: ServiceManager) -> Self {
        Self {
            config,
            service_manager,
        }
    }

    pub async fn execute(&self) -> Result<()> {
        info!("🚀 Executing MEVana system startup sequence");
        
        // Register all services
        self.register_services().await?;
        
        // Start services by tier with dependency resolution
        self.start_services_by_tier().await?;
        
        // Verify system health
        self.verify_system_health().await?;
        
        info!("✅ MEVana system startup sequence completed successfully");
        Ok(())
    }

    async fn register_services(&self) -> Result<()> {
        info!("📝 Registering all services...");
        
        for service_config in &self.config.services {
            self.service_manager.register_service(service_config.clone()).await?;
        }
        
        info!("✅ All services registered");
        Ok(())
    }

    async fn start_services_by_tier(&self) -> Result<()> {
        info!("🏗️  Starting services by tier with dependency resolution");

        // Group services by tier
        let mut services_by_tier: HashMap<ServiceTier, Vec<ServiceConfig>> = HashMap::new();
        for service in &self.config.services {
            services_by_tier.entry(service.tier.clone()).or_default().push(service.clone());
        }

        // Start services tier by tier
        let tiers = vec![
            ServiceTier::Infrastructure,
            ServiceTier::Core,
            ServiceTier::Supporting,
            ServiceTier::HotPath,
            ServiceTier::Monitoring,
        ];

        for tier in tiers {
            if let Some(services) = services_by_tier.get(&tier) {
                info!("🚀 Starting {:?} tier services ({} services)", tier, services.len());
                self.start_tier_services(services, &tier).await?;
                info!("✅ {:?} tier services started successfully", tier);
            }
        }

        Ok(())
    }

    async fn start_tier_services(&self, services: &[ServiceConfig], tier: &ServiceTier) -> Result<()> {
        match tier {
            ServiceTier::Infrastructure => {
                // Infrastructure services start first and need special handling
                self.start_infrastructure_services(services).await?;
            }
            ServiceTier::Core => {
                // Core services have dependencies and need careful ordering
                self.start_core_services(services).await?;
            }
            _ => {
                // Supporting, HotPath, and Monitoring services can start in parallel
                self.start_parallel_services(services).await?;
            }
        }
        
        Ok(())
    }

    async fn start_infrastructure_services(&self, services: &[ServiceConfig]) -> Result<()> {
        info!("🏗️  Starting infrastructure services (Docker Compose)");
        
        for service in services {
            info!("🚀 Starting infrastructure service: {}", service.name);
            
            // Start the service
            self.service_manager.start_service(&service.name).await?;
            
            // Wait for service to be healthy with extended timeout for infrastructure
            let health_timeout = Duration::from_secs(120); // 2 minutes for infrastructure
            match self.service_manager.wait_for_service_healthy(&service.name, health_timeout).await {
                Ok(_) => {
                    info!("✅ Infrastructure service '{}' is healthy", service.name);
                }
                Err(e) => {
                    error!("❌ Infrastructure service '{}' failed to become healthy: {}", service.name, e);
                    return Err(e);
                }
            }
            
            // Additional wait for infrastructure to fully initialize
            info!("⏳ Waiting for infrastructure to fully initialize...");
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
        
        info!("✅ Infrastructure services started and healthy");
        Ok(())
    }

    async fn start_core_services(&self, services: &[ServiceConfig]) -> Result<()> {
        info!("🔧 Starting core services with dependency resolution");
        
        // Build dependency graph
        let dependency_graph = self.build_dependency_graph(services);
        
        // Start services in dependency order
        let startup_order = self.resolve_startup_order(&dependency_graph)?;
        
        for service_name in startup_order {
            if let Some(service) = services.iter().find(|s| s.name == service_name) {
                info!("🚀 Starting core service: {}", service.name);
                
                // Start the service
                self.service_manager.start_service(&service.name).await?;
                
                // Wait for service to be healthy
                let health_timeout = Duration::from_secs(60); // 1 minute for core services
                match self.service_manager.wait_for_service_healthy(&service.name, health_timeout).await {
                    Ok(_) => {
                        info!("✅ Core service '{}' is healthy", service.name);
                    }
                    Err(e) => {
                        error!("❌ Core service '{}' failed to become healthy: {}", service.name, e);
                        if service.critical {
                            return Err(e);
                        } else {
                            warn!("⚠️  Non-critical service '{}' unhealthy, continuing...", service.name);
                        }
                    }
                }
                
                // Brief pause between core service starts
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
        
        info!("✅ Core services started successfully");
        Ok(())
    }

    async fn start_parallel_services(&self, services: &[ServiceConfig]) -> Result<()> {
        info!("⚡ Starting services in parallel");
        
        // Start all services in parallel
        let mut handles = Vec::new();
        
        for service in services {
            let service_manager = self.service_manager.clone();
            let service_name = service.name.clone();
            let is_critical = service.critical;
            
            let handle = tokio::spawn(async move {
                info!("🚀 Starting service: {}", service_name);
                
                // Start the service
                if let Err(e) = service_manager.start_service(&service_name).await {
                    error!("❌ Failed to start service '{}': {}", service_name, e);
                    return Err(e);
                }
                
                // Wait for service to be healthy
                let health_timeout = Duration::from_secs(45); // 45 seconds for supporting services
                match service_manager.wait_for_service_healthy(&service_name, health_timeout).await {
                    Ok(_) => {
                        info!("✅ Service '{}' is healthy", service_name);
                        Ok(())
                    }
                    Err(e) => {
                        error!("❌ Service '{}' failed to become healthy: {}", service_name, e);
                        if is_critical {
                            Err(e)
                        } else {
                            warn!("⚠️  Non-critical service '{}' unhealthy, continuing...", service_name);
                            Ok(())
                        }
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all services to complete
        for handle in handles {
            if let Err(e) = handle.await? {
                error!("❌ Parallel service startup failed: {}", e);
                // Continue with other services, but log the error
            }
        }
        
        info!("✅ Parallel services startup completed");
        Ok(())
    }

    fn build_dependency_graph(&self, services: &[ServiceConfig]) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();
        
        for service in services {
            graph.insert(service.name.clone(), service.dependencies.clone());
        }
        
        graph
    }

    fn resolve_startup_order(&self, dependency_graph: &HashMap<String, Vec<String>>) -> Result<Vec<String>> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        
        // Topological sort to resolve dependencies
        for service_name in dependency_graph.keys() {
            if !visited.contains(service_name) {
                self.dfs_visit(
                    service_name,
                    dependency_graph,
                    &mut visited,
                    &mut temp_visited,
                    &mut order,
                )?;
            }
        }
        
        order.reverse(); // Reverse to get correct startup order
        Ok(order)
    }

    fn dfs_visit(
        &self,
        service_name: &str,
        dependency_graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<()> {
        if temp_visited.contains(service_name) {
            return Err(anyhow::anyhow!("Circular dependency detected involving service: {}", service_name));
        }
        
        if visited.contains(service_name) {
            return Ok(());
        }
        
        temp_visited.insert(service_name.to_string());
        
        if let Some(dependencies) = dependency_graph.get(service_name) {
            for dep in dependencies {
                self.dfs_visit(dep, dependency_graph, visited, temp_visited, order)?;
            }
        }
        
        temp_visited.remove(service_name);
        visited.insert(service_name.to_string());
        order.push(service_name.to_string());
        
        Ok(())
    }

    async fn verify_system_health(&self) -> Result<()> {
        info!("🔍 Verifying overall system health");
        
        let services = self.service_manager.list_services().await;
        let mut critical_services_unhealthy = Vec::new();
        let mut non_critical_services_unhealthy = Vec::new();
        
        for service_instance in services {
            match self.service_manager.check_service_health(&service_instance.config.name).await {
                Ok(health_status) => {
                    match health_status {
                        crate::health_checker::HealthStatus::Healthy => {
                            info!("✅ Service '{}' is healthy", service_instance.config.name);
                        }
                        crate::health_checker::HealthStatus::Unknown => {
                            warn!("⚠️  Service '{}' health status unknown", service_instance.config.name);
                        }
                        _ => {
                            if service_instance.config.critical {
                                critical_services_unhealthy.push(service_instance.config.name.clone());
                            } else {
                                non_critical_services_unhealthy.push(service_instance.config.name.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Failed to check health for service '{}': {}", service_instance.config.name, e);
                    if service_instance.config.critical {
                        critical_services_unhealthy.push(service_instance.config.name.clone());
                    } else {
                        non_critical_services_unhealthy.push(service_instance.config.name.clone());
                    }
                }
            }
        }
        
        // Report system health status
        if !critical_services_unhealthy.is_empty() {
            error!("❌ Critical services are unhealthy: {:?}", critical_services_unhealthy);
            return Err(anyhow::anyhow!("System startup failed: critical services unhealthy"));
        }
        
        if !non_critical_services_unhealthy.is_empty() {
            warn!("⚠️  Non-critical services are unhealthy: {:?}", non_critical_services_unhealthy);
            warn!("⚠️  System will continue running with degraded functionality");
        }
        
        info!("✅ System health verification completed");
        info!("🎉 MEVana system is ready for arbitrage operations!");
        
        Ok(())
    }
}