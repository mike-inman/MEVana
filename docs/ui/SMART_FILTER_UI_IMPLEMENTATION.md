# Smart Filter UI Controls Implementation Plan

**Date:** 2025-09-11  
**Priority:** CRITICAL - SAME DAY IMPLEMENTATION  
**Objective:** Enable real-time smart filter parameter adjustment with immediate performance feedback

## 🎯 Implementation Overview

### Current State Analysis
The smart filter system in `crates/hotpath_arb_engine/src/smart_filter.rs` contains hardcoded configuration values that can only be set at startup. This severely limits operational flexibility and the ability to optimize trading performance in real-time.

**Key Parameters Requiring UI Controls:**
```rust
pub struct SmartFilterConfig {
    pub min_net_profit_sol: f64,     // Currently: 0.001 (~$0.20)
    pub min_profit_bps: i32,         // Currently: 15 bps
    pub max_profit_bps: i32,         // Currently: 100 bps  
    pub min_venue_success_rate: f64, // Currently: 0.0 (disabled)
    pub max_gas_budget_pct: f64,     // Currently: 30.0%
    pub max_concurrent_opportunities: u32, // Currently: 3
    pub competition_lookback_ms: u64,      // Currently: 5000ms
    pub learning_rate: f64,               // Currently: 0.1
}
```

## 🔧 Technical Implementation

### Phase 1: Backend API Extensions (1-2 hours)

#### 1.1 Add HTTP Server to HotPath Engine
**File:** `crates/hotpath_arb_engine/src/main.rs`

```rust
use axum::{
    extract::{Path, State}, 
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};

// Add to main() function after existing setup
let config_router = Router::new()
    .route("/smart-filter/config", get(get_config).put(update_config))
    .route("/smart-filter/stats", get(get_stats))
    .route("/smart-filter/performance", get(get_performance_impact))
    .route("/smart-filter/reset", put(reset_config))
    .with_state(shared_state.clone());

let app = Router::new()
    .nest("/api/v1", config_router)
    .merge(existing_metrics_router);

// Serve on existing port 8088
let listener = tokio::net::TcpListener::bind("0.0.0.0:8088").await?;
tokio::spawn(axum::serve(listener, app));
```

#### 1.2 Configuration State Management
**New Section in main.rs:**

```rust
#[derive(Clone)]
pub struct SharedConfigState {
    smart_filter: Arc<RwLock<SmartFilter>>,
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
    config_history: Arc<RwLock<VecDeque<ConfigChange>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub timestamp: DateTime<Utc>,
    pub parameter: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub applied_by: String,
    pub performance_before: PerformanceSnapshot,
    pub performance_after: Option<PerformanceSnapshot>, // Updated after 5 minutes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub opportunities_per_minute: f64,
    pub success_rate: f64,
    pub average_profit_bps: f64,
    pub execution_latency_us: u64,
}
```

#### 1.3 API Endpoint Handlers
**New File:** `crates/hotpath_arb_engine/src/config_api.rs`

```rust
use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};

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

pub async fn get_config(
    State(state): State<SharedConfigState>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let smart_filter = state.smart_filter.read().await;
    let performance = state.performance_tracker.read().await;
    
    Ok(Json(ConfigResponse {
        current_config: smart_filter.config.clone(),
        last_updated: performance.last_config_update,
        performance_impact: performance.calculate_recent_impact(),
    }))
}

pub async fn update_config(
    State(state): State<SharedConfigState>,
    Json(request): Json<ConfigUpdateRequest>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let mut smart_filter = state.smart_filter.write().await;
    let mut performance = state.performance_tracker.write().await;
    
    // Validate parameter update
    let new_config = validate_and_apply_update(&smart_filter.config, &request)?;
    
    // Capture performance before change
    let performance_before = performance.current_snapshot();
    
    // Apply configuration change
    smart_filter.update_config(new_config);
    
    // Record change for impact tracking
    let config_change = ConfigChange {
        timestamp: Utc::now(),
        parameter: request.parameter,
        old_value: get_parameter_value(&smart_filter.config, &request.parameter),
        new_value: request.value,
        applied_by: request.applied_by,
        performance_before,
        performance_after: None, // Will be updated by background task
    };
    
    state.config_history.write().await.push_back(config_change);
    
    // Return updated configuration
    Ok(Json(ConfigResponse {
        current_config: smart_filter.config.clone(),
        last_updated: Utc::now(),
        performance_impact: None, // Too early to measure impact
    }))
}
```

