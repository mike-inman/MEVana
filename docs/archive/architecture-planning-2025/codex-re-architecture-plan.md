# MEVana Re-Architecture Delivery Plan (AI Team Playbook)
Created: 2025-10-02

## 0. Governance & Shared Responsibilities
- **Architecture Lead**: Owns `codex-re-architecture-5.md`, approves interface changes, and arbitrates cross-module decisions.
- **Program Manager**: Maintains Kanban/issue tracker, sequences work packets, hosts weekly syncs, and resolves resource conflicts.
- **Orchestration Coordinator Team**: Manages shared infra (Kafka, Postgres, Redis), GitHub Actions secrets, deployment scripts, and environment rotations.
- **Integration & Tooling Team**: Builds shared fixtures, mock RPC services, `make` tasks, and module-specific GitHub Actions.
- **QA/Lint Guild**: Provides rustfmt/clippy/deny configs, secret scanners, security guidance, and runs lint/test sign-off jobs.
- **Documentation & Knowledge Hub Curators**: Ensure READMEs and knowledge cards have timestamps, maintain ops runbooks, and link external references.

## 1. Data Ingestion & Connectors Team
- **Scope**: Implement connectors for Solana RPC, Geyser, Shyft, Helius; expose `NormalizedEvent` over Kafka.
- **Deliverables**:
  * `crates/data-ingest` with source modules (`connectors/{geyser,rpc,shyft,helius}.rs`).
  * Health endpoints and latency metrics.
  * Replay fixtures + fixture runner.
- **Interfaces**: `trait DataSource`, `NormalizedEvent` struct (ref: Re-Architecture 5).
- **Tests**: Recorded slot fixtures, schema validation, rate-limit handling.
- **CI/CD**: `ci-data-ingest.yml`, `deploy-data-ingest.yml`.
- **Docs**: Update module README + architecture doc timestamp.

## 2. Normalization & Event Bus Team
- **Scope**: Own serialization, validation, schema registry, and Kafka topic provisioning.
- **Deliverables**: Schema definitions, compatibility tooling, topic managers.
- **Tests**: Schema diff regression, throughput benchmarks, sentinel alert tests.
- **CI/CD**: `ci-normalization.yml`, `deploy-event-bus.yml`.
- **Docs**: Document schema versioning and validation process.

## 3. Strategy Runtime & Plugin Suite Team
- **Scope**: Implement `StrategyPlugin` trait and built-in plugins (two-hop, triangle, multi-DEX, sniping); deterministic replay harness.
- **Deliverables**: Strategy core crate, plugin modules, plugin registry, replay CLI.
- **Tests**: Golden outputs for recorded events, property-based profit calculations, performance sanity.
- **CI/CD**: `ci-strategy.yml`, `deploy-strategy.yml`.
- **Docs**: Plugin manifests, configuration examples, knowledge hub entries.

## 4. Community Strategy Sandbox Team
- **Scope**: Sandbox host (WASM/isolated runtime), manifest validator, policy engine, experimental wallet guardrails.
- **Deliverables**: `community/sandbox` crate, policy enforcement, audit logging.
- **Tests**: Sandbox policy enforcement, crash resilience, resource exhaustion.
- **CI/CD**: `ci-sandbox.yml`, `deploy-sandbox.yml`.
- **Docs**: Sandbox usage guide, manifest requirements, safety FAQ.

## 5. Risk & Policy Gate Team
- **Scope**: Risk budgets, exposure tracking, signed policy bundles, circuit breakers.
- **Deliverables**: Risk engine crate, policy loader, circuit breaker integration.
- **Tests**: Scenario approvals/denials, kill-switch triggers, fuzzed signal inputs.
- **CI/CD**: `ci-risk.yml`, `deploy-risk.yml`.
- **Docs**: Policy schema doc, runbook for toggles, knowledge hub entry.

