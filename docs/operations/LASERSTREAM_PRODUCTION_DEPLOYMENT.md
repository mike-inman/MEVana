# Laserstream Production Deployment - Issues and Resolution

**Date**: September 17, 2025
**System**: MEVana Solana Monitoring Pipeline
**Status**: ✅ RESOLVED - Live data visible in production Grafana

## Overview

This document details the issues encountered when deploying the Laserstream Solana metrics dashboard from local development to production, and the systematic resolution approach that ultimately succeeded.

## Initial Goal

Deploy the locally-working Laserstream dashboard to production server (149.28.56.125) with the same configuration and functionality as the development environment.

## Problems Encountered

### 1. Missing Dashboard File
**Issue**: The `solana-laserstream-dashboard.json` dashboard file existed locally but was not present in the production Grafana dashboards directory.

**Symptoms**: No Laserstream dashboard visible in production Grafana interface.

**Root Cause**: Dashboard file was never copied to production during initial setup.

### 2. Container Networking Issues
**Issue**: Prometheus could not scrape metrics from the Laserstream service due to networking misconfiguration.

**Symptoms**:
- Prometheus target showing status "DOWN" (value = 0)
- Error: `docker exec prometheus-service wget http://172.17.0.1:9100/metrics` - connection timeout

**Root Cause**: Laserstream running as host process, but Prometheus container couldn't reach `172.17.0.1:9100` due to Docker networking restrictions.

### 3. Binary Compatibility Issues
**Issue**: Laserstream binary failed to run in Ubuntu 20.04 container.

**Symptoms**:
```
/laserstream-metrics: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.32' not found
/laserstream-metrics: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.33' not found
/laserstream-metrics: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.34' not found
```

**Root Cause**: Binary compiled on system with newer glibc than Ubuntu 20.04 provides.

### 4. Data Source Configuration Mismatch
**Issue**: Grafana data source pointed to wrong Prometheus hostname.

**Symptoms**:
```
Post "http://mevana-prometheus-1:9090/api/v1/query_range": dial tcp: lookup mevana-prometheus-1 on 127.0.0.11:53: server misbehaving
```

**Root Cause**: Grafana data source configured with `mevana-prometheus-1:9090` instead of correct service name `prometheus-service:9090`.

### 5. Dashboard Data Source UID Mismatch
**Issue**: Dashboard file contained incorrect data source UID references.

**Symptoms**: Dashboard panels showing "No data" or connection errors.

**Root Cause**: Local dashboard used UID `PBFA97CFB590B2093` while production used UID `prometheus`.

## Resolution Steps

### Step 1: Copy Dashboard File
```bash
scp /Users/michaelinman/Repos/MEVana/infrastructure/grafana/dashboards/solana-laserstream-dashboard.json root@149.28.56.125:/opt/MEVana/infrastructure/grafana/dashboards/
```

### Step 2: Containerize Laserstream Service
**Problem**: Host-based networking not working with containerized Prometheus.

**Solution**: Deploy Laserstream as Docker container on `mevana_net` network.

```bash
# Create Dockerfile with compatible base image
cat > Dockerfile << EOF
FROM ubuntu:22.04
COPY target/release/laserstream-metrics /laserstream-metrics
RUN chmod +x /laserstream-metrics
EXPOSE 9100
CMD ["/laserstream-metrics"]
EOF

# Build and run container
docker build -t laserstream-metrics .
docker run -d --name laserstream-metrics --network mevana_net -e RUST_LOG=info laserstream-metrics
```

### Step 3: Update Prometheus Configuration
```yaml
# /opt/MEVana/infrastructure/prometheus.core.yml
- job_name: 'laserstream-metrics'
  static_configs:
    - targets: ['laserstream-metrics:9100']  # Changed from 172.17.0.1:9100
  metrics_path: '/metrics'
  scrape_interval: 10s
  scrape_timeout: 5s
```

### Step 4: Fix Grafana Data Source
```bash
# Update data source URL via Grafana API
curl -X PUT -H "Content-Type: application/json" -u admin:admin123 \
"http://localhost:3000/api/datasources/2" -d '{
  "id": 2,
  "uid": "prometheus",
  "orgId": 1,
  "name": "Prometheus",
  "type": "prometheus",
  "access": "proxy",
  "url": "http://prometheus-service:9090",  # Fixed hostname
  "isDefault": true,
  "jsonData": {}
}'
```

