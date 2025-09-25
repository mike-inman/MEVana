# API Gateway Consolidation Plan
Created: 2025-09-25

## Purpose
Define the staged migration that evolves `api-service` into the single secured ingress for UI-facing traffic while keeping every existing port and service reachable during the transition.

## Background
- Current UI data pulls originate from multiple binaries: `arb-engine` (port 8085, REST + `/ws/arbs`), `hotpath_arb_engine` (8088, metrics + performance snapshots), `monitoring-service` (8091), and `api-service` (8092 REST/OpenAPI).
- Historical `ui-gateway` implementation (Axum + Kafka fanout) is no longer part of the workspace but its behaviour is required for the frontend (`/ws/prices`, `/ws/swaps`, `/ws/ping`, `/depth/:pool`).
- Port registry already reserves 8095 as `websocket_gateway` (see `config/ports.toml` and `crates/shared-core/src/port_config.rs`).

## Objectives
1. Provide a single authenticated HTTP/WebSocket surface for the UI without closing legacy ports until the UI has switched to the gateway.
2. Preserve OpenAPI/Swagger coverage so downstream consumers can track the contract.
3. Minimise churn by reusing the existing `api-service` crate and middleware stack.
4. Prepare for future modularisation where additional back-end capabilities can sit behind the gateway boundary.

## Target State Overview
- `api-service` continues to expose REST under `/api/v1/*` and gains a `/gateway/*` namespace for UI endpoints.
- WebSocket feeds (`/ws/prices`, `/ws/swaps`, `/ws/swaps.v2`, `/ws/ping`) and the Redis-backed `/depth/:pool` endpoint live inside `api-service`.
- The server binds on two host ports during migration: existing 8092 and the reserved 8095 (`MEVANA_WEBSOCKET_GATEWAY_PORT`).
- Internal HTTP calls from the gateway to `arb-engine`, `hotpath_arb_engine`, and `monitoring-service` replace the UI’s direct connections while retaining access for diagnostics.
- JWT middleware in `api-service` ensures downstream requests carry user context, letting us firewall legacy ports when ready.

## Implementation Plan

### Phase 0 — Discovery (Complete)
- Catalogue existing UI dependencies (REST + WS) sourced from docs and runtime configuration.
- Confirm Kafka topic requirements (`prices.latest`, `orca-swaps.parsed`, `alerts.arbitrage.opportunities`).

### Phase 1 — Gateway Module Scaffolding
- Add a `gateway` module inside `crates/api-service` with distinct submodules:
  - `websocket`: Kafka consumers, broadcast channels, `/ws/*` handlers, `/depth/:pool` Redis reader.
  - `proxy`: lightweight clients for `arb-engine`, `hotpath_arb_engine`, and `monitoring-service` (REST + optional WS handshake testing).
  - `routes`: router that mounts `/gateway` paths and wires middleware (rate limiting, auth, tracing).
- Introduce configuration struct binding `MEVANA_WEBSOCKET_GATEWAY_PORT`, Kafka topics, and Redis connection details.

### Phase 2 — Dual-Port Operation
- Update the Axum server bootstrap to optionally listen on both 8092 (`PORT`) and 8095 (`GATEWAY_PORT`) using shared state.
- Register new OpenAPI components documenting the `/gateway` endpoints while marking legacy direct-access URLs as “internal use”.
- Add Prometheus metrics for message fanout and proxy latency to ensure parity with the legacy gateway logs.

Deployment checklist:
- Set `MEVANA_GATEWAY_ENABLE=true` in `common.env` (or the systemd unit override) to activate the gateway listener.
- Ensure `REDIS_URL` and `KAFKA_BROKER` point to reachable hosts; the gateway reuses these for depth snapshots and Kafka fanout.
- Verify `/gateway/health`, `/gateway/ws/swaps`, and `/gateway/depth/<pool>` before allowing the UI to cut over.

### Phase 3 — UI Cutover
- Update UI configuration to consume `/gateway/ws/*` and `/gateway/*` REST calls (feature flag controlled).
- Deploy with dual bindings; monitor Kafka lag, WebSocket connection counts, and HTTP status metrics.
- Provide rollout checklist in deployment docs covering firewall rules, environment variables, and Grafana dashboard updates.

### Phase 4 — Port Hardening (Post-Cutover)
- Once UI traffic exclusively targets the gateway, tighten firewall rules so ports 8085/8088/8091/8092 are reachable only from trusted networks.
- Document diagnostic workflows that leverage the gateway instead of direct service access.
- Evaluate splitting gateway logic into its own crate/binary if operational requirements demand separate deploys.

## Security & Observability Considerations
- Reuse existing JWT middleware for REST; evaluate WebSocket token handshake to ensure authenticated subscriptions.
- Ensure service-to-service proxy calls include mTLS or network ACLs before closing the legacy ports.
- Extend logging/tracing with structured fields (`client_id`, `topic`, `latency_ms`) for fanout paths.
- Update Prometheus scrape config to include the new gateway metrics endpoint.

## Risks & Mitigations
- **Risk**: Topic fanout latency increases when moved into `api-service`.
  - *Mitigation*: Keep consumer processing lightweight; benchmark before cutover; add back-pressure metrics.
- **Risk**: Dual-port binding introduces startup complexity.
  - *Mitigation*: Feature-flag the secondary listener and add integration tests.
- **Risk**: UI depends on undocumented legacy endpoints.
  - *Mitigation*: Coordinate with UI team; add temporary request logging to inventory unknown calls.

## Open Questions
- Do we enforce per-tenant throttling at the gateway or rely on downstream rate limits?
- Should WebSocket subscriptions share the JWT session or use signed short-lived tokens?
- Is there a need for gRPC or SSE alongside WebSockets for future clients?

## Next Actions
1. Implement `gateway` module scaffolding (Phase 1).
2. Add configuration plumbing for the secondary listener and Kafka topics.
3. Draft UI migration guide referencing the new `/gateway` routes.
