# Hybrid Architecture: Ultra-Low Latency with Institutional Maturity

**Revision 2 - Integrating Codex (GPT-5) Architectural Patterns**

Created: 2025-10-02

---

## Design Philosophy

This architecture combines:
- **Ultra-low latency hot path** (<100ms end-to-end) for MEV capture
- **Institutional-grade observability** (deterministic replay, immutable audit trail)
- **AI-augmented decision making** (adaptive strategies with safety guardrails)
- **Operational maturity** (control plane, supervisor, portfolio service)

### Core Architectural Commitments

1. **Latency Discipline** - Hot path uses lockfree in-memory structures; async publish to event bus doesn't block
2. **Deterministic Replay** - All market events captured to immutable store; full system replay for testing/debugging
3. **AI as Advisory** - ML models suggest adjustments but hard risk limits always enforced first
4. **State Awareness** - Dedicated portfolio service maintains authoritative balances/positions
5. **Observability Everywhere** - Correlation IDs, structured traces, append-only audit log
6. **Operational Control** - Control plane for runtime config, emergency stops, strategy pause/resume
7. **Resilient Infrastructure** - Supervisor monitors all components, auto-failover, circuit breakers

---

## High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                            CONTROL & OBSERVABILITY PLANE                         │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Supervisor  │  │ Control Plane │  │ Ops Console  │  │  Audit Logger    │   │
│  │  (Health)    │  │ (GraphQL API) │  │  (React UI)  │  │  (Immutable)     │   │
│  └──────────────┘  └───────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
                                        │
        ┌───────────────────────────────┼────────────────────────────┐
        │                               │                            │
┌───────▼────────┐           ┌──────────▼───────┐         ┌─────────▼──────────┐
│  DATA LAYER    │           │  AI ORCHESTRATION│         │  PORTFOLIO SERVICE │
│                │           │     PLANE         │         │   (Authoritative)  │
│ • Multi-source │           │                  │         │                    │
│ • Validation   │           │ • Feature Store  │         │ • Balance Tracker  │
│ • Normalization│           │ • Model Registry │         │ • Reconciliation   │
│ • Replay Capture│          │ • Inference API  │         │ • Event Sourcing   │
└────┬───────────┘           │ • Drift Detection│         └─────────┬──────────┘
     │                       └──────────┬───────┘                   │
     │ Validated                        │ Advisories                │ State
     │ Events                           │                           │
     │                       ┌──────────▼───────────────────────────▼──────┐
     └──────────────────────▶│       STRATEGY LAYER                        │
                             │                                             │
                             │ • Alpha Strategies (Arb, Liq, Custom)       │
                             │ • Signal Generation                         │
                             │ • Feedback Loop (Outcomes → AI Learning)    │
                             └──────────┬──────────────────────────────────┘
                                        │ Signals
                             ┌──────────▼──────────┐
                             │   RISK MANAGER      │
                             │                     │
                             │ • Hard Limits First │
                             │ • AI Advisory Input │
                             │ • Exposure Tracking │
                             │ • Circuit Breakers  │
                             └──────────┬──────────┘
                                        │ Approved Signals
                             ┌──────────▼──────────┐
                             │  EXECUTION LAYER    │
                             │                     │
                             │ • Priority Queue    │
                             │ • Pre-flight Sim    │
                             │ • Jito/RPC Submit   │
                             │ • Outcome Tracking  │
                             └──────────┬──────────┘
                                        │
        ┌───────────────────────────────┴────────────────────────────┐
        │                                                             │