### Step 5: Fix Dashboard Data Source UIDs
```bash
# Update dashboard file to use production data source UID
sed -i 's/PBFA97CFB590B2093/prometheus/g' /opt/MEVana/infrastructure/grafana/dashboards/solana-laserstream-dashboard.json
```

## Configuration Differences: Local vs Production

| Component | Local Configuration | Production Configuration | Notes |
|-----------|-------------------|------------------------|-------|
| **Laserstream Deployment** | Host process | Docker container | Required for proper networking |
| **Base Image** | N/A | Ubuntu 22.04 | Ubuntu 20.04 incompatible with binary |
| **Network** | Host networking | `mevana_net` bridge | Container-to-container communication |
| **Prometheus Target** | `host.docker.internal:9100` | `laserstream-metrics:9100` | Service discovery via container name |
| **Data Source URL** | Auto-configured | `http://prometheus-service:9090` | Manual configuration required |
| **Dashboard Data Source UID** | `PBFA97CFB590B2093` | `prometheus` | Production uses simplified UID |
| **File Location** | `/infrastructure/grafana/dashboards/` | `/opt/MEVana/infrastructure/grafana/dashboards/` | Different base paths |

## Verification Commands

### Check Laserstream Container Status
```bash
docker ps | grep laserstream
docker logs laserstream-metrics
```

### Verify Prometheus Scraping
```bash
curl -s "http://localhost:9090/api/v1/query?query=up" | jq '.data.result[] | select(.metric.job=="laserstream-metrics")'
```

### Test Grafana Data Source Connection
```bash
curl -X POST -H "Content-Type: application/json" -u admin:admin123 \
"http://localhost:3000/api/datasources/proxy/2/api/v1/query?query=mevana_e2e_latency_ms_count"
```

### Confirm Live Metrics
```bash
curl -s "http://localhost:9090/api/v1/query?query=mevana_e2e_latency_ms_count" | jq '.data.result[0].value[1]'
```

## Success Metrics

- ✅ **Laserstream Container**: Running and generating metrics
- ✅ **Prometheus Scraping**: Target status UP (value = 1)
- ✅ **Live Data**: 734+ Jupiter transactions, 148+ unknown program transactions
- ✅ **Grafana Dashboard**: Visible data with real-time updates
- ✅ **Production Access**: `http://149.28.56.125:3000` (admin/admin123)

## Lessons Learned

### 1. **End-to-End Testing Critical**
- Verify complete user workflow, not just technical components
- Test actual dashboard visibility before declaring success
- Validate data source connectivity through web interface

### 2. **Container Networking Requires Consistency**
- All services should use same Docker network for reliable communication
- Host-to-container networking has limitations in production
- Service discovery works better with container names than IP addresses

### 3. **Configuration Drift Between Environments**
- Data source configurations don't automatically sync
- Dashboard UIDs and hostnames must be explicitly matched
- File deployment requires verification of successful copy

### 4. **Binary Compatibility Matters**
- Base image selection affects binary compatibility
- glibc version mismatches cause runtime failures
- Ubuntu LTS version differences can break deployments

## Future Deployment Checklist

1. **Pre-deployment**:
   - [ ] Verify all dashboard files exist in production
   - [ ] Check data source configurations match
   - [ ] Validate container base image compatibility
   - [ ] Test binary execution in target environment

2. **During deployment**:
   - [ ] Use container deployment for consistent networking
   - [ ] Update Prometheus targets to use container names
   - [ ] Verify service-to-service connectivity
   - [ ] Test data source queries via API

3. **Post-deployment**:
   - [ ] Verify dashboard appears in Grafana UI
   - [ ] Confirm live data is visible in charts
   - [ ] Test complete user workflow end-to-end
   - [ ] Document any configuration differences

## Related Files

- **Dashboard**: `/opt/MEVana/infrastructure/grafana/dashboards/solana-laserstream-dashboard.json`
- **Prometheus Config**: `/opt/MEVana/infrastructure/prometheus.core.yml`
- **Docker Compose**: `/opt/MEVana/infrastructure/docker-compose.server.yml`
- **Laserstream Dockerfile**: `/opt/laserstream-metrics/Dockerfile`

---

**Resolution Date**: September 17, 2025
**Total Time**: ~2 hours of systematic debugging
**Final Status**: ✅ Production dashboard displaying live Solana transaction data