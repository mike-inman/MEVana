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