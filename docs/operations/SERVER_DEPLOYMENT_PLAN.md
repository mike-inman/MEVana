# MEVana Server Deployment Plan

**Target**: Production server deployment
**Source**: Verified working local configuration
**Date**: 2025-09-19

## Pre-Deployment Requirements

### Server Prerequisites
```bash
# Required software
- Docker Engine 24.0+
- Docker Compose v2.20+
- Git
- SSH access
- Minimum 8GB RAM, 100GB disk space

# Network requirements
- Outbound HTTPS (443) for Docker Hub
- Outbound access to Solana RPC endpoints
- Internal network for container communication
```

### Environment Setup
```bash
# 1. Clone repository
git clone <repo-url> /opt/MEVana
cd /opt/MEVana

# 2. Create external Docker network
docker network create mevana_net

# 3. Verify prerequisites
./scripts/doctor.sh
```

## Critical Configuration Changes for Server

### 1. Host Network Binding Updates
**Current Issue**: `host.docker.internal` doesn't exist on Linux servers

**File**: `infrastructure/prometheus.yml`
```yaml
# CHANGE FROM:
- job_name: 'hotpath-arb-engine'
  static_configs:
    - targets: ['host.docker.internal:8099']

# CHANGE TO:
- job_name: 'hotpath-arb-engine'
  static_configs:
    - targets: ['hotpath-arb-engine:8088']
```

### 2. Service Environment Variables
**Critical**: Update all services to use server-appropriate values

```bash
# Environment file: .env.production
GEYSER_ENDPOINT=https://grpc.ny.shyft.to
X_TOKEN=efff2d23-97ae-4c5f-8b8b-9a6265001189
KAFKA_BROKER=kafka:9092
DATABASE_URL=postgresql://mev:mevpassword@postgres:5432/mevdb
REDIS_URL=redis://redis:6379/0
```

### 3. Port Exposure Configuration
**File**: `infrastructure/docker-compose.yml`

```yaml
# Update port bindings for server access
grafana:
  ports:
    - "3000:3000"    # Change from 3300 to standard 3000

prometheus:
  ports:
    - "9090:9090"    # Change from 9091 to standard 9090
```

### 4. Security Hardening
```yaml
# Grafana security updates
environment:
  - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD:-secure_password_here}
  - GF_USERS_ALLOW_SIGN_UP=false
  - GF_SECURITY_COOKIE_SECURE=true
  - GF_SECURITY_COOKIE_SAMESITE=strict
```

## Deployment Sequence

### Phase 1: Infrastructure Deployment
```bash
# 1. Start core infrastructure
cd /opt/MEVana
docker compose -f infrastructure/docker-compose.yml up -d \
  zookeeper kafka postgres redis prometheus grafana

# 2. Wait for health checks
docker compose ps
# Verify all services show "healthy"

# 3. Check logs for errors
docker compose logs grafana
docker compose logs prometheus
```

### Phase 2: MEVana Services Deployment
```bash
# 1. Build MEVana service images
cargo build --release

# 2. Start MEVana services
docker compose -f infrastructure/docker-compose.yml up -d \
  normalizer-service discovery-service executor-service

# 3. Start primary arbitrage engine
docker compose -f infrastructure/docker-compose.yml up -d \
  hotpath-arb-engine

# 4. Verify metrics collection
curl http://localhost:9090/targets
# Check all targets are UP
```

### Phase 3: Monitoring Verification
```bash
# 1. Access Grafana
http://server-ip:3000
# Login: admin / [your-secure-password]

# 2. Verify dashboard
# Navigate to "MEVana System Status"
# Confirm all 4 panels showing data

# 3. Check Prometheus targets
http://server-ip:9090/targets
# All should be UP and green
```

## Critical File Mapping

