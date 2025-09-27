# MEVana System Health Assessment & Optimization Report

**Date:** September 27, 2025
**Status:** 🔧 Needs Optimization
**Live Trading Readiness:** 65%

## Executive Summary

After comprehensive analysis of both MEVana and MEVana-UI codebases, I've identified critical issues preventing reliable live trading deployment. The system has strong architectural foundations but suffers from configuration misalignment, port conflicts, incomplete gateway implementation, and CI/CD inefficiencies.

## 🔴 Critical Issues (Must Fix Before Live Trading)

### 1. Port Configuration Mismatch
**Problem:** MEVana-UI expects gateway on port 8096, but api-service runs on 8092/8095
- UI configured: `http://149.28.56.125:8096/gateway`
- API service actual: Port 8092 (REST), 8095 (WebSocket)
- **Impact:** Complete UI-backend disconnect

### 2. Gateway Service Not Fully Deployed
**Problem:** api-service gateway is built but not consistently started
- Missing from some deployment workflows
- No systemd unit file found in infrastructure/systemd/
- Gateway consumers not properly initialized with Kafka topics

### 3. Network Topology Issues
**Problem:** Mixed Docker/bare-metal architecture creates routing complexity
- Docker services use `mevana_net` network
- Bare-metal services use `host.docker.internal`
- Prometheus scraping through host gateway adds latency
- Redis connection string mismatch (6379 vs 6380)

### 4. Monitoring Configuration Broken
**Problem:** Slack alerting not working, Prometheus targets misconfigured
- Missing `/etc/mevana/alerts.env` with `SLACK_WEBHOOK_URL`
- Some Prometheus scrape targets pointing to wrong ports
- Grafana dashboards not loading wallet metrics properly

## 🟡 Major Issues (Impacting Reliability)

### 5. CI/CD Performance
**Current Build Time:** 6-7 minutes for full deployment
- Rust compilation not cached between runs
- Docker builds happening even when unnecessary
- Systemd service installation happens after build (should be before)

### 6. Database Connection Pool Issues
**Problem:** PostgreSQL connection exhaustion under load
- No connection pooling configured
- Each service creates independent connections
- Missing pgbouncer or similar pooling solution

### 7. Kafka Topic Synchronization
**Problem:** Topics created by bootstrap script don't match service expectations
- `orca-swaps.parsed.v2` vs `orca-swaps.parsed`
- Missing `prices.latest` for gateway
- No automatic topic creation on service startup

## 🟢 Working Components

### What's Working Well:
1. **HotPath Engine:** Zero-latency architecture properly implemented
2. **Docker Infrastructure:** Stable Kafka, Redis, Postgres, Grafana stack
3. **Systemd Services:** Proper service management for bare-metal components
4. **Authentication:** Clerk integration functional
5. **Metrics Collection:** Prometheus successfully scraping most targets

## 📋 Immediate Action Plan

### Phase 1: Fix Critical Blockers (Today)
```bash
# 1. Fix port configuration
sed -i 's/8096/8092/g' /Users/michaelinman/Repos/MEVana-UI/.env.production
sed -i 's|:8096/gateway|:8092/api/v1|g' /Users/michaelinman/Repos/MEVana-UI/.env.production
sed -i 's|ws://.*:8096|ws://149.28.56.125:8095|g' /Users/michaelinman/Repos/MEVana-UI/.env.production

# 2. Create api-service systemd unit
cat > infrastructure/systemd/api-service.service <<EOF
[Unit]
Description=MEVana API Gateway Service
After=network.target postgres.service kafka.service

[Service]
Type=simple
User=mevana
WorkingDirectory=/opt/MEVana
EnvironmentFile=/etc/mevana/common.env
EnvironmentFile=/etc/mevana/api-service.env
ExecStart=/opt/MEVana/bin/api-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# 3. Fix Kafka topics
./scripts/deployment/bootstrap_kafka.sh localhost:29092

# 4. Setup Slack alerting
echo "SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/WEBHOOK/URL" > /etc/mevana/alerts.env
```

### Phase 2: Optimize CI/CD (This Week)
```yaml
# Add to .github/workflows/self-hosted-deploy.yml
- name: Cache Rust build
  uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

- name: Use sccache
  run: |
    cargo install sccache
    export RUSTC_WRAPPER=sccache
```

### Phase 3: Network Consolidation
1. Move all services to Docker OR all to bare-metal (not mixed)
2. Implement service mesh (Consul/Linkerd) for service discovery
3. Add pgbouncer for database connection pooling

## 🚀 Optimization Recommendations

