# MEVana Production Server Inventory Report

**Server:** mevana-nyc-01 (149.28.56.125)
**Date:** September 27, 2025
**Status:** ⚠️ Partially Operational (6/7 services running)

## 🐳 Docker Infrastructure Services (All Running ✅)

| Service | Container Name | Status | Ports | Purpose |
|---------|---------------|--------|-------|---------|
| **Kafka** | mevana-kafka-1 | ✅ Up 16 hours | 29092→29092 | Message broker |
| **Zookeeper** | mevana-zookeeper-1 | ✅ Up 16 hours | 2181 (internal) | Kafka coordination |
| **PostgreSQL** | mevana-postgres-1 | ✅ Up 16 hours | 5433→5432 | Primary database |
| **Redis** | mevana-redis-1 | ✅ Up 16 hours | 6380→6379 | Cache & rate limiting |
| **Prometheus** | mevana-prometheus-1 | ✅ Up 16 hours | 9090→9090, 9091→9090 | Metrics collection |
| **Grafana** | mevana-grafana-1 | ✅ Up 7 hours | 3000→3000, 3001→3000 | Dashboards |
| **Kafka UI** | mevana-kafka-ui-1 | ✅ Up 16 hours | 8084→8080 | Kafka admin |
| **Adminer** | mevana-adminer-1 | ✅ Up 16 hours | 8087→8080 | Database admin |

**Docker Network:** mevana_net (external)

## 🔧 Bare-Metal Services (systemd)

| Service | Status | Port(s) | Health | CPU% | Issues |
|---------|--------|---------|--------|------|--------|
| **hotpath-arb-engine** | ✅ Running | 8088 (metrics), 8089 (API) | ✅ 200 | 2.4% | Working perfectly |
| **normalizer-service** | ✅ Running | 9901 | ✅ 200 | 0.9% | Operational |
| **discovery-service** | ✅ Running | 9102 | ✅ 200 | 0.9% | Operational |
| **executor-service** | ✅ Running | 9898 (admin), 9904 (metrics) | ✅ 200 | 3.6% | Operational |
| **api-service** | ✅ Running | 8092 (REST), 8095 (WS) | ✅ 200/404 | 0.1% | Gateway endpoint returns 404 |
| **monitoring-service** | ✅ Running | 8090 | ✅ 200 | 0.0% | **Using wrong port (should be 8091)** |
| **wallet-vault** | 🔴 Crashing | 8089 (intended) | ❌ | - | **Port conflict with hotpath** |

## 🔴 Critical Issues Found

### 1. **wallet-vault Service Crashing**
- **Problem:** Trying to bind port 8089, already used by hotpath-arb-engine
- **Error:** `Address already in use (os error 98)`
- **Restart count:** 202+ times
- **Solution:** Need to configure wallet-vault to use port 8090 (currently used by monitoring-service)

### 2. **Port Configuration Conflicts**
```
Port 8089: hotpath-arb-engine (API endpoint) ← conflicts with wallet-vault
Port 8090: monitoring-service ← should be on 8091
Port 8091: FREE ← monitoring-service should be here
```

### 3. **Missing Configuration Files**
- `/etc/mevana/wallet-vault.env` - Not found (causing default port conflict)
- `/etc/mevana/monitoring.env` - Missing

### 4. **API Gateway Issues**
- Gateway WebSocket endpoint (8095) returns 404 on /health
- MEVana-UI configured to connect to port 8096 (not listening)

## 📊 Port Mapping Summary

### Currently Active Listeners
```
8088 - hotpath_arb_engine (metrics) ✅
8089 - hotpath_arb_engine (API) ✅
8090 - monitoring-service ⚠️ (wrong port)
8092 - api-server (REST) ✅
8095 - api-server (WebSocket) ✅
9102 - discovery-service ✅
9898 - executor-service (admin) ✅
9901 - normalizer-service ✅
9904 - executor-service (metrics) ✅

Infrastructure (Docker):
3000 - Grafana ✅
5433 - PostgreSQL ✅
6380 - Redis ✅
8084 - Kafka UI ✅
8087 - Adminer ✅
9090 - Prometheus ✅
29092 - Kafka ✅
```

