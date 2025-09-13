use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, warn, error};

use crate::smart_filter::{SmartFilterConfig, SmartFilter, FilterStats};

/// Shared state for configuration management
#[derive(Clone)]
pub struct SharedConfigState {
    pub smart_filter: Arc<RwLock<SmartFilter>>,
    pub performance_tracker: Arc<RwLock<PerformanceTracker>>,
    pub config_history: Arc<RwLock<VecDeque<ConfigChange>>>,
    pub database_pool: Option<Arc<PgPool>>, // Added for PostgreSQL NOTIFY trigger integration
}

/// Configuration change tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub parameter: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub applied_by: String,
    pub performance_before: PerformanceSnapshot,
    pub performance_after: Option<PerformanceSnapshot>,
}

/// Performance snapshot for impact tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub opportunities_per_minute: f64,
    pub success_rate: f64,
    pub average_profit_bps: f64,
    pub execution_latency_us: u64,
    pub timestamp: DateTime<Utc>,
}

/// Performance tracking state
pub struct PerformanceTracker {
    pub last_config_update: DateTime<Utc>,
    pub opportunity_count_window: VecDeque<(DateTime<Utc>, u64)>,
    pub success_count_window: VecDeque<(DateTime<Utc>, bool)>,
    pub profit_window: VecDeque<(DateTime<Utc>, f64)>,
    pub latency_window: VecDeque<(DateTime<Utc>, u64)>,
    pub next_change_id: u64,
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self {
            last_config_update: Utc::now(),
            opportunity_count_window: VecDeque::new(),
            success_count_window: VecDeque::new(),
            profit_window: VecDeque::new(),
            latency_window: VecDeque::new(),
            next_change_id: 1,
        }
    }
}

impl PerformanceTracker {
    pub fn current_snapshot(&self) -> PerformanceSnapshot {
        let now = Utc::now();
        let five_minutes_ago = now - chrono::Duration::minutes(5);
        
        // Calculate opportunities per minute
        let recent_opportunities: u64 = self.opportunity_count_window
            .iter()
            .filter(|(timestamp, _)| *timestamp >= five_minutes_ago)
            .map(|(_, count)| *count)
            .sum();
        let opportunities_per_minute = recent_opportunities as f64 / 5.0;
        
        // Calculate success rate
        let recent_successes = self.success_count_window
            .iter()
            .filter(|(timestamp, _)| *timestamp >= five_minutes_ago)
            .count();
        let recent_attempts = self.success_count_window
            .iter()
            .filter(|(timestamp, _)| *timestamp >= five_minutes_ago)
            .filter(|(_, success)| *success)
            .count();
        let success_rate = if recent_attempts > 0 {
            recent_successes as f64 / recent_attempts as f64
        } else {
            0.0
        };
        
        // Calculate average profit
        let recent_profits: Vec<f64> = self.profit_window
            .iter()
            .filter(|(timestamp, _)| *timestamp >= five_minutes_ago)
            .map(|(_, profit)| *profit)
            .collect();
        let average_profit_bps = if !recent_profits.is_empty() {
            recent_profits.iter().sum::<f64>() / recent_profits.len() as f64
        } else {
            0.0
        };
        
        // Calculate average latency
        let recent_latencies: Vec<u64> = self.latency_window
            .iter()
            .filter(|(timestamp, _)| *timestamp >= five_minutes_ago)
            .map(|(_, latency)| *latency)
            .collect();
        let execution_latency_us = if !recent_latencies.is_empty() {
            recent_latencies.iter().sum::<u64>() / recent_latencies.len() as u64
        } else {
            0
        };
        
        PerformanceSnapshot {
            opportunities_per_minute,
            success_rate,
            average_profit_bps,
            execution_latency_us,
            timestamp: now,
        }
    }
    
    pub fn record_opportunity(&mut self, count: u64) {
        let now = Utc::now();
        self.opportunity_count_window.push_back((now, count));
        
        // Keep only last hour of data
        let one_hour_ago = now - chrono::Duration::hours(1);
        self.opportunity_count_window.retain(|(timestamp, _)| *timestamp >= one_hour_ago);
    }
    
    pub fn record_execution(&mut self, success: bool, profit_bps: f64, latency_us: u64) {
        let now = Utc::now();
        self.success_count_window.push_back((now, success));
        self.profit_window.push_back((now, profit_bps));
        self.latency_window.push_back((now, latency_us));
        
        // Keep only last hour of data
        let one_hour_ago = now - chrono::Duration::hours(1);
        self.success_count_window.retain(|(timestamp, _)| *timestamp >= one_hour_ago);
        self.profit_window.retain(|(timestamp, _)| *timestamp >= one_hour_ago);
        self.latency_window.retain(|(timestamp, _)| *timestamp >= one_hour_ago);
    }
    