#### 1.4 Performance Impact Tracking
**Background Task in main.rs:**

```rust
// Background task to measure configuration impact
tokio::spawn(async move {
    let mut interval = interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        // Update performance impact for recent config changes
        let mut config_history = shared_state.config_history.write().await;
        let current_performance = shared_state.performance_tracker.read().await;
        
        for change in config_history.iter_mut() {
            if change.performance_after.is_none() && 
               Utc::now().signed_duration_since(change.timestamp).num_minutes() >= 5 {
                change.performance_after = Some(current_performance.current_snapshot());
            }
        }
        
        // Keep only last 100 changes
        if config_history.len() > 100 {
            config_history.pop_front();
        }
    }
});
```

### Phase 2: Web UI Implementation (2-3 hours)

#### 2.1 React Component Structure
**Directory:** `web-ui/src/components/SmartFilterControls/`

```typescript
// SmartFilterControls.tsx
import React, { useState, useEffect } from 'react';
import { useWebSocket } from '../hooks/useWebSocket';
import { SmartFilterConfig, PerformanceMetrics } from '../types/trading';

export const SmartFilterControls: React.FC = () => {
    const [config, setConfig] = useState<SmartFilterConfig | null>(null);
    const [performance, setPerformance] = useState<PerformanceMetrics | null>(null);
    const [pendingChanges, setPendingChanges] = useState<Map<string, any>>(new Map());
    
    const { sendMessage, lastMessage, connectionStatus } = useWebSocket(
        'ws://localhost:8088/ws/smart-filter'
    );

    // Real-time parameter controls
    const ParameterSlider: React.FC<{
        name: string;
        value: number;
        min: number;
        max: number;
        step: number;
        unit: string;
        description: string;
        onChange: (value: number) => void;
    }> = ({ name, value, min, max, step, unit, description, onChange }) => {
        const [localValue, setLocalValue] = useState(value);
        const [debounceTimer, setDebounceTimer] = useState<NodeJS.Timeout | null>(null);

        const handleChange = (newValue: number) => {
            setLocalValue(newValue);
            
            // Debounce updates to avoid too frequent API calls
            if (debounceTimer) clearTimeout(debounceTimer);
            
            setDebounceTimer(setTimeout(() => {
                onChange(newValue);
                updateParameter(name, newValue);
            }, 500));
        };

        return (
            <div className="parameter-control">
                <label>{name}</label>
                <input
                    type="range"
                    min={min}
                    max={max}
                    step={step}
                    value={localValue}
                    onChange={(e) => handleChange(Number(e.target.value))}
                />
                <span className="value">{localValue.toFixed(3)}{unit}</span>
                <p className="description">{description}</p>
            </div>
        );
    };

    const updateParameter = async (parameter: string, value: any) => {
        try {
            const response = await fetch('/api/v1/smart-filter/config', {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    parameter,
                    value,
                    applied_by: 'web-ui-user' // In production, use actual user ID
                })
            });
            
            if (!response.ok) throw new Error('Failed to update parameter');
            
            const updatedConfig = await response.json();
            setConfig(updatedConfig.current_config);
            
        } catch (error) {
            console.error('Error updating parameter:', error);
            // Show user notification
        }
    };

    return (
        <div className="smart-filter-controls">
            <h2>Smart Filter Controls</h2>
            
            <div className="connection-status">
                Status: {connectionStatus === 'connected' ? '🟢 Connected' : '🔴 Disconnected'}
            </div>

            {config && (
                <div className="parameter-grid">
                    <ParameterSlider
                        name="Min Net Profit (SOL)"
                        value={config.min_net_profit_sol}
                        min={0.0001}
                        max={0.01}
                        step={0.0001}
                        unit=" SOL"
                        description="Minimum profit after gas costs"
                        onChange={(value) => setConfig({...config, min_net_profit_sol: value})}
                    />
                    
                    <ParameterSlider
                        name="Min Profit (BPS)"
                        value={config.min_profit_bps}
                        min={5}
                        max={50}
                        step={1}
                        unit=" bps"
                        description="Minimum profit in basis points"
                        onChange={(value) => setConfig({...config, min_profit_bps: value})}
                    />
                    
                    <ParameterSlider
                        name="Max Profit (BPS)"
                        value={config.max_profit_bps}
                        min={50}
                        max={200}
                        step={5}
                        unit=" bps"
                        description="Maximum profit (avoid suspicious opportunities)"
                        onChange={(value) => setConfig({...config, max_profit_bps: value})}
                    />
                    
                    <ParameterSlider
                        name="Gas Budget (%)"
                        value={config.max_gas_budget_pct}
                        min={10}
                        max={50}
                        step={1}
                        unit="%"
                        description="Maximum gas cost as percentage of profit"
                        onChange={(value) => setConfig({...config, max_gas_budget_pct: value})}
                    />
                    
                    <ParameterSlider
                        name="Venue Success Rate (%)"
                        value={config.min_venue_success_rate * 100}
                        min={0}
                        max={90}
                        step={5}
                        unit="%"
                        description="Minimum historical success rate for venue pairs"
                        onChange={(value) => setConfig({...config, min_venue_success_rate: value / 100})}
                    />
                </div>
            )}
        </div>
    );
};
```

