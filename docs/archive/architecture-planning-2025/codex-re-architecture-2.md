# Re-Architecture 2 – Modular System Blueprint
Created: 2025-10-02

## Guiding Qualities → Architectural Commitments
- Deterministic Logic → pure strategy engines with immutable inputs, recorded market snapshots, and explicit side-effect boundaries.
- Latency Discipline → hot-path crates compiled as independent binaries, async pipelines with bounded queues, and co-located strategy/risk execution services.
- Guardrails First → risk gateway microservice enforcing exposure policy ahead of any venue adapter; configuration stored in property store + signed policy bundles.
- Observability Everywhere → metrics/logging SDK shared across crates, OpenTelemetry spans from feed capture through execution, structured event bus for audits.
- State Awareness → dedicated portfolio service maintaining authoritative balances, synchronized via event sourcing + venue reconciliation jobs.
- Modular Strategy Framework → plug-in registry with typed traits for signal generation, sizing, and post-trade feedback; sandbox harness for deterministic backtests.
- Resilience → supervisor that monitors feeds/venues, automatic failover between primary/secondary providers, per-component watchdogs with exponential backoff.
- Security Hygiene → secrets isolated in KMS-backed sidecar, wallet operations routed through signing service with hardware backing, strict RBAC for operator APIs.
- Verification Pipeline → simulation environment that replays captured feeds and mirrors risk/execution flow before prod promotion, enforced by CI gate.
- Operational Ergonomics → control plane APIs + dashboard to pause/resume strategies, tweak risk thresholds, view readiness state, and request rollbacks.

## Layered Logical Architecture
```
┌──────────────┐   ┌────────────────┐   ┌────────────────────┐   ┌─────────────────────┐
│ Data Sources │──▶│ Feed Ingestion │──▶│ Normalization Bus  │──▶│ Strategy Plugins     │
└──────────────┘   └────────────────┘   └────────────────────┘   └──────────┬──────────┘
                                                                       Signals │
                                                                               ▼
                                                                   ┌────────────────────┐
                                                                   │ Risk & Policy Gate │
                                                                   └──────────┬─────────┘
                                                                          Orders │
                                                                               ▼
                                                      ┌──────────────────┐  ┌──────────────┐
                                                      │ Execution Router │──│ Venue Adapters│
                                                      └────────┬─────────┘  └──────┬───────┘
                                                               │                   │
                                                               ▼                   ▼
                                                   ┌────────────────────┐ ┌────────────────────┐
                                                   │ Portfolio Service  │ │ Event & Metrics Bus│
                                                   └────────┬───────────┘ └────────┬──────────┘
                                                            │                      │
                                                            ▼                      ▼
                                                ┌────────────────────┐   ┌────────────────────┐
                                                │ Treasury & Funding │   │ Ops Console / APIs │
                                                └────────────────────┘   └────────────────────┘
```

## Module Breakdown & Suggested Crate/Service Layout
```
crates/
├── feeds/                      # Geyser, CEX, DeFi connectors (pure ingestion)
│   ├── geyser_client/
│   ├── rest_ws_adapter/
│   └── feed-common/
├── normalization/
│   ├── spl_swap_decoder/
│   ├── cex_book_normalizer/
│   └── normalized-models/
├── strategy/
│   ├── core/                   # Traits: Strategy, Signal, Evaluator, Feedback
│   ├── plugins/
│   │   ├── stat_arb/
│   │   ├── mev_hotpath/
│   │   └── trend_follow/
│   └── sandbox/                # Deterministic backtest harness
├── risk/
│   ├── policy-engine/
│   ├── exposure-calculator/
│   └── compliance/
├── execution/
│   ├── router/
│   ├── venue-jito/
│   ├── venue-cex/
│   └── failover/
├── portfolio/
│   ├── position-service/
│   ├── wallet-inventory/
│   └── reconciliation/
├── observability/
│   ├── telemetry-sdk/
│   ├── metrics-exporter/
│   └── audit-log/
└── platform/
    ├── config-store/
    ├── control-plane/
    └── operator-ui/

services/
├── feed-gateway (Rust, async) – consolidates feeds and publishes to Kafka/NATS
├── strategy-engine (Rust) – loads strategy plugins, emits trade intents
├── risk-gateway (Rust) – synchronous guardrail enforcement, policy signatures
├── execution-coordinator (Rust) – manages routing, batching, flash-loan orchestration
├── portfolio-service (Rust) – event-sourced balance + pnl tracker
├── ops-console (TypeScript/React + GraphQL) – dashboards, toggles, audit views
└── signer-service (Rust) – HSM/KMS-backed signing abstraction
```

