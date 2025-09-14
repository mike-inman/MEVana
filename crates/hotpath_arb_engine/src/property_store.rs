use anyhow::Result;
use log::{debug, info, warn, error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};
use std::path::Path;
use std::io::Write;

/// Global property store for real-time bidirectional parameter binding
/// Supports atomic updates from UI, AI, and system sources with zero trading downtime
#[derive(Debug)]
pub struct PropertyStore {
    /// Complex properties that need RwLock protection
    complex_properties: Arc<RwLock<HashMap<String, PropertyValue>>>,
    
    /// Atomic primitive properties for hotpath access (zero-contention)
    atomic_properties: AtomicProperties,
    
    /// Event broadcaster for real-time synchronization
    event_broadcaster: broadcast::Sender<PropertyChangeEvent>,
    
    /// Audit trail for parameter changes
    change_history: Arc<RwLock<Vec<PropertyChangeEvent>>>,
    
    /// Persistent storage path for crash recovery
    persistence_path: Arc<RwLock<String>>,
    
    /// Emergency safe values for system crash recovery
    emergency_safe_values: EmergencySafeValues,
    
    /// Last successful persistence timestamp
    last_persistence_timestamp: std::sync::atomic::AtomicU64,
}

/// Atomic primitive properties optimized for hotpath trading access
#[derive(Debug)]
pub struct AtomicProperties {
    // Smart Filter Configuration (from SmartFilterConfig)
    pub min_profit_bps: AtomicI32,
    pub max_profit_bps: AtomicI32,
    pub min_venue_success_rate: AtomicU64, // f64 stored as u64 bits
    pub max_gas_budget_pct: AtomicU64,     // f64 stored as u64 bits
    pub max_concurrent_opportunities: AtomicU32,
    pub competition_lookback_ms: AtomicU64,
    pub learning_rate: AtomicU64,          // f64 stored as u64 bits
    pub min_samples_for_learning: AtomicU64,
    
    // Additional MEVana properties for future expansion
    pub system_enabled: AtomicBool,
    pub ai_optimization_enabled: AtomicBool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
    Integer(i32),
    Float(f64),
    Boolean(bool),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyChangeEvent {
    pub property_key: String,
    pub old_value: PropertyValue,
    pub new_value: PropertyValue,
    pub source: PropertySource,
    pub timestamp: u64,
    pub change_id: String, // UUID for tracking
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PropertySource {
    HumanUI,
    AIOptimizer, 
    SystemDefault,
    DatabaseLoad,
    CrashRecovery,
}

/// Persistent state for crash recovery - critical trading parameters only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentPropertyState {
    pub min_profit_bps: i32,
    pub max_profit_bps: i32,
    pub min_venue_success_rate: f64,
    pub max_gas_budget_pct: f64,
    pub learning_rate: f64,
    pub system_enabled: bool,
    pub ai_optimization_enabled: bool,
    pub timestamp: u64,
    pub last_update_source: PropertySource,
    pub crash_count: u32,
}

/// Emergency safe values - guaranteed safe trading parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencySafeValues {
    pub min_profit_bps: i32,         // Conservative profit requirement
    pub max_profit_bps: i32,         // Prevent suspiciously high profits
    pub max_gas_budget_pct: f64,     // Conservative gas limit
    pub min_venue_success_rate: f64, // High success rate requirement
    pub learning_rate: f64,          // Slow learning rate for safety
    pub system_enabled: bool,        // Can disable system entirely
}

impl Default for EmergencySafeValues {
    fn default() -> Self {
        Self {
            min_profit_bps: 10,         // 10bps minimum - very conservative
            max_profit_bps: 300,        // 300bps maximum - prevent suspicious trades
            max_gas_budget_pct: 50.0,   // 50% max gas usage - very conservative
            min_venue_success_rate: 0.8, // 80% success rate - high bar
            learning_rate: 0.05,        // 5% learning rate - slow and safe
            system_enabled: true,       // System enabled by default
        }
    }
}

