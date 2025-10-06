# Modular Architecture for High-Performance Trading Bot

## High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ORCHESTRATION LAYER                              │
│  ┌────────────────┐  ┌──────────────┐  ┌─────────────────────────┐    │
│  │ Health Monitor │  │ Config Mgmt  │  │  Circuit Breakers       │    │
│  └────────────────┘  └──────────────┘  └─────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        │                           │                           │
┌───────▼────────┐         ┌────────▼────────┐        ┌────────▼────────┐
│  DATA LAYER    │         │  STRATEGY       │        │  EXECUTION      │
│                │────────▶│  LAYER          │───────▶│  LAYER          │
│ • Multi-source │         │                 │        │                 │
│ • Validation   │         │ • Alpha Engine  │        │ • Order Mgmt    │
│ • Normalization│         │ • Risk Engine   │        │ • Routing       │
│ • State Mgmt   │         │ • Signal Gen    │        │ • Monitoring    │
└────────────────┘         └─────────────────┘        └─────────────────┘
        │                           │                           │
        │                           │                           │
┌───────▼───────────────────────────▼───────────────────────────▼────────┐
│                      INFRASTRUCTURE LAYER                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │ Metrics  │  │  Logging │  │ Database │  │  Cache   │  │  Queue  │ │
│  │(Prometheus)│ │(Tracing) │  │(TimeSeries)│ │ (Redis) │  │(Optional)│ │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └─────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

## Detailed Component Breakdown

### 1. DATA LAYER (Quality & Reliability - #3)

```
data-layer/
├── sources/
│   ├── geyser_stream.rs       # Direct Geyser gRPC streaming
│   ├── rpc_client.rs          # Multi-endpoint RPC fallback
│   ├── websocket_feed.rs      # Alternative WS feeds (Helius, etc.)
│   └── source_trait.rs        # Common interface for all sources
│
├── validation/
│   ├── cross_validator.rs     # Multi-source consensus checking
│   ├── state_validator.rs     # Blockchain state consistency
│   ├── timestamp_validator.rs # Data freshness checks
│   └── anomaly_detector.rs    # Statistical outlier detection
│
├── normalization/
│   ├── transaction_parser.rs  # Parse raw tx to canonical format
│   ├── pool_state_builder.rs  # Build DEX pool state from events
│   └── token_registry.rs      # Token metadata management
│
├── state/
│   ├── pool_manager.rs        # In-memory pool state (lockfree)
│   ├── token_graph.rs         # Token adjacency graph for routing
│   └── snapshot_manager.rs    # Periodic state snapshots for recovery
│
└── lib.rs                     # Public API
```

**Key Traits:**
```rust
/// All data sources implement this
pub trait DataSource: Send + Sync {
    async fn subscribe(&self, filters: Filters) -> Result<DataStream>;
    async fn health_check(&self) -> HealthStatus;
    fn latency_stats(&self) -> LatencyMetrics;
}

/// Validated, normalized market event
pub struct MarketEvent {
    pub timestamp_ns: u64,        // Nanosecond precision
    pub source: SourceId,
    pub event_type: EventType,    // Swap, Update, Create
    pub pool_id: PoolId,
    pub data: EventData,
    pub confidence: Confidence,   // From cross-validation
}
```

---

### 2. STRATEGY LAYER (Economic Soundness & Risk - #2, #5)

```
strategy-layer/
├── alpha/
│   ├── arbitrage_detector.rs   # Triangle/multi-hop arb detection
│   ├── liquidation_scanner.rs  # Lending protocol liquidations
│   ├── opportunity_ranker.rs   # Score/prioritize opportunities
│   └── profit_calculator.rs    # Precise PnL with all fees
│
├── risk/
│   ├── position_sizer.rs       # Kelly criterion, fixed %, etc.
│   ├── slippage_estimator.rs   # ML-based price impact prediction
│   ├── exposure_tracker.rs     # Track aggregate exposure by asset
│   └── risk_limits.rs          # Per-strategy risk budgets
│
├── signals/
│   ├── signal_generator.rs     # Combine alpha + risk → actionable signal
│   ├── signal_filter.rs        # Apply thresholds, blacklists
│   └── signal_aggregator.rs    # Combine multiple strategy signals
│
├── adaptive/
│   ├── market_regime.rs        # Detect trending/ranging/volatile regimes
│   ├── parameter_optimizer.rs  # Online learning for thresholds
│   └── feedback_loop.rs        # Learn from execution outcomes
│
└── lib.rs
```

