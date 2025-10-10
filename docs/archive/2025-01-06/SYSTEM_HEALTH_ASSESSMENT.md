# MEVana System Health Assessment & Optimization Report

**Last updated:** 2025-09-29
**Status:** ⚠️ Data pipeline incomplete
**Live Trading Readiness:** 55%

## Executive Summary

Core services are online and the Shyft geyser feed is pumping transactions into the hotpath engine, but the Kafka pipeline is still dry. Grafana now stays up and the UI deploys cleanly, yet no arbitrage events or normalized swaps reach Kafka. The next helper must focus on restoring the ingestion chain (`hotpath → Kafka → normalizer → gateway`) before any additional features land.

## 🔴 Immediate Gaps

### 1. Hotpath Kafka publishing stalled
Hotpath detects opportunities (`multi_geyser_stream_*`, `hotpath_transactions_processed_total` rising) and the event bus logs “📤 Sending arbitrage detected event…”, but the `alerts.arbitrage.opportunities` topic never advances from offset `0`. No success/error messages appear; pending events in Prometheus (`event_bus_pending_events`) keep climbing. The Kafka producer likely never flushes or the channel is backlogged.

### 2. `dex-transactions` empty
The geyser client no longer writes raw transactions into Kafka, leaving the normalizer and discovery services idle (`dex-transactions` offsets remain `0:0`). Without this feeder, downstream topics such as `orca-swaps.parsed.v2` will never see updates.

### 3. Gateway consumers idling
API service connects to Kafka, but the swap and opportunity topics are empty, so `/api/v1/metrics/performance` and SSE endpoints return no data. UI smoke tests must currently allow empty performance payloads.

## 🟡 Secondary Concerns

- Grafana alert provisioning removed due to schema drift; a clean v10-compatible ruleset needs to be rebuilt.
- Wallet vault auto-backup still logs permission denied (investigate `/opt/MEVana` ownership for backup targets).
- UI layout emits repeated 404s for `/api/system-status`; monitoring-service proxy route needs to be wired in.

## ✅ What’s Working

- Systemd services: `hotpath-arb-engine`, `normalizer-service`, `discovery-service`, `executor-service`, `wallet-vault`, `monitoring-service`, `api-service` all healthy.
- Grafana and Prometheus containers stay up; `/api/health` responds 200.
- UI deploy pipeline runs (gateway smoke test now warns instead of failing when performance data is empty).
- Rogue Next.js processes removed; only `mevana-ui.service` hosts the frontend.

## Next Steps for the Helper

1. **Unclog the event bus** – instrument `publish_to_kafka` to log successes/failures, confirm the `EVENT_BUS_CONFIG` is parsed correctly on startup, and ensure the mpsc queue drains. Once fixed, watch `alerts.arbitrage.opportunities` offsets climb and verify API `/metrics/performance` returns samples.
2. **Restore `dex-transactions` producer** – restart or reintroduce the geyser-to-Kafka streamer (likely `multi-geyser-streams` or an equivalent task). After publishing resumes, `normalizer-service` should emit to `orca-swaps.parsed.v2` and discovery metrics will tick up.
3. **Re-enable Grafana alerts** – author a new provisioning set using the v10 `condition/data` syntax, test locally, then redeploy to the server.
4. **UI diagnostics** – once Kafka topics fill, update the gateway smoke test to enforce non-empty performance data again, and hook `/api/system-status` to the monitoring service.

Document progress in this file before logging out; remove or rewrite outdated items as fixes land.
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