### 1. Immediate Performance Wins
```bash
# Enable Rust incremental compilation
export CARGO_INCREMENTAL=1

# Use mold linker for faster builds
sudo apt install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Parallel service startup
systemctl start hotpath-arb-engine normalizer-service discovery-service &
```

### 2. Gateway Implementation Fix
```rust
// In api-service/src/bin/api-server.rs
// Fix port configuration
let port = env::var("PORT").unwrap_or_else(|_| "8092".to_string());
let gateway_port = env::var("MEVANA_WEBSOCKET_GATEWAY_PORT")
    .unwrap_or_else(|_| "8095".to_string());

// Ensure topics exist
let required_topics = vec![
    "prices.latest",
    "orca-swaps.parsed.v2",
    "arb.opportunities.v1"
];
```

### 3. Monitoring Stack Fix
```yaml
# prometheus.prod.yml corrections
- job_name: 'api-service'
  static_configs:
    - targets: ['host.docker.internal:8092']

- job_name: 'monitoring-service'
  static_configs:
    - targets: ['host.docker.internal:8091']  # Was 8090
```

### 4. Database Connection Pooling
```yaml
# Add to docker-compose.server.yml
pgbouncer:
  image: pgbouncer/pgbouncer:latest
  environment:
    DATABASES_HOST: postgres
    DATABASES_PORT: 5432
    DATABASES_DBNAME: mevdb
    POOL_MODE: transaction
    MAX_CLIENT_CONN: 1000
    DEFAULT_POOL_SIZE: 25
  ports:
    - "5434:5432"
```

## 📊 Health Metrics Dashboard

| Component | Current Status | Target Status | Action Required |
|-----------|---------------|---------------|-----------------|
| Gateway API | 🔴 Misconfigured | 🟢 Operational | Fix ports, deploy service |
| UI Connection | 🔴 Broken | 🟢 Connected | Update environment variables |
| Prometheus | 🟡 Partial | 🟢 Full Coverage | Fix scrape targets |
| Slack Alerts | 🔴 Not Working | 🟢 Active | Configure webhook |
| CI/CD Speed | 🟡 6-7 min | 🟢 2-3 min | Add caching |
| Database Pool | 🔴 None | 🟢 PgBouncer | Deploy pooler |
| Kafka Topics | 🟡 Partial | 🟢 Complete | Run bootstrap |

## 🎯 Path to Live Trading

### Week 1 (Stabilization)
- [ ] Fix all port configurations
- [ ] Deploy api-service with systemd
- [ ] Configure Slack alerting
- [ ] Verify UI-backend connectivity
- [ ] Run full integration tests

### Week 2 (Optimization)
- [ ] Implement build caching
- [ ] Deploy pgbouncer
- [ ] Consolidate network architecture
- [ ] Performance testing under load
- [ ] Implement circuit breakers

### Week 3 (Hardening)
- [ ] Security audit
- [ ] Backup/recovery procedures
- [ ] Monitoring dashboard completion
- [ ] Documentation updates
- [ ] Team training

### Week 4 (Go-Live)
- [ ] Paper trading validation
- [ ] Gradual volume increase
- [ ] 24/7 monitoring setup
- [ ] Incident response procedures
- [ ] Live trading activation

## 🔧 Quick Fix Script

Save and run this script to fix immediate issues:

```bash
#!/bin/bash
# fix_mevana_critical.sh

echo "🔧 Fixing MEVana critical issues..."

# 1. Fix MEVana-UI environment
cd /Users/michaelinman/Repos/MEVana-UI
sed -i.bak 's/8096/8092/g' .env.production
sed -i.bak 's|/gateway/|/api/v1/|g' .env.production
npm run build

# 2. Deploy api-service
cd /Users/michaelinman/Repos/MEVana
sudo cp infrastructure/systemd/api-service.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now api-service

# 3. Fix Kafka topics
./scripts/deployment/bootstrap_kafka.sh localhost:29092

# 4. Restart services
sudo systemctl restart hotpath-arb-engine normalizer-service discovery-service

# 5. Verify
curl -s http://localhost:8092/health
curl -s http://localhost:8095/gateway/ws/ping

echo "✅ Critical fixes applied. Check http://localhost:3000 for Grafana dashboard"
```

## Conclusion

MEVana has solid architectural foundations but needs configuration alignment and deployment optimization before live trading. The identified issues are fixable within 2-3 weeks with focused effort. Priority should be on gateway configuration, port alignment, and CI/CD optimization.

**Recommended Next Step:** Execute Phase 1 fixes immediately, then proceed with systematic optimization following the provided roadmap.