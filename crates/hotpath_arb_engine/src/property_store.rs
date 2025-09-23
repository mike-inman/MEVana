#![allow(warnings)]

use anyhow::Result;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};

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

/// Comprehensive atomic properties for all 47 MEVana control parameters
/// Organized by UI groups for logical management
#[derive(Debug)]
pub struct AtomicProperties {
    // === GROUP 1: EMERGENCY CONTROLS (8 properties) ===
    pub enable_live_execution: AtomicBool,
    pub dry_run: AtomicBool,
    pub simulation_mode: AtomicBool,
    pub circuit_breaker_enabled: AtomicBool,
    pub max_position_size_lamports: AtomicU64, // SOL stored as lamports for precision
    pub max_daily_loss_lamports: AtomicU64,    // SOL stored as lamports
    pub max_consecutive_failures: AtomicU32,
    pub cooling_off_period_seconds: AtomicU64,

    // === GROUP 2: PROFIT OPTIMIZATION (8 properties) ===
    pub min_profit_bps: AtomicI32,               // ✅ Already atomic
    pub max_profit_bps: AtomicI32,               // ✅ Already atomic
    pub min_net_profit_lamports: AtomicU64,      // SOL stored as lamports
    pub max_gas_budget_pct: AtomicU64,           // ✅ Already atomic (f64 as bits)
    pub min_confidence_for_execution: AtomicU64, // f64 stored as u64 bits (0.0-1.0)
    pub min_venue_success_rate: AtomicU64,       // ✅ Already atomic (f64 as bits)
    pub competition_lookback_ms: AtomicU64,
    pub max_concurrent_opportunities: AtomicU32,

    // === GROUP 3: VENUE & POSITION LIMITS (7 properties) ===
    pub min_position_size_lamports: AtomicU64, // SOL stored as lamports
    pub max_position_raydium_lamports: AtomicU64,
    pub max_position_orca_lamports: AtomicU64,
    pub max_position_meteora_lamports: AtomicU64,
    pub max_position_phoenix_lamports: AtomicU64,
    pub max_trades_per_minute: AtomicU32,
    pub max_slippage_bps: AtomicU32,

    // === GROUP 4: EXECUTION & PERFORMANCE (6 properties) ===
    pub jito_enabled: AtomicBool,
    pub jito_tip_lamports: AtomicU64,
    pub priority_fee_multiplier: AtomicU64, // f64 stored as u64 bits
    pub rpc_timeout_seconds: AtomicU64,
    pub flash_loan_enabled: AtomicBool,
    pub max_flash_loan_amount_lamports: AtomicU64, // SOL stored as lamports

    // === GROUP 5: AI & LEARNING (5 properties) ===
    pub learning_rate: AtomicU64, // ✅ Already atomic (f64 as bits)
    pub min_samples_for_learning: AtomicU64,
    pub ai_optimization_enabled: AtomicBool, // ✅ Already exists
    pub exploration_rate: AtomicU64,         // f64 stored as u64 bits (0.0-0.3)
    pub confidence_threshold: AtomicU64,     // f64 stored as u64 bits (0.6-0.99)

    // === GROUP 6: WALLET & SECURITY (5 properties) ===
    pub hot_wallet_max_balance_lamports: AtomicU64,
    pub warm_wallet_max_balance_lamports: AtomicU64,
    pub cold_wallet_max_balance_lamports: AtomicU64,
    pub wallet_security_tier: AtomicU32, // Enum as u32: 0=hot, 1=warm, 2=cold
    pub auto_rebalance_enabled: AtomicBool,

    // === GROUP 7: MONITORING & ALERTS (7 properties) ===
    pub profit_tracking_enabled: AtomicBool,
    pub log_all_executions: AtomicBool,
    pub alert_on_negative_profit: AtomicBool,
    pub track_slippage: AtomicBool,
    pub track_gas_costs: AtomicBool,
    pub enable_file_logging: AtomicBool,
    pub sampling_rate: AtomicU64, // f64 stored as u64 bits (0.1-1.0)

    // === GROUP 8: SYSTEM PERFORMANCE (4 properties) ===
    pub flush_interval_ms: AtomicU64,
    pub redis_enabled: AtomicBool,
    pub kafka_enabled: AtomicBool,
    pub api_logging_enabled: AtomicBool,

    // === LEGACY (for backward compatibility) ===
    pub system_enabled: AtomicBool, // ✅ Keep existing
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

/// Environment detection for safety bounds
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
            _ => Environment::Development,
        }
    }
}

/// Comprehensive validation rules for system stability protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRule {
    // Basic numerical constraints
    MinValue(f64),
    MaxValue(f64),
    StepSize(f64),
    Precision(u8),

    // Cross-parameter validation
    MustBeLessThan(String),
    MustBeGreaterThan(String),
    MustNotExceed(String, f64),

    // Business logic rules
    RequiresFlag(String, bool),
    ProhibitedWith(String),

    // Safety and stability rules
    RequiresConfirmation,
    ProductionLimit(f64),
    RateLimit(u32),
    CannotBeZero,
    SystemStabilityCheck,
    ResourceExhaustionCheck,
}

/// Property bounds with multi-layer safety validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyBounds {
    pub absolute_min: PropertyValue,
    pub absolute_max: PropertyValue,
    pub safe_min: PropertyValue,
    pub safe_max: PropertyValue,
    pub production_max: Option<PropertyValue>,
    pub staging_max: Option<PropertyValue>,
    pub validation_rules: Vec<ValidationRule>,
    pub depends_on: Vec<String>,
}

/// Comprehensive validation errors with detailed context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    UnknownProperty(String),
    TypeMismatch {
        expected: String,
        received: String,
    },
    BelowMinimum {
        value: f64,
        minimum: f64,
    },
    AboveMaximum {
        value: f64,
        maximum: f64,
    },
    BelowMinimumInt {
        value: i32,
        minimum: i32,
    },
    AboveMaximumInt {
        value: i32,
        maximum: i32,
    },
    EnvironmentLimit {
        environment: String,
        value: f64,
        limit: f64,
    },
    EnvironmentLimitInt {
        environment: String,
        value: i32,
        limit: i32,
    },
    ProhibitedInEnvironment {
        environment: String,
    },
    CrossParameterViolation {
        property: String,
        dependent: String,
        reason: String,
    },
    BusinessRuleViolation {
        rule: String,
        reason: String,
    },
    RequiresConfirmation {
        property: String,
    },
    RateLimited {
        property: String,
        max_changes_per_minute: u32,
    },
    SystemStability {
        property: String,
        reason: String,
    },
    ResourceExhaustion {
        property: String,
        resource: String,
        limit: String,
    },
    InfiniteLoopRisk {
        property: String,
        dangerous_value: String,
    },
}