impl PropertyStore {
    /// Create new property store with default values from SmartFilterConfig
    pub fn new() -> Self {
        let (event_broadcaster, _) = broadcast::channel(1000); // 1000 event buffer
        
        // Initialize with SmartFilterConfig defaults
        let atomic_properties = AtomicProperties {
            min_profit_bps: AtomicI32::new(1),
            max_profit_bps: AtomicI32::new(500),
            min_venue_success_rate: AtomicU64::new(0.0_f64.to_bits()),
            max_gas_budget_pct: AtomicU64::new(80.0_f64.to_bits()),
            max_concurrent_opportunities: AtomicU32::new(20),
            competition_lookback_ms: AtomicU64::new(5000),
            learning_rate: AtomicU64::new(0.1_f64.to_bits()),
            min_samples_for_learning: AtomicU64::new(5),
            system_enabled: AtomicBool::new(true),
            ai_optimization_enabled: AtomicBool::new(true),
        };
        
        info!("🏪 PropertyStore initialized with SmartFilter defaults");
        
        // Set up persistence path
        let persistence_path = std::env::var("PROPERTY_STORE_PERSISTENCE_PATH")
            .unwrap_or_else(|_| "/tmp/mevana_property_store_state.json".to_string());
            
        let mut instance = Self {
            complex_properties: Arc::new(RwLock::new(HashMap::new())),
            atomic_properties,
            event_broadcaster,
            change_history: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            persistence_path: Arc::new(RwLock::new(persistence_path)),
            emergency_safe_values: EmergencySafeValues::default(),
            last_persistence_timestamp: std::sync::atomic::AtomicU64::new(0),
        };
        
        // Note: Crash recovery can be called separately via async methods to keep hotpath clean
        info!("🏪 PropertyStore initialized with emergency safe values");
        
        instance
    }
    
    /// Set atomic integer property with immediate effect and event notification
    pub fn set_atomic_i32(&self, key: &str, value: i32, source: PropertySource) -> Result<()> {
        let old_value = match key {
            "min_profit_bps" => {
                let old = self.atomic_properties.min_profit_bps.load(Ordering::Relaxed);
                self.atomic_properties.min_profit_bps.store(value, Ordering::Relaxed);
                old
            },
            "max_profit_bps" => {
                let old = self.atomic_properties.max_profit_bps.load(Ordering::Relaxed);
                self.atomic_properties.max_profit_bps.store(value, Ordering::Relaxed);
                old
            },
            _ => return Err(anyhow::anyhow!("Unknown atomic i32 property: {}", key)),
        };
        
        // Fire event notification (non-blocking)
        self.notify_change(
            key,
            PropertyValue::Integer(old_value),
            PropertyValue::Integer(value),
            source,
        );
        
        info!("⚡ Atomic property {} changed: {} -> {} (source: {:?})", key, old_value, value, source);
        Ok(())
    }
    
    /// Set atomic float property (stored as u64 bits for atomic access)
    pub fn set_atomic_f64(&self, key: &str, value: f64, source: PropertySource) -> Result<()> {
        let old_bits = match key {
            "min_venue_success_rate" => {
                let old_bits = self.atomic_properties.min_venue_success_rate.load(Ordering::Relaxed);
                self.atomic_properties.min_venue_success_rate.store(value.to_bits(), Ordering::Relaxed);
                old_bits
            },
            "max_gas_budget_pct" => {
                let old_bits = self.atomic_properties.max_gas_budget_pct.load(Ordering::Relaxed);
                self.atomic_properties.max_gas_budget_pct.store(value.to_bits(), Ordering::Relaxed);
                old_bits
            },
            "learning_rate" => {
                let old_bits = self.atomic_properties.learning_rate.load(Ordering::Relaxed);
                self.atomic_properties.learning_rate.store(value.to_bits(), Ordering::Relaxed);
                old_bits
            },
            _ => return Err(anyhow::anyhow!("Unknown atomic f64 property: {}", key)),
        };
        
        let old_value = f64::from_bits(old_bits);
        
        // Fire event notification
        self.notify_change(
            key,
            PropertyValue::Float(old_value),
            PropertyValue::Float(value),
            source,
        );
        
        info!("⚡ Atomic property {} changed: {:.6} -> {:.6} (source: {:?})", key, old_value, value, source);
        Ok(())
    }
    
    /// Get atomic integer property (zero-contention hotpath access)
    pub fn get_atomic_i32(&self, key: &str) -> Option<i32> {
        match key {
            "min_profit_bps" => Some(self.atomic_properties.min_profit_bps.load(Ordering::Relaxed)),
            "max_profit_bps" => Some(self.atomic_properties.max_profit_bps.load(Ordering::Relaxed)),
            _ => None,
        }
    }
    