**Key Traits:**
```rust
/// Strategy generates actionable signals
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;

    async fn process_event(
        &self,
        event: &MarketEvent,
        state: &MarketState
    ) -> Vec<Signal>;

    fn risk_budget(&self) -> RiskBudget;
    fn metrics(&self) -> StrategyMetrics;
}

/// Actionable trading signal
pub struct Signal {
    pub strategy: StrategyId,
    pub opportunity: Opportunity,  // What to trade
    pub expected_profit: f64,      // In USD
    pub confidence: f64,           // 0.0-1.0
    pub urgency: Urgency,          // How fast to execute
    pub risk_estimate: RiskMetrics,
    pub ttl_ms: u64,              // Time-to-live
}

/// Opportunity types
pub enum Opportunity {
    Arbitrage(ArbPath),
    Liquidation(Position),
    // ... extensible
}
```

---

### 3. EXECUTION LAYER (Latency & Execution Quality - #1, #4)

```
execution-layer/
├── order_manager/
│   ├── order_builder.rs        # Construct optimal transaction
│   ├── priority_queue.rs       # Urgency-based signal queue
│   └── deduplicator.rs         # Avoid duplicate signals
│
├── routing/
│   ├── transaction_builder.rs  # Solana transaction construction
│   ├── bundle_builder.rs       # Jito bundle assembly
│   ├── simulation_engine.rs    # Pre-flight simulation
│   └── fee_optimizer.rs        # Dynamic priority fee adjustment
│
├── submission/
│   ├── jito_client.rs          # Private Jito mempool
│   ├── rpc_submitter.rs        # Standard RPC submission
│   └── retry_manager.rs        # Exponential backoff, circuit breakers
│
├── monitoring/
│   ├── execution_tracker.rs    # Track tx lifecycle
│   ├── success_analyzer.rs     # Post-execution PnL analysis
│   └── failure_analyzer.rs     # Categorize failures for learning
│
└── lib.rs
```

**Key Traits:**
```rust
/// Execute a signal into a blockchain transaction
pub trait Executor: Send + Sync {
    async fn execute(
        &self,
        signal: Signal,
        wallet: &Wallet
    ) -> Result<ExecutionResult>;

    async fn simulate(&self, signal: &Signal) -> SimulationResult;
    fn success_rate(&self) -> f64;
}

/// Result of execution attempt
pub struct ExecutionResult {
    pub tx_signature: Option<Signature>,
    pub status: ExecutionStatus,  // Submitted, Confirmed, Failed
    pub actual_profit: Option<f64>,
    pub latency_ms: u64,
    pub gas_used: u64,
    pub failure_reason: Option<FailureReason>,
}
```

---

### 4. ORCHESTRATION LAYER (Observability & Adaptability - #6, #8)

```
orchestration-layer/
├── lifecycle/
│   ├── system_manager.rs       # Start/stop all components
│   ├── health_monitor.rs       # Aggregate health checks
│   └── graceful_shutdown.rs    # Clean shutdown on signals
│
├── config/
│   ├── config_manager.rs       # Hot-reload configuration
│   ├── feature_flags.rs        # Enable/disable strategies at runtime
│   └── environment.rs          # Dev/staging/prod configs
│
├── safety/
│   ├── circuit_breaker.rs      # Auto-pause on anomalies
│   ├── loss_limiter.rs         # Daily/hourly loss limits
│   └── emergency_stop.rs       # Manual kill switch
│
├── telemetry/
│   ├── metrics_collector.rs    # Prometheus metrics
│   ├── trace_exporter.rs       # OpenTelemetry tracing
│   └── audit_logger.rs         # Append-only decision log
│
└── lib.rs
```