#### 2.2 Real-time Performance Impact Visualization
```typescript
// PerformanceImpactChart.tsx
import React, { useMemo } from 'react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend } from 'recharts';

export const PerformanceImpactChart: React.FC<{
    configHistory: ConfigChange[];
    performanceData: PerformanceMetrics[];
}> = ({ configHistory, performanceData }) => {
    
    const chartData = useMemo(() => {
        return performanceData.map((perf, index) => ({
            timestamp: perf.timestamp,
            opportunities_per_minute: perf.opportunities_per_minute,
            success_rate: perf.success_rate * 100,
            avg_profit_bps: perf.average_profit_bps,
            // Overlay configuration changes
            config_change: configHistory.find(change => 
                Math.abs(new Date(change.timestamp).getTime() - new Date(perf.timestamp).getTime()) < 60000
            )?.parameter || null
        }));
    }, [configHistory, performanceData]);

    return (
        <div className="performance-impact-chart">
            <h3>Performance Impact Over Time</h3>
            <LineChart width={800} height={400} data={chartData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="timestamp" />
                <YAxis yAxisId="left" />
                <YAxis yAxisId="right" orientation="right" />
                <Tooltip />
                <Legend />
                
                <Line 
                    yAxisId="left"
                    type="monotone" 
                    dataKey="opportunities_per_minute" 
                    stroke="#8884d8" 
                    name="Opportunities/min"
                />
                <Line 
                    yAxisId="right"
                    type="monotone" 
                    dataKey="success_rate" 
                    stroke="#82ca9d" 
                    name="Success Rate %"
                />
                <Line 
                    yAxisId="right"
                    type="monotone" 
                    dataKey="avg_profit_bps" 
                    stroke="#ffc658" 
                    name="Avg Profit (bps)"
                />
            </LineChart>
            
            {/* Configuration change markers */}
            <div className="config-change-markers">
                {configHistory.slice(-10).map((change, index) => (
                    <div key={index} className="config-marker">
                        <span className="timestamp">{change.timestamp}</span>
                        <span className="parameter">{change.parameter}</span>
                        <span className="value">{change.old_value} → {change.new_value}</span>
                    </div>
                ))}
            </div>
        </div>
    );
};
```

### Phase 3: Database Schema Updates (30 minutes)

