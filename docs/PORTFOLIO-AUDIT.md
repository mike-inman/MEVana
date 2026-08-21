# MEVana Portfolio Architecture Audit

Date: 2026-08-21

## 1. Executive Summary

MEVana is a multi-crate Rust workspace for Solana market-data ingestion, normalized swap/event processing, edge-based arbitrage detection, execution experimentation, and operations monitoring.

The strongest implemented system is an event-driven Rust pipeline around Geyser/Yellowstone input, Kafka topics, edge assembly, hot-path arbitrage detection, PostgreSQL-backed operational state, Redis/Prometheus/Grafana observability, and Docker/systemd/GitHub Actions deployment assets.

The repo also contains substantial prototype and legacy surface. Several execution, Jito, flash-loan, and route-template paths are implemented as scaffolds or simulations rather than proven live trading. Employer-facing documentation should present the project as a real-time Solana trading systems platform/prototype, not as a proven production MEV bot with live profitable execution.

Public repository readiness is blocked until secret-bearing config/docs/scripts and generated/local artifacts are removed or rewritten. The scan found committed env files, provider token examples that appear real, cookies, logs, `.DS_Store` files, and keypair-looking files. Do not publish publicly until credentials are rotated and history is scrubbed.

## 2. Repository Map

Concise top-level structure:

```text
.
|-- Cargo.toml                     # Rust workspace
|-- crates/                        # Rust services and libraries
|   |-- hotpath_arb_engine/         # primary Geyser + hot-path arbitrage service
|   |-- executor-service/           # opportunity execution worker/API and wallet ops
|   |-- normalizer-service/         # Kafka transaction normalizer utilities
|   |-- edge-assembler/             # normalized pool updates -> EdgeDelta stream
|   |-- arb-engine/                 # arbitrage signal detection library and bins
|   |-- api-service/                # REST/WebSocket API and Kafka fanout
|   |-- kafka_db_writer/            # Kafka opportunity/execution persistence worker
|   |-- price-aggregator/           # swap stream -> price snapshots
|   |-- monitoring-service/         # Redis/Postgres monitoring API
|   |-- wallet-vault/               # encrypted wallet vault service
|   |-- wallet-metrics-exporter/    # wallet Prometheus exporter
|   |-- shared-core/, shared-protocol/
|   |-- normalizer-runtime/, normalizer-generated/, normalizer-codegen/
|   |-- flash-loan-providers/, flash-loan-receiver/
|   |-- whirlpool-core/             # Orca Whirlpool decode/math/quoter
|   |-- dex-registry/, discovery-service/, token-registry-service/
|   |-- mevana-engine/, mevana-idl-registry/, resilience-framework/
|   `-- _archive/                   # excluded legacy crates
|-- database/                       # SQL schema and migrations
|-- infrastructure/                 # Docker Compose, Prometheus, Grafana, systemd
|-- scripts/                        # CI, deployment, lifecycle, monitoring, testing scripts
|-- config/                         # environment/config samples and local env files
|-- docs/                           # architecture, operations, strategy, archived docs
|-- .github/workflows/              # CI/CD and self-hosted deployment workflows
|-- logs/, backups/, target/        # local/generated artifacts present in checkout
```

Important workspace facts:

- `Cargo.toml` defines 25 workspace members and default-members limited to `hotpath_arb_engine`, `executor-service`, `normalizer-generated`, `normalizer-service`, and `edge-assembler`.
- Excluded crates include historical DEX services such as `orca-service`, `raydium-service`, `meteora-service`, `jupiter-service`, `pump-service`, and archive directories.
- Binaries discovered from Cargo metadata include `hotpath_arb_engine`, `executor-service`, `normalizer-service`, `edge-assembler`, `arb-engine`, `arb_hot`, `arb_replay`, `price-aggregator`, `kafka_db_writer`, `api-server`, `wallet-vault`, `wallet-metrics-exporter`, `discovery-service`, `dex-registry`, and several test/probe binaries.

## 3. Runtime Architecture

Normal runtime path, based on active source:

```text
Solana Geyser/Yellowstone/Shyft
  -> hotpath_arb_engine
     -> subscribes to program-specific Geyser transaction streams
     -> processes selected DEX transactions
     -> optionally publishes raw DEX transactions to Kafka topic dex-transactions
     -> feeds in-process FastEngine / EdgeArbitrageDetector
     -> emits arbitrage opportunities to Kafka topic alerts.arbitrage.opportunities
     -> exposes Prometheus metrics and auxiliary HTTP/WebSocket APIs
     -> optionally writes orderbook/history/config data to PostgreSQL
     -> optionally publishes Redis metrics for monitoring-service

