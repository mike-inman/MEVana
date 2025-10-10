# Re-Architecture 4 – Modular Stack (Refined)
Created: 2025-10-02

## Purpose
Iterate on the modular architecture with clearer interface contracts and a developer-facing workspace map inspired by multi-layer trait definitions.

## Layered System Overview
```
┌──────────────┐   ┌────────────────┐   ┌────────────────────┐   ┌────────────────────┐
│ Data Feeds   │──▶│ Ingestion Hub  │──▶│ Normalized Event Bus│──▶│ Strategy Runtime   │
└──────────────┘   └────────────────┘   └────────────────────┘   └──────────┬─────────┘
                                                                       Signals │
                                                                               ▼
                                                      ┌────────────────────┐  Advisories ┌───────────────┐
                                                      │ Risk & Policy Gate │◀────────────│ AI Orchestrator│
                                                      └──────────┬─────────┘             └───────────────┘
                                                             Orders │
                                                                     ▼
                                        ┌──────────────────┐   ┌──────────────────┐
                                        │ Execution Router │──▶│ Venue Adapters   │
                                        └────────┬─────────┘   └──────┬──────────┘
                                                 │ Fills            │
                                                 ▼                  ▼
                                   ┌────────────────────────┐ ┌─────────────────────┐
                                   │ Portfolio Authority    │ │ Event & Metrics Bus │
                                   └──────────┬─────────────┘ └──────────┬─────────┘
                                              │                           │
                                              ▼                           ▼
                              ┌────────────────────────┐   ┌────────────────────────┐
                              │ Feature Store (Live)   │   │ Ops Control Plane      │
                              └────────────────────────┘   └────────────────────────┘
```

## Core Modules & Responsibilities
1. **Ingestion Hub** – multiplexer for Geyser, RPC, and third-party feeds; performs timestamp and signature sanity checks before emitting `NormalizedEvent`s onto the bus.
2. **Strategy Runtime** – loads deterministic strategy plugins, decorates their outputs with AI advisories, and emits typed `TradeSignal`s.
3. **Risk & Policy Gate** – enforces capital, compliance, and scenario-based policies; AI scores treated as hints, never overrides.
4. **Execution Router** – translates approved intents into venue-specific route plans, handles flash-loan orchestration, and replays failed legs via fallback trees.
5. **Portfolio Authority** – authoritative record of wallet balances, exposures, and funding movements. Exposes both read models (gRPC) and event streams.
6. **Ops Control Plane** – GraphQL/REST API + dashboard for pausing strategies, updating configs, viewing readiness state, and accessing audit trails.
7. **AI Orchestrator** – hosts inference endpoints, feature transformations, drift detection, and feedback processing. Interacts with control plane for shadow/canary rolls.

## Interface Contracts
| Layer | Trait / Schema | Key Methods / Fields |
|-------|----------------|----------------------|
| Ingestion | `trait DataSource` | `async fn subscribe(&self, filter)` → `NormalizedStream`; `fn health()`; `fn latency()` |
| Event Bus | `NormalizedEvent` struct | `{ timestamp_ns, venue, product_id, payload, confidence }` |
| Strategy | `trait StrategyPlugin` | `fn id()`, `async fn on_event(event, state) -> Vec<TradeSignal>`, `fn risk_budget()` |
| Signal | `TradeSignal` struct | `{ signal_id, strategy_id, opportunity, expected_edge_bps, size_hint, confidence, ttl_ms }` |
| Risk | `trait RiskAdvisor` | `fn pre_check(signal) -> RiskFinding`; `fn approve(batch, portfolio) -> Vec<OrderIntent>` |
| Execution | `trait VenueAdapter` | `async fn prepare(intent) -> RoutePlan`; `async fn submit(plan, signer) -> ExecutionReport`; `fn capabilities()` |
| Portfolio | `PortfolioSnapshot` struct | `{ timestamp_ns, balances, exposure_by_asset, pnl, limits }` |
| Ops Control | `mutations` | `pause_strategy(id)`, `set_limit(limit_id, value)`, `trigger_replay(run_id)` |

## Updated Workspace Layout
```
crates/
├── data-ingest/
│   ├── src/sources/{geyser,rpc,websocket}.rs
│   ├── src/validation/{latency,crosscheck,timestamp}.rs
│   └── src/lib.rs
├── normalization/
│   ├── src/events.rs
│   ├── src/serializers.rs
│   └── src/lib.rs
├── strategy/
│   ├── core/ (StrategyPlugin trait, TradeSignal types)
│   ├── plugins/{arb,mev_triad,liquidation} /
│   └── sandbox/ (deterministic replay harness)
├── risk/
│   ├── policy-engine/
│   ├── exposure-calculator/
│   └── compliance/
├── execution/
│   ├── router/
│   ├── venue-{jito,cex,defi}/
│   └── signer-client/
├── portfolio/
│   ├── authority/ (event-sourced store)
│   ├── reconciliation/
│   └── funding/
├── ai/
│   ├── orchestrator/
│   ├── feature-store/
│   └── feedback/
├── ops/
│   ├── control-plane/ (GraphQL + auth)
│   └── operator-ui/ (frontend)
└── observability/
    ├── telemetry-sdk/
    ├── audit-ledger/
    └── alerting-hooks/

services/
├── ingest-gateway (Rust async)
├── strategy-engine (Rust)
├── risk-gateway (Rust)
├── execution-coordinator (Rust)
├── portfolio-service (Rust)
├── ai-orchestrator (Rust + optional GPU workers)
├── ops-console (TypeScript/React)
└── signer-service (Rust w/ HSM)
```

## Runtime Flow (Hot Path)
1. `ingest-gateway` reads raw feeds, validates payloads, and writes `NormalizedEvent`s to Kafka topic `market.normalized`.
2. `strategy-engine` consumes the topic, routes events through active `StrategyPlugin`s, and publishes `strategy.signals` with strict ordering guarantees.
3. `risk-gateway` pulls both `strategy.signals` and the latest `PortfolioSnapshot`, applies risk policies, and emits `risk.order_intents`.
4. `execution-coordinator` transforms intents into `RoutePlan`s, calls relevant `VenueAdapter`s, and reports results to `execution.fills` + Prometheus.
5. `portfolio-service` listens to fills and funding updates, mutates the event-sourced ledger, and republishes fresh snapshots.
6. `ai-orchestrator` shadows the flow by scoring opportunities, logging rationales, and raising alerts when drift or confidence anomalies arise.
7. `ops-console` consumes metrics/audit events, enabling operators to pause strategies or adjust limits in real time.

## Safety & Operations Enhancements
- **Immutable Audit Stream** – every signal, override, and execution result appended to `audit.events` (append-only store + S3).
- **Circuit Breakers** – `risk-gateway` exposes breaker status; ops can hard-stop via control plane API which propagates to strategy runtime.
- **Replay Harness** – recorded Kafka windows can be replayed through strategy/risk/execution in simulation clusters, producing comparable metrics before rolling forward.

## Attribution
Authored by: Codex (GPT-5)
