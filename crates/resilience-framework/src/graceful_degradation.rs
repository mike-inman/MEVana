use moka::future::Cache;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    Cache,
    StaticValue(serde_json::Value),
    Alternative(String),
    Disabled,
}

#[derive(Debug, Clone)]
pub struct ServiceLevel {
    pub name: String,
    pub priority: u8,
    pub enabled: bool,
    pub fallback: FallbackStrategy,
    pub load_threshold: f64,
}

pub struct GracefulDegradation {
    service_levels: Arc<RwLock<Vec<ServiceLevel>>>,
    cache: Arc<Cache<String, serde_json::Value>>,
    system_load: Arc<RwLock<SystemLoad>>,
    config: DegradationConfig,
}

#[derive(Clone)]
pub struct DegradationConfig {
    pub cache_size: u64,
    pub cache_ttl: Duration,
    pub load_check_interval: Duration,
    pub auto_recovery: bool,
    pub recovery_threshold: f64,
    pub degradation_threshold: f64,
}

impl Default for DegradationConfig {
    fn default() -> Self {
        DegradationConfig {
            cache_size: 10000,
            cache_ttl: Duration::from_secs(300),
            load_check_interval: Duration::from_secs(10),
            auto_recovery: true,
            recovery_threshold: 0.5,
            degradation_threshold: 0.8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemLoad {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_connections: usize,
    pub error_rate: f64,
    pub response_time_p99: Duration,
    pub last_updated: Instant,
}

impl Default for SystemLoad {
    fn default() -> Self {
        SystemLoad {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            active_connections: 0,
            error_rate: 0.0,
            response_time_p99: Duration::from_millis(100),
            last_updated: Instant::now(),
        }
    }
}

impl GracefulDegradation {
    pub fn new(config: DegradationConfig) -> Self {
        let cache = Arc::new(Cache::builder()
            .max_capacity(config.cache_size)
            .time_to_live(config.cache_ttl)
            .build());
        
        let auto_recovery = config.auto_recovery;
        let degradation = GracefulDegradation {
            service_levels: Arc::new(RwLock::new(Vec::new())),
            cache,
            system_load: Arc::new(RwLock::new(SystemLoad::default())),
            config,
        };
        
        if auto_recovery {
            degradation.start_auto_recovery();
        }
        
        degradation
    }
    
    pub async fn register_service(&self, service_level: ServiceLevel) {
        let mut levels = self.service_levels.write().await;
        levels.push(service_level);
        levels.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
    
    pub async fn execute_with_degradation<T, F, E>(
        &self,
        service_name: &str,
        cache_key: Option<&str>,
        operation: F,
    ) -> Result<T, DegradationError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        T: serde::Serialize + serde::de::DeserializeOwned + Clone + Send + 'static,
        E: std::fmt::Display,
    {
        let service_level = {
            let levels = self.service_levels.read().await;
            levels.iter()
                .find(|level| level.name == service_name)
                .cloned()
        };
        
        let service_level = match service_level {
            Some(level) => level,
            None => return Err(DegradationError::ServiceNotRegistered(service_name.to_string())),
        };
        
        if !service_level.enabled {
            return self.apply_fallback(&service_level, cache_key).await;
        }
        
        let current_load = self.get_current_load().await;
        
        if self.should_degrade(&service_level, &current_load).await {
            log::warn!(
                "Applying degradation for service {} due to high load ({})",
                service_name, current_load.cpu_usage
            );
            return self.apply_fallback(&service_level, cache_key).await;
        }
        
        let start_time = Instant::now();
        
        match operation.await {
            Ok(result) => {
                if let Some(key) = cache_key {
                    let value = serde_json::to_value(&result)
                        .map_err(|e| DegradationError::SerializationError(e.to_string()))?;
                    self.cache.insert(key.to_string(), value).await;
                }
                
                self.record_success(&service_level, start_time.elapsed()).await;
                Ok(result)
            }
            Err(error) => {
                log::warn!(
                    "Operation failed for service {}: {}",
                    service_name, error
                );
                
                self.record_failure(&service_level).await;
                self.apply_fallback(&service_level, cache_key).await
            }
        }
    }
    
    async fn should_degrade(&self, service_level: &ServiceLevel, load: &SystemLoad) -> bool {
        let overall_load = (load.cpu_usage + load.memory_usage + load.error_rate) / 3.0;
        overall_load > service_level.load_threshold
    }
    
    async fn apply_fallback<T>(&self, service_level: &ServiceLevel, cache_key: Option<&str>) -> Result<T, DegradationError>
    where
        T: serde::de::DeserializeOwned + Clone + Send + 'static,
    {
        match &service_level.fallback {
            FallbackStrategy::Cache => {
                if let Some(key) = cache_key {
                    if let Some(cached_value) = self.cache.get(key).await {
                        let result: T = serde_json::from_value(cached_value)
                            .map_err(|e| DegradationError::DeserializationError(e.to_string()))?;
                        return Ok(result);
                    }
                }
                Err(DegradationError::NoFallbackAvailable)
            }
            FallbackStrategy::StaticValue(value) => {
                let result: T = serde_json::from_value(value.clone())
                    .map_err(|e| DegradationError::DeserializationError(e.to_string()))?;
                Ok(result)
            }
            FallbackStrategy::Alternative(alternative_service) => {
                Err(DegradationError::AlternativeNotImplemented(alternative_service.clone()))
            }
            FallbackStrategy::Disabled => {
                Err(DegradationError::ServiceDisabled)
            }
        }
    }
    
    pub async fn update_system_load(&self, load: SystemLoad) {
        let mut system_load = self.system_load.write().await;
        *system_load = load;
    }
    
    async fn get_current_load(&self) -> SystemLoad {
        self.system_load.read().await.clone()
    }
    
    async fn record_success(&self, _service_level: &ServiceLevel, _duration: Duration) {
        // Update metrics for successful operations
        log::debug!("Recorded success for service {}", _service_level.name);
    }
    
    async fn record_failure(&self, service_level: &ServiceLevel) {
        // Update failure metrics and potentially disable service
        log::warn!("Recorded failure for service {}", service_level.name);
    }
    
    pub async fn enable_service(&self, service_name: &str) {
        let mut levels = self.service_levels.write().await;
        if let Some(level) = levels.iter_mut().find(|l| l.name == service_name) {
            level.enabled = true;
            log::info!("Enabled service: {}", service_name);
        }
    }
    
    pub async fn disable_service(&self, service_name: &str) {
        let mut levels = self.service_levels.write().await;
        if let Some(level) = levels.iter_mut().find(|l| l.name == service_name) {
            level.enabled = false;
            log::warn!("Disabled service: {}", service_name);
        }
    }
    
    pub async fn get_service_status(&self) -> Vec<ServiceStatus> {
        let levels = self.service_levels.read().await;
        levels.iter().map(|level| ServiceStatus {
            name: level.name.clone(),
            enabled: level.enabled,
            priority: level.priority,
            load_threshold: level.load_threshold,
        }).collect()
    }
    
    fn start_auto_recovery(&self) {
        let degradation = GracefulDegradation {
            service_levels: self.service_levels.clone(),
            cache: self.cache.clone(),
            system_load: self.system_load.clone(),
            config: self.config.clone(),
        };
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(degradation.config.load_check_interval);
            
            loop {
                interval.tick().await;
                
                let current_load = degradation.get_current_load().await;
                let overall_load = (current_load.cpu_usage + current_load.memory_usage) / 2.0;
                
                if overall_load < degradation.config.recovery_threshold {
                    let mut levels = degradation.service_levels.write().await;
                    for level in levels.iter_mut() {
                        if !level.enabled && overall_load < level.load_threshold {
                            level.enabled = true;
                            log::info!("Auto-recovered service: {}", level.name);
                        }
                    }
                } else if overall_load > degradation.config.degradation_threshold {
                    let mut levels = degradation.service_levels.write().await;
                    for level in levels.iter_mut() {
                        if level.enabled && overall_load > level.load_threshold && level.priority < 5 {
                            level.enabled = false;
                            log::warn!("Auto-disabled service due to load: {}", level.name);
                        }
                    }
                }
            }
        });
    }
    
    pub async fn clear_cache(&self) {
        self.cache.invalidate_all();
        log::info!("Cleared degradation cache");
    }
    
    pub async fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.cache.entry_count(),
            weighted_size: self.cache.weighted_size(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub name: String,
    pub enabled: bool,
    pub priority: u8,
    pub load_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: u64,
    pub weighted_size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DegradationError {
    #[error("Service not registered: {0}")]
    ServiceNotRegistered(String),
    #[error("Service is disabled")]
    ServiceDisabled,
    #[error("No fallback available")]
    NoFallbackAvailable,
    #[error("Alternative service not implemented: {0}")]
    AlternativeNotImplemented(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_service_registration() {
        let degradation = GracefulDegradation::new(DegradationConfig::default());
        
        let service_level = ServiceLevel {
            name: "test_service".to_string(),
            priority: 5,
            enabled: true,
            fallback: FallbackStrategy::Cache,
            load_threshold: 0.8,
        };
        
        degradation.register_service(service_level).await;
        
        let status = degradation.get_service_status().await;
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].name, "test_service");
    }
    
    #[tokio::test]
    async fn test_service_disable_enable() {
        let degradation = GracefulDegradation::new(DegradationConfig::default());
        
        let service_level = ServiceLevel {
            name: "test_service".to_string(),
            priority: 5,
            enabled: true,
            fallback: FallbackStrategy::Cache,
            load_threshold: 0.8,
        };
        
        degradation.register_service(service_level).await;
        
        degradation.disable_service("test_service").await;
        let status = degradation.get_service_status().await;
        assert!(!status[0].enabled);
        
        degradation.enable_service("test_service").await;
        let status = degradation.get_service_status().await;
        assert!(status[0].enabled);
    }
}