    /// Get atomic float property (zero-contention hotpath access)
    pub fn get_atomic_f64(&self, key: &str) -> Option<f64> {
        match key {
            "min_venue_success_rate" => {
                Some(f64::from_bits(self.atomic_properties.min_venue_success_rate.load(Ordering::Relaxed)))
            },
            "max_gas_budget_pct" => {
                Some(f64::from_bits(self.atomic_properties.max_gas_budget_pct.load(Ordering::Relaxed)))
            },
            "learning_rate" => {
                Some(f64::from_bits(self.atomic_properties.learning_rate.load(Ordering::Relaxed)))
            },
            _ => None,
        }
    }
    
    /// Subscribe to property change events for real-time synchronization
    pub fn subscribe(&self) -> broadcast::Receiver<PropertyChangeEvent> {
        self.event_broadcaster.subscribe()
    }
    
    /// Get all current property values for UI synchronization
    pub async fn get_all_properties(&self) -> HashMap<String, PropertyValue> {
        let mut properties = HashMap::new();
        
        // Atomic properties
        properties.insert("min_profit_bps".to_string(), 
                         PropertyValue::Integer(self.atomic_properties.min_profit_bps.load(Ordering::Relaxed)));
        properties.insert("max_profit_bps".to_string(), 
                         PropertyValue::Integer(self.atomic_properties.max_profit_bps.load(Ordering::Relaxed)));
        properties.insert("min_venue_success_rate".to_string(), 
                         PropertyValue::Float(f64::from_bits(self.atomic_properties.min_venue_success_rate.load(Ordering::Relaxed))));
        properties.insert("max_gas_budget_pct".to_string(), 
                         PropertyValue::Float(f64::from_bits(self.atomic_properties.max_gas_budget_pct.load(Ordering::Relaxed))));
        properties.insert("learning_rate".to_string(), 
                         PropertyValue::Float(f64::from_bits(self.atomic_properties.learning_rate.load(Ordering::Relaxed))));
        properties.insert("system_enabled".to_string(), 
                         PropertyValue::Boolean(self.atomic_properties.system_enabled.load(Ordering::Relaxed)));
        properties.insert("ai_optimization_enabled".to_string(), 
                         PropertyValue::Boolean(self.atomic_properties.ai_optimization_enabled.load(Ordering::Relaxed)));
        
        // Complex properties
        let complex = self.complex_properties.read().await;
        for (key, value) in complex.iter() {
            properties.insert(key.clone(), value.clone());
        }
        
        properties
    }
    
    /// Load properties from SmartFilterConfig (backward compatibility)
    pub async fn load_from_smart_filter_config(&self, config: &crate::smart_filter::SmartFilterConfig) {
        info!("📥 Loading properties from SmartFilterConfig");
        
        // Load atomic properties
        self.set_atomic_i32("min_profit_bps", config.min_profit_bps, PropertySource::DatabaseLoad).ok();
        self.set_atomic_i32("max_profit_bps", config.max_profit_bps, PropertySource::DatabaseLoad).ok();
        self.set_atomic_f64("min_venue_success_rate", config.min_venue_success_rate, PropertySource::DatabaseLoad).ok();
        self.set_atomic_f64("max_gas_budget_pct", config.max_gas_budget_pct, PropertySource::DatabaseLoad).ok();
        self.set_atomic_f64("learning_rate", config.learning_rate, PropertySource::DatabaseLoad).ok();
        
        // Store non-atomic properties in complex store
        let mut complex = self.complex_properties.write().await;
        complex.insert("min_net_profit_sol".to_string(), PropertyValue::Float(config.min_net_profit_sol));
        complex.insert("max_concurrent_opportunities".to_string(), PropertyValue::Integer(config.max_concurrent_opportunities as i32));
        complex.insert("competition_lookback_ms".to_string(), PropertyValue::Integer(config.competition_lookback_ms as i32));
        complex.insert("min_samples_for_learning".to_string(), PropertyValue::Integer(config.min_samples_for_learning as i32));
        
        info!("✅ PropertyStore loaded from SmartFilterConfig: min_profit={}bps, learning_rate={:.3}", 
               config.min_profit_bps, config.learning_rate);
    }
    