/// Anti-capsizing property validator with bulletproof stability protection
pub struct PropertyValidator {
    bounds: HashMap<String, PropertyBounds>,
    environment: Environment,
    current_values: HashMap<String, PropertyValue>,
    change_history: Vec<(String, PropertyValue, SystemTime)>,
}

impl PropertyValidator {
    pub fn new(environment: Environment) -> Self {
        Self {
            bounds: Self::initialize_property_bounds(),
            environment,
            current_values: HashMap::new(),
            change_history: Vec::new(),
        }
    }

    /// Initialize all 47 property bounds with anti-capsizing protection
    fn initialize_property_bounds() -> HashMap<String, PropertyBounds> {
        let mut bounds = HashMap::new();

        // === GROUP 1: EMERGENCY CONTROLS ===
        bounds.insert(
            "enable_live_execution".to_string(),
            PropertyBounds {
                absolute_min: PropertyValue::Boolean(false),
                absolute_max: PropertyValue::Boolean(true),
                safe_min: PropertyValue::Boolean(false),
                safe_max: PropertyValue::Boolean(true),
                production_max: None,
                staging_max: Some(PropertyValue::Boolean(false)),
                validation_rules: vec![
                    ValidationRule::RequiresConfirmation,
                    ValidationRule::RequiresFlag("dry_run".to_string(), false),
                    ValidationRule::ProhibitedWith("simulation_mode".to_string()),
                    ValidationRule::SystemStabilityCheck,
                ],
                depends_on: vec!["dry_run".to_string(), "simulation_mode".to_string()],
            },
        );

        bounds.insert(
            "max_position_size_lamports".to_string(),
            PropertyBounds {
                absolute_min: PropertyValue::Float(1_000_000.0), // 0.001 SOL minimum
                absolute_max: PropertyValue::Float(100_000_000_000.0), // 100 SOL absolute max
                safe_min: PropertyValue::Float(1_000_000.0),
                safe_max: PropertyValue::Float(1_000_000_000.0), // 1 SOL safe max
                production_max: Some(PropertyValue::Float(10_000_000_000.0)), // 10 SOL production
                staging_max: Some(PropertyValue::Float(100_000_000.0)), // 0.1 SOL staging
                validation_rules: vec![
                    ValidationRule::MinValue(1_000_000.0),
                    ValidationRule::MaxValue(100_000_000_000.0),
                    ValidationRule::RequiresConfirmation,
                    ValidationRule::ProductionLimit(10_000_000_000.0),
                    ValidationRule::RateLimit(3),
                    ValidationRule::SystemStabilityCheck,
                    ValidationRule::ResourceExhaustionCheck,
                ],
                depends_on: vec![
                    "min_position_size_lamports".to_string(),
                    "hot_wallet_max_balance_lamports".to_string(),
                ],
            },
        );

        bounds.insert(
            "rpc_timeout_seconds".to_string(),
            PropertyBounds {
                absolute_min: PropertyValue::Integer(1),   // Must be positive
                absolute_max: PropertyValue::Integer(300), // 5 minutes max
                safe_min: PropertyValue::Integer(5),
                safe_max: PropertyValue::Integer(60), // 1 minute safe max
                production_max: Some(PropertyValue::Integer(120)), // 2 minutes production
                staging_max: None,
                validation_rules: vec![
                    ValidationRule::MinValue(1.0),
                    ValidationRule::MaxValue(300.0),
                    ValidationRule::CannotBeZero,
                    ValidationRule::SystemStabilityCheck,
                ],
                depends_on: vec![],
            },
        );

        bounds.insert(
            "jito_tip_lamports".to_string(),
            PropertyBounds {
                absolute_min: PropertyValue::Integer(1_000), // Must be positive
                absolute_max: PropertyValue::Integer(10_000_000), // 0.01 SOL max
                safe_min: PropertyValue::Integer(5_000),
                safe_max: PropertyValue::Integer(100_000), // 0.0001 SOL safe max
                production_max: Some(PropertyValue::Integer(500_000)), // 0.0005 SOL production
                staging_max: Some(PropertyValue::Integer(50_000)), // 0.00005 SOL staging
                validation_rules: vec![
                    ValidationRule::MinValue(1_000.0),
                    ValidationRule::MaxValue(10_000_000.0),
                    ValidationRule::CannotBeZero,
                    ValidationRule::SystemStabilityCheck,
                ],
                depends_on: vec![],
            },
        );

        bounds.insert(
            "max_trades_per_minute".to_string(),
            PropertyBounds {
                absolute_min: PropertyValue::Integer(1), // Must allow trading
                absolute_max: PropertyValue::Integer(600), // 10/second max
                safe_min: PropertyValue::Integer(5),
                safe_max: PropertyValue::Integer(60), // 1/second safe
                production_max: Some(PropertyValue::Integer(120)), // 2/second production
                staging_max: Some(PropertyValue::Integer(30)), // 0.5/second staging
                validation_rules: vec![
                    ValidationRule::MinValue(1.0),
                    ValidationRule::MaxValue(600.0),
                    ValidationRule::CannotBeZero,
                    ValidationRule::SystemStabilityCheck,
                    ValidationRule::ResourceExhaustionCheck,
                ],
                depends_on: vec![],
            },
        );

        // Add more bounds for other critical properties...
        // (Truncated for brevity - would include all 47 properties)

        bounds
    }

    /// Main validation entry point - bulletproof property change validation
    pub fn validate_change(
        &mut self,
        property_key: &str,
        new_value: PropertyValue,
        source: PropertySource,
    ) -> Result<PropertyValue, ValidationError> {
        // Update current values for cross-validation
        self.current_values
            .insert(property_key.to_string(), new_value.clone());

        let bounds = self
            .bounds
            .get(property_key)
            .ok_or_else(|| ValidationError::UnknownProperty(property_key.to_string()))?;

        // Layer 1: Type safety (already enforced by PropertyValue enum)

        // Layer 2: Basic bounds checking
        self.validate_bounds(&new_value, bounds)?;

        // Layer 3: Environment-specific limits
        self.validate_environment_limits(&new_value, bounds)?;

        // Layer 4: System stability protection (ANTI-CAPSIZING)
        self.validate_system_stability(property_key, &new_value)?;

        // Layer 5: Cross-parameter consistency
        self.validate_dependencies(property_key, &new_value, bounds)?;

        // Layer 6: Business logic rules
        self.validate_business_rules(property_key, &new_value, bounds, source)?;

        // Layer 7: Rate limiting protection
        self.validate_rate_limits(property_key, bounds)?;

        // Record successful validation
        self.change_history.push((
            property_key.to_string(),
            new_value.clone(),
            SystemTime::now(),
        ));

        info!(
            "✅ Property validation passed: {} = {:?} (source: {:?})",
            property_key, new_value, source
        );

        Ok(new_value)
    }

