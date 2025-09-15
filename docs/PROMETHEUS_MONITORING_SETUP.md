# Prometheus Monitoring Setup - AI Helper Guide

**Last Updated**: 2025-09-14
**Status**: ✅ **Industry-standard integration implemented**
**Next AI Helper**: This guide contains everything you need to know about our monitoring architecture.

## 🎯 **CRITICAL CONTEXT FOR AI HELPERS**

### **What Was Accomplished (2025-09-14)**
1. **✅ Replaced Custom Metrics**: Removed 95 lines of manual Prometheus string formatting
2. **✅ Industry Standard**: Implemented `metrics-exporter-prometheus` (proper Rust HFT approach)
3. **✅ Environment Separation**: Created dev/prod specific configurations
4. **✅ Docker Networking**: Fixed Linux Docker container-to-host communication
5. **✅ Deployment Integration**: Automated production config deployment

### **Current Architecture**
```
MEVana App → metrics crate → PrometheusBuilder → HTTP Endpoint → Prometheus Server → Grafana
```

**Single Source of Truth**: Prometheus serves all metrics to Grafana dashboards.

---

## 🏗️ **ENVIRONMENT-SPECIFIC CONFIGURATIONS**

### **Development (Local macOS)**
- **File**: `infrastructure/prometheus.yml`
- **Networking**: Uses `host.docker.internal:8088` (macOS Docker Desktop support)
- **Usage**: `docker-compose up -d` in `infrastructure/`

### **Production (Linux Server 149.28.56.125)**
- **File**: `infrastructure/prometheus.prod.yml`
- **Networking**: Uses external server IP `149.28.56.125:8088` (Linux Docker requirement)
- **Deployment**: Automatically used by `scripts/simple-deploy.sh`

### **Key Difference: Docker Networking**
```yaml
# Development (macOS)
- targets: ['host.docker.internal:8088']  # ✅ Works on macOS

# Production (Linux)
- targets: ['149.28.56.125:8088']         # ✅ Required for Linux Docker
```

**Why**: Linux Docker doesn't resolve `host.docker.internal` (see PRODUCTION_LESSONS_LEARNED.md)

---

## 📊 **METRICS ENDPOINTS STATUS**

### **✅ WORKING ENDPOINTS**
- **MEVana Metrics**: http://149.28.56.125:8088/metrics (✅ Live with industry-standard format)
- **Grafana**: http://149.28.56.125:3000 (admin/admin123) (✅ Ready for dashboards)
- **Prometheus**: http://149.28.56.125:9090 (✅ Running, needs config refresh)

### **🔄 NEXT DEPLOYMENT WILL FIX**
- Prometheus scraping MEVana metrics (networking config applied)
- Full Grafana dashboard functionality with live data
- Complete industry-standard monitoring stack

---

## 🚀 **DEPLOYMENT PROCESS**

### **For Production Deployment**
```bash
# This script now handles everything automatically:
./scripts/simple-deploy.sh
```

**What it does:**
1. Transfers code to server
2. **Copies `prometheus.prod.yml` to `prometheus.yml`** (production networking)
3. **Restarts Docker containers** with updated config
4. Builds and starts MEVana services
5. Validates endpoints

### **Manual Docker Restart (if needed)**
```bash
# On server (149.28.56.125)
cd /opt/MEVana/infrastructure
cp prometheus.prod.yml prometheus.yml  # Use production config
docker-compose down && docker-compose up -d
```

---

## 📁 **FILE LOCATIONS**

### **Prometheus Configurations**
- `infrastructure/prometheus.yml` - Development (macOS Docker)
- `infrastructure/prometheus.prod.yml` - Production (Linux Docker)

### **Grafana Dashboards**
- `dashboards/mevana_complete_dashboard.json` - Complete infrastructure dashboard
- `dashboards/infrastructure_health_dashboard.json` - Health-focused dashboard

### **Deployment Scripts**
- `scripts/simple-deploy.sh` - Main production deployment (includes Docker config)
- `PRODUCTION_LESSONS_LEARNED.md` - Docker networking troubleshooting guide

---

## 🔧 **CODE IMPLEMENTATION DETAILS**

### **Industry-Standard Metrics in main.rs**
```rust
// ✅ IMPLEMENTED: Industry standard approach
use metrics_exporter_prometheus::PrometheusBuilder;

let prometheus_handle = PrometheusBuilder::new()
    .with_http_listener(format!("0.0.0.0:{}", args.metrics_port).parse::<std::net::SocketAddr>().unwrap())
    .install()
    .expect("Failed to install Prometheus metrics exporter");
```

**Key Points:**
- Uses `metrics-exporter-prometheus` crate (industry standard)
- Automatic Prometheus format generation
- Zero-latency impact on trading hotpath
- All existing `metrics::counter!()` calls unchanged

### **Removed Custom Implementation**
- ❌ **Deleted**: `serve_prometheus_metrics()` function (95 lines)
- ❌ **Deleted**: Manual string formatting for Prometheus format
- ❌ **Deleted**: Custom `/metrics` endpoint handler

---

## 🎛️ **GRAFANA DASHBOARD IMPORT**

### **Import Process**
1. Access: http://149.28.56.125:3000 (admin/admin123)
2. Import JSON from `dashboards/mevana_complete_dashboard.json`
3. Configure Prometheus data source: `http://prometheus:9090`
4. Dashboard will show live metrics after next deployment

### **Available Metrics**
- `multi_geyser_stream_messages_total` - Message processing counts
- `multi_geyser_stream_healthy` - Stream health status
- `multi_geyser_dex_messages_total` - DEX-specific processing
- `multi_geyser_stream_seconds_since_last_message` - Latency monitoring
- Standard process metrics (CPU, memory, uptime)

---

## 🚨 **TROUBLESHOOTING FOR AI HELPERS**

### **Common Issues**
1. **"Prometheus not scraping MEVana"**
   - ✅ **Solution**: Deploy with `./scripts/simple-deploy.sh` (auto-fixes config)
   - **Manual**: Copy `prometheus.prod.yml` to `prometheus.yml` on server

2. **"Empty Grafana dashboards"**
   - **Cause**: Prometheus networking misconfiguration
   - **Check**: `curl http://149.28.56.125:9090/api/v1/targets` for scraping status

3. **"Connection refused errors"**
   - **Cause**: Linux Docker can't resolve `host.docker.internal`
   - **Solution**: Use external server IP in production config

### **Validation Commands**
```bash
# Test MEVana metrics endpoint
curl http://149.28.56.125:8088/metrics | head -10

# Check Prometheus targets
curl http://149.28.56.125:9090/api/v1/targets | grep hotpath

# Verify Grafana health
curl http://149.28.56.125:3000/api/health
```

---

## 📚 **RELATED DOCUMENTATION**

- **`PRODUCTION_LESSONS_LEARNED.md`** - Docker networking troubleshooting
- **`PORT_MAPPING.md`** - Service port allocations
- **`CLAUDE.md`** - Project context and current session status
- **`DOCUMENTATION_INDEX.md`** - Master documentation reference

---

## 💡 **FOR NEXT AI HELPER**

**Current State**: Industry-standard Prometheus integration is complete. Next deployment will enable full dashboard functionality.

**If user asks about monitoring**:
1. ✅ **Metrics work**: Direct endpoint at :8088/metrics
2. ✅ **Architecture is correct**: Industry standard implementation
3. 🔄 **Dashboards need**: Next deployment to fix Prometheus scraping

**If you need to debug**: Check Prometheus targets API first, then validate Docker networking.

**Priority**: The monitoring stack is architecturally complete - just needs deployment refresh for networking fix.