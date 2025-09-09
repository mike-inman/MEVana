//! Health monitoring and system status
//!
//! Provides comprehensive system health monitoring with:
//! - Component health aggregation
//! - System resource monitoring
//! - Health endpoint for load balancers
//! - Alerting integration points

use crate::error::{ApiError, ApiResult};
use super::{
    database::{DatabaseHealthChecker, DatabaseComprehensiveHealth},
    external_services::{ResilientHttpClient, ExternalServiceHealthInfo},
};
use resilience_framework::health::ServiceHealth;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, warn, error, instrument};

/// System health monitor
pub struct HealthMonitor {
    database_checker: DatabaseHealthChecker,
    http_client: ResilientHttpClient,
    system_info: SystemInfoCollector,
}

impl HealthMonitor {
    /// Create new health monitor
    pub fn new(
        database_checker: DatabaseHealthChecker,
        http_client: ResilientHttpClient,
    ) -> Self {
        Self {
            database_checker,
            http_client,
            system_info: SystemInfoCollector::new(),
        }
    }
    
    /// Get comprehensive system health
    #[instrument(skip(self))]
    pub async fn get_system_health(&self) -> SystemHealthReport {
        debug!("Starting comprehensive system health check");
        
        let start_time = std::time::Instant::now();
        
        // Check database health
        let database_health = self.database_checker.comprehensive_check().await;
        
        // Check external services health
        let external_services_health = self.http_client.check_all_services().await;
        
        // Collect system resources
        let system_resources = self.system_info.collect_system_info().await;
        
        // Calculate overall health
        let overall_health = Self::calculate_overall_health(
            &database_health,
            &external_services_health,
            &system_resources,
        );
        
        let check_duration = start_time.elapsed();
        
        debug!("System health check completed in {:?}", check_duration);
        
        SystemHealthReport {
            overall_health,
            database: database_health,
            external_services: external_services_health,
            system_resources,
            check_duration,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Get basic health status for load balancer health checks
    #[instrument(skip(self))]
    pub async fn get_basic_health(&self) -> BasicHealthStatus {
        let start_time = std::time::Instant::now();
        
        // Quick database connectivity check
        let db_health = self.database_checker.check_connectivity().await
            .unwrap_or_else(|_| {
                crate::resilience::database::DatabaseHealthInfo {
                    status: ServiceHealth::Unhealthy,
                    connectivity_check_duration: Duration::from_secs(0),
                    pool_stats: None,
                    last_check: chrono::Utc::now(),
                    error_message: Some("Health check failed".to_string()),
                }
            });
        
        let check_duration = start_time.elapsed();
        
        let status = match db_health.status {
            ServiceHealth::Healthy => HealthStatus::Healthy,
            ServiceHealth::Degraded => HealthStatus::Degraded,
            ServiceHealth::Unhealthy => HealthStatus::Unhealthy,
        };
        
        BasicHealthStatus {
            status,
            database_connected: matches!(db_health.status, ServiceHealth::Healthy | ServiceHealth::Degraded),
            check_duration,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Calculate overall system health
    fn calculate_overall_health(
        database_health: &DatabaseComprehensiveHealth,
        external_services_health: &HashMap<String, ExternalServiceHealthInfo>,
        system_resources: &SystemResourceInfo,
    ) -> HealthStatus {
        // Database health is critical
        if matches!(database_health.overall_status, ServiceHealth::Unhealthy) {
            return HealthStatus::Unhealthy;
        }
        
        // System resources are critical
        if system_resources.is_critical() {
            return HealthStatus::Unhealthy;
        }
        
        // Count external service health
        let external_unhealthy = external_services_health
            .values()
            .filter(|s| matches!(s.status, ServiceHealth::Unhealthy))
            .count();
        
        let external_degraded = external_services_health
            .values()
            .filter(|s| matches!(s.status, ServiceHealth::Degraded))
            .count();
        
        // If more than half of external services are down, system is unhealthy
        if external_services_health.len() > 0 && 
           external_unhealthy > external_services_health.len() / 2 {
            return HealthStatus::Unhealthy;
        }
        
        // If database is degraded or some services are down, system is degraded
        if matches!(database_health.overall_status, ServiceHealth::Degraded) ||
           external_unhealthy > 0 ||
           external_degraded > 0 ||
           system_resources.is_degraded() {
            return HealthStatus::Degraded;
        }
        
        HealthStatus::Healthy
    }
    
    /// Get health metrics for monitoring
    pub async fn get_health_metrics(&self) -> HealthMetrics {
        let system_health = self.get_system_health().await;
        
        HealthMetrics {
            database_response_time_ms: system_health.database.connectivity.connectivity_check_duration.as_millis() as u64,
            external_services_count: system_health.external_services.len() as u32,
            healthy_external_services: system_health.external_services.values()
                .filter(|s| matches!(s.status, ServiceHealth::Healthy))
                .count() as u32,
            unhealthy_external_services: system_health.external_services.values()
                .filter(|s| matches!(s.status, ServiceHealth::Unhealthy))
                .count() as u32,
            cpu_usage_percent: system_health.system_resources.cpu_usage_percent,
            memory_usage_percent: system_health.system_resources.memory_usage_percent,
            disk_usage_percent: system_health.system_resources.disk_usage_percent,
            health_check_duration_ms: system_health.check_duration.as_millis() as u64,
        }
    }
}

/// System resource information collector
pub struct SystemInfoCollector;

impl SystemInfoCollector {
    pub fn new() -> Self {
        Self
    }
    
    /// Collect system resource information
    pub async fn collect_system_info(&self) -> SystemResourceInfo {
        // In a real implementation, you would use system monitoring crates
        // like `sysinfo` or platform-specific APIs
        SystemResourceInfo {
            cpu_usage_percent: self.get_cpu_usage().await,
            memory_usage_percent: self.get_memory_usage().await,
            disk_usage_percent: self.get_disk_usage().await,
            load_average: self.get_load_average().await,
            uptime_seconds: self.get_uptime().await,
            thread_count: self.get_thread_count().await,
        }
    }
    
    async fn get_cpu_usage(&self) -> f64 {
        // Placeholder - would implement actual CPU monitoring
        15.5
    }
    
    async fn get_memory_usage(&self) -> f64 {
        // Placeholder - would implement actual memory monitoring
        45.2
    }
    
    async fn get_disk_usage(&self) -> f64 {
        // Placeholder - would implement actual disk monitoring
        32.1
    }
    
    async fn get_load_average(&self) -> f64 {
        // Placeholder - would implement actual load average
        0.8
    }
    
    async fn get_uptime(&self) -> u64 {
        // Placeholder - would implement actual uptime
        3600
    }
    
    async fn get_thread_count(&self) -> u32 {
        // Placeholder - would implement actual thread counting
        25
    }
}

/// Complete system health report
#[derive(Debug, Serialize)]
pub struct SystemHealthReport {
    pub overall_health: HealthStatus,
    pub database: DatabaseComprehensiveHealth,
    pub external_services: HashMap<String, ExternalServiceHealthInfo>,
    pub system_resources: SystemResourceInfo,
    pub check_duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Basic health status for load balancers
#[derive(Debug, Serialize)]
pub struct BasicHealthStatus {
    pub status: HealthStatus,
    pub database_connected: bool,
    pub check_duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// System resource information
#[derive(Debug, Serialize)]
pub struct SystemResourceInfo {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub load_average: f64,
    pub uptime_seconds: u64,
    pub thread_count: u32,
}

impl SystemResourceInfo {
    /// Check if system resources are at critical levels
    pub fn is_critical(&self) -> bool {
        self.cpu_usage_percent > 95.0 ||
        self.memory_usage_percent > 95.0 ||
        self.disk_usage_percent > 95.0
    }
    
    /// Check if system resources are degraded
    pub fn is_degraded(&self) -> bool {
        self.cpu_usage_percent > 80.0 ||
        self.memory_usage_percent > 80.0 ||
        self.disk_usage_percent > 80.0
    }
}

/// Health metrics for monitoring systems
#[derive(Debug, Serialize)]
pub struct HealthMetrics {
    pub database_response_time_ms: u64,
    pub external_services_count: u32,
    pub healthy_external_services: u32,
    pub unhealthy_external_services: u32,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub health_check_duration_ms: u64,
}