    pub fn calculate_recent_impact(&self) -> Option<PerformanceImpact> {
        // This would compare performance before/after last config change
        // For now, return a placeholder - actual impact is calculated elsewhere
        Some(PerformanceImpact {
            opportunities_change_pct: 0.0,
            success_rate_change_pct: 0.0,
            profit_change_pct: 0.0,
            confidence_score: 0.5,
        })
    }
}

impl SharedConfigState {
    pub async fn new(smart_filter: SmartFilter) -> Self {
        Self {
            smart_filter: Arc::new(RwLock::new(smart_filter)),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker::default())),
            config_history: Arc::new(RwLock::new(VecDeque::new())),
            database_pool: None, // No database pool in basic constructor
        }
    }
    
    /// New constructor with database pool for PostgreSQL NOTIFY integration
    pub async fn new_with_database(smart_filter: SmartFilter, database_pool: Arc<PgPool>) -> Self {
        Self {
            smart_filter: Arc::new(RwLock::new(smart_filter)),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker::default())),
            config_history: Arc::new(RwLock::new(VecDeque::new())),
            database_pool: Some(database_pool),
        }
    }
}

/// Environment-aware configuration defaults
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Environment {
    Development,
    Staging, 
    Production,
}

impl Environment {
    pub fn from_env() -> Self {
        match std::env::var("ENVIRONMENT").as_deref() {
            Ok("production") | Ok("prod") => Environment::Production,
            Ok("staging") | Ok("stage") => Environment::Staging,
            _ => Environment::Development, // Default to development for safety
        }
    }
}


/// Database configuration loading functionality
pub async fn load_configuration_from_database(
    database_url: Option<&str>
) -> Result<SmartFilterConfig> {
    let database_url = match database_url {
        Some(url) => url.to_string(),
        None => std::env::var("DATABASE_URL")
            .context("DATABASE_URL not set and no URL provided")?,
    };
    
    info!("📖 Connecting to database for configuration loading...");
    let pool = PgPool::connect(&database_url).await
        .context("Failed to connect to database for configuration loading")?;
    
    // Query the current_smart_filter_config view to get latest configuration
    let rows = sqlx::query!(
        r#"
        SELECT parameter_name, new_value, timestamp, applied_by 
        FROM current_smart_filter_config 
        ORDER BY timestamp DESC
        "#
    )
    .fetch_all(&pool)
    .await
    .context("Failed to query configuration from database")?;
    
    if rows.is_empty() {
        info!("📝 No configuration found in database, will use environment defaults");
        return Err(anyhow::anyhow!("No configuration found in database"));
    }
    
    info!("📖 Found {} configuration parameters in database", rows.len());
    
    // Start with environment defaults and apply database overrides
    let env_name = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    let mut config = match env_name.as_str() {
        "production" | "prod" => SmartFilterConfig::production_defaults(),
        "staging" | "stage" => SmartFilterConfig::staging_defaults(),
        _ => SmartFilterConfig::development_defaults(),
    };
    
    // Apply database configuration values
    for row in rows {
        let param_name = &row.parameter_name;
        let value = &row.new_value;
        
        // Skip if either param_name or value is None
        if let (Some(name), Some(json_value)) = (param_name, value) {
            match name.as_str() {
                "min_net_profit_sol" => {
                    if let Ok(val) = serde_json::from_value::<f64>(json_value.clone()) {
                        config.min_net_profit_sol = val;
                        info!("📖 Loaded min_net_profit_sol: {}", val);
                    }
                },
                "min_profit_bps" => {
                    if let Ok(val) = serde_json::from_value::<i32>(json_value.clone()) {
                        config.min_profit_bps = val;
                        info!("📖 Loaded min_profit_bps: {}", val);
                    }
                },
                "max_profit_bps" => {
                    if let Ok(val) = serde_json::from_value::<i32>(json_value.clone()) {
                        config.max_profit_bps = val;
                        info!("📖 Loaded max_profit_bps: {}", val);
                    }
                },
                "min_venue_success_rate" => {
                    if let Ok(val) = serde_json::from_value::<f64>(json_value.clone()) {
                        config.min_venue_success_rate = val;
                        info!("📖 Loaded min_venue_success_rate: {}", val);
                    }
                },
                "max_gas_budget_pct" => {
                    if let Ok(val) = serde_json::from_value::<f64>(json_value.clone()) {
                        config.max_gas_budget_pct = val;
                        info!("📖 Loaded max_gas_budget_pct: {}", val);
                    }
                },
                "max_concurrent_opportunities" => {
                    if let Ok(val) = serde_json::from_value::<u32>(json_value.clone()) {
                        config.max_concurrent_opportunities = val;
                        info!("📖 Loaded max_concurrent_opportunities: {}", val);
                    }
                },
                "competition_lookback_ms" => {
                    if let Ok(val) = serde_json::from_value::<u64>(json_value.clone()) {
                        config.competition_lookback_ms = val;
                        info!("📖 Loaded competition_lookback_ms: {}", val);
                    }
                },
                "learning_rate" => {
                    if let Ok(val) = serde_json::from_value::<f64>(json_value.clone()) {
                        config.learning_rate = val;
                        info!("📖 Loaded learning_rate: {}", val);
                    }
                },
                "min_samples_for_learning" => {
                    if let Ok(val) = serde_json::from_value::<u64>(json_value.clone()) {
                        config.min_samples_for_learning = val;
                        info!("📖 Loaded min_samples_for_learning: {}", val);
                    }
                },
                _ => {
                    warn!("📖 Unknown configuration parameter in database: {}", name);
                }
            }
        }
    }
    
    // Validate the loaded configuration
    config.validate()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Loaded configuration failed validation")?;
    
    info!("✅ Successfully loaded and validated configuration from database");
    Ok(config)
}