## Key Interfaces & Data Contracts
- `market::NormalizedEvent` – enum covering trades, order-book deltas, mempool events; produced by feed services, consumed by strategies.
- `strategy::Signal` – pure struct `{ opportunity_id, venue_set, product, confidence, sizing_hint }`.
- `risk::OrderIntent` – enriched signal after sizing, includes compliance metadata, exposure deltas, kill-switch tags.
- `execution::RoutePlan` – final instruction list with venue adapter hints, latency class, and fallback hierarchy.
- Event bus topics (Kafka): `market.normalized.*`, `strategy.signals`, `risk.orders`, `execution.fills`, `portfolio.state`, `audit.events`.

## Runtime Flow
1. **Feed Gateway** normalizes raw exchange/RPC data and publishes `NormalizedEvent`s.
2. **Strategy Engine** consumes events in deterministic batches, executes active plugins in priority order, emits `Signal`s.
3. **Risk Gateway** merges `Signal`s with current exposure from Portfolio Service, enforces policy rules, and outputs `OrderIntent`s.
4. **Execution Coordinator** transforms intents into `RoutePlan`s, engages venue adapters (CEX REST/WS, DEX program, Jito bundle), monitors acknowledgements.
5. **Portfolio Service** ingests fills/settlements, updates positions, rebalances wallet tiers, and exposes read models via gRPC/GraphQL.
6. **Observability stack** records metrics and audit events at each hop; Ops Console visualizes readiness, risk posture, and strategy performance.

## Extensibility Patterns
- Strategy plugins compiled as dynamic libraries (e.g., `cdylib`) loaded by the Strategy Engine; interface versioned via `abi_hash`.
- Risk policies defined declaratively (YAML/JSON), signed offline, and hot-reloaded via Config Store; change events broadcast to risk nodes.
- Venue adapters implement a `VenueAdapter` trait with `prepare`, `submit`, `cancel`, `status_stream`; adapters live in separate crates for testability.
- Backtests replay recorded Kafka topics through Strategy Engine + Risk Gateway in simulation mode, producing comparable metrics to live runs.

## Observability & Control Plane
- Telemetry SDK wraps tracing/metrics/logging; each service injects correlation IDs (`trade_id`, `opportunity_id`, `execution_trace`).
- Ops Console communicates with Control Plane service (GraphQL/REST) to issue commands: pause strategy, adjust limits, rotate keys, trigger replay.
- Audit log captures every strategy decision, risk override, manual action with signed entries stored in append-only ledger (e.g., immudb/S3).

## Deployment & Environments
- **Environments**: dev (minikube/docker-compose), staging (full stack with shadow trading), prod (HA k8s or bare-metal + Nomad).
- **CI/CD**: cargo workspace tests → strategy sandbox golden outputs → integration tests (risk + execution) → replay gate.
- **Secrets**: Vault/KMS with short-lived tokens, signer-service connected to HSM; hot wallets in secure enclaves, warm/cold offline.
- **Resilience**: per-service health checks, circuit breakers for feed latency, auto-failover to secondary RPC/exchange endpoints.

## Attribution
Authored by: Codex (GPT-5)