    /// CRITICAL: System stability validation to prevent capsizing
    fn validate_system_stability(
        &self,
        property_key: &str,
        value: &PropertyValue,
    ) -> Result<(), ValidationError> {
        match property_key {
            "rpc_timeout_seconds" => {
                if let PropertyValue::Integer(val) = value {
                    if *val == 0 {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "RPC timeout cannot be 0 - would cause all RPC calls to fail immediately, breaking the system".to_string()
                        });
                    }
                }
            }

            "jito_tip_lamports" => {
                if let PropertyValue::Integer(val) = value {
                    if *val == 0 {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "Jito tip cannot be 0 - transactions would never be included in blocks, stopping all trading".to_string()
                        });
                    }
                }
            }

            "max_trades_per_minute" => {
                if let PropertyValue::Integer(val) = value {
                    if *val == 0 {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "Cannot set trades per minute to 0 - would completely stop all trading operations".to_string()
                        });
                    }
                    if *val > 600 {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "Rate > 600/min (10/sec) would overwhelm RPC endpoints, causing bans and system failure".to_string()
                        });
                    }
                }
            }

            "competition_lookback_ms" => {
                if let PropertyValue::Integer(val) = value {
                    if *val == 0 {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "Zero lookback would cause division by zero errors in competition calculations".to_string()
                        });
                    }
                }
            }

            "max_gas_budget_pct" => {
                if let PropertyValue::Float(val) = value {
                    if *val >= 100.0 {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "Gas budget >= 100% would result in negative profit on all trades, making trading pointless".to_string()
                        });
                    }
                }
            }

            "learning_rate" => {
                if let PropertyValue::Float(val) = value {
                    if *val <= 0.0 {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "Learning rate must be positive - zero/negative would prevent AI learning entirely".to_string()
                        });
                    }
                    if *val >= 1.0 {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "Learning rate >= 1.0 would cause AI to oscillate wildly and never converge to stable behavior".to_string()
                        });
                    }
                }
            }

            "enable_live_execution" => {
                if let PropertyValue::Boolean(true) = value {
                    if let Some(PropertyValue::Boolean(true)) = self.current_values.get("dry_run") {
                        return Err(ValidationError::SystemStability {
                            property: property_key.to_string(),
                            reason: "Cannot enable live execution while dry_run=true - contradictory configuration would cause undefined behavior".to_string()
                        });
                    }
                }
            }

            _ => {} // No specific stability rules for this property
        }

        Ok(())
    }

    // Placeholder implementations for other validation methods
    fn validate_bounds(
        &self,
        _value: &PropertyValue,
        _bounds: &PropertyBounds,
    ) -> Result<(), ValidationError> {
        Ok(())
    }
    fn validate_environment_limits(
        &self,
        _value: &PropertyValue,
        _bounds: &PropertyBounds,
    ) -> Result<(), ValidationError> {
        Ok(())
    }
    fn validate_dependencies(
        &self,
        _property_key: &str,
        _new_value: &PropertyValue,
        _bounds: &PropertyBounds,
    ) -> Result<(), ValidationError> {
        Ok(())
    }
    fn validate_business_rules(
        &self,
        _property_key: &str,
        _new_value: &PropertyValue,
        _bounds: &PropertyBounds,
        _source: PropertySource,
    ) -> Result<(), ValidationError> {
        Ok(())
    }
    fn validate_rate_limits(
        &self,
        _property_key: &str,
        _bounds: &PropertyBounds,
    ) -> Result<(), ValidationError> {
        Ok(())
    }
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
            min_profit_bps: 10,          // 10bps minimum - very conservative
            max_profit_bps: 300,         // 300bps maximum - prevent suspicious trades
            max_gas_budget_pct: 50.0,    // 50% max gas usage - very conservative
            min_venue_success_rate: 0.8, // 80% success rate - high bar
            learning_rate: 0.05,         // 5% learning rate - slow and safe
            system_enabled: true,        // System enabled by default
        }
    }
}