/// Complete configuration loading with fallback hierarchy
pub async fn load_smart_filter_configuration(
    database_url: Option<&str>
) -> SmartFilterConfig {
    // Try to load from database first
    match load_configuration_from_database(database_url).await {
        Ok(config) => {
            info!("📖 Using configuration loaded from database");
            config
        },
        Err(e) => {
            warn!("⚠️ Could not load configuration from database: {}", e);
            let env_name = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
            
            match env_name.as_str() {
                "production" | "prod" => {
                    error!("🚨 Production environment but database config loading failed! Using safe fallback defaults");
                    SmartFilterConfig::safe_fallback_defaults()
                },
                "staging" | "stage" => {
                    info!("🔧 Using staging defaults");
                    SmartFilterConfig::staging_defaults()
                },
                _ => {
                    info!("🔧 Using development defaults");
                    SmartFilterConfig::development_defaults()
                }
            }
        }
    }
}

/// Request/Response types
#[derive(Serialize, Deserialize)]
pub struct ConfigUpdateRequest {
    pub parameter: String,
    pub value: serde_json::Value,
    pub applied_by: String,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub current_config: SmartFilterConfig,
    pub last_updated: DateTime<Utc>,
    pub performance_impact: Option<PerformanceImpact>,
}

#[derive(Serialize)]
pub struct PerformanceImpact {
    pub opportunities_change_pct: f64,
    pub success_rate_change_pct: f64,
    pub profit_change_pct: f64,
    pub confidence_score: f64, // 0-1, based on data age and sample size
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub filter_stats: FilterStats,
    pub venue_performance: HashMap<String, (f64, u64)>,
    pub current_performance: PerformanceSnapshot,
    pub config_changes_today: usize,
}

/// API endpoint handlers
pub async fn get_config(
    State(state): State<SharedConfigState>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let smart_filter = state.smart_filter.read().await;
    let performance = state.performance_tracker.read().await;
    
    Ok(Json(ConfigResponse {
        current_config: smart_filter.get_config().clone(),
        last_updated: performance.last_config_update,
        performance_impact: performance.calculate_recent_impact(),
    }))
}