**Key Patterns:**
```rust
/// Main orchestrator ties everything together
pub struct TradingEngine {
    data_layer: Arc<DataLayer>,
    strategies: Vec<Box<dyn Strategy>>,
    executor: Arc<dyn Executor>,
    risk_manager: Arc<RiskManager>,
    circuit_breaker: Arc<CircuitBreaker>,
    metrics: Arc<MetricsCollector>,
}

impl TradingEngine {
    pub async fn run(&self) -> Result<()> {
        let mut events = self.data_layer.subscribe_all().await?;

        while let Some(event) = events.next().await {
            // 1. Validate event quality
            if !self.data_layer.validate(&event).await {
                continue;
            }

            // 2. Generate signals from all strategies
            let signals = self.generate_signals(&event).await;

            // 3. Apply risk management
            let approved = self.risk_manager.approve(signals).await;

            // 4. Execute approved signals
            for signal in approved {
                if self.circuit_breaker.is_open() {
                    break;
                }
                self.executor.execute(signal, &self.wallet).await?;
            }

            // 5. Record metrics
            self.metrics.record_event(&event);
        }
    }
}
```

---

## Data Flow & Communication Patterns

### Event Flow (Hot Path - <100ms target)

```
┌─────────────┐
│   Geyser    │
│  (Primary)  │──┐
└─────────────┘  │
                 │  ┌──────────────┐     ┌──────────────┐
┌─────────────┐  ├─▶│ Multi-Source │────▶│  Validation  │
│  RPC Pool   │──┤  │ Aggregator   │     │  (Consensus) │
│ (Fallback)  │  │  └──────────────┘     └──────┬───────┘
└─────────────┘  │                              │
                 │                              │ Validated
┌─────────────┐  │                              │ Event
│  WebSocket  │──┘                              │
│ (Tertiary)  │                                 │
└─────────────┘                          ┌──────▼───────┐
                                         │ State Update │
         ┌───────────────────────────────│  (Lockfree)  │
         │                               └──────┬───────┘
         │                                      │
         │   ┌──────────────────────────────────┘
         │   │
    ┌────▼───▼────┐   ┌────────────┐   ┌────────────┐
    │ Strategy A  │   │ Strategy B │   │ Strategy C │
    │  (Arb)      │   │ (Liq)      │   │ (Custom)   │
    └──────┬──────┘   └─────┬──────┘   └─────┬──────┘
           │                │                 │
           │  Signal        │  Signal         │  Signal
           └────────┬───────┴─────────────────┘
                    │
             ┌──────▼──────┐
             │   Signal    │
             │ Aggregator  │
             └──────┬──────┘
                    │
             ┌──────▼──────┐
             │    Risk     │
             │   Manager   │
             └──────┬──────┘
                    │ Approved
                    │ Signals
             ┌──────▼──────┐
             │  Priority   │
             │   Queue     │
             └──────┬──────┘
                    │
         ┌──────────┴──────────┐
         │                     │
    ┌────▼────┐         ┌──────▼──────┐
    │ Simulate│         │   Execute   │
    │  (Pre)  │────────▶│ (Jito/RPC)  │
    └─────────┘         └──────┬──────┘
                               │
                        ┌──────▼──────┐
                        │  Track &    │
                        │  Learn      │
                        └─────────────┘
```

### Communication Patterns

**1. Data Layer → Strategy Layer**
- **Pattern**: Broadcast (one-to-many)
- **Mechanism**: `tokio::sync::broadcast` channel
- **Latency**: <1ms (in-memory)
- **Backpressure**: Strategies must keep up or miss events

```rust
// In data layer
let (tx, _) = broadcast::channel(10_000);
tx.send(validated_event)?;

// In each strategy
let mut rx = tx.subscribe();
while let Ok(event) = rx.recv().await {
    self.process(event).await;
}
```

**2. Strategy Layer → Execution Layer**
- **Pattern**: Multi-producer, single-consumer priority queue
- **Mechanism**: `tokio::sync::mpsc` + custom priority queue
- **Latency**: <10ms
- **Backpressure**: Executor controls flow with bounded channel