#### 3.1 Configuration Management Tables
**File:** `database/migrations/smart_filter_config.sql`

```sql
-- Configuration history and state management
CREATE TABLE smart_filter_config_history (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    parameter_name VARCHAR(100) NOT NULL,
    old_value JSONB,
    new_value JSONB NOT NULL,
    applied_by VARCHAR(255) NOT NULL,
    performance_before JSONB,
    performance_after JSONB,
    
    CONSTRAINT valid_parameter_name CHECK (
        parameter_name IN (
            'min_net_profit_sol', 'min_profit_bps', 'max_profit_bps',
            'min_venue_success_rate', 'max_gas_budget_pct', 
            'max_concurrent_opportunities', 'competition_lookback_ms', 'learning_rate'
        )
    )
);

-- Performance impact tracking
CREATE TABLE parameter_performance_impact (
    id SERIAL PRIMARY KEY,
    config_change_id INTEGER REFERENCES smart_filter_config_history(id),
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    opportunities_per_minute DECIMAL(8,2),
    success_rate DECIMAL(5,4),
    average_profit_bps DECIMAL(8,2),
    execution_latency_us BIGINT,
    sample_size INTEGER
);

-- Indexes for performance
CREATE INDEX idx_config_history_timestamp ON smart_filter_config_history(timestamp DESC);
CREATE INDEX idx_config_history_parameter ON smart_filter_config_history(parameter_name, timestamp DESC);
CREATE INDEX idx_performance_impact_timestamp ON parameter_performance_impact(timestamp DESC);

-- View for latest configuration
CREATE VIEW current_smart_filter_config AS
SELECT DISTINCT ON (parameter_name)
    parameter_name,
    new_value,
    timestamp,
    applied_by
FROM smart_filter_config_history
ORDER BY parameter_name, timestamp DESC;
```

## 🚀 Deployment Steps

### Step 1: Backend Implementation (Priority 1)
```bash
cd /Users/michaelinman/Repos/MEVana/crates/hotpath_arb_engine

# Add new dependencies to Cargo.toml
# axum = "0.7" (already exists)
# serde_json = { workspace = true } (already exists)

# Create new API module
touch src/config_api.rs

# Add to src/lib.rs:
# pub mod config_api;

# Update main.rs with HTTP server and state management
```

### Step 2: Database Migration
```bash
# Apply database schema updates
psql -h localhost -p 5433 -U mevana -d mevana -f database/migrations/smart_filter_config.sql
```

### Step 3: Test API Endpoints
```bash
# Test configuration retrieval
curl http://localhost:8088/api/v1/smart-filter/config

# Test parameter update
curl -X PUT http://localhost:8088/api/v1/smart-filter/config \
  -H "Content-Type: application/json" \
  -d '{"parameter": "min_profit_bps", "value": 20, "applied_by": "test"}'

# Verify performance impact tracking
curl http://localhost:8088/api/v1/smart-filter/performance
```

### Step 4: UI Integration (if web UI exists)
```bash
# If React UI exists, add smart filter components
# Otherwise, use direct API calls for initial testing
```

## 📊 Success Metrics

### Immediate Validation (Today)
- [ ] API endpoints respond within 1 second
- [ ] Parameter updates apply immediately without restart
- [ ] Configuration history is persisted to database
- [ ] Performance impact tracking begins recording data

### Performance Validation (This Week)
- [ ] Real-time parameter adjustment improves opportunity capture by 10%+
- [ ] Configuration rollback capability works within 5 seconds
- [ ] UI provides clear correlation between parameter changes and performance
- [ ] Database performance remains optimal with configuration logging

## 🔧 Implementation Priority

1. **CRITICAL (Today):** Backend API implementation for parameter updates
2. **HIGH (Today):** Database schema and persistence layer
3. **MEDIUM (This Week):** Basic UI controls and visualization
4. **LOW (Next Week):** Advanced analytics and automated optimization

This implementation plan enables immediate operational flexibility while providing a foundation for advanced trading optimization capabilities.