pub async fn update_config(
    State(state): State<SharedConfigState>,
    Json(request): Json<ConfigUpdateRequest>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    // Validate parameter update
    let new_config = {
        let smart_filter = state.smart_filter.read().await;
        validate_and_apply_update(smart_filter.get_config(), &request)
            .map_err(|_| StatusCode::BAD_REQUEST)?
    };
    
    // Capture performance before change
    let performance_before = {
        let performance = state.performance_tracker.read().await;
        performance.current_snapshot()
    };
    
    // Apply configuration change
    {
        let mut smart_filter = state.smart_filter.write().await;
        smart_filter.update_config(new_config.clone());
    }
    
    // Record change for impact tracking
    let change_id = {
        let mut performance = state.performance_tracker.write().await;
        let id = performance.next_change_id;
        performance.next_change_id += 1;
        performance.last_config_update = Utc::now();
        id
    };
    
    // PHASE 2.2: Trigger PostgreSQL NOTIFY for real-time configuration updates (before moving values)
    if let Some(db_pool) = &state.database_pool {
        let parameter_name = request.parameter.clone();
        let new_value_json = serde_json::to_value(&request.value).unwrap_or(serde_json::Value::Null);
        let applied_by = request.applied_by.clone();
        
        if let Err(e) = sqlx::query!(
            r#"
            INSERT INTO smart_filter_config_history (parameter_name, new_value, applied_by, timestamp)
            VALUES ($1, $2, $3, $4)
            "#,
            parameter_name,
            new_value_json,
            applied_by,
            Utc::now()
        ).execute(db_pool.as_ref()).await {
            error!("❌ Failed to trigger PostgreSQL NOTIFY for config update: {}", e);
        } else {
            info!("📡 PostgreSQL NOTIFY triggered for parameter: {} -> {} (applied by {})", 
                parameter_name, new_value_json, applied_by);
        }
    } else {
        warn!("⚠️ No database pool available for PostgreSQL NOTIFY - real-time updates disabled");
    }

    let config_change = ConfigChange {
        id: change_id,
        timestamp: Utc::now(),
        parameter: request.parameter.clone(),
        old_value: {
            let smart_filter = state.smart_filter.read().await;
            get_parameter_value(smart_filter.get_config(), &request.parameter)
                .unwrap_or(serde_json::Value::Null)
        },
        new_value: request.value,
        applied_by: request.applied_by,
        performance_before,
        performance_after: None,
    };
    
    {
        let mut config_history = state.config_history.write().await;
        config_history.push_back(config_change);
        
        // Keep only last 100 changes
        if config_history.len() > 100 {
            config_history.pop_front();
        }
    }
    
    log::info!(
        "🔧 Smart filter parameter updated: {} (change ID: {})",
        request.parameter,
        change_id
    );
    
    Ok(Json(ConfigResponse {
        current_config: new_config,
        last_updated: Utc::now(),
        performance_impact: None, // Too early to measure impact
    }))
}

pub async fn get_stats(
    State(state): State<SharedConfigState>,
) -> Result<Json<StatsResponse>, StatusCode> {
    let smart_filter = state.smart_filter.read().await;
    let performance = state.performance_tracker.read().await;
    let config_history = state.config_history.read().await;
    
    let filter_stats = smart_filter.get_stats().await;
    let venue_performance = smart_filter.get_venue_performance().await;
    let current_performance = performance.current_snapshot();
    
    // Count config changes from today
    let today = Utc::now().date_naive();
    let config_changes_today = config_history
        .iter()
        .filter(|change| change.timestamp.date_naive() == today)
        .count();
    
    Ok(Json(StatsResponse {
        filter_stats,
        venue_performance,
        current_performance,
        config_changes_today,
    }))
}

pub async fn get_performance_impact(
    State(state): State<SharedConfigState>,
) -> Result<Json<Vec<ConfigChange>>, StatusCode> {
    let config_history = state.config_history.read().await;
    let recent_changes: Vec<ConfigChange> = config_history
        .iter()
        .rev()  // Most recent first
        .take(20)  // Last 20 changes
        .cloned()
        .collect();
    
    Ok(Json(recent_changes))
}

pub async fn reset_config(
    State(state): State<SharedConfigState>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let default_config = SmartFilterConfig::default();
    
    // Capture performance before reset
    let performance_before = {
        let performance = state.performance_tracker.read().await;
        performance.current_snapshot()
    };
    
    // Apply reset
    {
        let mut smart_filter = state.smart_filter.write().await;
        smart_filter.update_config(default_config.clone());
    }
    
    // Record reset
    let change_id = {
        let mut performance = state.performance_tracker.write().await;
        let id = performance.next_change_id;
        performance.next_change_id += 1;
        performance.last_config_update = Utc::now();
        id
    };
    
    let config_change = ConfigChange {
        id: change_id,
        timestamp: Utc::now(),
        parameter: "all_parameters".to_string(),
        old_value: serde_json::Value::String("previous_config".to_string()),
        new_value: serde_json::Value::String("default_config".to_string()),
        applied_by: "system_reset".to_string(),
        performance_before,
        performance_after: None,
    };
    
    {
        let mut config_history = state.config_history.write().await;
        config_history.push_back(config_change);
    }
    
    log::info!("🔄 Smart filter configuration reset to defaults");
    
    Ok(Json(ConfigResponse {
        current_config: default_config,
        last_updated: Utc::now(),
        performance_impact: None,
    }))
}

