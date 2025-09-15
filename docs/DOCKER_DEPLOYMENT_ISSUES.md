# Docker Deployment Issues and Solutions

**Last Updated**: 2025-09-15
**Context**: Issues encountered during production Prometheus deployment
**Priority**: HIGH - Prevents automated deployment

## 🚨 **CRITICAL ISSUES IDENTIFIED**

### **1. Docker Compose Not Available on Server**
**Problem**: Server `149.28.56.125` doesn't have `docker-compose` installed
```bash
# Error encountered:
bash: line 1: docker-compose: command not found
docker: 'compose' is not a docker command.
```

**Impact**:
- Deployment script fails at Docker configuration step (exit code 127)
- Cannot restart monitoring stack automatically
- Prometheus config changes don't take effect

**Root Cause**:
- Production server has Docker but not Docker Compose
- Our deployment scripts assume Docker Compose is available

### **2. Prometheus Config Reload Issue**
**Problem**: Even after copying new `prometheus.yml`, Prometheus container continues using old configuration

**Evidence**:
```bash
# File shows correct config:
- targets: ['149.28.56.125:8088']

# But Prometheus API shows old config:
"scrapeUrl":"http://172.17.0.1:8088/metrics"
```

**Root Cause**: Prometheus needs explicit config reload or container restart with proper volume mounting

---

## 🔧 **IMMEDIATE FIXES NEEDED**

### **Fix 1: Install Docker Compose on Server**
```bash
# On server 149.28.56.125:
curl -SL https://github.com/docker/compose/releases/latest/download/docker-compose-linux-x86_64 -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose
```

### **Fix 2: Alternative Docker Management**
```bash
# Use native Docker commands instead of docker-compose:
docker restart $(docker ps -q --filter name=prometheus)
docker restart $(docker ps -q --filter name=grafana)
```

### **Fix 3: Force Prometheus Config Reload**
```bash
# Method 1: Container restart (works)
docker restart $(docker ps -q --filter name=prometheus)

# Method 2: SIGHUP signal (if container supports it)
docker exec $(docker ps -q --filter name=prometheus) kill -HUP 1
```

---

## 📝 **DEPLOYMENT SCRIPT FIXES**

### **Updated Script Section**
Replace the docker-compose section in `scripts/simple-deploy.sh`:

```bash
# OLD (causes failure):
docker-compose down && docker-compose up -d

# NEW (robust approach):
echo "🐳 Updating Prometheus configuration..."
cp prometheus.prod.yml prometheus.yml

# Check if docker-compose is available
if command -v docker-compose >/dev/null 2>&1; then
    echo "📦 Using Docker Compose..."
    docker-compose down 2>/dev/null || true
    docker-compose up -d
elif command -v docker >/dev/null 2>&1; then
    echo "📦 Using native Docker commands..."
    docker restart $(docker ps -q --filter name=prometheus) || true
    docker restart $(docker ps -q --filter name=grafana) || true
else
    echo "❌ Neither docker-compose nor docker found"
    exit 1
fi

echo "⏳ Waiting for containers to restart..."
sleep 10
```

---

## 🔄 **VERIFICATION STEPS**

### **1. Check Docker Compose Availability**
```bash
ssh root@149.28.56.125 "command -v docker-compose && echo 'Available' || echo 'Missing'"
```

### **2. Verify Prometheus Config Reload**
```bash
# Check targets API shows new configuration
curl -s "http://149.28.56.125:9090/api/v1/targets" | grep "149.28.56.125"
```

### **3. Test End-to-End Metrics Flow**
```bash
# 1. Check MEVana metrics
curl http://149.28.56.125:8088/metrics | grep multi_geyser

# 2. Check Prometheus can query them
curl -s "http://149.28.56.125:9090/api/v1/query?query=multi_geyser_stream_messages_total"

# 3. Verify Grafana data source
curl http://149.28.56.125:3000/api/health
```

---

## 📋 **PREVENTION STRATEGY**

### **1. Server Preparation Script**
Create `scripts/setup-server.sh`:
```bash
#!/bin/bash
# Ensure server has all required dependencies

# Install Docker Compose
curl -SL https://github.com/docker/compose/releases/latest/download/docker-compose-linux-x86_64 -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Verify installation
docker-compose --version
docker --version

echo "✅ Server preparation complete"
```

### **2. Pre-deployment Validation**
Add to deployment scripts:
```bash
# Validate environment before proceeding
ssh $SERVER_USER@$SERVER_IP "command -v docker && command -v docker-compose" || {
    echo "❌ Missing Docker dependencies. Run setup-server.sh first"
    exit 1
}
```

### **3. Post-deployment Health Check**
```bash
# Comprehensive validation
validate_monitoring_stack() {
    local retries=5
    for i in $(seq 1 $retries); do
        if curl -s "http://$SERVER_IP:9090/api/v1/query?query=up" | grep -q "success"; then
            echo "✅ Prometheus operational"
            return 0
        fi
        echo "⏳ Waiting for Prometheus... ($i/$retries)"
        sleep 10
    done
    echo "❌ Prometheus health check failed"
    return 1
}
```

---

## 🎯 **NEXT ACTIONS**

### **Immediate (Today)**
1. ✅ Document issues (this file)
2. 🔄 **Install Docker Compose on server**
3. 🔄 **Update deployment script with fallback logic**
4. 🔄 **Test full deployment end-to-end**

### **This Week**
1. Create server setup script
2. Add pre-deployment validation
3. Implement comprehensive health checks
4. Test deployment on clean server

---

## 🚨 **FOR NEXT AI HELPER**

**Current State**: Docker infrastructure partially working but needs manual intervention

**Key Issues**:
- Server missing Docker Compose (causes deployment failures)
- Prometheus config reload requires explicit container restart

---

Legacy notes
- Additional historical details were migrated from the root repository into `docs/operations/DOCKER_DEPLOYMENT_ISSUES_legacy.md` on 2025-09-15 for reference.
- No automated validation of monitoring stack health

**Quick Fix**: Run these commands to get monitoring working:
```bash
ssh root@149.28.56.125 "cd /opt/MEVana/infrastructure && docker restart \$(docker ps -q --filter name=prometheus)"
```

**Long-term**: Implement the server setup script and deployment validation outlined above.

---

## 📚 **RELATED DOCUMENTATION**

- **PROMETHEUS_MONITORING_SETUP.md** - Overall architecture
- **PRODUCTION_LESSONS_LEARNED.md** - Docker networking issues
- **scripts/simple-deploy.sh** - Main deployment script (needs updates)
