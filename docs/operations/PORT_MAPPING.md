# MEVana Port Allocation Guide
Last updated: 2025-09-26

Keep this page as the single source of truth for every TCP port we bind in development, staging, and production. When you add a new listener, update this table first to prevent collisions or firewall surprises.

## Infrastructure (Docker Compose)
| Service | Host Port (Prod*) | Container/Internal | Purpose | Notes |
|---------|-------------------|--------------------|---------|-------|
| Zookeeper | — | 2181 | Kafka coordination | Internal only on `mevana_net`; bare-metal services never talk to it directly. |
| Kafka | 29092 | 9092 | Kafka broker | Use `localhost:29092` from systemd services; containers use `kafka:9092`. |
| Kafka UI | 8084 | 8080 | Kafka administration UI | Behind basic auth; optional in production. |
| Redis | 6380 (dev) / — (prod) | 6379 | Caching + rate limits | Local `docker-compose.core.yml` maps 6380; server stack keeps Redis internal—point services at `redis://redis:6379/0`. |
| Redis UI | 8083 | 8001 | Redis Insight | Diagnostics only; disable in hardened environments. |
| PostgreSQL | 5433 | 5432 | Primary database | Systemd services use `postgresql://localhost:5433/...`. |
| Adminer | 8087 | 8080 | Postgres admin UI | Disable in production if not needed. |
| Prometheus | 9090 | 9090 | Metrics collector | Scrapes bare-metal services via `host.docker.internal`. Local core stack maps to 9091. |
| Grafana | 3000 | 3000 | Observability dashboards | Provisioned from `infrastructure/grafana/**`. Local core stack maps to 3001. |

\* “Prod” refers to `docker-compose.server.yml`. The developer-focused `docker-compose.core.yml` publishes slightly different host ports (see notes).

## Bare-Metal Application Services (systemd)
| Unit / Service | Port(s) | Purpose | Notes |
|----------------|---------|---------|-------|
| `hotpath-arb-engine.service` | 8088 (metrics), 8089 (readiness/`/performance`) | Zero-latency MEV detection | Prometheus scrapes `host.docker.internal:8088`. Keep 8089 free for the readiness probe. |
| `normalizer-service.service` | 9901 | Transaction normalization metrics | Kafka in/out only; HTTP surface is metrics + health. |
| `discovery-service.service` | 9102 | Program discovery metrics | Used by Prometheus and diagnostics. |
| `edge-assembler` (optional) | 9902 | Real-time edge processing | Spawn manually via dev scripts; reserve the port even when idle. |
| `executor-service.service` | 9898 (admin API & health), 9904 (metrics) | Trade execution + wallet administration | Grafana buttons call the 9898 admin API via Clerk. |
| `wallet-vault.service` | 8090 | Wallet metadata & encryption boundary | Health at `/health`, REST at `/wallets`. |
| `monitoring-service.service` | 8091 | Aggregated trading metrics + `/api/system/status` | Update UI & scripts to hit 8091 (not 8090). |
| `api-service.service` | 8092 (REST), 8095 (gateway WebSocket/SSE when `MEVANA_GATEWAY_ENABLE=true`) | Public API + UI gateway | Ensure Kafka topics (`prices.latest`, `orca-swaps.parsed.v2`, etc.) exist before enabling the gateway. |
| Token registry (embedded in executor) | 9898 | Token metadata endpoint | Shares the executor admin listener; no separate unit. |

## UI and Operator Interfaces
| Component | Host Port | Protocol | Notes |
|-----------|-----------|----------|-------|
| MEVana UI (`mevana-ui.service`) | 4200 | HTTP/HTTPS | Next.js frontend; be sure `NEXT_PUBLIC_GATEWAY_*` variables point at 8095. |
| Grafana | 3000 | HTTP/HTTPS | Admin credentials + Slack contact point require `/etc/mevana/alerts.env`. |
| Prometheus | 9090 | HTTP | Useful for ad-hoc queries and target inspection. |
| Kafka UI | 8084 | HTTP | Restrict via firewall in production. |
| Adminer | 8087 | HTTP | Disable outside trusted networks. |
| Legacy Arb Engine (optional) | 3002 | HTTP | Reserved for compatibility dashboards; keep closed unless needed. |
| Future Web UI (staging placeholder) | 3200 | HTTP | Allowed by firewall; ensure unused ports stay blocked externally if not in service. |

## Firewall Snapshot (mevana-nyc-01)
| Rule | Port | Status | Notes |
|------|------|--------|-------|
| 1 | 22/tcp | ALLOW | SSH management. |
| 2 | 8088/tcp | ALLOW | `hotpath-arb-engine` metrics. |
| 3 | 8080/tcp | ALLOW | Legacy HTTP (Kafka UI); confirm decommission status. |
| 4 | 3000/tcp | ALLOW | Grafana. |
| 5 | 9090/tcp | ALLOW | Prometheus. |
| 6 | 8090/tcp | ALLOW | `wallet-vault`. |
| 7 | 9901/tcp | ALLOW | `normalizer-service`. |
| 8 | 9102/tcp | ALLOW | `discovery-service`. |
| 9 | 9904/tcp | ALLOW | `executor-service` metrics. |
| 10 | 8089/tcp | ALLOW | `hotpath-arb-engine` readiness / performance. |
| 11 | 3002/tcp | ALLOW | Legacy arb-engine path (staging). |
| 12 | 3200/tcp | ALLOW | Future web UI placeholder. |
| 13 | 4200/tcp | ALLOW | MEVana UI. |
| 14 | 8095/tcp | ALLOW | API gateway (websocket/SSE). |
| 15 | 8096/tcp | ALLOW | Reserved (verify requirement). |
| 16 | 9898/tcp | ALLOW | `executor-service` admin + token registry. |
| 17–32 | (v6 duplicates) | ALLOW | IPv6 equivalents for above ports. |

> Firewall observation: 8080/3002/3200/8096 are open even though no active services were found bound in compose/systemd. Confirm whether they’re still needed or close them to reduce exposure.

## Reserved & Legacy Ports
| Port | Status | Reason |
|------|--------|--------|
| 8085 | Reserved | Legacy `arb-engine` WebSocket API; keep clear for compatibility tests. |
| 8093–8099 | Reserved | Future API or gateway expansions. |
| 9000–9099 | Reserved | High-throughput internal services (leave unused unless ratified). |
| 9500–9599 | Reserved | Operational tooling. |
| 9800–9897 | Reserved | Experimental or perftest listeners. |

## Operational Notes
- Containers reach bare-metal services through `host.docker.internal:<port>`; Prometheus’ scrape configs rely on this mapping.
- Keep `ufw` (or your firewall of choice) aligned with the table above. Typical openings: `3000`, `4200`, `8084`, `8087`, `8090–8095`, `9090`, `29092`, `5433`.
- Before binding a new port, search this document and the repo for existing usage. Update the appropriate table and, when applicable, adjust Prometheus and Grafana provisioning files.
- When running the developer stack (`docker-compose.core.yml`), be mindful that Grafana maps to `3001` and Prometheus to `9091`. Adjust env vars or port forwarding as needed to avoid conflicts with the production mappings listed above.
- All new services MUST publish metrics on a distinct port or path; document them here before merging.