┌───────▼──────────┐                                      ┌──────────▼────────┐
│  EVENT BUS       │                                      │  REPLAY STORE     │
│  (Kafka/NATS)    │                                      │  (Immutable)      │
│                  │                                      │                   │
│ • Async Publish  │                                      │ • Market Events   │
│ • Audit Trail    │                                      │ • Execution Logs  │
│ • Multi-Consumer │                                      │ • CI/CD Replay    │
└──────────────────┘                                      └───────────────────┘
```

---

## Detailed Component Breakdown

### 1. DATA LAYER (Latency + Determinism)

```
data-layer/
├── sources/
│   ├── geyser_stream.rs       # Primary: Direct Geyser gRPC
│   ├── rpc_pool.rs            # Failover: Multi-endpoint RPC
│   ├── websocket_feed.rs      # Tertiary: Alternative WS feeds
│   └── traits.rs              # DataSource trait
│
├── validation/
│   ├── cross_validator.rs     # Multi-source consensus
│   ├── state_validator.rs     # Blockchain state consistency
│   ├── timestamp_validator.rs # Data freshness checks
│   └── anomaly_detector.rs    # Statistical outlier detection
│
├── normalization/
│   ├── tx_parser.rs           # Parse raw tx to canonical format
│   ├── pool_state.rs          # DEX pool state from events
│   └── token_registry.rs      # Token metadata management
│
├── state/
│   ├── pool_manager.rs        # Lockfree in-memory pool state
│   ├── token_graph.rs         # Token adjacency graph
│   └── snapshot.rs            # Periodic state snapshots
│
├── replay/
│   ├── capture.rs             # Record all events to immutable store
│   ├── loader.rs              # Load historical events for replay
│   └── determinism.rs         # Ensure deterministic event ordering
│
└── aggregator.rs              # Multi-source coordinator + async event bus publish
```

**Key Traits:**
```rust
#[async_trait]
pub trait DataSource: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&self, filters: Filters) -> Result<BoxStream<RawEvent>>;
    async fn health_check(&self) -> HealthStatus;
    fn latency_percentiles(&self) -> LatencyStats;
    fn source_id(&self) -> SourceId;
}

/// Validated event with replay metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedEvent {
    pub event_id: EventId,             // Unique, monotonic
    pub timestamp_ns: u64,
    pub event_type: EventType,
    pub pool_id: PoolId,
    pub data: EventData,
    pub sources: Vec<SourceId>,        // Multi-source confirmation
    pub confidence: f64,
    pub replay_metadata: ReplayMetadata, // For deterministic replay
}

/// Immutable capture for deterministic replay
pub struct ReplayMetadata {
    pub sequence_number: u64,
    pub ingestion_timestamp_ns: u64,
    pub snapshot_hash: Hash,           // State hash at this point
}
```

**Hot Path vs. Audit Path:**
```rust
// In data layer aggregator
pub async fn publish_event(&self, event: ValidatedEvent) {
    // HOT PATH: In-memory broadcast (non-blocking, <1ms)
    self.hot_path_tx.send(event.clone()).ok();

    // AUDIT PATH: Async publish to event bus (fire-and-forget)
    let event_bus = self.event_bus.clone();
    let audit_log = self.audit_log.clone();
    let replay_store = self.replay_store.clone();

    tokio::spawn(async move {
        // Parallel async writes (don't block hot path)
        tokio::join!(
            event_bus.publish("market.validated", &event),
            audit_log.append(&event),
            replay_store.persist(&event),
        );
    });
}
```

---

<function_calls>
<invoke name="TodoWrite">
<parameter name="todos">[{"content": "Integrate Codex's deterministic replay architecture", "status": "completed", "activeForm": "Integrating Codex's deterministic replay architecture"}, {"content": "Add AI orchestration and feature store components", "status": "in_progress", "activeForm": "Adding AI orchestration and feature store components"}, {"content": "Design portfolio service for authoritative state", "status": "pending", "activeForm": "Designing portfolio service for authoritative state"}, {"content": "Add event bus integration for audit trail", "status": "pending", "activeForm": "Adding event bus integration for audit trail"}, {"content": "Create control plane and supervisor architecture", "status": "pending", "activeForm": "Creating control plane and supervisor architecture"}, {"content": "Update data flow diagrams", "status": "pending", "activeForm": "Updating data flow diagrams"}, {"content": "Revise file structure with new components", "status": "pending", "activeForm": "Revising file structure with new components"}]