## 6. Execution Router & Venue Adapter Team
- **Scope**: Route planner, `DexIntegration` adapters (Raydium, Orca, Meteora, Jito), gas/tip optimizer, fallback logic.
- **Deliverables**: Execution router crate, adapter modules, signer client.
- **Tests**: Transaction simulations, mocked venue responses, latency SLO checks.
- **CI/CD**: `ci-execution.yml`, `deploy-execution.yml`.
- **Docs**: Venue adapter README, signing workflow, troubleshooting guide.

## 7. Portfolio Authority & Funding Team
- **Scope**: Event-sourced ledger, wallet tier management, reconciliation jobs, read-model APIs.
- **Deliverables**: Portfolio service crate (event store, projections), gRPC/GraphQL endpoints.
- **Tests**: Ledger consistency, recovery scenarios, concurrency handling.
- **CI/CD**: `ci-portfolio.yml`, `deploy-portfolio.yml`.
- **Docs**: Data model spec, reconciliation procedures, API docs.

## 8. AI Orchestrator & Feature Store Team
- **Scope**: Inference service, feature transformations, drift detection, feedback ingestion, model registry.
- **Deliverables**: AI orchestrator service, feature store connectors, drift monitors.
- **Tests**: Shadow/canary suites, drift alarms, inference audit logs, latency budgets.
- **CI/CD**: `ci-ai.yml`, `deploy-ai.yml`.
- **Docs**: Model promotion guide, feature catalog, feedback pipeline runbook.

## 9. Observability & Telemetry Team
- **Scope**: Telemetry SDK, OTEL exporters, metrics catalog, dashboards, alert definitions.
- **Deliverables**: Shared telemetry crate, Grafana dashboards, alert manager configs.
- **Tests**: Metric schema validation, trace propagation, alert smoke tests.
- **CI/CD**: `ci-observability.yml`, `deploy-observability.yml`.
- **Docs**: Metrics glossary, dashboard how-to, incident response checklist.

## 10. Ops Control Plane & Knowledge Hub Team
- **Scope**: Control APIs (GraphQL/REST), operator UI, knowledge cards, audit viewer, RBAC.
- **Deliverables**: Control plane service, frontend dashboard, knowledge hub content pipeline.
- **Tests**: API contract tests, UI regression, RBAC checks, doc timestamp validator.
- **CI/CD**: `ci-ops.yml`, `deploy-ops.yml`.
- **Docs**: Operator guide, knowledge hub structure, linking to external Solana resources.

## 11. Testing & QA Structure
- Each module team: Primary Coder + Tester/Doc Writer + QA Liaison (shared).
- Integration & Tooling maintains nightly multi-module simulations and common fixtures.
- QA Guild runs lint/security workflows before merges, manages `cargo fmt`, `clippy`, `deny`, secret scans.

## 12. Deployment Lifecycle
1. Module team opens PR with feature + tests + docs; module CI must pass.
2. QA liaison approves after lint/security checks.
3. Merge triggers module-specific GH Action to build/push artifact to staging.
4. Orchestration Coordinator verifies health, then promotes to prod via `deploy-<module>.yml`.
5. Integration sprints run e2e simulations (strategy→risk→execution) when tranches complete.
6. Update `codex-re-architecture-5.md` with actual implementation notes and cross-links.

## 13. Communication Cadence
- Daily async updates (Slack/issue comments) per module.
- Weekly live sync: architecture lead, program manager, orchestration coordinator, module leads.
- Cross-team design reviews held before interface changes enter development.

## 14. Entry Checklist for Each Team
- Review the interface section in `codex-re-architecture-5.md`.
- Pull latest scaffolding branch for your module.
- Confirm GitHub Actions workflow names and environment secrets.
- Align with Integration & Tooling on fixtures.
- Coordinate with QA liaison for lint/security requirements.
- Schedule documentation updates with Knowledge Hub curators.

Authored by: Codex (GPT-5)