```rust
// Strategies produce signals
signal_tx.send(signal).await?;

// Executor consumes by priority
while let Some(signal) = signal_rx.recv().await {
    let ranked = priority_queue.insert(signal);
    let next = priority_queue.pop_urgent();
    self.execute(next).await;
}
```

**3. Cross-Cutting Concerns (Metrics, Logging)**
- **Pattern**: Fire-and-forget async writes
- **Mechanism**: Separate async task with buffered channel
- **Latency**: 0ms (non-blocking)

```rust
// From hot path - never blocks
metrics_tx.try_send(metric).ok();

// Background task batches writes
loop {
    let batch = collect_batch(&mut metrics_rx, 100, 100ms).await;
    prometheus_registry.record_batch(batch);
}
```

---

## Complete File Structure

```
trading-bot/
├── Cargo.toml                      # Workspace definition
├── .env.example                    # Environment template
├── config/
│   ├── dev.toml                    # Development config
│   ├── staging.toml                # Staging config
│   └── prod.toml                   # Production config
│
├── crates/
│   ├── common/                     # Shared utilities
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs            # Common types (PoolId, TokenId, etc.)
│   │       ├── error.rs            # Error hierarchy
│   │       ├── metrics.rs          # Metric definitions
│   │       └── time.rs             # High-precision timestamps
│   │
│   ├── data-layer/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs              # Public API
│   │       ├── sources/
│   │       │   ├── mod.rs
│   │       │   ├── traits.rs       # DataSource trait
│   │       │   ├── geyser.rs       # Geyser implementation
│   │       │   ├── rpc_pool.rs     # Multi-RPC with failover
│   │       │   └── websocket.rs    # WebSocket feeds
│   │       │
│   │       ├── validation/
│   │       │   ├── mod.rs
│   │       │   ├── cross_validator.rs
│   │       │   ├── state_validator.rs
│   │       │   ├── timestamp_validator.rs
│   │       │   └── anomaly_detector.rs
│   │       │
│   │       ├── normalization/
│   │       │   ├── mod.rs
│   │       │   ├── tx_parser.rs
│   │       │   ├── pool_state.rs
│   │       │   └── token_registry.rs
│   │       │
│   │       ├── state/
│   │       │   ├── mod.rs
│   │       │   ├── pool_manager.rs
│   │       │   ├── token_graph.rs
│   │       │   └── snapshot.rs
│   │       │
│   │       └── aggregator.rs       # Multi-source coordinator
│   │
│   ├── strategy-layer/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs           # Strategy trait
│   │       │
│   │       ├── alpha/
│   │       │   ├── mod.rs
│   │       │   ├── arbitrage.rs    # Triangle/multi-hop detection
│   │       │   ├── liquidation.rs  # Lending liquidations
│   │       │   ├── ranker.rs       # Opportunity ranking
│   │       │   └── profit_calc.rs  # Precise PnL calculation
│   │       │
│   │       ├── risk/
│   │       │   ├── mod.rs
│   │       │   ├── position_sizer.rs
│   │       │   ├── slippage.rs     # ML-based estimation
│   │       │   ├── exposure.rs     # Aggregate exposure tracking
│   │       │   └── limits.rs       # Risk budget enforcement
│   │       │
│   │       ├── signals/
│   │       │   ├── mod.rs
│   │       │   ├── generator.rs
│   │       │   ├── filter.rs
│   │       │   └── aggregator.rs
│   │       │
│   │       └── adaptive/
│   │           ├── mod.rs
│   │           ├── regime.rs       # Market regime detection
│   │           ├── optimizer.rs    # Parameter tuning
│   │           └── feedback.rs     # Learn from outcomes
│   │
│   ├── execution-layer/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs           # Executor trait
│   │       │
│   │       ├── order/
│   │       │   ├── mod.rs
│   │       │   ├── builder.rs
│   │       │   ├── queue.rs        # Priority queue
│   │       │   └── dedup.rs
│   │       │
│   │       ├── routing/
│   │       │   ├── mod.rs
│   │       │   ├── tx_builder.rs   # Solana transaction construction
│   │       │   ├── bundle.rs       # Jito bundle assembly
│   │       │   ├── simulation.rs   # Pre-flight checks
│   │       │   └── fee_optimizer.rs
│   │       │
│   │       ├── submission/
│   │       │   ├── mod.rs
│   │       │   ├── jito.rs         # Jito client
│   │       │   ├── rpc.rs          # Standard RPC
│   │       │   └── retry.rs        # Retry logic
│   │       │
│   │       └── monitoring/
│   │           ├── mod.rs
│   │           ├── tracker.rs      # Execution tracking
│   │           ├── success_analyzer.rs
│   │           └── failure_analyzer.rs
│   │
│   ├── orchestration-layer/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── engine.rs           # Main TradingEngine
│   │       │
│   │       ├── lifecycle/
│   │       │   ├── mod.rs
│   │       │   ├── manager.rs
│   │       │   ├── health.rs
│   │       │   └── shutdown.rs
│   │       │
│   │       ├── config/
│   │       │   ├── mod.rs
│   │       │   ├── loader.rs       # Hot-reload support
│   │       │   ├── feature_flags.rs
│   │       │   └── environment.rs
│   │       │
│   │       ├── safety/
│   │       │   ├── mod.rs
│   │       │   ├── circuit_breaker.rs
│   │       │   ├── loss_limiter.rs
│   │       │   └── emergency_stop.rs
│   │       │
│   │       └── telemetry/
│   │           ├── mod.rs
│   │           ├── metrics.rs      # Prometheus
│   │           ├── tracing.rs      # OpenTelemetry
│   │           └── audit_log.rs
│   │
│   └── bot-cli/                    # Main binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # CLI entry point
│           ├── commands/
│           │   ├── run.rs          # Run trading bot
│           │   ├── backtest.rs     # Backtest strategies
│           │   ├── paper_trade.rs  # Paper trading mode
│           │   └── analyze.rs      # Post-trade analysis
│           └── wallet.rs           # Wallet management
│
├── tests/                          # Integration tests
│   ├── e2e_arbitrage.rs
│   ├── data_validation.rs
│   ├── execution_flow.rs
│   └── failure_scenarios.rs
│
├── benches/                        # Performance benchmarks
│   ├── latency_suite.rs
│   └── throughput_suite.rs
│
├── scripts/
│   ├── start_dev.sh
│   ├── backtest.sh
│   └── deploy_prod.sh
│
└── docs/
    ├── architecture.md
    ├── strategy_guide.md
    ├── operations.md
    └── api_reference.md
```

