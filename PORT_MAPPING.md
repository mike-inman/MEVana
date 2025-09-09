# MEVana Port Mapping - All Services

## Infrastructure Services (Docker)
| Service | Internal Port | External Port | Purpose |
|---------|--------------|---------------|---------|
| Zookeeper | 2181 | 2181 | Kafka coordination |
| Kafka | 9092/29092 | 9092/29092 | Message broker |
| **Kafka-UI** | 8080 | **8080** | Kafka management |
| Redis | 6379 | 6379 | Caching |
| Redis-UI | 8081 | 8083 | Redis management |
| PostgreSQL | 5432 | 5433 | Database |
| Adminer | 8080 | 8082 | Database management |
| Prometheus | 9090 | 9090 | Metrics collection |
| Grafana | 3000 | 3000 | Metrics visualization |

## Trading Services (Native)
| Service | Port | Purpose |
|---------|------|---------|
| **Arbitrage Engine** | **8085** | API and metrics |
| Whirlpool Stream | - | Kafka producer only |
| Executor Service | 8084 | Health checks |

## Wallet, Auth & UI Services (Planned / Emerging)
| Service | Proposed Host Port | Reasoning / Notes |
|---------|--------------------|-------------------|
| wallet-vault | 8090 | Matches existing docs (Secure Wallet API); avoids 808x conflicts already used (8080/8082/8083/8084/8085/8088) |
| api-service (public API) | 8091 | Sequential to wallet-vault; keeps related surface area contiguous |
| auth-gateway (Clerk/Auth layer) | 3001 | 3000 occupied by Grafana; 3001 conventional for Node/Next auth edge |
| Web UI (Next.js) | 3200 (proposed) | Avoids 3000 (Grafana) & 3001 (auth); leaves headroom for other dashboards; can switch to 5173 if Vite |
| hotpath-arb-engine (metrics) | 8088 | Already used in configs (keep) |
| normalizer-service (metrics) | 9901 | Internal metrics/health (do not expose externally) |
| edge-assembler (metrics) | 9902 | Internal metrics |
| arb-engine (metrics) | 9903 | Internal metrics |
| executor-service (metrics) | 9904 | Internal metrics |
| system-orchestrator (metrics) | 9999 | Default orchestrator port; keep reserved |

## Reserved / Port Allocation Strategy
| Range | Allocation | Notes |
|-------|-----------|-------|
| 2181 | Zookeeper | Infrastructure |
| 29092 | Kafka external | Client access (internal 9092) |
| 3000 | Grafana | Metrics dashboard |
| 3001 | Auth Gateway | JWT / session boundary |
| 3100-3199 | Future dashboards | Reserved (avoid clashes) |
| 3200 | Primary Web UI | Proposed selection (changeable) |
| 4200 | (Optional alt) Frontend dev | If Angular/Vite alt needed |
| 5173 | (Optional alt) Vite dev | Use if switching toolchain |
| 5433 | Postgres | Host-mapped (container 5432) |
| 6379 | Redis | Cache / session |
| 8080 | Kafka-UI | Existing |
| 8082 | Adminer | DB management |
| 8083 | Redis-UI | Existing |
| 8084 | dex-stream-universal | Streaming health / metrics |
| 8085 | Arbitrage Engine | API/metrics (doc) |
| 8088 | Hotpath Arb Engine | Performance metrics |
| 8090 | wallet-vault | Secure wallet API boundary |
| 8091 | api-service | Public API aggregation |
| 8092-8099 | Future API services | Keep contiguous block |
| 9000-9099 | (Available) | Leave gap for future high-load services |
| 9090 | Prometheus | Metrics collector |
| 9500-9599 | (Future) Internal tools | Reserve for ops/debug |
| 9800-9899 | (Future) Experimental | Ephemeral services |
| 9900-9999 | Internal Rust metrics | Avoid external exposure |

## Action Items
1. Update orchestrator-config.toml to align Postgres URL (5433) and add wallet/api/auth services when introduced.
2. Expose only externally required ports (close metrics ports in production with firewall rules).
3. Maintain this file as the source of truth before allocating any new port.
4. Consider adding a CI check to prevent duplicate port assignments.

## Verification Checklist (Quick)
- [ ] No overlapping host ports in use
- [ ] Metrics ports not colliding with UI/API ports
- [ ] Postgres mapped port matches application configuration
- [ ] Web UI chosen port (3200) not already bound (validate before launch)

## Operational Notes
- Postgres logical backup script: `scripts/backup_postgres.sh` (creates `backups/pg-backup-<db>-<timestamp>.sql.gz`). Run before destructive operations.
- Web UI reserved host port: 3200 (update here first if changed).
- For new services allocate from contiguous blocks (see Reserved / Port Allocation Strategy) and update this file in the same commit.

## Port Conflict Resolution
- **Kafka-UI uses port 8080** (Docker internal)
- **Arbitrage Engine moved to port 8085** (native service)
- This prevents conflicts between Docker and native services

## Access URLs
- Kafka UI: http://localhost:8080
- Arbitrage API: http://localhost:8085/ai/config
- Grafana: http://localhost:3000 (admin/admin123)
- Redis UI: http://localhost:8083 (admin/admin123)
- Prometheus: http://localhost:9090
- Adminer: http://localhost:8082

## Network Configuration
- Docker services use `mevana_net` network
- Native services connect to Docker services via localhost ports
- No port conflicts between infrastructure and trading services