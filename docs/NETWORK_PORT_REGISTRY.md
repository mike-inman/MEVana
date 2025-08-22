### MEVana Network Port Registry

Last updated: 2025-08-20 03:36 UTC

This document tracks bind addresses and ports for all services to prevent conflicts across local dev, Docker, and production. Update this file whenever a port or bind address changes.

#### Port Allocation Policy
- 8084 is RESERVED for dex-stream-universal (primary data origin). Do not assign this port to any other service.
- Reserve one health/HTTP port per service. Avoid reusing ports across services on the same host.
- Always set network via env/config files; do not hardcode.
- Use 8088+ for temporary local experiments to avoid collisions with reserved ports.
- Check availability before starting: `lsof -nP -iTCP:<port> -sTCP:LISTEN`.

#### Core Services (HTTP/Health)

| Service | Bind address env | Default bind | Health/HTTP endpoints | Source refs | Notes |
|---|---|---|---|---|---|
| token-registry-service | `BIND_ADDRESS` | `0.0.0.0:8083` | `/healthz` | `crates/token-registry-service/src/main.rs` | Use 8083 for registry; avoid 8084 to prevent conflicts. |
| dex-stream-universal | `BIND_ADDRESS`, `HEALTH_PORT` | `0.0.0.0:8084` | `/health`, `/metrics` | `crates/dex-stream-universal/src/main.rs`, `env.*` | Standardized on 8084. |
| normalizer-service | `BIND_ADDRESS`, `HEALTH_PORT` | `0.0.0.0:8085` | `/health`, `/metrics` | `crates/normalizer-service/Dockerfile`, `network-config.yml` | Default 8085. |
| shyft-validator | `BIND_ADDRESS`, `HEALTH_PORT` | `0.0.0.0:8086` | `/health`, `/metrics` | `crates/shyft-validator/README.md`, `network-config.yml` | Default 8086. |
| shyft-pool-depth | `BIND_ADDRESS`, `HEALTH_PORT` | `0.0.0.0:8087` | `/health`, `/metrics` | `crates/shyft-pool-depth/network-config.yml` | Default 8087. |
| ui-gateway | `BIND_ADDRESS` | `0.0.0.0:8081` | `/healthz` | `crates/ui-gateway/src/main.rs` | Configurable via env; default 8081. |

#### External Services / Dependencies

- Redis: `6379` (host → container mapping typically `6379:6379`)
- Kafka: `9092` (internal), host listener `29092`
- Zookeeper: `2181`
- Prometheus: `9090`
- Grafana: `3000`

References: `infrastructure/docker-compose.yml`, `crates/*/docker-compose.yml`, `crates/dex-stream-universal/env.monitoring`.

#### Known Host Port Mappings (Docker)
- Example mappings from `infrastructure/docker-compose.yml`:
  - `8084:8084`, `8085:8085` (multiple services)
  - `8083:8081` (host 8083 mapped to a service internal 8081)
  - `9090:9090` (Prometheus), `3000:3000` (Grafana), `29092:29092` (Kafka HOST), `2181:2181` (ZK)

When running locally and in Docker simultaneously, ensure host ports (left side of mapping) do not collide with locally started binaries.

#### Current Observations (local dev)
- 2025-08-20 03:33 UTC:
  - 8084: Docker is listening due to `dex-stream-universal` container (expected). No stray local processes found.
  - 8083: No active listeners detected at this moment; previously used by token-registry-service.
  - 8085: Docker is listening (likely normalizer-service in containers).

#### Recommended Reserved Port Map (host)
- 8081: ui-gateway (HTTP/WS)
- 8083: token-registry-service
- 8084: dex-stream-universal (health/metrics) [RESERVED]
- 8085: normalizer-service (health/metrics)
- 8086: shyft-validator (health/metrics)
- 8087: shyft-pool-depth (health/metrics)
- 8088+: ad hoc local tests

#### Update Procedure
1. Before starting a new service, choose a reserved or free port per this registry.
2. Set `BIND_ADDRESS`/`HEALTH_PORT` in env or config files (never in code).
3. Verify availability: `lsof -nP -iTCP:<port> -sTCP:LISTEN`.
4. Update this document with:
   - Change summary
   - Timestamp
   - Adjusted reserved map and references

#### Change Log
- 2025-08-20 03:25 UTC: Initial registry created. Noted 8084 conflict; standardized token-registry-service on 8083. Marked 8084 as RESERVED for dex-stream-universal.
- 2025-08-20 03:36 UTC: Externalized `ui-gateway` bind to `BIND_ADDRESS`; updated registry.
- 2025-08-20 03:33 UTC: Audited ports — 8084 in use by dex-stream-universal (Docker), no accidental local listeners; 8083 currently free.