---

## Key Interfaces & Trait Boundaries

### 1. Data Layer Public API

```rust
// crates/data-layer/src/lib.rs

/// Main entry point for consuming market data
pub struct DataLayer {
    sources: Vec<Box<dyn DataSource>>,
    validator: CrossValidator,
    state: Arc<MarketState>,
    event_tx: broadcast::Sender<ValidatedEvent>,
}

impl DataLayer {
    pub async fn new(config: DataConfig) -> Result<Self>;

    /// Subscribe to validated market events
    pub fn subscribe(&self) -> broadcast::Receiver<ValidatedEvent>;

    /// Get current market state snapshot
    pub fn state(&self) -> Arc<MarketState>;

    /// Get data quality metrics
    pub fn metrics(&self) -> DataMetrics;
}

// Core trait for all data sources
#[async_trait]
pub trait DataSource: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&self, filters: Filters) -> Result<BoxStream<RawEvent>>;
    async fn health_check(&self) -> HealthStatus;
    fn latency_percentiles(&self) -> LatencyStats;
    fn source_id(&self) -> SourceId;
}

/// Validated, multi-source consensus event
#[derive(Clone, Debug)]
pub struct ValidatedEvent {
    pub timestamp_ns: u64,
    pub event_type: EventType,
    pub pool_id: PoolId,
    pub data: EventData,
    pub sources: Vec<SourceId>,  // Which sources confirmed this
    pub confidence: f64,          // 0.0-1.0 confidence score
}

/// In-memory market state (lockfree access)
pub struct MarketState {
    pools: DashMap<PoolId, PoolState>,       // Concurrent hashmap
    token_graph: RwLock<TokenGraph>,
    last_update: AtomicU64,                  // Nanosecond timestamp
}

impl MarketState {
    /// Lockfree pool state read
    pub fn get_pool(&self, id: &PoolId) -> Option<PoolState>;

    /// Find arbitrage paths between tokens
    pub fn find_paths(&self, from: TokenId, to: TokenId, max_hops: usize) -> Vec<Path>;

    /// Estimate swap output (constant product formula)
    pub fn estimate_swap(&self, pool: &PoolId, input: TokenAmount) -> Result<SwapEstimate>;
}
```