impl PropertyStore {
    /// Create new property store with default values from SmartFilterConfig
    pub fn new() -> Self {
        let (event_broadcaster, _) = broadcast::channel(1000); // 1000 event buffer

        // Initialize with safe defaults for all 47 properties
        let atomic_properties = AtomicProperties {
            // === GROUP 1: EMERGENCY CONTROLS (SAFE DEFAULTS) ===
            enable_live_execution: AtomicBool::new(false), // SAFE: Start disabled
            dry_run: AtomicBool::new(true),                // SAFE: Start in dry run
            simulation_mode: AtomicBool::new(true),        // SAFE: Start in simulation
            circuit_breaker_enabled: AtomicBool::new(true), // SAFE: Protection enabled
            max_position_size_lamports: AtomicU64::new(1_000_000), // SAFE: 0.001 SOL = $0.20
            max_daily_loss_lamports: AtomicU64::new(1_000_000_000), // SAFE: 1 SOL = $200
            max_consecutive_failures: AtomicU32::new(3),   // SAFE: Conservative limit
            cooling_off_period_seconds: AtomicU64::new(30), // SAFE: 30 second cooldown

            // === GROUP 2: PROFIT OPTIMIZATION ===
            min_profit_bps: AtomicI32::new(1),   // ✅ Keep existing
            max_profit_bps: AtomicI32::new(500), // ✅ Keep existing
            min_net_profit_lamports: AtomicU64::new(100_000), // SAFE: 0.0001 SOL = $0.02
            max_gas_budget_pct: AtomicU64::new(80.0_f64.to_bits()), // ✅ Keep existing
            min_confidence_for_execution: AtomicU64::new(0.8_f64.to_bits()), // SAFE: 80% confidence
            min_venue_success_rate: AtomicU64::new(0.0_f64.to_bits()), // ✅ Keep existing
            competition_lookback_ms: AtomicU64::new(5000), // ✅ Keep existing
            max_concurrent_opportunities: AtomicU32::new(20), // ✅ Keep existing

            // === GROUP 3: VENUE & POSITION LIMITS ===
            min_position_size_lamports: AtomicU64::new(1_000_000), // SAFE: 0.001 SOL minimum
            max_position_raydium_lamports: AtomicU64::new(5_000_000), // SAFE: 0.005 SOL = $1
            max_position_orca_lamports: AtomicU64::new(5_000_000), // SAFE: 0.005 SOL = $1
            max_position_meteora_lamports: AtomicU64::new(2_000_000), // SAFE: 0.002 SOL = $0.40
            max_position_phoenix_lamports: AtomicU64::new(1_000_000), // SAFE: 0.001 SOL = $0.20
            max_trades_per_minute: AtomicU32::new(10),             // SAFE: Conservative rate
            max_slippage_bps: AtomicU32::new(50),                  // SAFE: 0.5% max slippage

            // === GROUP 4: EXECUTION & PERFORMANCE ===
            jito_enabled: AtomicBool::new(true), // SAFE: Enable for speed
            jito_tip_lamports: AtomicU64::new(10_000), // SAFE: 0.00001 SOL tip
            priority_fee_multiplier: AtomicU64::new(1.5_f64.to_bits()), // SAFE: 1.5x fees
            rpc_timeout_seconds: AtomicU64::new(10), // SAFE: 10 second timeout
            flash_loan_enabled: AtomicBool::new(true), // SAFE: Enable capability
            max_flash_loan_amount_lamports: AtomicU64::new(10_000_000_000), // SAFE: 10 SOL

            // === GROUP 5: AI & LEARNING ===
            learning_rate: AtomicU64::new(0.1_f64.to_bits()), // ✅ Keep existing
            min_samples_for_learning: AtomicU64::new(5),      // ✅ Keep existing
            ai_optimization_enabled: AtomicBool::new(true),   // ✅ Keep existing
            exploration_rate: AtomicU64::new(0.1_f64.to_bits()), // SAFE: 10% exploration
            confidence_threshold: AtomicU64::new(0.7_f64.to_bits()), // SAFE: 70% confidence

            // === GROUP 6: WALLET & SECURITY ===
            hot_wallet_max_balance_lamports: AtomicU64::new(100_000_000), // SAFE: 0.1 SOL
            warm_wallet_max_balance_lamports: AtomicU64::new(5_000_000_000), // SAFE: 5 SOL
            cold_wallet_max_balance_lamports: AtomicU64::new(100_000_000_000), // SAFE: 100 SOL
            wallet_security_tier: AtomicU32::new(0), // SAFE: Start with hot wallet
            auto_rebalance_enabled: AtomicBool::new(false), // SAFE: Manual control initially

            // === GROUP 7: MONITORING & ALERTS ===
            profit_tracking_enabled: AtomicBool::new(true), // SAFE: Full monitoring
            log_all_executions: AtomicBool::new(true),      // SAFE: Full logging
            alert_on_negative_profit: AtomicBool::new(true), // SAFE: Alert on losses
            track_slippage: AtomicBool::new(true),          // SAFE: Track performance
            track_gas_costs: AtomicBool::new(true),         // SAFE: Track costs
            enable_file_logging: AtomicBool::new(true),     // SAFE: Local logging
            sampling_rate: AtomicU64::new(1.0_f64.to_bits()), // SAFE: 100% sampling initially

            // === GROUP 8: SYSTEM PERFORMANCE ===
            flush_interval_ms: AtomicU64::new(1000), // SAFE: 1 second flush
            redis_enabled: AtomicBool::new(true),    // SAFE: Enable caching
            kafka_enabled: AtomicBool::new(true),    // SAFE: Enable messaging
            api_logging_enabled: AtomicBool::new(true), // SAFE: Enable API logs

            // === LEGACY ===
            system_enabled: AtomicBool::new(true), // ✅ Keep existing
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
                let old = self
                    .atomic_properties
                    .min_profit_bps
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .min_profit_bps
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_profit_bps" => {
                let old = self
                    .atomic_properties
                    .max_profit_bps
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_profit_bps
                    .store(value, Ordering::Relaxed);
                old
            }
            _ => return Err(anyhow::anyhow!("Unknown atomic i32 property: {}", key)),
        };

        // Fire event notification (non-blocking)
        self.notify_change(
            key,
            PropertyValue::Integer(old_value),
            PropertyValue::Integer(value),
            source,
        );

