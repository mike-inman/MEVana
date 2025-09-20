# MEVana Production Deployment Strategy & CI/CD Analysis

**Date**: 2025-09-19
**Goal**: Bulletproof production deployment with dev-to-prod mirroring
**Status**: Infrastructure exists, needs configuration alignment

## 🔍 Current Infrastructure Analysis

### Existing GitHub Actions (Self-Hosted Runner)
```yaml
# Located: .github/workflows/
✅ self-hosted-deploy.yml          # Main deployment workflow
✅ self-hosted-auto-heal.yml       # Auto-healing and restart
✅ self-hosted-diagnostics.yml     # Comprehensive diagnostics
✅ self-hosted-restart.yml         # Service restart
✅ self-hosted-cargo-check.yml     # Build verification
✅ hotpath-cargo-check.yml         # Hotpath-specific checks
```

### Deployment Scripts Available
```bash
# Build & Deploy
scripts/deployment/deploy-server.sh         # Full server setup
scripts/deployment/direct_deploy.sh         # Quick deployment
scripts/deployment/build-and-ship.sh        # Build + ship
scripts/deployment/deploy-vultr.sh          # Cloud-specific

# Lifecycle Management
scripts/lifecycle/start_system.sh           # System startup
scripts/lifecycle/shutdown_system.sh        # Clean shutdown
scripts/lifecycle/start_clean.sh            # Fresh start

# Monitoring & Health
scripts/ci/post-deployment-diagnostics.sh   # Post-deploy checks
scripts/ci/verify-prometheus-targets.sh     # Metrics verification
scripts/monitoring/quick_health_check.sh    # Health verification
```

### Existing Server Infrastructure
```yaml
# On server at /opt/MEVana
✅ Docker infrastructure running
✅ Self-hosted GitHub Actions runner
✅ Prometheus/Grafana monitoring
✅ Kafka/Redis/PostgreSQL data layer
✅ MEVana services (may need config updates)
```

## 🎯 Critical Gap Analysis

### 1. **Configuration Misalignment**
**Problem**: Local working config ≠ Server config
```bash
# Local (WORKING)
./infrastructure/grafana/dashboards/curated/  # Fixed volume paths
mevana-system-status.json                     # Working dashboard

# Server (UNKNOWN STATE)
/opt/MEVana/infrastructure/                    # May have old broken config
```

### 2. **Monitoring Setup**
**Problem**: Server may not have the working dashboard
```bash
# Local verified metrics:
- hotpath_multi_venue_transactions  ✅ Working
- geyser_transactions_processed_total ✅ Working
- hotpath_transactions_processed_total ✅ Working

# Server status: NEEDS VERIFICATION
```

### 3. **Environment Configuration**
**Problem**: Server environment variables may be different
```bash
# Local (working):
KAFKA_BROKER=localhost:29092
DATABASE_URL=postgresql://mev:mevpassword@localhost:5433/mevdb

# Server (may differ):
# Container networking vs host networking differences
```

## 🚀 Bulletproof Deployment Strategy

### Phase 1: Pre-Deployment Discovery
```bash
# 1. Server state assessment
./scripts/doctor.sh                          # Environment verification
./scripts/ci/post-deployment-diagnostics.sh  # Current state analysis
docker ps                                    # Running services
curl localhost:8088/metrics | head -10       # Metrics availability

# 2. Configuration audit
diff local-config server-config              # Compare configurations
./scripts/ci/verify-prometheus-targets.sh    # Check monitoring setup
```

### Phase 2: Configuration Sync
```bash
# 1. Sync working local config to server
rsync -av infrastructure/ server:/opt/MEVana/infrastructure/

# 2. Apply working dashboard
cp infrastructure/grafana/dashboards/curated/mevana-system-status.json \
   server:/opt/MEVana/infrastructure/grafana/dashboards/curated/

# 3. Fix volume paths in docker-compose
# Ensure ./infrastructure/grafana/ paths are correct
```

### Phase 3: Staged Deployment
```bash
# 1. Infrastructure-only restart
docker-compose restart prometheus grafana

# 2. Verify monitoring before app deployment
curl http://localhost:3000/api/health        # Grafana health
curl http://localhost:9090/-/healthy         # Prometheus health

# 3. Deploy application services
# Using existing GitHub Actions workflow
```

### Phase 4: Production Validation
```bash
# 1. Verify live data flow
curl http://localhost:8088/metrics | grep hotpath_multi_venue_transactions

# 2. Dashboard functionality check
# Access Grafana → MEVana System Status → All 4 panels showing data

# 3. End-to-end monitoring verification
./scripts/ci/run-all-diagnostics.sh
```

## 🔄 Dev-to-Prod Mirroring Plan

