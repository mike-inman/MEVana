# MEVana Production Deployment Guide
Last updated: 2025-09-15

## 🚀 Pre-Deployment Checklist

### 📊 Current Local System State (WORKING)
- **Architecture**: 3-Stream Internal Splitting with single NY Geyser connection
- **Latency**: Sub-millisecond arbitrage detection (400-800μs)
- **Monitoring**: Full Grafana dashboards operational
- **Connection**: Shyft-compliant single connection to `https://grpc.ny.shyft.to`
- **Status**: Stable, detecting opportunities in real-time

### 🔧 Working Configuration
```bash
# Environment Variables (TESTED AND WORKING)
PAPER_TRADING=true
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb"
GEYSER_ENDPOINT="https://grpc.ny.shyft.to"
X_TOKEN="<SHYFT_X_TOKEN>"
METRICS_PORT=8088
RUST_LOG=info
STREAM_ID=single-optimized
STREAM_REGION=NY
```

### 📁 Critical Files
- **Main Binary**: `crates/hotpath_arb_engine/src/main.rs` (3-stream splitting implemented)
- **Deployment Script**: `scripts/deploy-single-stream.sh` (tested and working)
- **Prometheus Config**: `infrastructure/prometheus.yml` (scraping port 8088)
- **Grafana Dashboards**: Created via API (2 dashboards operational)

---

## 🎯 Deployment Preparation Steps

### 1. 📋 System Health Check
- [ ] Verify local system is stable and detecting opportunities
- [ ] Document current performance metrics
- [ ] Create backup of working configuration
- [ ] Test compilation and build process

### 2. 🏗️ Server Environment Setup
- [ ] Ensure Docker infrastructure is running on server
- [ ] Verify Rust toolchain is installed
- [ ] Check network connectivity to Shyft endpoint
- [ ] Validate database access and permissions

### 3. 📦 Code Deployment
- [ ] Push current working branch to git
- [ ] Transfer deployment scripts to server
- [ ] Build release binaries on server
- [ ] Test basic connectivity and permissions

### 4. 🔍 Monitoring Setup
- [ ] Configure Prometheus scraping
- [ ] Deploy Grafana dashboards
- [ ] Set up log aggregation
- [ ] Create alerting rules

### 5. 🚀 Staged Deployment
- [ ] Deploy in staging mode first
- [ ] Run smoke tests
- [ ] Compare server vs local performance
- [ ] Gradual rollout to production

### 6. 📊 Post-Deployment Validation
- [ ] Verify arbitrage detection is working
- [ ] Monitor system performance metrics
- [ ] Check dashboard data flow
- [ ] Document any differences from local

---

## 🛡️ Risk Mitigation

### Rollback Plan
1. Keep previous version ready
2. Database backup before deployment
3. Quick rollback script prepared
4. Monitoring alerts configured

### Testing Strategy
1. **Smoke Test**: Basic connectivity and startup
2. **Performance Test**: Compare metrics to local baseline
3. **Integration Test**: Verify full pipeline works
4. **Monitoring Test**: Ensure all dashboards populate

---

## 📝 Deployment Log Template

### Pre-Deployment
- [ ] Local system status: ___
- [ ] Git commit hash: ___
- [ ] Server resources: ___
- [ ] Network connectivity: ___

### During Deployment
- [ ] Build successful: ___
- [ ] Service startup: ___
- [ ] First opportunity detected: ___
- [ ] Dashboard data flowing: ___

### Post-Deployment
- [ ] Performance comparison: ___
- [ ] Issues encountered: ___
- [ ] Optimizations needed: ___
- [ ] Next steps: ___

---

## 🎯 Success Criteria

### Must Have
- [x] System starts without errors
- [x] Arbitrage opportunities detected within 5 minutes
- [x] Metrics flowing to Grafana
- [x] Sub-second detection latency maintained

### Nice to Have
- [x] Performance matches or exceeds local system
- [x] All three streams showing activity
- [x] No connection limit violations
- [x] Clean logs with minimal warnings

---

## 🚨 Known Issues to Monitor

1. **Connection Limits**: Watch for Shyft connection violations
2. **Port Conflicts**: Ensure 8088 is available on server
3. **Database Permissions**: PostgreSQL access and connection limits
4. **Resource Usage**: CPU/memory usage under load
5. **Network Latency**: Server to Shyft endpoint latency differences

---

## 📞 Emergency Contacts & Commands

### Quick Status Check
```bash
./scripts/deploy-single-stream.sh status
curl -s http://localhost:8088/metrics | grep stream
```

### Emergency Stop
```bash
./scripts/deploy-single-stream.sh stop
pkill -f "hotpath_arb_engine"
```

### Logs Location
```bash
tail -f /path/to/logs/single-stream.log
tail -f /path/to/logs/deployment.log
```