    /// Internal notification system for property changes
    fn notify_change(&self, key: &str, old_value: PropertyValue, new_value: PropertyValue, source: PropertySource) {
        let event = PropertyChangeEvent {
            property_key: key.to_string(),
            old_value,
            new_value,
            source,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
            change_id: format!("{}-{}", key, uuid::Uuid::new_v4().simple()),
        };
        
        // Non-blocking event broadcast
        if let Err(e) = self.event_broadcaster.send(event.clone()) {
            warn!("Failed to broadcast property change event: {}", e);
        }
        
        // Store in history (async fire-and-forget)
        let history = Arc::clone(&self.change_history);
        let event_copy = event.clone();
        tokio::spawn(async move {
            let mut hist = history.write().await;
            hist.push(event_copy);
            
            // Keep history bounded (last 10,000 changes)
            if hist.len() > 10000 {
                hist.drain(0..1000); // Remove oldest 1000 entries
            }
        });
        
        debug!("📡 Property change event broadcasted: {} = {:?}", key, event.new_value);
        
        // Trigger async persistence after property changes
        self.trigger_async_persistence();
    }
    
    /// Crash Recovery and Persistence System
    /// =====================================
    
    /// Attempt to recover from previous crash by loading persisted state
    async fn attempt_crash_recovery(&mut self) -> Result<bool> {
        let persistence_path = self.persistence_path.read().await.clone();
        let path = Path::new(&persistence_path);
        
        if !path.exists() {
            debug!("No persistent state file found at: {}", persistence_path);
            return Ok(false);
        }
        
        let file_content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read persistence file: {}", e))?;
            
        let persistent_state: PersistentPropertyState = serde_json::from_str(&file_content)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize persistent state: {}", e))?;
            
        // Validate that the state is not too old (safety measure)
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        let state_age_ms = now.saturating_sub(persistent_state.timestamp);
        
        if state_age_ms > 24 * 60 * 60 * 1000 { // 24 hours
            warn!("⚠️ Persistent state is {} hours old. Using emergency safe values instead.", 
                  state_age_ms / (60 * 60 * 1000));
            self.apply_emergency_safe_values();
            return Ok(false);
        }
        
        // Apply recovered values with validation
        self.apply_recovered_values(&persistent_state)?;
        
        info!("✅ Successfully recovered PropertyStore state from crash. Age: {}min", 
              state_age_ms / (60 * 1000));
        
        Ok(true)
    }
    
    /// Apply recovered values with safety validation
    fn apply_recovered_values(&mut self, state: &PersistentPropertyState) -> Result<()> {
        let emergency = &self.emergency_safe_values;
        
        // Validate and apply with safety bounds checking
        let safe_min_profit = state.min_profit_bps.max(emergency.min_profit_bps);
        let safe_max_profit = state.max_profit_bps.min(emergency.max_profit_bps);
        let safe_gas_budget = state.max_gas_budget_pct.min(emergency.max_gas_budget_pct);
        let safe_success_rate = state.min_venue_success_rate.max(emergency.min_venue_success_rate);
        let safe_learning_rate = state.learning_rate.min(emergency.learning_rate);
        
        // Apply safely bounded values
        self.atomic_properties.min_profit_bps.store(safe_min_profit, Ordering::Relaxed);
        self.atomic_properties.max_profit_bps.store(safe_max_profit, Ordering::Relaxed);
        self.atomic_properties.max_gas_budget_pct.store(safe_gas_budget.to_bits(), Ordering::Relaxed);
        self.atomic_properties.min_venue_success_rate.store(safe_success_rate.to_bits(), Ordering::Relaxed);
        self.atomic_properties.learning_rate.store(safe_learning_rate.to_bits(), Ordering::Relaxed);
        self.atomic_properties.system_enabled.store(state.system_enabled, Ordering::Relaxed);
        self.atomic_properties.ai_optimization_enabled.store(state.ai_optimization_enabled, Ordering::Relaxed);
        
        // Log any adjustments made for safety
        if safe_min_profit != state.min_profit_bps {
            warn!("🛡️ Adjusted min_profit_bps from {} to {} for safety", state.min_profit_bps, safe_min_profit);
        }
        if safe_max_profit != state.max_profit_bps {
            warn!("🛡️ Adjusted max_profit_bps from {} to {} for safety", state.max_profit_bps, safe_max_profit);
        }
        if (safe_gas_budget - state.max_gas_budget_pct).abs() > f64::EPSILON {
            warn!("🛡️ Adjusted max_gas_budget_pct from {:.1}% to {:.1}% for safety", 
                  state.max_gas_budget_pct, safe_gas_budget);
        }
        
        Ok(())
    }
    