### 1. **Exact Configuration Mirroring**
```yaml
# Local Development
infrastructure/docker-compose.yml:
  grafana:
    ports: ["3300:3000"]                     # Local dev port
    volumes:
      - ./infrastructure/grafana/...         # WORKING paths

# Production Server
infrastructure/docker-compose.server.yml:
  grafana:
    ports: ["3000:3000"]                     # Standard port
    volumes:
      - ./infrastructure/grafana/...         # SAME paths as local
```

### 2. **Monitoring Parity**
```bash
# Ensure identical metrics collection
Local:  prometheus.yml (11 scrape targets)
Server: prometheus.yml (SAME 11 targets)

Local:  mevana-system-status.json (4 panels)
Server: mevana-system-status.json (SAME 4 panels)
```

### 3. **Environment Variable Standardization**
```bash
# Create .env.template for both environments
GEYSER_ENDPOINT=https://grpc.ny.shyft.to
X_TOKEN=efff2d23-97ae-4c5f-8b8b-9a6265001189
KAFKA_BROKER=kafka:9092                     # Docker internal name
DATABASE_URL=postgresql://mev:mevpassword@postgres:5432/mevdb
```

## 🤖 Automated CI/CD Workflow

### Trigger: Manual Deployment
```yaml
# .github/workflows/production-deploy.yml
on: workflow_dispatch

jobs:
  pre-deploy-audit:
    - name: Check server state
    - name: Verify configuration differences
    - name: Health check current services

  deploy:
    needs: pre-deploy-audit
    - name: Sync local working config
    - name: Restart monitoring stack
    - name: Deploy application services
    - name: Verify metrics collection

  post-deploy-validation:
    needs: deploy
    - name: Dashboard functionality test
    - name: Live data flow verification
    - name: End-to-end health check
```

### Existing Workflows Enhancement
```bash
# Enhance self-hosted-deploy.yml:
1. Add pre-deployment config sync
2. Add dashboard verification step
3. Add metrics validation

# Enhance self-hosted-auto-heal.yml:
1. Add dashboard health check
2. Add metrics target verification
```

## 🛡️ Rollback Strategy

### Immediate Rollback Triggers
```bash
# Auto-rollback if:
- Grafana dashboard not loading
- Prometheus targets failing
- No live arbitrage metrics
- MEVana services not responding
```

### Rollback Execution
```bash
# 1. Stop all services
docker-compose down

# 2. Restore last known good config
git checkout HEAD~1 infrastructure/

# 3. Restart with previous configuration
docker-compose up -d

# 4. Verify rollback success
./scripts/ci/verify-prometheus-targets.sh
```

## 📋 Pre-Deployment Checklist

### Configuration Verification
- [ ] Local dashboard working (4 panels, live data)
- [ ] Volume paths corrected (./infrastructure/grafana/)
- [ ] Prometheus targets all UP
- [ ] Environment variables documented
- [ ] Server state documented

### Deployment Readiness
- [ ] Docker network: mevana_net exists
- [ ] GitHub secrets configured
- [ ] Self-hosted runner healthy
- [ ] Backup of current server config
- [ ] Rollback procedure tested

### Post-Deployment Validation
- [ ] Grafana accessible at server:3000
- [ ] MEVana System Status dashboard loads
- [ ] All 4 panels showing live data
- [ ] Arbitrage detection metrics flowing
- [ ] Prometheus targets all green

## 🎯 Immediate Next Steps

### 1. **Configuration Discovery** (Priority 1)
```bash
# Run on server to understand current state
ssh server "cd /opt/MEVana && ./scripts/doctor.sh"
ssh server "cd /opt/MEVana && docker ps"
ssh server "cd /opt/MEVana && curl localhost:8088/metrics | head -10"
```

### 2. **Config Sync** (Priority 2)
```bash
# Sync working local config to server
scp -r infrastructure/ server:/opt/MEVana/
scp scripts/doctor.sh server:/opt/MEVana/scripts/
```

### 3. **Validation** (Priority 3)
```bash
# Use existing GitHub Actions to deploy with new config
# Trigger: self-hosted-deploy workflow
# Monitor: self-hosted-diagnostics workflow
```

## 🔍 Risk Assessment

### Low Risk
- ✅ Infrastructure already exists
- ✅ Self-hosted runner working
- ✅ Monitoring stack present

### Medium Risk
- ⚠️ Configuration differences
- ⚠️ Volume path mismatches
- ⚠️ Environment variable differences

### High Risk
- 🚨 Dashboard functionality unknown
- 🚨 Live metrics collection unknown
- 🚨 Data persistence at risk

**Mitigation**: Thorough pre-deployment discovery + staged rollout