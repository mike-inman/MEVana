# Network Port Registry
## MEVana Services Port Allocation

**Updated**: August 25, 2025  
**Purpose**: Track network addresses and ports to prevent conflicts

## 🌐 **PORT ALLOCATION STRATEGY**

### **Port Ranges**
- **8080-8089**: Main application services
- **9090-9099**: Metrics and monitoring
- **8090-8099**: Health checks and status

## 📋 **CURRENT ALLOCATIONS**

| Service | Port | Type | Purpose | Status | Notes |
|---------|------|------|---------|--------|-------|
| **dex-stream-universal** | 8084 | HTTP | **PRIMARY DATA ORIGIN** | 🔒 RESERVED | Core Geyser stream - DO NOT REASSIGN |
| **whirlpool-tx-stream** | 8091 | HTTP | Whirlpool TX-only stream | ✅ Active | Uses slim Dockerfile; filtered to Whirlpool program |
| **token-registry-service** | 8083 | HTTP | Token metadata API | ✅ Active | Default assignment |
| **ui-gateway** | 8082 | HTTP | WebSocket gateway | ✅ Active | UI connections |
| **dex-registry** | 8085 | HTTP | DEX registry API | ✅ Active | New service |
| **executor-service** | 8080 | HTTP | Arbitrage executor | ✅ Active | Isolated service |
| **normalizer-service** | 8081 | HTTP | Data normalizer | ✅ Active | Processing pipeline |
| **shyft-pool-depth** | 8086 | HTTP | Pool depth service | ✅ Active | Shyft integration |
| **shyft-validator** | 8087 | HTTP | Transaction validator | ✅ Active | Validation pipeline |
| **price-aggregator** | 8088 | HTTP | Price aggregation | ✅ Active | Price feeds |
| **jupiter-integration** | 8089 | HTTP | Jupiter API client | ✅ Active | Signal generation |

### **Metrics Ports (Prometheus)**
| Service | Port | Purpose | Status |
|---------|------|---------|---------|
| **dex-stream-universal** | 9084 | Geyser metrics | ✅ Active |
| **dex-registry** | 9091 | Registry metrics | ✅ Active |
| **executor-service** | 9090 | Executor metrics | ✅ Active |
| **normalizer-service** | 9081 | Normalizer metrics | ✅ Active |
| **jupiter-integration** | 9089 | Jupiter metrics | ✅ Active |

### **Health Check Ports**
| Service | Port | Purpose | Status |
|---------|------|---------|---------|
| **executor-service** | 8080 | Health endpoint | ✅ Active |

## 🔧 **CONFIGURATION PATTERNS**

### **Environment Variables**
```bash
# Main service port
HTTP_PORT=8085
BIND_ADDRESS=0.0.0.0:${HTTP_PORT}

# Metrics port  
METRICS_PORT=9091

# Health check port
HEALTH_CHECK_PORT=8090
```

### **Service Discovery**
```bash
# Registry lookups
DEX_REGISTRY_URL=http://localhost:8085
TOKEN_REGISTRY_URL=http://localhost:8083
EXECUTOR_SERVICE_URL=http://localhost:8080
```

## 🚨 **RESERVED PORTS**

### **Port 8084 - CRITICAL RESERVE**
- **Service**: `dex-stream-universal`
- **Purpose**: Primary Geyser data stream origin
- **Status**: 🔒 **PERMANENTLY RESERVED**
- **Warning**: **DO NOT REASSIGN** - This is the foundation data source

### **Port Conflict Resolution**
1. Check `lsof -i :PORT` before assignment
2. Update this document immediately upon changes
3. Use `netstat -tulpn | grep PORT` for verification
4. Prefer environment variable configuration

## 📊 **PORT USAGE AUDIT**

### **Current System Status** (August 25, 2025)
```bash
# Active listeners audit
lsof -i :8080-8089  # Main service ports
lsof -i :9080-9099  # Metrics ports

# Expected services running:
# ✅ dex-stream-universal:8084
# ✅ token-registry-service:8083  
# ✅ ui-gateway:8082
# ✅ dex-registry:8085
# ✅ executor-service:8080
```

### **Service Dependencies**
```
dex-stream-universal:8084 → (PRIMARY DATA SOURCE)
    ↓
normalizer-service:8081 → processes Geyser stream
    ↓
dex-registry:8085 → enriches with metadata
    ↓
executor-service:8080 → consumes enriched data
```

## 🔄 **DOCKER COMPOSE MAPPING**

```yaml
services:
  dex-stream-universal:
    ports:
      - "8084:8084"  # RESERVED - DO NOT CHANGE
      - "9084:9084"  # Metrics
      
  dex-registry:
    ports:
      - "8085:8085"  # HTTP API
      - "9091:9091"  # Metrics
      
  executor-service:
    ports:
      - "8080:8080"  # HTTP API
      - "9090:9090"  # Metrics
```

## 🔧 **TROUBLESHOOTING**

### **Port Conflict Detection**
```bash
# Check if port is in use
lsof -i :8085

# Kill process using port (if safe)
sudo kill -9 $(lsof -t -i:8085)

# Find available ports in range
for port in {8080..8089}; do
  if ! lsof -i :$port > /dev/null 2>&1; then
    echo "Port $port is available"
  fi
done
```

### **Service Health Checks**
```bash
# Test service connectivity
curl -f http://localhost:8085/api/v1/status  # dex-registry
curl -f http://localhost:8083/tokens          # token-registry
curl -f http://localhost:8084/health          # dex-stream-universal
```

## 📈 **FUTURE EXPANSION**

### **Reserved for Growth**
- **8090-8099**: Additional services
- **9092-9099**: Additional metrics endpoints
- **7000-7999**: Development/testing services

### **External Integrations**
- **6379**: Redis (standard)
- **29092**: Kafka (standard)
- **5432**: PostgreSQL (if upgraded from SQLite)

---

**🚨 CRITICAL REMINDER**: Port 8084 is the **PRIMARY DATA ORIGIN** for the entire MEVana system. Any disruption to this service affects all downstream arbitrage operations.

**📝 UPDATE POLICY**: Any port changes must be immediately reflected in this document with timestamp and reason for change.