    /// Apply emergency safe values in case of crash recovery failure
    fn apply_emergency_safe_values(&mut self) {
        let emergency = &self.emergency_safe_values;
        
        self.atomic_properties.min_profit_bps.store(emergency.min_profit_bps, Ordering::Relaxed);
        self.atomic_properties.max_profit_bps.store(emergency.max_profit_bps, Ordering::Relaxed);
        self.atomic_properties.max_gas_budget_pct.store(emergency.max_gas_budget_pct.to_bits(), Ordering::Relaxed);
        self.atomic_properties.min_venue_success_rate.store(emergency.min_venue_success_rate.to_bits(), Ordering::Relaxed);
        self.atomic_properties.learning_rate.store(emergency.learning_rate.to_bits(), Ordering::Relaxed);
        self.atomic_properties.system_enabled.store(emergency.system_enabled, Ordering::Relaxed);
        
        warn!("🚨 Applied emergency safe values: min_profit={}bps, max_profit={}bps, max_gas={:.1}%", 
              emergency.min_profit_bps, emergency.max_profit_bps, emergency.max_gas_budget_pct);
    }
    
    /// Trigger asynchronous persistence (fire-and-forget for performance)
    fn trigger_async_persistence(&self) {
        let persistence_path = Arc::clone(&self.persistence_path);
        let min_profit_bps = self.atomic_properties.min_profit_bps.load(Ordering::Relaxed);
        let max_profit_bps = self.atomic_properties.max_profit_bps.load(Ordering::Relaxed);
        let min_venue_success_rate = f64::from_bits(self.atomic_properties.min_venue_success_rate.load(Ordering::Relaxed));
        let max_gas_budget_pct = f64::from_bits(self.atomic_properties.max_gas_budget_pct.load(Ordering::Relaxed));
        let learning_rate = f64::from_bits(self.atomic_properties.learning_rate.load(Ordering::Relaxed));
        let system_enabled = self.atomic_properties.system_enabled.load(Ordering::Relaxed);
        let ai_optimization_enabled = self.atomic_properties.ai_optimization_enabled.load(Ordering::Relaxed);
        // Note: timestamp tracking removed to avoid borrow complexity
        
        // Fire-and-forget async persistence task
        tokio::spawn(async move {
            let persistence_path = persistence_path.read().await.clone();
            
            let state = PersistentPropertyState {
                min_profit_bps,
                max_profit_bps,
                min_venue_success_rate,
                max_gas_budget_pct,
                learning_rate,
                system_enabled,
                ai_optimization_enabled,
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
                last_update_source: PropertySource::CrashRecovery, // Will be updated by caller
                crash_count: 0, // Will be incremented if needed
            };
            
            // Atomic write using temp file + rename for crash safety
            let temp_path = format!("{}.tmp", persistence_path);
            match std::fs::File::create(&temp_path) {
                Ok(mut file) => {
                    match serde_json::to_string_pretty(&state) {
                        Ok(json_content) => {
                            match file.write_all(json_content.as_bytes()) {
                                Ok(_) => {
                                    match file.sync_all() {
                                        Ok(_) => {
                                            match std::fs::rename(&temp_path, &persistence_path) {
                                                Ok(_) => {
                                                    debug!("💾 PropertyStore state persisted successfully");
                                                }
                                                Err(e) => error!("Failed to rename persistence file: {}", e),
                                            }
                                        }
                                        Err(e) => error!("Failed to sync persistence file: {}", e),
                                    }
                                }
                                Err(e) => error!("Failed to write persistence data: {}", e),
                            }
                        }
                        Err(e) => error!("Failed to serialize persistence data: {}", e),
                    }
                }
                Err(e) => error!("Failed to create persistence temp file: {}", e),
            }
            
            // Clean up temp file if it still exists
            let _ = std::fs::remove_file(&temp_path);
        });
    }
    