---

### 2. Strategy Layer Public API

```rust
// crates/strategy-layer/src/lib.rs

/// Strategy trait - implement this for custom strategies
#[async_trait]
pub trait Strategy: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    /// Process a market event and generate signals
    async fn process_event(
        &self,
        event: &ValidatedEvent,
        state: &MarketState,
    ) -> Vec<Signal>;

    /// Risk budget for this strategy
    fn risk_budget(&self) -> RiskBudget;

    /// Real-time metrics
    fn metrics(&self) -> StrategyMetrics;

    /// Called when execution completes (for learning)
    async fn on_execution_result(&self, result: &ExecutionResult);
}

/// Actionable trading signal
#[derive(Clone, Debug)]
pub struct Signal {
    pub id: SignalId,
    pub strategy: StrategyId,
    pub timestamp_ns: u64,
    pub ttl_ms: u64,                    // Time to live

    pub opportunity: Opportunity,
    pub expected_profit_usd: f64,
    pub confidence: f64,                // 0.0-1.0
    pub urgency: Urgency,               // Critical, High, Medium, Low

    pub risk: RiskMetrics,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub enum Opportunity {
    Arbitrage {
        path: Vec<PoolId>,              // Swap path
        input_token: TokenId,
        input_amount: u64,
        expected_output: u64,
        min_output: u64,                // Slippage protection
    },
    Liquidation {
        protocol: ProtocolId,
        position: PositionId,
        collateral_token: TokenId,
        debt_token: TokenId,
        liquidation_amount: u64,
    },
}

#[derive(Clone, Debug)]
pub struct RiskMetrics {
    pub position_size_usd: f64,
    pub estimated_slippage_bps: f64,
    pub max_loss_usd: f64,
    pub exposure_by_token: HashMap<TokenId, f64>,
}

/// Risk management engine
pub struct RiskManager {
    limits: RiskLimits,
    exposure_tracker: ExposureTracker,
    slippage_model: SlippageEstimator,
}

impl RiskManager {
    /// Filter signals through risk checks
    pub async fn approve(&self, signals: Vec<Signal>) -> Vec<Signal>;

    /// Check if a signal meets risk criteria
    pub fn check_signal(&self, signal: &Signal) -> RiskDecision;

    /// Update exposure after execution
    pub fn record_execution(&self, result: &ExecutionResult);
}

#[derive(Debug)]
pub enum RiskDecision {
    Approved,
    Rejected(RejectionReason),
    Modified(Signal),  // Adjusted position size
}
```

---

### 3. Execution Layer Public API

```rust
// crates/execution-layer/src/lib.rs

/// Main executor trait
#[async_trait]
pub trait Executor: Send + Sync {
    /// Execute a signal (blocking until submission)
    async fn execute(
        &self,
        signal: Signal,
        wallet: &Wallet,
    ) -> Result<ExecutionResult>;

    /// Simulate execution without submitting
    async fn simulate(&self, signal: &Signal) -> Result<SimulationResult>;

    /// Get executor metrics
    fn metrics(&self) -> ExecutorMetrics;
}

/// Concrete executor implementation
pub struct SolanaExecutor {
    jito_client: Arc<JitoClient>,
    rpc_clients: Vec<Arc<RpcClient>>,
    simulation_engine: SimulationEngine,
    fee_optimizer: FeeOptimizer,
    tracker: ExecutionTracker,
}

impl SolanaExecutor {
    pub async fn new(config: ExecutorConfig) -> Result<Self>;
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub signal_id: SignalId,
    pub tx_signature: Option<Signature>,
    pub status: ExecutionStatus,

    pub actual_profit_usd: Option<f64>,
    pub fees_paid: u64,                  // Lamports
    pub gas_used: u64,

    pub latency: ExecutionLatency,
    pub failure_reason: Option<FailureReason>,
}

#[derive(Debug)]
pub struct ExecutionLatency {
    pub build_ms: f64,
    pub simulate_ms: f64,
    pub submit_ms: f64,
    pub confirm_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug)]
pub enum ExecutionStatus {
    Submitted,
    Simulated,
    Confirmed,
    Failed,
    Expired,
}

#[derive(Debug)]
pub enum FailureReason {
    SimulationFailed(String),
    InsufficientFunds,
    SlippageExceeded,
    TransactionExpired,
    NetworkError(String),
    Unknown(String),
}

/// Pre-flight simulation result
#[derive(Debug)]
pub struct SimulationResult {
    pub success: bool,
    pub logs: Vec<String>,
    pub units_consumed: u64,
    pub estimated_profit: Option<f64>,
    pub error: Option<String>,
}
```