/// Create the router for configuration API
pub fn create_config_router(state: SharedConfigState) -> Router {
    Router::new()
        .route("/smart-filter/config", get(get_config).put(update_config))
        .route("/smart-filter/stats", get(get_stats))
        .route("/smart-filter/performance", get(get_performance_impact))
        .route("/smart-filter/reset", put(reset_config))
        .with_state(state)
}

/// Helper function to validate and apply configuration updates
fn validate_and_apply_update(
    current_config: &SmartFilterConfig,
    request: &ConfigUpdateRequest,
) -> Result<SmartFilterConfig> {
    let mut new_config = current_config.clone();
    
    match request.parameter.as_str() {
        "min_net_profit_sol" => {
            let value: f64 = serde_json::from_value(request.value.clone())
                .context("Invalid value for min_net_profit_sol")?;
            if value < 0.0001 || value > 0.1 {
                return Err(anyhow::anyhow!("min_net_profit_sol must be between 0.0001 and 0.1"));
            }
            new_config.min_net_profit_sol = value;
        }
        "min_profit_bps" => {
            let value: i32 = serde_json::from_value(request.value.clone())
                .context("Invalid value for min_profit_bps")?;
            if value < 1 || value > 100 {
                return Err(anyhow::anyhow!("min_profit_bps must be between 1 and 100"));
            }
            new_config.min_profit_bps = value;
        }
        "max_profit_bps" => {
            let value: i32 = serde_json::from_value(request.value.clone())
                .context("Invalid value for max_profit_bps")?;
            if value < 50 || value > 500 {
                return Err(anyhow::anyhow!("max_profit_bps must be between 50 and 500"));
            }
            new_config.max_profit_bps = value;
        }
        "min_venue_success_rate" => {
            let value: f64 = serde_json::from_value(request.value.clone())
                .context("Invalid value for min_venue_success_rate")?;
            if value < 0.0 || value > 1.0 {
                return Err(anyhow::anyhow!("min_venue_success_rate must be between 0.0 and 1.0"));
            }
            new_config.min_venue_success_rate = value;
        }
        "max_gas_budget_pct" => {
            let value: f64 = serde_json::from_value(request.value.clone())
                .context("Invalid value for max_gas_budget_pct")?;
            if value < 5.0 || value > 80.0 {
                return Err(anyhow::anyhow!("max_gas_budget_pct must be between 5.0 and 80.0"));
            }
            new_config.max_gas_budget_pct = value;
        }
        "max_concurrent_opportunities" => {
            let value: u32 = serde_json::from_value(request.value.clone())
                .context("Invalid value for max_concurrent_opportunities")?;
            if value < 1 || value > 10 {
                return Err(anyhow::anyhow!("max_concurrent_opportunities must be between 1 and 10"));
            }
            new_config.max_concurrent_opportunities = value;
        }
        "competition_lookback_ms" => {
            let value: u64 = serde_json::from_value(request.value.clone())
                .context("Invalid value for competition_lookback_ms")?;
            if value < 1000 || value > 30000 {
                return Err(anyhow::anyhow!("competition_lookback_ms must be between 1000 and 30000"));
            }
            new_config.competition_lookback_ms = value;
        }
        "learning_rate" => {
            let value: f64 = serde_json::from_value(request.value.clone())
                .context("Invalid value for learning_rate")?;
            if value < 0.01 || value > 1.0 {
                return Err(anyhow::anyhow!("learning_rate must be between 0.01 and 1.0"));
            }
            new_config.learning_rate = value;
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown parameter: {}", request.parameter));
        }
    }
    
    Ok(new_config)
}

/// Helper function to get current parameter value as JSON
fn get_parameter_value(config: &SmartFilterConfig, parameter: &str) -> Option<serde_json::Value> {
    match parameter {
        "min_net_profit_sol" => Some(serde_json::json!(config.min_net_profit_sol)),
        "min_profit_bps" => Some(serde_json::json!(config.min_profit_bps)),
        "max_profit_bps" => Some(serde_json::json!(config.max_profit_bps)),
        "min_venue_success_rate" => Some(serde_json::json!(config.min_venue_success_rate)),
        "max_gas_budget_pct" => Some(serde_json::json!(config.max_gas_budget_pct)),
        "max_concurrent_opportunities" => Some(serde_json::json!(config.max_concurrent_opportunities)),
        "competition_lookback_ms" => Some(serde_json::json!(config.competition_lookback_ms)),
        "learning_rate" => Some(serde_json::json!(config.learning_rate)),
        _ => None,
    }
}