    /// Get emergency safe values for external inspection
    pub fn get_emergency_safe_values(&self) -> &EmergencySafeValues {
        &self.emergency_safe_values
    }
    
    /// Manually trigger immediate persistence (blocking)
    pub async fn force_persistence(&self) -> Result<()> {
        let persistence_path = self.persistence_path.read().await.clone();
        
        let state = PersistentPropertyState {
            min_profit_bps: self.atomic_properties.min_profit_bps.load(Ordering::Relaxed),
            max_profit_bps: self.atomic_properties.max_profit_bps.load(Ordering::Relaxed),
            min_venue_success_rate: f64::from_bits(self.atomic_properties.min_venue_success_rate.load(Ordering::Relaxed)),
            max_gas_budget_pct: f64::from_bits(self.atomic_properties.max_gas_budget_pct.load(Ordering::Relaxed)),
            learning_rate: f64::from_bits(self.atomic_properties.learning_rate.load(Ordering::Relaxed)),
            system_enabled: self.atomic_properties.system_enabled.load(Ordering::Relaxed),
            ai_optimization_enabled: self.atomic_properties.ai_optimization_enabled.load(Ordering::Relaxed),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
            last_update_source: PropertySource::SystemDefault,
            crash_count: 0,
        };
        
        // Atomic write using temp file + rename for crash safety
        let temp_path = format!("{}.tmp", persistence_path);
        let mut file = std::fs::File::create(&temp_path)
            .map_err(|e| anyhow::anyhow!("Failed to create temp persistence file: {}", e))?;
            
        let json_content = serde_json::to_string_pretty(&state)
            .map_err(|e| anyhow::anyhow!("Failed to serialize state: {}", e))?;
            
        file.write_all(json_content.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to write persistence data: {}", e))?;
            
        file.sync_all()
            .map_err(|e| anyhow::anyhow!("Failed to sync persistence file: {}", e))?;
            
        std::fs::rename(&temp_path, &persistence_path)
            .map_err(|e| anyhow::anyhow!("Failed to rename persistence file: {}", e))?;
            
        self.last_persistence_timestamp.store(state.timestamp, Ordering::Relaxed);
        info!("💾 PropertyStore state force-persisted successfully");
        
        Ok(())
    }
}

impl Default for PropertyStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Global property store instance (lazy-initialized)
use std::sync::OnceLock;
static GLOBAL_PROPERTY_STORE: OnceLock<PropertyStore> = OnceLock::new();

/// Get global property store instance
pub fn get_property_store() -> &'static PropertyStore {
    GLOBAL_PROPERTY_STORE.get_or_init(|| PropertyStore::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_atomic_property_updates() {
        let store = PropertyStore::new();
        
        // Test atomic i32 property
        store.set_atomic_i32("min_profit_bps", 50, PropertySource::HumanUI).unwrap();
        assert_eq!(store.get_atomic_i32("min_profit_bps"), Some(50));
        
        // Test atomic f64 property
        store.set_atomic_f64("learning_rate", 0.25, PropertySource::AIOptimizer).unwrap();
        assert!((store.get_atomic_f64("learning_rate").unwrap() - 0.25).abs() < f64::EPSILON);
    }
    
    #[tokio::test]
    async fn test_event_subscription() {
        let store = PropertyStore::new();
        let mut receiver = store.subscribe();
        
        // Change a property
        store.set_atomic_i32("min_profit_bps", 75, PropertySource::AIOptimizer).unwrap();
        
        // Should receive event
        let event = receiver.recv().await.unwrap();
        assert_eq!(event.property_key, "min_profit_bps");
        assert!(matches!(event.source, PropertySource::AIOptimizer));
    }
    
    #[tokio::test]
    async fn test_concurrent_access() {
        let store = Arc::new(PropertyStore::new());
        let mut handles = vec![];
        
        // Spawn 100 concurrent tasks updating properties
        for i in 0..100 {
            let store_clone = Arc::clone(&store);
            handles.push(tokio::spawn(async move {
                store_clone.set_atomic_i32("min_profit_bps", i, PropertySource::AIOptimizer).unwrap();
            }));
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Should have some final value
        assert!(store.get_atomic_i32("min_profit_bps").is_some());
    }
}