---

### 4. Orchestration Layer Public API

```rust
// crates/orchestration-layer/src/lib.rs

/// Main trading engine
pub struct TradingEngine {
    config: EngineConfig,
    data_layer: Arc<DataLayer>,
    strategies: Vec<Box<dyn Strategy>>,
    risk_manager: Arc<RiskManager>,
    executor: Arc<dyn Executor>,
    circuit_breaker: Arc<CircuitBreaker>,
    metrics: Arc<MetricsCollector>,
}

impl TradingEngine {
    pub async fn new(config: EngineConfig) -> Result<Self>;

    /// Start the trading engine (blocking)
    pub async fn run(&self) -> Result<()>;

    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<()>;

    /// Get aggregated system health
    pub fn health(&self) -> SystemHealth;
}

#[derive(Debug)]
pub struct EngineConfig {
    pub mode: OperatingMode,
    pub data: DataConfig,
    pub strategies: Vec<StrategyConfig>,
    pub risk: RiskConfig,
    pub execution: ExecutorConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone)]
pub enum OperatingMode {
    Live,
    PaperTrading,
    Backtest { from: DateTime<Utc>, to: DateTime<Utc> },
}

/// Circuit breaker for system protection
pub struct CircuitBreaker {
    state: Arc<RwLock<BreakerState>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn is_open(&self) -> bool;
    pub fn trip(&self, reason: String);
    pub fn reset(&self);

    /// Record an execution result (may trip breaker)
    pub fn record(&self, result: &ExecutionResult);
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub max_loss_usd_per_hour: f64,
    pub max_consecutive_failures: usize,
    pub min_success_rate: f64,           // 0.0-1.0
    pub cooldown_duration: Duration,
}

/// Aggregated system health
#[derive(Debug)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub data_layer: ComponentHealth,
    pub strategies: Vec<StrategyHealth>,
    pub executor: ComponentHealth,
    pub circuit_breaker: BreakerState,
}

#[derive(Debug)]
pub enum HealthStatus {
    Healthy,
    Degraded(Vec<String>),
    Unhealthy(Vec<String>),
}
```

---

## Example Usage

### Running the Bot

```rust
// crates/bot-cli/src/main.rs

use orchestration_layer::{TradingEngine, EngineConfig, OperatingMode};
use strategy_layer::{ArbitrageStrategy, RiskManager};
use data_layer::DataConfig;
use execution_layer::SolanaExecutor;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = EngineConfig {
        mode: OperatingMode::PaperTrading,
        data: DataConfig {
            geyser_endpoint: "https://grpc.ny.shyft.to".into(),
            rpc_endpoints: vec![
                "https://api.mainnet-beta.solana.com".into(),
                "https://solana-api.projectserum.com".into(),
            ],
            websocket_endpoints: vec![],
            cross_validation: true,
            min_sources_for_consensus: 2,
        },
        strategies: vec![
            StrategyConfig::Arbitrage {
                max_hops: 3,
                min_profit_bps: 25,
                position_size_usd: 1000.0,
            }
        ],
        risk: RiskConfig {
            max_position_size_usd: 5000.0,
            max_daily_loss_usd: 1000.0,
            max_slippage_bps: 50.0,
        },
        execution: ExecutorConfig {
            use_jito: true,
            max_priority_fee: 0.001,
            confirmation_timeout_ms: 30_000,
        },
        telemetry: TelemetryConfig {
            metrics_port: 8088,
            enable_tracing: true,
            log_level: "info".into(),
        },
    };

    // Build trading engine
    let engine = TradingEngine::new(config).await?;

    // Run until interrupted
    tokio::select! {
        _ = engine.run() => {},
        _ = tokio::signal::ctrl_c() => {
            println!("Shutting down gracefully...");
            engine.shutdown().await?;
        }
    }

    Ok(())
}
```

