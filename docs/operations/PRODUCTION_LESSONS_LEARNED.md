# Production Lessons Learned - Monitoring & Infrastructure
Last updated: 2025-09-15

**Last Updated**: 2025-09-14
**Context**: Server monitoring stack troubleshooting during production deployment

---

## 🚨 **CRITICAL LESSON: Docker Container Port Mapping Issues**

### **The Problem**
During production deployment, discovered that Prometheus was running inside Docker but **not accessible from the host** due to missing port mappings.

### **Symptoms Observed**
1. ✅ **Grafana accessible**: http://149.28.56.125:3000 (working)
2. ❌ **Prometheus inaccessible**: http://149.28.56.125:9090 (connection refused)
3. ❌ **No metrics in Grafana**: Empty dashboards, no data source connectivity
4. ✅ **MEVana service running**: http://149.28.56.125:8088/metrics (working)

### **Root Cause Analysis**
```bash
# Investigation revealed the issue:
docker ps | grep prometheus
# OUTPUT: No port mapping shown (missing 0.0.0.0:9090->9090/tcp)
```

**Problem**: Prometheus container was running but Docker port mapping was not properly established, even though docker-compose.yml specified ports.

### **Linux vs macOS Docker Networking Differences**
- **macOS Docker Desktop**: `host.docker.internal` works for host access
- **Linux Docker**: Must use bridge IP `172.17.0.1` or proper network configuration
- **Production servers**: Always test container-to-host networking explicitly

---

## 🔧 **Technical Solutions Applied**

### **1. Container Networking Fix**
```yaml
# Fixed Prometheus configuration for Linux Docker (production)
- job_name: 'hotpath-engine'
  static_configs:
    - targets: ['149.28.56.125:8088']  # External IP for Linux Docker (production)

# For local development (macOS Docker Desktop)
- job_name: 'hotpath-engine'
  static_configs:
    - targets: ['host.docker.internal:8088']  # Works on macOS only
```

**Key Learning**: Linux Docker doesn't resolve `host.docker.internal` - must use external server IP or configure Docker bridge networking properly.

### **2. Docker Compose Port Verification**
```bash
# Always verify port mappings after container start
docker ps | grep -E "(prometheus|grafana)"
# Should show: 0.0.0.0:9090->9090/tcp for Prometheus
```

### **3. Container Restart Approach**
```bash
# Proper container management sequence
docker stop prometheus
docker start prometheus  # Picks up docker-compose port configuration
# Alternative: Recreate container with docker-compose
```

---

## 📋 **Production Deployment Checklist - Monitoring Stack**

### **Pre-Deployment Validation**
- [ ] **Docker Compose Syntax**: Validate YAML syntax and port mappings
- [ ] **Network Architecture**: Confirm container-to-host networking strategy
- [ ] **Port Conflicts**: Check if ports 3000, 9090 are available on server
- [ ] **DNS Resolution**: Test internal container DNS (grafana:3000, prometheus:9090)

### **Post-Deployment Verification**
- [ ] **Port Mapping Check**: `docker ps` shows correct port bindings
- [ ] **External Access Test**: `curl localhost:9090` and `curl localhost:3000`
- [ ] **Service Health**: Check container health status and logs
- [ ] **Cross-Service Communication**: Grafana can reach Prometheus data source
- [ ] **Metrics Collection**: Prometheus successfully scraping targets

### **Container-to-Host Service Communication**
- [ ] **Bridge IP Discovery**: `docker network inspect bridge` to find gateway IP
- [ ] **Host Service Reachability**: Test `curl 172.17.0.1:8088/metrics` from container
- [ ] **Firewall Rules**: Ensure no iptables blocking Docker bridge traffic

---

## 🎯 **Specific MEVana Production Environment**

### **Working Configuration**
```bash
# Verified Working Endpoints
- Grafana: http://149.28.56.125:3000 ✅ (credentials: admin/admin123)
- MEVana HotPath Engine: http://149.28.56.125:8088/metrics ✅
- Prometheus: http://149.28.56.125:9090 ⚠️ (requires port mapping fix)
```

### **Service Dependencies**
1. **PostgreSQL**: Port 5433 (Docker) - Working ✅
2. **MEVana Engine**: Port 8088 (Host) - Working ✅
3. **Prometheus**: Port 9090 (Container) - **Needs Fix** ⚠️
4. **Grafana**: Port 3000 (Container) - Working ✅

---

## 💡 **Prevention Strategies for Future Deployments**

### **1. Automated Health Checks**
Create a validation script that tests all monitoring endpoints after deployment:
```bash
#!/bin/bash
# monitoring_health_check.sh
endpoints=(
    "http://localhost:3000/api/health"           # Grafana
    "http://localhost:9090/-/healthy"            # Prometheus
    "http://localhost:8088/metrics"              # MEVana Engine
)
for endpoint in "${endpoints[@]}"; do
    curl -f "$endpoint" || echo "❌ FAILED: $endpoint"
done
```

### **2. Docker Compose Validation**
```bash
# Pre-deployment container validation
docker-compose config  # Validate syntax
docker-compose ps       # Verify running state
docker-compose logs     # Check for startup errors
```

### **3. Network Troubleshooting Tools**
- **Port scanning**: `nmap localhost` to verify open ports
- **Container inspection**: `docker inspect <container>` for networking details
- **Bridge network**: `docker network ls` and `docker network inspect bridge`

---

## 🔮 **Infrastructure Health Dashboard Requirements**

Based on this troubleshooting experience, the health dashboard should monitor:

### **Critical Metrics**
1. **Container Status**: Up/Down status of all Docker containers
2. **Port Accessibility**: External reachability of all service ports
3. **Service Health Checks**: Application-level health endpoints
4. **Network Connectivity**: Container-to-container and container-to-host communication
5. **Resource Utilization**: Memory, CPU, disk space per container

### **Alert Conditions**
- Container restart/failure
- Port binding failures
- Service endpoint timeouts
- Network connectivity loss
- Resource exhaustion warnings

---

## 📚 **Documentation Updates Needed**

1. **Update CLAUDE.md**: Add Docker networking troubleshooting section
2. **Update PORT_MAPPING.md**: Document all production service ports
3. **Create MONITORING_RUNBOOK.md**: Step-by-step monitoring stack deployment
4. **Update deployment scripts**: Add monitoring stack health checks

---

## 🎉 **Success Metrics**

- ✅ **Identified root cause**: Docker port mapping configuration
- ✅ **Systematic troubleshooting**: Container networking, DNS resolution, port binding
- ✅ **Platform differences**: macOS vs Linux Docker networking documented
- ✅ **Prevention strategies**: Health checks and validation procedures established

**Next Action**: Fix Prometheus port mapping and validate full monitoring stack before implementing infrastructure health dashboard.