### Configuration Files (Must Copy Exactly)
```bash
# Source → Server destination
infrastructure/prometheus.yml → /opt/MEVana/infrastructure/prometheus.yml
infrastructure/grafana/datasources/prometheus.yml → /opt/MEVana/infrastructure/grafana/datasources/prometheus.yml
infrastructure/grafana/dashboards/curated/dashboard.yml → /opt/MEVana/infrastructure/grafana/dashboards/curated/dashboard.yml
infrastructure/grafana/dashboards/curated/mevana-system-status.json → /opt/MEVana/infrastructure/grafana/dashboards/curated/mevana-system-status.json
```

### Docker Compose Files
```bash
# Primary deployment file
infrastructure/docker-compose.yml

# Additional configurations
infrastructure/docker-compose.core.yml
infrastructure/docker-compose.server.yml
```

## Data Persistence Strategy

### Volume Creation
```bash
# Create named volumes
docker volume create mevana_postgres_data
docker volume create mevana_prometheus_data
docker volume create mevana_grafana_data
```

### Backup Strategy
```bash
# Database backup
docker exec infrastructure-postgres-1 pg_dump -U mev mevdb > backup_$(date +%Y%m%d).sql

# Grafana backup
docker exec infrastructure-grafana-1 tar -czf - /var/lib/grafana > grafana_backup_$(date +%Y%m%d).tar.gz

# Prometheus backup
docker exec infrastructure-prometheus-1 tar -czf - /prometheus > prometheus_backup_$(date +%Y%m%d).tar.gz
```

## Health Check Commands

### Service Health Verification
```bash
# 1. Docker container health
docker ps --format "table {{.Names}}\t{{.Status}}"

# 2. MEVana metrics endpoints
curl -s http://localhost:8088/metrics | head -5    # Hotpath engine
curl -s http://localhost:9901/metrics | head -5    # Normalizer
curl -s http://localhost:9102/metrics | head -5    # Discovery

# 3. Prometheus targets
curl -s http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | {job: .labels.job, health: .health}'

# 4. Grafana dashboard
curl -s http://localhost:3000/api/health
```

### Live Data Verification
```bash
# Check live transaction processing
curl -s http://localhost:8088/metrics | grep hotpath_transactions_processed_total

# Check arbitrage detection
curl -s http://localhost:8088/metrics | grep hotpath_multi_venue_transactions

# Check Geyser streams
curl -s http://localhost:8088/metrics | grep geyser_transactions_processed_total
```

## Rollback Plan

### If Deployment Fails
```bash
# 1. Stop all services
docker compose -f infrastructure/docker-compose.yml down

# 2. Clean up volumes (if needed)
docker volume rm mevana_postgres_data mevana_prometheus_data mevana_grafana_data

# 3. Reset network
docker network rm mevana_net
docker network create mevana_net

# 4. Restart from Phase 1
```

### If Monitoring Breaks
```bash
# 1. Check Prometheus config
docker exec infrastructure-prometheus-1 promtool check config /etc/prometheus/prometheus.yml

# 2. Restart monitoring stack
docker compose restart prometheus grafana

# 3. Re-verify targets
curl http://localhost:9090/targets
```

## Success Criteria

### Deployment Complete When:
- ✅ All containers show "healthy" status
- ✅ Prometheus shows all targets UP (green)
- ✅ Grafana dashboard displays live data in all 4 panels
- ✅ Transaction processing rates > 0
- ✅ Arbitrage detection metrics updating
- ✅ No error logs in container outputs

### Live Data Flowing When:
- ✅ `hotpath_transactions_processed_total` incrementing
- ✅ `geyser_transactions_processed_total` incrementing
- ✅ `hotpath_multi_venue_transactions` incrementing
- ✅ Dashboard graphs show real-time activity
- ✅ Arbitrage detection panel shows orange line with data

## Emergency Contacts & Documentation

- **Configuration Source**: `/Users/michaelinman/Repos/MEVana/docs/operations/CURRENT_DEPLOYMENT_CONFIGURATION.md`
- **Dashboard JSON**: Working copy in `infrastructure/grafana/dashboards/curated/mevana-system-status.json`
- **Prometheus Config**: Verified working in `infrastructure/prometheus.yml`
- **Doctor Script**: `./scripts/doctor.sh` for environment verification