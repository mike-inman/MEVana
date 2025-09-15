# MEVana Deployment Architecture

## Infrastructure Dependencies (Order Matters)

### 1. Foundation Prerequisites
- **Rust Toolchain** (cargo, rustc)
- **Docker Engine** + Docker Compose
- **Network Connectivity** to external services
- **System Resources** (RAM, CPU, Storage)

### 2. Infrastructure Services (Docker Containers)
```
PostgreSQL (5433) → Redis (6379) → Kafka (9092) → Prometheus (9090) → Grafana (3000)
     ↓                   ↓              ↓              ↓              ↓
   Database           Cache        Message Bus     Metrics       Dashboard
```

### 3. Application Services (Native Processes)
```
MEVana HotPath Engine (8088) → Monitoring Service (8090) → API Service (8080)
           ↓                         ↓                         ↓
    Core Trading Engine         System Monitor           Web Interface
```

## Port Mapping & Network Plan

### Container Ports (Docker Internal)
| Service    | Internal | External | Purpose               |
|------------|----------|----------|-----------------------|
| PostgreSQL | 5432     | 5433     | Database              |
| Redis      | 6379     | 6379     | Cache                 |
| Kafka      | 9092     | 9092     | Message Queue         |
| Zookeeper  | 2181     | 2181     | Kafka Coordination    |
| Prometheus | 9090     | 9090     | Metrics Collection    |
| Grafana    | 3000     | 3000     | Dashboard             |
| Adminer    | 8080     | 8082     | DB Admin              |
| Redis UI   | 8081     | 8083     | Cache Admin           |
| Kafka UI   | 8080     | 8080     | Queue Admin           |

### Application Ports (Host)
| Service           | Port | Purpose               | Data Verification      |
|-------------------|------|-----------------------|------------------------|
| HotPath Engine    | 8088 | Core Trading + Metrics | /metrics endpoint     |
| Monitoring        | 8090 | System Health         | /health endpoint      |
| API Service       | 8080 | Web Interface         | /api/status endpoint  |
| Wallet Vault      | 8090 | Wallet Management     | /vault/status         |

## Environment Variables by Service

### Core Application
```bash
DATABASE_URL=postgresql://mev:mevpassword@localhost:5433/mevdb
GEYSER_ENDPOINT=https://grpc.ny.shyft.to
X_TOKEN=efff2d23-97ae-4c5f-8b8b-9a6265001189
KAFKA_BROKER=localhost:29092
METRICS_PORT=8088
RUST_LOG=info
PAPER_TRADING=true
```

### Infrastructure
```bash
POSTGRES_USER=mev
POSTGRES_PASSWORD=mevpassword
POSTGRES_DB=mevdb
REDIS_URL=redis://localhost:6379
PROMETHEUS_CONFIG=prometheus.prod.yml
```

## Dependency Chain Verification

### 1. Infrastructure Health Checks
```bash
# PostgreSQL
psql $DATABASE_URL -c "SELECT 1;"

# Redis
redis-cli ping

# Kafka
kafka-console-consumer --bootstrap-server localhost:29092 --topic __consumer_offsets --timeout-ms 5000

# Prometheus
curl -s http://localhost:9090/-/healthy

# Grafana
curl -s http://localhost:3000/api/health
```

### 2. Application Health Checks
```bash
# HotPath Engine - Check metrics AND data
curl -s http://localhost:8088/metrics | grep -E "(up|arbitrage_opportunities|geyser_messages)"

# Monitoring Service
curl -s http://localhost:8090/health

# Verify Data Flow
curl -s http://localhost:8088/metrics | grep "arbitrage_opportunities_total" | grep -v "0"
```

### 3. Network Connectivity Tests
```bash
# External Geyser Connection
curl -s --connect-timeout 5 https://grpc.ny.shyft.to/health || echo "Geyser unreachable"

# Container-to-Host Communication
docker exec prometheus wget -qO- http://host.docker.internal:8088/metrics | head -3

# Cross-Service Communication
docker exec grafana curl -s http://prometheus:9090/api/v1/query?query=up
```

## Data Verification Standards

### "Working" Definition:
1. **Process Running** ✓
2. **Port Accessible** ✓
3. **Data Flowing** ✓
4. **No Conflicts** ✓
5. **Expected Metrics Present** ✓

### Example: HotPath Engine Verification
```bash
# 1. Process check
pgrep -f hotpath_arb_engine >/dev/null || exit 1

# 2. Port check
curl -s --connect-timeout 5 http://localhost:8088/metrics >/dev/null || exit 1

# 3. Data check
METRICS=$(curl -s http://localhost:8088/metrics)
echo "$METRICS" | grep -q "arbitrage_opportunities_detected_total" || exit 1
echo "$METRICS" | grep -q "multi_geyser_stream_healthy" || exit 1

# 4. Conflict check
lsof -i :8088 | grep -v hotpath && echo "Port conflict detected" && exit 1

# 5. Data freshness check
LAST_MESSAGE=$(echo "$METRICS" | grep "geyser_last_message_timestamp" | awk '{print $2}')
CURRENT_TIME=$(date +%s)
TIME_DIFF=$((CURRENT_TIME - LAST_MESSAGE))
[ $TIME_DIFF -lt 60 ] || echo "Stale data detected"
```