### Implementing a Custom Strategy

```rust
// Custom mean reversion strategy

use strategy_layer::{Strategy, Signal, ValidatedEvent, MarketState};
use async_trait::async_trait;

pub struct MeanReversionStrategy {
    lookback_window: Duration,
    z_score_threshold: f64,
    position_size_usd: f64,
    // ... state tracking
}

#[async_trait]
impl Strategy for MeanReversionStrategy {
    fn name(&self) -> &str { "mean_reversion_v1" }
    fn version(&self) -> &str { "1.0.0" }

    async fn process_event(
        &self,
        event: &ValidatedEvent,
        state: &MarketState,
    ) -> Vec<Signal> {
        // 1. Update price history
        self.update_history(event);

        // 2. Calculate z-score
        let z_score = self.calculate_z_score(event.pool_id);

        // 3. Generate signal if threshold exceeded
        if z_score.abs() > self.z_score_threshold {
            vec![Signal {
                strategy: self.id(),
                opportunity: self.build_opportunity(event, z_score),
                expected_profit_usd: self.estimate_profit(z_score),
                confidence: self.calculate_confidence(z_score),
                urgency: Urgency::Medium,
                risk: self.estimate_risk(event),
                // ...
            }]
        } else {
            vec![]
        }
    }

    fn risk_budget(&self) -> RiskBudget {
        RiskBudget {
            max_position_size_usd: self.position_size_usd,
            max_concurrent_positions: 3,
            max_daily_loss_usd: 500.0,
        }
    }

    async fn on_execution_result(&self, result: &ExecutionResult) {
        // Learn from outcomes
        self.update_model(result);
    }

    fn metrics(&self) -> StrategyMetrics {
        self.metrics.snapshot()
    }
}
```

---

## Key Design Principles Applied

### 1. **Latency & Speed** ✅
- Lockfree data structures (`DashMap`, `Arc<RwLock<>>`)
- Zero-copy event broadcasting (`tokio::broadcast`)
- Direct Geyser streaming (no intermediate queues on hot path)
- Async everywhere for maximum concurrency

### 2. **Risk Management** ✅
- Dedicated `RiskManager` with position sizing, slippage estimation
- `CircuitBreaker` for automatic system protection
- Per-strategy risk budgets
- Pre-flight simulation before execution

### 3. **Data Quality** ✅
- Multi-source validation layer
- Consensus-based event validation
- Anomaly detection
- Health checks on all data sources

### 4. **Execution Quality** ✅
- Pre-flight simulation
- Jito bundle support for MEV protection
- Dynamic fee optimization
- Comprehensive failure categorization

### 5. **Observability** ✅
- Prometheus metrics at every layer
- OpenTelemetry distributed tracing
- Append-only audit log for compliance
- Real-time health monitoring

### 6. **Modularity** ✅
- Clean trait boundaries between layers
- Pluggable strategies via `Strategy` trait
- Pluggable data sources via `DataSource` trait
- Pluggable executors via `Executor` trait

### 7. **Security** ✅
- Wallet isolation
- Paper trading mode
- Circuit breakers and loss limits
- No secrets in config files

### 8. **Adaptability** ✅
- Hot-reload configuration
- Feature flags for strategies
- Learning loop (`on_execution_result`)
- Market regime detection

This architecture scales from simple arbitrage to complex multi-strategy systems while maintaining sub-100ms latency and comprehensive risk controls.