Kafka normalized path
  -> normalizer-service consumes raw/decoded transaction topics
  -> edge-assembler consumes parsed Orca pool/tick topics
  -> edge-assembler emits edge.delta.v1 and edge.snapshot.v1
  -> hotpath_arb_engine consumes edge.delta.v1 and evaluates edge cycles
  -> executor-service consumes opportunities and produces execution results
  -> kafka_db_writer persists opportunity/execution events into PostgreSQL

Observability/control plane
  -> Prometheus scrapes service metrics
  -> Grafana dashboards and alert rules consume Prometheus
  -> monitoring-service reads Redis/Postgres and exposes API views
  -> api-service exposes REST/WebSocket API and Kafka fanout streams
```

Startup examples:

- `crates/hotpath_arb_engine/src/main.rs` loads `.env`, parses CLI/env (`GEYSER_ENDPOINT`, `X_TOKEN`, `METRICS_PORT`), initializes smart-filter config, property store, optional Kafka publisher, optional PostgreSQL config/listener/history, Prometheus exporter, Redis metrics publisher, then starts Geyser streams.
- `crates/executor-service/src/main.rs` starts either a Geyser-only diagnostic mode or the execution service with DB, wallet, Kafka, metrics/admin routes, and signal handling.
- `crates/edge-assembler/src/main.rs` creates a Kafka consumer/producer, starts a Warp `/metrics` and `/health` server, subscribes to parsed venue topics, and loops on Kafka messages.
- `crates/kafka_db_writer/src/main.rs` consumes opportunity/execution topics, batches messages, and writes them in PostgreSQL transactions.

Shutdown/error handling is mixed. Kafka loops log and sleep on errors; executor-service imports `tokio::signal`; hotpath Geyser stream tasks log termination errors but are mostly long-running loops. There is circuit-breaker logic in both detection and execution areas, but not a uniform service lifecycle abstraction.

## 4. Major Components

Implemented and active/default:

- `hotpath_arb_engine`: primary service for Yellowstone/Geyser ingestion, stream routing, property/config APIs, metrics, Kafka opportunity publishing, edge-delta ingestion, and in-process arbitrage signal routing.
- `arb-engine`: reusable detection logic. `FastEngine` stores quote snapshots and owns `EdgeArbitrageDetector`; edge detection uses `DashMap`, stale-edge pruning, 2-leg and localized 3-hop scanning, and signal emission.
- `edge-assembler`: consumes parsed pool/tick data, converts Orca Whirlpool state into protobuf `EdgeDelta`, publishes delta/snapshot Kafka messages, and exposes Prometheus metrics.
- `normalizer-runtime` and `normalizer-generated`: normalized swap model, amount inference, generated decoder registry.
- `executor-service`: Kafka opportunity consumer/execution worker plus admin/health/wallet HTTP surface. Includes Shyft RPC client, wallet service, Jito optimizer, bundle builder, and risk controls, but live execution has scaffolded/simulated pieces.
- `kafka_db_writer`: asynchronous Kafka-to-Postgres persistence.
- `price-aggregator`: consumes swap events and emits rolling price snapshots when pair price changes.
- `monitoring-service`, `wallet-metrics-exporter`, `api-service`: monitoring/API/fanout services around Redis, Postgres, Prometheus, Kafka, Shyft, and Jupiter price APIs.

Implemented but optional/feature-gated:

- `arb-engine` features include `development`, `stubs`, `production`, `diagnostics`, `profiling`, `whirlpool`, `raydium`, `meteora`, `full`, `experimental`, `dev-full`, and `prod-basic`.
- `executor-service` features include `strict-accounts` and `admin-mtls`.
- `hotpath_arb_engine` has `experimental-hooks`.
- `monitoring-service` defaults to its `database` feature.
- `token-registry-service` has optional `redis` and `external-sources`.

Prototype/experimental:

- `transaction_templates` builds route/template abstractions, but hydration returns placeholder system-program transactions.
- `hotpath_arb_engine::jito_integration` has bundle tracking and a tip transaction helper, but bundle submission is explicitly simulated.
- `arb-engine::fringe_risk_manager` enforces rules, but risk profiles use mock liquidity/volatility data.
- Several AI strategy/intelligence modules and documents appear experimental and should not be described as proven production ML.

Legacy/dead or excluded:

- `crates/_archive/*`, `_archive/*`, and excluded standalone DEX service crates are not part of the active workspace.
- Existing docs mention many components that no longer appear active or are explicitly excluded by the root workspace.

## 5. Solana Integrations

Implemented and active:

- Yellowstone/Geyser gRPC: `hotpath_arb_engine` imports `yellowstone_grpc_client::GeyserGrpcClient` and `yellowstone_grpc_proto`, builds Geyser clients, subscribes to streams, and processes `SubscribeUpdateTransaction`.
- Shyft Geyser endpoint defaults and tokens are wired through CLI/env in hotpath and executor config. This is real connectivity code, but committed hard-coded token values are a public-repo blocker.
- Solana JSON-RPC via Shyft client: `executor-service/src/shyft_client.rs` implements `getAccountInfo`, `getTokenAccountsByOwner`, `simulateTransaction`, `sendTransaction`, and `getLatestBlockhash` requests.
- Wallet/keypair handling: executor wallet service loads configured keypair paths; wallet-vault stores encrypted keypairs in Postgres and can export/decrypt through API code.
- Orca Whirlpool: `whirlpool-core` decodes Whirlpool/TickArray data and quotes CLMM transitions; `edge-assembler` uses `ClmmQuoter` to create Orca edge deltas.

Implemented but optional/partial:

- Jupiter: scanner and integration code exist for quote comparison/multi-hop opportunity scanning. It defaults to paper trading in hotpath config unless env overrides disable it. Wallet metrics also uses Jupiter price API fallback.
- Jito through Shyft client: executor service contains a real HTTP call path to a Shyft/Jito bundle endpoint, retry/tip escalation logic, and status parsing. Separately, hotpath's `jito_integration` simulates submission. Treat this as partial, not end-to-end proven.
- Transaction simulation and blockhash handling exist through `ShyftClient::simulate_transaction` and `get_recent_blockhash`; bundle builder maintains blockhash/template state.
- Priority fees and compute budgets appear in dependencies and optimizer parameters, but route construction still has placeholder/simplified pieces.

Prototype or not proven:

- Flash loans: provider modules and optimization abstractions exist (`flash-loan-providers`, `flash-loan-receiver`, executor flash-loan optimizer), but parts are strategy/planning scaffolds and should not be marketed as live flash-loan execution.
- Address lookup tables: no strong active implementation evidence found in the audited source slices.
- Sandwich/backrun/liquidation: references/comments exist, but no confirmed complete active strategy implementation found.

## 6. Data Flow

Market-data ingress:

- Geyser transaction streams enter `hotpath_arb_engine`, filtered by configured Solana program IDs. Multi-endpoint mode creates region/stream configurations; same-endpoint mode consolidates to one connection with internal routing.
- Optional raw Kafka publishing sends encoded `SubscribeUpdateTransaction` payloads to `DEX_TRANSACTIONS_TOPIC`/`dex-transactions` using a low-latency `mpsc` worker and `rdkafka::FutureProducer`.
- `normalizer-service` consumes Kafka transaction payloads and uses Yellowstone proto decoding plus normalizer runtime/generated extractors to emit parsed swap shapes.
- `edge-assembler` consumes Orca parsed pool/tick topics (`orca-swaps.parsed.v2`, `orca-tickarrays.parsed.v2` defaults), updates `PoolManager`, and emits protobuf edge deltas/snapshots.
- `price-aggregator` consumes swap events, computes raw `amount_out / amount_in` prices, suppresses insignificant changes, and publishes price snapshots.

Caching/backpressure:

- Hotpath raw Kafka publisher uses bounded `tokio::mpsc` and drops/enumerates failures when full.
- Edge detection uses `DashMap` caches keyed by token pair and pool, plus token indexes.
- Jupiter integration and token registry use local caches.
- Kafka consumers generally manual-commit after processing; some error paths log and continue.

## 7. Opportunity Detection

Implemented:

- Two-leg cross-venue arbitrage from edge deltas. `EdgeArbitrageDetector` partitions fresh edges by direction, compares venue-disjoint sell/buy edges, evaluates profit, chooses the best signal, records latency/profit metrics, and emits `ArbSignal`.
- Localized 3-hop edge cycles. After 2-leg scanning, the detector calls a localized 3-hop scan seeded by the updated pair's two tokens. Tests include `edge_profit_three_leg_unit.rs` and `arb_signal_generation_three_leg.rs`.
- Quote-based fallback arbitrage in `FastEngine::scan_pair`, comparing stored quotes across opposite directions and different DEX labels.
- Jupiter multi-hop scanner. `JupiterArbitrageScanner` periodically queries an integration layer across token sets and amounts, filters by price impact and hop count, and defaults to paper-trading accumulation unless configured otherwise.

Risk/filtering:

- Profit thresholds via `ARB_MIN_PROFIT_BPS` and atomic runtime updates.
- Slippage threshold via `ARB_MAX_SLIPPAGE_BPS`.
- Stale edge rejection through `EDGE_MAX_STALENESS_MS` / property store.
- Fringe token risk manager performs position, liquidity, volatility, risk-score, and minimum-profit checks, but profile data is mocked.
- Smart filter/property store exposes many runtime controls and validation bounds.

Do not claim:

- Liquidation, sandwich, or backrun systems are complete.
- Jupiter scanner is live execution by default; it is paper-trading by default.
- Risk profile liquidity/volatility is externally verified; the active implementation uses mock profile creation.

## 8. Execution Pipeline

Intended path:

```text
ArbSignal / opportunity
  -> hotpath execution queue and/or Kafka alerts.arbitrage.opportunities
  -> executor-service ExecutionEngine consumes Kafka
  -> parse/convert opportunity
  -> dedupe and risk/circuit validation
  -> optional flash-loan planning
  -> BundleBuilder / JitoOptimizer
  -> simulation/preflight or live submit, depending on flags
  -> produce execution result
  -> kafka_db_writer/API persistence/monitoring
```

Implemented:

- `ExecutionEngine` parses both modern opportunity JSON and legacy hotpath payloads, maps venues, computes stable IDs, and stores execution state.
- Dedupe cache tracks recent opportunity hashes with TTL/max size.
- Execution result model tracks statuses from `Received` through `Finalized`, including `PreflightSuccess`, `Submitted`, `Unlanded`, `Rejected`, and `Failed`.
- `ShyftClient` can simulate and send base64 transactions via JSON-RPC.
- Wallet service supports balance checks, reserved balances, active trade tracking, wallet rotation/failover concepts, admin transfer/airdrop routes, and wallet health.
- Executor orchestrator builds execution plans with risk parameters, optional flash loan optimization, Jito optimization, and metrics.

Partial/scaffold:

- `BundleBuilder` has venue registry and route-template caching, but parts create placeholder system-program transactions and simplified templates.
- `hotpath_arb_engine::jito_integration` explicitly simulates bundle submission.
- Executor `JitoOptimizer` uses `ShyftClient::submit_jito_bundle`, but the audited code includes simulated timing/profit and simplified bundle payload handling.
- End-to-end real transaction construction for each DEX route is not fully proven from source.

## 9. Performance Engineering

Evidence-backed performance work:

- Rust/Tokio async services with `rdkafka` consumers/producers and low-latency producer settings (`linger.ms=0`, small buffering, manual Kafka commits).
- Hotpath separation: `hotpath_arb_engine` directly embeds `FastEngine` and `EdgeArbitrageDetector` to avoid serializing every detection path.
- Lock-conscious structures: `DashMap`, atomics, bounded `mpsc`, `RwLock`, `OnceCell`, and per-path caches are used throughout hotpath and executor code.
- Edge freshness controls and latency metrics record nanosecond edge timestamps and wall-clock staleness.
- `whirlpool-core` contains CLMM math/quoter functions for marginal price and tick-boundary sizing.
- `edge-assembler` computes bidirectional edge deltas per pool update and publishes snapshots on an interval.
- Bench/profile hooks exist via `arb-engine` optional `profiling = ["pprof"]`.
- CI includes focused hotpath cargo checks, workspace clippy/build, and cargo-deny.

Limitations:

- Some performance claims in comments use "zero latency" or similar phrasing; documentation should translate this to concrete design: hot-path separation, bounded channels, atomics, low-latency Kafka settings, and cached edge graphs.
- No full reproducible benchmark report was created in this audit.

## 10. Infrastructure & Persistence

Persistence:

- PostgreSQL schemas live in `database/schema.sql`, `database/complete_schema.sql`, and migrations under `database/migrations/`.
- Tables/views include arbitrage opportunities, execution results, performance metrics, smart filter config/history, preflight summary/metrics, and transaction logger objects.
- `kafka_db_writer` persists opportunity and execution event streams asynchronously in transactions.
- `hotpath_arb_engine` optionally connects to PostgreSQL for smart filter config, `LISTEN`/`NOTIFY` style real-time config updates, and off-hotpath orderbook history.
- `api-service`, `monitoring-service`, `wallet-vault`, and `resilience-framework` use `sqlx`/Postgres pools.
- Redis is used for monitoring metrics sharing, API/gateway/fanout, and optional token-registry support.

Deployment assets:

- Dockerfiles exist at root and per-service for hotpath, normalizer, executor, discovery, monitoring, token registry, and arb-engine.
- Compose files exist for local, core, full, server, monitoring, and example stacks.
- systemd units exist for core services and supporting exporters.
- GitHub Actions include cargo-deny, rust lint/build, hotpath cargo check, GHCR build/deploy, self-hosted deploy/diagnostics/restart/cleanup/auto-heal, and repo hygiene.

## 11. Observability

Implemented:

- Prometheus metrics exporters in hotpath (`metrics-exporter-prometheus`), edge-assembler (`prometheus` + Warp), wallet-metrics-exporter, wallet-vault, api-service, and monitoring-service.
- Grafana dashboards under `infrastructure/grafana/dashboards/curated/` for dry-run insights, enhanced arbitrage, live test monitoring, and system status.
- Prometheus alert rules under `infrastructure/prometheus/rules/geyser_alerts.yml` for Geyser down/stalled/message-rate/Dex-message/latency conditions.
- `monitoring-service` collects Redis keys such as arbitrage stats, performance metrics, and system status, and can query Postgres for live arbitrage/performance views.
- Structured/log metrics include opportunity counts, rejection reasons, preflight simulation counts, gas/slippage estimates, wallet balances, token prices, edge staleness, circuit breaker state, and Kafka publish/drop counters.

Risks:

- Some Grafana queries reference metric names that may not exactly match active code names. Verify dashboards against a live Prometheus scrape before using them in portfolio screenshots.
- Many logs are committed and should be removed before publication.

## 12. Risk Controls

Implemented controls:

- Safe defaults in `LiveTradingConfig`: live execution disabled, dry-run enabled, simulation enabled, small position sizes, daily loss limit, confidence threshold, circuit breaker, max trades per minute, venue-specific size caps.
- Validation prevents live execution while `dry_run=true` or `simulation_mode=true`, and requires Jito keypair path when Jito is enabled for live mode.
- Edge stale-data rejection and runtime threshold controls.
- Profit and slippage thresholds in edge/Jupiter configs.
- Executor risk parameters: max position size, max gas, min profit threshold, max execution time, circuit breaker flag, daily loss limit.
- Execution engine has dedupe cache, circuit counters for failures/unlanded results, daily realized PnL tracking, and pre-submit snapshots for reconciliation.
- Wallet service balance checks, reserved balances, daily transaction count, wallet rotation/deactivation, active trade tracking, and health status.
- Admin auth paths support Clerk/static token/allowlist combinations; mTLS is feature-gated for admin routes.

Partial:

- Some risk fields are configured but not always fully enforced in every path.
- Fringe risk data is mocked.
- Actual on-chain profit reconciliation exists as data structures and snapshots, but the full live reconciliation pipeline was not proven end to end in this audit.

## 13. Tests

Source-level test coverage found:

- `arb-engine/tests/`: AI/metrics endpoints, signal generation, three-leg arb, edge decode/parsing/profit, circuit breaker, live edge audit, log rotation, metric aliases.
- `executor-service/tests/`: mTLS handshake/reload/reload-binary and smoke connect.
- `hotpath_arb_engine/tests/live_transaction_data_test.rs`: live endpoint/Prometheus checks, gated by environment flags in places.
- Unit tests inside libraries such as `normalizer-runtime`, `wallet-vault`, `resilience-framework`, and `api-service`.
- Scripts under `scripts/testing/` for observability, Shyft APIs, tickarray fallback, validation pipeline, gateway fanout smoke, latency tests, and end-to-end tests.

Coverage that appears strong:

- Edge parsing/profit/detection units.
- Circuit breaker mechanics.
- Auth/mTLS behavior in executor-service.
- Wallet encryption/decryption unit coverage.
- Normalized amount inference.

Coverage gaps:

- No verified full live trading E2E proof from Geyser to landed transaction was established.
- Many live/infrastructure tests depend on external services and env credentials.
- Some integration paths are smoke/probe scripts rather than deterministic CI tests.

## 14. Public Repo Hygiene

Blocking concerns, redacted by category:

- `CLAUDE.md`: hard-coded provider token and database password examples.
- `docs/LIVE_TEST_SESSION_2025-01-08.md`, `docs/GEYSER_MONITORING_ALERTS.md`, `docs/ai/AI_CONTEXT.md`, `docs/Shyft/congestion_problem.md`: hard-coded provider token/API key examples.
- `config/environments/development.env`, `config/environments/production.env`, `config/environments/geyser-streamer.env`: committed env files with token/password categories.
- `docker-compose.local.yml`, `infrastructure/docker-compose.yml`: committed token/password defaults.
- `test_hotpath.sh`, `scripts/geyser_recovery.sh`, `scripts/deployment/deploy-vultr.sh`, `_archive/scripts/deployment/*`: hard-coded provider token categories.
- `.claude/settings.local.json`, `crates/hotpath_arb_engine/.claude/settings.local.json`: local command history containing provider token categories.
- `infrastructure/cookies.txt`: committed cookie file.
- `infrastructure/keys/id.json`, `config/keys/jito-keypair.json`: keypair-looking files; confirm whether tracked and remove/rotate if real.
- `infrastructure/scripts/*.log`, `logs/*.log`, `backups/shutdown.log`: committed/local logs may contain operational details.
- `.DS_Store` files across repo, docs, config, crates, infrastructure, database, scripts.
- `target/`, `kafka-test/target/`, `latency_results/`, `pids/`, `profiles/metadata/`: generated/local runtime artifacts should not be in a portfolio repo unless intentionally curated.

Recommended action:

1. Rotate all exposed provider tokens and any keypairs that may have been committed.
2. Remove real env files, cookies, keypair files, logs, `.DS_Store`, and generated artifacts from Git.
3. Add/strengthen `.gitignore`.
4. Rewrite docs/scripts to use placeholders only.
5. If the repo has already been pushed publicly or shared, scrub Git history with an appropriate secret-removal workflow before making it public.

## 15. Employer-Facing Strengths

Evidence-backed themes:

- Low-latency Rust systems design: Tokio async services, atomics, bounded channels, DashMap caches, hot-path separation.
- Event-driven architecture: Kafka topics for raw transactions, parsed swaps, edge deltas, opportunities, execution results, and DB writer decoupling.
- Solana data-plane integration: Yellowstone/Geyser ingestion, Solana JSON-RPC methods, transaction simulation/submission hooks, blockhash handling, wallet keypair patterns.
- DEX/market-structure modeling: Orca Whirlpool decoding, CLMM quoter/math, edge graph construction, bidirectional pool edges.
- Arbitrage detection algorithms: 2-leg cross-venue and localized 3-hop edge-cycle detection with profit/staleness/size checks.
- Observability: Prometheus metrics, Grafana dashboards, alert rules, wallet metrics, Redis/Postgres monitoring service.
- Operational automation: Docker Compose, service Dockerfiles, systemd units, GitHub Actions CI/CD/self-hosted deploy workflows, deployment diagnostics.
- Risk/safety controls: dry-run defaults, live-trading guards, size limits, stale-data rejection, circuit breakers, wallet controls, auth/mTLS paths.
- Secure wallet service design: encrypted wallet vault, key rotation, backups, JWT/API-key auth middleware, operation metrics.
- Resilience patterns: retry, circuit breaker, dead-letter queue, graceful degradation framework.

## 16. Representative Code

- `crates/hotpath_arb_engine/src/main.rs`: primary runtime orchestration, Geyser stream setup, metrics, Kafka, PostgreSQL, Redis, and hotpath signal flow.
- `crates/arb-engine/src/edge_integration.rs`: edge cache, stale-edge pruning, 2-leg/3-hop arbitrage detection, metrics, signal emission.
- `crates/arb-engine/src/engine_fast.rs`: quote cache and bridge from normalized quotes/edges into signal production with risk-manager handoff.
- `crates/edge-assembler/src/main.rs`: Kafka consumer/producer service that converts parsed Whirlpool pool updates into protobuf edge deltas.
- `crates/whirlpool-core/src/quoter.rs`: CLMM quote/marginal-price/tick-boundary logic.
- `crates/executor-service/src/execution_engine.rs`: Kafka opportunity parsing, execution result lifecycle, dedupe/risk state, DB and wallet integration.
- `crates/executor-service/src/shyft_client.rs`: Solana RPC/Jito/Shyft client methods for balance, simulation, transaction submission, blockhash, and bundle status.
- `crates/executor-service/src/bundle_builder.rs`: route-template, blockhash, venue registry, and transaction-building scaffolding worth reviewing with caveats.
- `crates/executor-service/src/wallet_service.rs`: wallet loading, balance/reservation checks, active trade state, transfer/airdrop support, wallet health.
- `crates/wallet-vault/src/encryption.rs` and `crates/wallet-vault/src/vault.rs`: encrypted wallet storage and vault operations.
- `crates/kafka_db_writer/src/main.rs`: batched Kafka-to-Postgres persistence with transaction boundaries.
- `crates/resilience-framework/src/circuit_breaker.rs`, `retry.rs`, `dead_letter_queue.rs`: reusable reliability patterns.

## 17. Claims That Should NOT Be Made

Do not claim:

- "Production live MEV bot" or "profitable live trading system" unless supported by separate live trading evidence.
- "Zero latency"; say "low-latency/hot-path optimized".
- "Fully implemented Jito bundle submission" across the system; one hotpath module simulates Jito, while executor has a partial Shyft/Jito path.
- "Fully implemented flash loans"; provider abstractions and planning exist, but full on-chain execution was not proven.
- "Supports all major Solana DEXes" as active integrations. Orca Whirlpool is the strongest active implementation; Raydium/Meteora/Phoenix/OpenBook/Jupiter/Pump artifacts range from partial adapters/generated decoders to scaffold/references.
- "Complete sandwich/backrun/liquidation strategies"; no complete active implementation was verified.
- "Production secret hygiene"; current repo contains publication blockers.
- "Dashboards all verified live"; queries should be checked against an active Prometheus scrape.
- "End-to-end live execution tested"; tests are a mix of unit, smoke, external-live, and script-based checks.

## 18. Recommended README Structure

Suggested README outline:

1. Project title and one-sentence positioning: "Rust/Solana real-time market-data and arbitrage research platform."
2. Status banner: portfolio/research system; live trading disabled by default; public repo sanitized.
3. Architecture diagram showing Geyser -> Kafka/normalizer/edge assembler -> detector -> executor/DB -> observability.
4. What is implemented now:
   - Geyser ingestion
   - edge graph construction
   - 2-leg/3-hop detection
   - Orca Whirlpool math/quoting
   - Kafka/Postgres persistence
   - Prometheus/Grafana
   - executor and wallet-vault scaffolding
5. What is experimental:
   - Jupiter scanner paper trading
   - Jito/flash loan execution
   - AI strategy modules
6. Repository map and crate list.
7. Local development quickstart using placeholder env variables only.
8. Running focused tests.
9. Observability screenshots/dashboards after verifying metric names.
10. Security note: no real keys; live trading requires separate private config.
11. Representative code links.
12. Roadmap/future work.