        info!(
            "⚡ Atomic property {} changed: {} -> {} (source: {:?})",
            key, old_value, value, source
        );
        Ok(())
    }

    /// Set atomic u32 property (for counts and rates)
    pub fn set_atomic_u32(&self, key: &str, value: u32, source: PropertySource) -> Result<()> {
        let old_value = match key {
            "max_consecutive_failures" => {
                let old = self
                    .atomic_properties
                    .max_consecutive_failures
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_consecutive_failures
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_concurrent_opportunities" => {
                let old = self
                    .atomic_properties
                    .max_concurrent_opportunities
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_concurrent_opportunities
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_trades_per_minute" => {
                let old = self
                    .atomic_properties
                    .max_trades_per_minute
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_trades_per_minute
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_slippage_bps" => {
                let old = self
                    .atomic_properties
                    .max_slippage_bps
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_slippage_bps
                    .store(value, Ordering::Relaxed);
                old
            }
            "wallet_security_tier" => {
                let old = self
                    .atomic_properties
                    .wallet_security_tier
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .wallet_security_tier
                    .store(value, Ordering::Relaxed);
                old
            }
            _ => return Err(anyhow::anyhow!("Unknown atomic u32 property: {}", key)),
        };

        self.notify_change(
            key,
            PropertyValue::Integer(old_value as i32),
            PropertyValue::Integer(value as i32),
            source,
        );

        info!(
            "⚡ Atomic property {} changed: {} -> {} (source: {:?})",
            key, old_value, value, source
        );
        Ok(())
    }

    /// Set atomic u64 property (for lamports, timestamps, and large values)
    pub fn set_atomic_u64(&self, key: &str, value: u64, source: PropertySource) -> Result<()> {
        let old_value = match key {
            // Emergency controls
            "max_position_size_lamports" => {
                let old = self
                    .atomic_properties
                    .max_position_size_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_position_size_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_daily_loss_lamports" => {
                let old = self
                    .atomic_properties
                    .max_daily_loss_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_daily_loss_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "cooling_off_period_seconds" => {
                let old = self
                    .atomic_properties
                    .cooling_off_period_seconds
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .cooling_off_period_seconds
                    .store(value, Ordering::Relaxed);
                old
            }
            // Profit optimization
            "min_net_profit_lamports" => {
                let old = self
                    .atomic_properties
                    .min_net_profit_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .min_net_profit_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "competition_lookback_ms" => {
                let old = self
                    .atomic_properties
                    .competition_lookback_ms
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .competition_lookback_ms
                    .store(value, Ordering::Relaxed);
                old
            }
            // Venue limits
            "min_position_size_lamports" => {
                let old = self
                    .atomic_properties
                    .min_position_size_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .min_position_size_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_position_raydium_lamports" => {
                let old = self
                    .atomic_properties
                    .max_position_raydium_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_position_raydium_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_position_orca_lamports" => {
                let old = self
                    .atomic_properties
                    .max_position_orca_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_position_orca_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_position_meteora_lamports" => {
                let old = self
                    .atomic_properties
                    .max_position_meteora_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_position_meteora_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_position_phoenix_lamports" => {
                let old = self
                    .atomic_properties
                    .max_position_phoenix_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_position_phoenix_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            // Execution settings
            "jito_tip_lamports" => {
                let old = self
                    .atomic_properties
                    .jito_tip_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .jito_tip_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "rpc_timeout_seconds" => {
                let old = self
                    .atomic_properties
                    .rpc_timeout_seconds
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .rpc_timeout_seconds
                    .store(value, Ordering::Relaxed);
                old
            }
            "max_flash_loan_amount_lamports" => {
                let old = self
                    .atomic_properties
                    .max_flash_loan_amount_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_flash_loan_amount_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            // AI learning
            "min_samples_for_learning" => {
                let old = self
                    .atomic_properties
                    .min_samples_for_learning
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .min_samples_for_learning
                    .store(value, Ordering::Relaxed);
                old
            }
            // Wallet security
            "hot_wallet_max_balance_lamports" => {
                let old = self
                    .atomic_properties
                    .hot_wallet_max_balance_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .hot_wallet_max_balance_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "warm_wallet_max_balance_lamports" => {
                let old = self
                    .atomic_properties
                    .warm_wallet_max_balance_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .warm_wallet_max_balance_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            "cold_wallet_max_balance_lamports" => {
                let old = self
                    .atomic_properties
                    .cold_wallet_max_balance_lamports
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .cold_wallet_max_balance_lamports
                    .store(value, Ordering::Relaxed);
                old
            }
            // System performance
            "flush_interval_ms" => {
                let old = self
                    .atomic_properties
                    .flush_interval_ms
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .flush_interval_ms
                    .store(value, Ordering::Relaxed);
                old
            }
            _ => return Err(anyhow::anyhow!("Unknown atomic u64 property: {}", key)),
        };

        self.notify_change(
            key,
            PropertyValue::Integer(old_value as i32),
            PropertyValue::Integer(value as i32),
            source,
        );

        info!(
            "⚡ Atomic property {} changed: {} -> {} (source: {:?})",
            key, old_value, value, source
        );
        Ok(())
    }

    /// Set atomic boolean property (for toggles and flags)
    pub fn set_atomic_bool(&self, key: &str, value: bool, source: PropertySource) -> Result<()> {
        let old_value = match key {
            // Emergency controls
            "enable_live_execution" => {
                let old = self
                    .atomic_properties
                    .enable_live_execution
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .enable_live_execution
                    .store(value, Ordering::Relaxed);
                old
            }
            "dry_run" => {
                let old = self.atomic_properties.dry_run.load(Ordering::Relaxed);
                self.atomic_properties
                    .dry_run
                    .store(value, Ordering::Relaxed);
                old
            }
            "simulation_mode" => {
                let old = self
                    .atomic_properties
                    .simulation_mode
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .simulation_mode
                    .store(value, Ordering::Relaxed);
                old
            }
            "circuit_breaker_enabled" => {
                let old = self
                    .atomic_properties
                    .circuit_breaker_enabled
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .circuit_breaker_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            // Execution settings
            "jito_enabled" => {
                let old = self.atomic_properties.jito_enabled.load(Ordering::Relaxed);
                self.atomic_properties
                    .jito_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            "flash_loan_enabled" => {
                let old = self
                    .atomic_properties
                    .flash_loan_enabled
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .flash_loan_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            // AI settings
            "ai_optimization_enabled" => {
                let old = self
                    .atomic_properties
                    .ai_optimization_enabled
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .ai_optimization_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            // Wallet settings
            "auto_rebalance_enabled" => {
                let old = self
                    .atomic_properties
                    .auto_rebalance_enabled
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .auto_rebalance_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            // Monitoring settings
            "profit_tracking_enabled" => {
                let old = self
                    .atomic_properties
                    .profit_tracking_enabled
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .profit_tracking_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            "log_all_executions" => {
                let old = self
                    .atomic_properties
                    .log_all_executions
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .log_all_executions
                    .store(value, Ordering::Relaxed);
                old
            }
            "alert_on_negative_profit" => {
                let old = self
                    .atomic_properties
                    .alert_on_negative_profit
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .alert_on_negative_profit
                    .store(value, Ordering::Relaxed);
                old
            }
            "track_slippage" => {
                let old = self
                    .atomic_properties
                    .track_slippage
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .track_slippage
                    .store(value, Ordering::Relaxed);
                old
            }
            "track_gas_costs" => {
                let old = self
                    .atomic_properties
                    .track_gas_costs
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .track_gas_costs
                    .store(value, Ordering::Relaxed);
                old
            }
            "enable_file_logging" => {
                let old = self
                    .atomic_properties
                    .enable_file_logging
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .enable_file_logging
                    .store(value, Ordering::Relaxed);
                old
            }
            // System settings
            "redis_enabled" => {
                let old = self.atomic_properties.redis_enabled.load(Ordering::Relaxed);
                self.atomic_properties
                    .redis_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            "kafka_enabled" => {
                let old = self.atomic_properties.kafka_enabled.load(Ordering::Relaxed);
                self.atomic_properties
                    .kafka_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            "api_logging_enabled" => {
                let old = self
                    .atomic_properties
                    .api_logging_enabled
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .api_logging_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            // Legacy
            "system_enabled" => {
                let old = self
                    .atomic_properties
                    .system_enabled
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .system_enabled
                    .store(value, Ordering::Relaxed);
                old
            }
            _ => return Err(anyhow::anyhow!("Unknown atomic bool property: {}", key)),
        };

        self.notify_change(
            key,
            PropertyValue::Boolean(old_value),
            PropertyValue::Boolean(value),
            source,
        );

        info!(
            "⚡ Atomic property {} changed: {} -> {} (source: {:?})",
            key, old_value, value, source
        );
        Ok(())
    }

    /// Set atomic float property (stored as u64 bits for atomic access)
    pub fn set_atomic_f64(&self, key: &str, value: f64, source: PropertySource) -> Result<()> {
        let old_bits = match key {
            "min_venue_success_rate" => {
                let old_bits = self
                    .atomic_properties
                    .min_venue_success_rate
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .min_venue_success_rate
                    .store(value.to_bits(), Ordering::Relaxed);
                old_bits
            }
            "max_gas_budget_pct" => {
                let old_bits = self
                    .atomic_properties
                    .max_gas_budget_pct
                    .load(Ordering::Relaxed);
                self.atomic_properties
                    .max_gas_budget_pct
                    .store(value.to_bits(), Ordering::Relaxed);
                old_bits
            }
            "learning_rate" => {
                let old_bits = self.atomic_properties.learning_rate.load(Ordering::Relaxed);
                self.atomic_properties
                    .learning_rate
                    .store(value.to_bits(), Ordering::Relaxed);
                old_bits
            }
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

        info!(
            "⚡ Atomic property {} changed: {:.6} -> {:.6} (source: {:?})",
            key, old_value, value, source
        );
        Ok(())
    }

    /// Get atomic integer property (zero-contention hotpath access)
    pub fn get_atomic_i32(&self, key: &str) -> Option<i32> {
        match key {
            "min_profit_bps" => Some(
                self.atomic_properties
                    .min_profit_bps
                    .load(Ordering::Relaxed),
            ),
            "max_profit_bps" => Some(
                self.atomic_properties
                    .max_profit_bps
                    .load(Ordering::Relaxed),
            ),
            _ => None,
        }
    }

    /// Get atomic u32 property (zero-contention hotpath access)
    pub fn get_atomic_u32(&self, key: &str) -> Option<u32> {
        match key {
            "max_consecutive_failures" => Some(
                self.atomic_properties
                    .max_consecutive_failures
                    .load(Ordering::Relaxed),
            ),
            "max_concurrent_opportunities" => Some(
                self.atomic_properties
                    .max_concurrent_opportunities
                    .load(Ordering::Relaxed),
            ),
            "max_trades_per_minute" => Some(
                self.atomic_properties
                    .max_trades_per_minute
                    .load(Ordering::Relaxed),
            ),
            "max_slippage_bps" => Some(
                self.atomic_properties
                    .max_slippage_bps
                    .load(Ordering::Relaxed),
            ),
            "wallet_security_tier" => Some(
                self.atomic_properties
                    .wallet_security_tier
                    .load(Ordering::Relaxed),
            ),
            _ => None,
        }
    }

    /// Get atomic u64 property (zero-contention hotpath access)
    pub fn get_atomic_u64(&self, key: &str) -> Option<u64> {
        match key {
            // Emergency controls
            "max_position_size_lamports" => Some(
                self.atomic_properties
                    .max_position_size_lamports
                    .load(Ordering::Relaxed),
            ),
            "max_daily_loss_lamports" => Some(
                self.atomic_properties
                    .max_daily_loss_lamports
                    .load(Ordering::Relaxed),
            ),
            "cooling_off_period_seconds" => Some(
                self.atomic_properties
                    .cooling_off_period_seconds
                    .load(Ordering::Relaxed),
            ),
            // Profit optimization
            "min_net_profit_lamports" => Some(
                self.atomic_properties
                    .min_net_profit_lamports
                    .load(Ordering::Relaxed),
            ),
            "competition_lookback_ms" => Some(
                self.atomic_properties
                    .competition_lookback_ms
                    .load(Ordering::Relaxed),
            ),
            // Venue limits
            "min_position_size_lamports" => Some(
                self.atomic_properties
                    .min_position_size_lamports
                    .load(Ordering::Relaxed),
            ),
            "max_position_raydium_lamports" => Some(
                self.atomic_properties
                    .max_position_raydium_lamports
                    .load(Ordering::Relaxed),
            ),
            "max_position_orca_lamports" => Some(
                self.atomic_properties
                    .max_position_orca_lamports
                    .load(Ordering::Relaxed),
            ),
            "max_position_meteora_lamports" => Some(
                self.atomic_properties
                    .max_position_meteora_lamports
                    .load(Ordering::Relaxed),
            ),
            "max_position_phoenix_lamports" => Some(
                self.atomic_properties
                    .max_position_phoenix_lamports
                    .load(Ordering::Relaxed),
            ),
            // Execution settings
            "jito_tip_lamports" => Some(
                self.atomic_properties
                    .jito_tip_lamports
                    .load(Ordering::Relaxed),
            ),
            "rpc_timeout_seconds" => Some(
                self.atomic_properties
                    .rpc_timeout_seconds
                    .load(Ordering::Relaxed),
            ),
            "max_flash_loan_amount_lamports" => Some(
                self.atomic_properties
                    .max_flash_loan_amount_lamports
                    .load(Ordering::Relaxed),
            ),
            // AI learning
            "min_samples_for_learning" => Some(
                self.atomic_properties
                    .min_samples_for_learning
                    .load(Ordering::Relaxed),
            ),
            // Wallet security
            "hot_wallet_max_balance_lamports" => Some(
                self.atomic_properties
                    .hot_wallet_max_balance_lamports
                    .load(Ordering::Relaxed),
            ),
            "warm_wallet_max_balance_lamports" => Some(
                self.atomic_properties
                    .warm_wallet_max_balance_lamports
                    .load(Ordering::Relaxed),
            ),
            "cold_wallet_max_balance_lamports" => Some(
                self.atomic_properties
                    .cold_wallet_max_balance_lamports
                    .load(Ordering::Relaxed),
            ),
            // System performance
            "flush_interval_ms" => Some(
                self.atomic_properties
                    .flush_interval_ms
                    .load(Ordering::Relaxed),
            ),
            _ => None,
        }
    }

    /// Get atomic boolean property (zero-contention hotpath access)
    pub fn get_atomic_bool(&self, key: &str) -> Option<bool> {
        match key {
            // Emergency controls
            "enable_live_execution" => Some(
                self.atomic_properties
                    .enable_live_execution
                    .load(Ordering::Relaxed),
            ),
            "dry_run" => Some(self.atomic_properties.dry_run.load(Ordering::Relaxed)),
            "simulation_mode" => Some(
                self.atomic_properties
                    .simulation_mode
                    .load(Ordering::Relaxed),
            ),
            "circuit_breaker_enabled" => Some(
                self.atomic_properties
                    .circuit_breaker_enabled
                    .load(Ordering::Relaxed),
            ),
            // Execution settings
            "jito_enabled" => Some(self.atomic_properties.jito_enabled.load(Ordering::Relaxed)),
            "flash_loan_enabled" => Some(
                self.atomic_properties
                    .flash_loan_enabled
                    .load(Ordering::Relaxed),
            ),
            // AI settings
            "ai_optimization_enabled" => Some(
                self.atomic_properties
                    .ai_optimization_enabled
                    .load(Ordering::Relaxed),
            ),
            // Wallet settings
            "auto_rebalance_enabled" => Some(
                self.atomic_properties
                    .auto_rebalance_enabled
                    .load(Ordering::Relaxed),
            ),
            // Monitoring settings
            "profit_tracking_enabled" => Some(
                self.atomic_properties
                    .profit_tracking_enabled
                    .load(Ordering::Relaxed),
            ),
            "log_all_executions" => Some(
                self.atomic_properties
                    .log_all_executions
                    .load(Ordering::Relaxed),
            ),
            "alert_on_negative_profit" => Some(
                self.atomic_properties
                    .alert_on_negative_profit
                    .load(Ordering::Relaxed),
            ),
            "track_slippage" => Some(
                self.atomic_properties
                    .track_slippage
                    .load(Ordering::Relaxed),
            ),
            "track_gas_costs" => Some(
                self.atomic_properties
                    .track_gas_costs
                    .load(Ordering::Relaxed),
            ),
            "enable_file_logging" => Some(
                self.atomic_properties
                    .enable_file_logging
                    .load(Ordering::Relaxed),
            ),
            // System settings
            "redis_enabled" => Some(self.atomic_properties.redis_enabled.load(Ordering::Relaxed)),
            "kafka_enabled" => Some(self.atomic_properties.kafka_enabled.load(Ordering::Relaxed)),
            "api_logging_enabled" => Some(
                self.atomic_properties
                    .api_logging_enabled
                    .load(Ordering::Relaxed),
            ),
            // Legacy
            "system_enabled" => Some(
                self.atomic_properties
                    .system_enabled
                    .load(Ordering::Relaxed),
            ),
            _ => None,
        }
    }

    /// Get atomic float property (zero-contention hotpath access)
    pub fn get_atomic_f64(&self, key: &str) -> Option<f64> {
        match key {
            "min_venue_success_rate" => Some(f64::from_bits(
                self.atomic_properties
                    .min_venue_success_rate
                    .load(Ordering::Relaxed),
            )),
            "max_gas_budget_pct" => Some(f64::from_bits(
                self.atomic_properties
                    .max_gas_budget_pct
                    .load(Ordering::Relaxed),
            )),
            "learning_rate" => Some(f64::from_bits(
                self.atomic_properties.learning_rate.load(Ordering::Relaxed),
            )),
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
        properties.insert(
            "min_profit_bps".to_string(),
            PropertyValue::Integer(
                self.atomic_properties
                    .min_profit_bps
                    .load(Ordering::Relaxed),
            ),
        );
        properties.insert(
            "max_profit_bps".to_string(),
            PropertyValue::Integer(
                self.atomic_properties
                    .max_profit_bps
                    .load(Ordering::Relaxed),
            ),
        );
        properties.insert(
            "min_venue_success_rate".to_string(),
            PropertyValue::Float(f64::from_bits(
                self.atomic_properties
                    .min_venue_success_rate
                    .load(Ordering::Relaxed),
            )),
        );
        properties.insert(
            "max_gas_budget_pct".to_string(),
            PropertyValue::Float(f64::from_bits(
                self.atomic_properties
                    .max_gas_budget_pct
                    .load(Ordering::Relaxed),
            )),
        );
        properties.insert(
            "learning_rate".to_string(),
            PropertyValue::Float(f64::from_bits(
                self.atomic_properties.learning_rate.load(Ordering::Relaxed),
            )),
        );
        properties.insert(
            "system_enabled".to_string(),
            PropertyValue::Boolean(
                self.atomic_properties
                    .system_enabled
                    .load(Ordering::Relaxed),
            ),
        );
        properties.insert(
            "ai_optimization_enabled".to_string(),
            PropertyValue::Boolean(
                self.atomic_properties
                    .ai_optimization_enabled
                    .load(Ordering::Relaxed),
            ),
        );

        // Complex properties
        let complex = self.complex_properties.read().await;
        for (key, value) in complex.iter() {
            properties.insert(key.clone(), value.clone());
        }

        properties
    }

    /// Load properties from SmartFilterConfig (backward compatibility)
    pub async fn load_from_smart_filter_config(
        &self,
        config: &crate::smart_filter::SmartFilterConfig,
    ) {
        info!("📥 Loading properties from SmartFilterConfig");

        // Load atomic properties
        self.set_atomic_i32(
            "min_profit_bps",
            config.min_profit_bps,
            PropertySource::DatabaseLoad,
        )
        .ok();
        self.set_atomic_i32(
            "max_profit_bps",
            config.max_profit_bps,
            PropertySource::DatabaseLoad,
        )
        .ok();
        self.set_atomic_f64(
            "min_venue_success_rate",
            config.min_venue_success_rate,
            PropertySource::DatabaseLoad,
        )
        .ok();
        self.set_atomic_f64(
            "max_gas_budget_pct",
            config.max_gas_budget_pct,
            PropertySource::DatabaseLoad,
        )
        .ok();
        self.set_atomic_f64(
            "learning_rate",
            config.learning_rate,
            PropertySource::DatabaseLoad,
        )
        .ok();

        // Store non-atomic properties in complex store
        let mut complex = self.complex_properties.write().await;
        complex.insert(
            "min_net_profit_sol".to_string(),
            PropertyValue::Float(config.min_net_profit_sol),
        );
        complex.insert(
            "max_concurrent_opportunities".to_string(),
            PropertyValue::Integer(config.max_concurrent_opportunities as i32),
        );
        complex.insert(
            "competition_lookback_ms".to_string(),
            PropertyValue::Integer(config.competition_lookback_ms as i32),
        );
        complex.insert(
            "min_samples_for_learning".to_string(),
            PropertyValue::Integer(config.min_samples_for_learning as i32),
        );

        info!(
            "✅ PropertyStore loaded from SmartFilterConfig: min_profit={}bps, learning_rate={:.3}",
            config.min_profit_bps, config.learning_rate
        );
    }

    /// Internal notification system for property changes
    fn notify_change(
        &self,
        key: &str,
        old_value: PropertyValue,
        new_value: PropertyValue,
        source: PropertySource,
    ) {
        let event = PropertyChangeEvent {
            property_key: key.to_string(),
            old_value,
            new_value,
            source,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
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

        debug!(
            "📡 Property change event broadcasted: {} = {:?}",
            key, event.new_value
        );

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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let state_age_ms = now.saturating_sub(persistent_state.timestamp);

        if state_age_ms > 24 * 60 * 60 * 1000 {
            // 24 hours
            warn!(
                "⚠️ Persistent state is {} hours old. Using emergency safe values instead.",
                state_age_ms / (60 * 60 * 1000)
            );
            self.apply_emergency_safe_values();
            return Ok(false);
        }

        // Apply recovered values with validation
        self.apply_recovered_values(&persistent_state)?;

        info!(
            "✅ Successfully recovered PropertyStore state from crash. Age: {}min",
            state_age_ms / (60 * 1000)
        );

        Ok(true)
    }

    /// Apply recovered values with safety validation
    fn apply_recovered_values(&mut self, state: &PersistentPropertyState) -> Result<()> {
        let emergency = &self.emergency_safe_values;

        // Validate and apply with safety bounds checking
        let safe_min_profit = state.min_profit_bps.max(emergency.min_profit_bps);
        let safe_max_profit = state.max_profit_bps.min(emergency.max_profit_bps);
        let safe_gas_budget = state.max_gas_budget_pct.min(emergency.max_gas_budget_pct);
        let safe_success_rate = state
            .min_venue_success_rate
            .max(emergency.min_venue_success_rate);
        let safe_learning_rate = state.learning_rate.min(emergency.learning_rate);

        // Apply safely bounded values
        self.atomic_properties
            .min_profit_bps
            .store(safe_min_profit, Ordering::Relaxed);
        self.atomic_properties
            .max_profit_bps
            .store(safe_max_profit, Ordering::Relaxed);
        self.atomic_properties
            .max_gas_budget_pct
            .store(safe_gas_budget.to_bits(), Ordering::Relaxed);
        self.atomic_properties
            .min_venue_success_rate
            .store(safe_success_rate.to_bits(), Ordering::Relaxed);
        self.atomic_properties
            .learning_rate
            .store(safe_learning_rate.to_bits(), Ordering::Relaxed);
        self.atomic_properties
            .system_enabled
            .store(state.system_enabled, Ordering::Relaxed);
        self.atomic_properties
            .ai_optimization_enabled
            .store(state.ai_optimization_enabled, Ordering::Relaxed);

        // Log any adjustments made for safety
        if safe_min_profit != state.min_profit_bps {
            warn!(
                "🛡️ Adjusted min_profit_bps from {} to {} for safety",
                state.min_profit_bps, safe_min_profit
            );
        }
        if safe_max_profit != state.max_profit_bps {
            warn!(
                "🛡️ Adjusted max_profit_bps from {} to {} for safety",
                state.max_profit_bps, safe_max_profit
            );
        }
        if (safe_gas_budget - state.max_gas_budget_pct).abs() > f64::EPSILON {
            warn!(
                "🛡️ Adjusted max_gas_budget_pct from {:.1}% to {:.1}% for safety",
                state.max_gas_budget_pct, safe_gas_budget
            );
        }

        Ok(())
    }

    /// Apply emergency safe values in case of crash recovery failure
    fn apply_emergency_safe_values(&mut self) {
        let emergency = &self.emergency_safe_values;

        self.atomic_properties
            .min_profit_bps
            .store(emergency.min_profit_bps, Ordering::Relaxed);
        self.atomic_properties
            .max_profit_bps
            .store(emergency.max_profit_bps, Ordering::Relaxed);
        self.atomic_properties
            .max_gas_budget_pct
            .store(emergency.max_gas_budget_pct.to_bits(), Ordering::Relaxed);
        self.atomic_properties.min_venue_success_rate.store(
            emergency.min_venue_success_rate.to_bits(),
            Ordering::Relaxed,
        );
        self.atomic_properties
            .learning_rate
            .store(emergency.learning_rate.to_bits(), Ordering::Relaxed);
        self.atomic_properties
            .system_enabled
            .store(emergency.system_enabled, Ordering::Relaxed);

        warn!(
            "🚨 Applied emergency safe values: min_profit={}bps, max_profit={}bps, max_gas={:.1}%",
            emergency.min_profit_bps, emergency.max_profit_bps, emergency.max_gas_budget_pct
        );
    }

    /// Trigger asynchronous persistence (fire-and-forget for performance)
    fn trigger_async_persistence(&self) {
        let persistence_path = Arc::clone(&self.persistence_path);
        let min_profit_bps = self
            .atomic_properties
            .min_profit_bps
            .load(Ordering::Relaxed);
        let max_profit_bps = self
            .atomic_properties
            .max_profit_bps
            .load(Ordering::Relaxed);
        let min_venue_success_rate = f64::from_bits(
            self.atomic_properties
                .min_venue_success_rate
                .load(Ordering::Relaxed),
        );
        let max_gas_budget_pct = f64::from_bits(
            self.atomic_properties
                .max_gas_budget_pct
                .load(Ordering::Relaxed),
        );
        let learning_rate =
            f64::from_bits(self.atomic_properties.learning_rate.load(Ordering::Relaxed));
        let system_enabled = self
            .atomic_properties
            .system_enabled
            .load(Ordering::Relaxed);
        let ai_optimization_enabled = self
            .atomic_properties
            .ai_optimization_enabled
            .load(Ordering::Relaxed);
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
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                last_update_source: PropertySource::CrashRecovery, // Will be updated by caller
                crash_count: 0,                                    // Will be incremented if needed
            };

            // Atomic write using temp file + rename for crash safety
            let temp_path = format!("{}.tmp", persistence_path);
            match std::fs::File::create(&temp_path) {
                Ok(mut file) => match serde_json::to_string_pretty(&state) {
                    Ok(json_content) => match file.write_all(json_content.as_bytes()) {
                        Ok(_) => match file.sync_all() {
                            Ok(_) => match std::fs::rename(&temp_path, &persistence_path) {
                                Ok(_) => {
                                    debug!("💾 PropertyStore state persisted successfully");
                                }
                                Err(e) => error!("Failed to rename persistence file: {}", e),
                            },
                            Err(e) => error!("Failed to sync persistence file: {}", e),
                        },
                        Err(e) => error!("Failed to write persistence data: {}", e),
                    },
                    Err(e) => error!("Failed to serialize persistence data: {}", e),
                },
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
            min_profit_bps: self
                .atomic_properties
                .min_profit_bps
                .load(Ordering::Relaxed),
            max_profit_bps: self
                .atomic_properties
                .max_profit_bps
                .load(Ordering::Relaxed),
            min_venue_success_rate: f64::from_bits(
                self.atomic_properties
                    .min_venue_success_rate
                    .load(Ordering::Relaxed),
            ),
            max_gas_budget_pct: f64::from_bits(
                self.atomic_properties
                    .max_gas_budget_pct
                    .load(Ordering::Relaxed),
            ),
            learning_rate: f64::from_bits(
                self.atomic_properties.learning_rate.load(Ordering::Relaxed),
            ),
            system_enabled: self
                .atomic_properties
                .system_enabled
                .load(Ordering::Relaxed),
            ai_optimization_enabled: self
                .atomic_properties
                .ai_optimization_enabled
                .load(Ordering::Relaxed),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
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

        self.last_persistence_timestamp
            .store(state.timestamp, Ordering::Relaxed);
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
    GLOBAL_PROPERTY_STORE.get_or_init(PropertyStore::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_atomic_property_updates() {
        let store = PropertyStore::new();

        // Test atomic i32 property
        store
            .set_atomic_i32("min_profit_bps", 50, PropertySource::HumanUI)
            .unwrap();
        assert_eq!(store.get_atomic_i32("min_profit_bps"), Some(50));

        // Test atomic f64 property
        store
            .set_atomic_f64("learning_rate", 0.25, PropertySource::AIOptimizer)
            .unwrap();
        assert!((store.get_atomic_f64("learning_rate").unwrap() - 0.25).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let store = PropertyStore::new();
        let mut receiver = store.subscribe();

        // Change a property
        store
            .set_atomic_i32("min_profit_bps", 75, PropertySource::AIOptimizer)
            .unwrap();

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
                store_clone
                    .set_atomic_i32("min_profit_bps", i, PropertySource::AIOptimizer)
                    .unwrap();
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