### Expected But Missing
```
8091 - monitoring-service (should be here)
8090 - wallet-vault (should be here)
8096 - Nothing (UI expects gateway here)
```

## 🛠️ Immediate Fix Commands

```bash
# 1. Fix wallet-vault port configuration
ssh root@149.28.56.125 << 'EOF'
cat > /etc/mevana/wallet-vault.env << 'CONFIG'
WALLET_VAULT_PORT=8090
WALLET_VAULT_AUTH_REQUIRED=true
WALLET_VAULT_SECRET="mevana-vault-secure-secret-2024"
CONFIG
chown root:mevana /etc/mevana/wallet-vault.env
chmod 640 /etc/mevana/wallet-vault.env
EOF

# 2. Fix monitoring-service port
ssh root@149.28.56.125 << 'EOF'
cat > /etc/mevana/monitoring.env << 'CONFIG'
MONITORING_PORT=8091
CONFIG
chown root:mevana /etc/mevana/monitoring.env
chmod 640 /etc/mevana/monitoring.env
EOF

# 3. Restart services with correct ports
ssh root@149.28.56.125 << 'EOF'
systemctl stop wallet-vault monitoring-service
sleep 2
systemctl start monitoring-service
sleep 2
systemctl start wallet-vault
systemctl status wallet-vault monitoring-service --no-pager
EOF
```

## 📈 Resource Usage

- **Total Services:** 14 (8 Docker + 6 Bare-metal)
- **Memory Usage:** Low (~5% total)
- **CPU Usage:** Low (< 10% combined)
- **Disk I/O:** Normal
- **Network:** Active (Kafka streaming)

## ✅ What's Working Well

1. **Infrastructure Layer:** All Docker services healthy
2. **Core Trading Path:** hotpath → normalizer → executor chain operational
3. **API Gateway:** Running and serving REST/WebSocket
4. **Monitoring:** Prometheus collecting metrics from most services
5. **Database:** PostgreSQL stable with connections from all services

## 🎯 Recommendations

### Immediate Actions (5 minutes)
1. Fix wallet-vault port conflict (use script above)
2. Move monitoring-service to port 8091
3. Update MEVana-UI to use port 8092 instead of 8096

### Short-term (This Week)
1. Create missing env files with proper templates
2. Add health check endpoints to all services
3. Implement proper service dependency management in systemd
4. Setup log aggregation (all services logging to /opt/MEVana/logs/)

### Architecture Decision
**Keep bare-metal deployment for custom services** - The advantages are clear:
- Fast iteration (no Docker rebuild)
- Direct debugging access
- Better performance for latency-critical services
- Simpler deployment pipeline

**Keep Docker for infrastructure** - Also clear benefits:
- Stable, tested configurations
- Easy backup/restore
- Network isolation
- Version management

## 📋 Service Dependency Map

```
Docker Infrastructure:
├── PostgreSQL (5433)
├── Redis (6380)
├── Kafka (29092)
│   └── Zookeeper (2181)
├── Prometheus (9090)
│   └── Scrapes all bare-metal services
└── Grafana (3000)
    └── Queries Prometheus

Bare-Metal Services:
├── hotpath-arb-engine (8088/8089)
│   └── Direct Geyser connection
├── normalizer-service (9901)
│   └── Kafka consumer/producer
├── discovery-service (9102)
│   └── Kafka consumer
├── executor-service (9898/9904)
│   └── Database + Kafka
├── api-service (8092/8095)
│   └── Database + Kafka + Redis
├── monitoring-service (8090→8091)
│   └── Aggregates from all services
└── wallet-vault (failed→8090)
    └── Database + Encryption
```

## Conclusion

The system is **85% operational** with clear, fixable issues. The hybrid Docker/bare-metal approach is working well and should be maintained. Priority fixes:
1. Resolve port conflicts (5 min)
2. Fix UI connection points (10 min)
3. Ensure all services have proper config files (20 min)

With these fixes, the system will be ready for intensive testing before live trading deployment.