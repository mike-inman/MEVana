# 🚨 MASTER PORT ASSIGNMENTS - AUTHORITATIVE REFERENCE 🚨

**THIS DOCUMENT SUPERSEDES ALL OTHER PORT DOCUMENTATION**
Last Updated: September 27, 2025
Status: **FINAL - DO NOT DEVIATE**

## ⚡ CRITICAL: Port Assignments (MUST USE THESE)

### Bare-Metal Services (systemd)

| Service | Port | Purpose | Environment Variable | Notes |
|---------|------|---------|---------------------|--------|
| **hotpath-arb-engine** | 8088 | Metrics | `METRICS_PORT=8088` | Prometheus scraping |
| **hotpath-arb-engine** | 8089 | API/Performance | (automatic from 8088+1) | `/performance` endpoint |
| **wallet-vault** | **8090** | REST API | `VAULT_PORT=8090` | **NOT 8089 - that's hotpath!** |
| **monitoring-service** | **8091** | Metrics/API | `MONITORING_PORT=8091` | **NOT 8090 - that's wallet!** |
| **api-service** | 8092 | REST Gateway | `PORT=8092` | Main API gateway |
| **api-service** | 8095 | WebSocket/SSE | `MEVANA_WEBSOCKET_GATEWAY_PORT=8095` | Real-time streams |
| **normalizer-service** | 9901 | Metrics | `METRICS_PORT=9901` | Kafka pipeline metrics |
| **discovery-service** | 9102 | Metrics | `DISCOVERY_METRICS_PORT=9102` | Service discovery |
| **executor-service** | 9898 | Admin API | `HEALTH_CHECK_PORT=9898` | Admin operations |
| **executor-service** | 9904 | Metrics | `METRICS_PORT=9904` | Prometheus metrics |

### Docker Infrastructure (NEVER CHANGE)

| Service | Host Port | Container Port | Notes |
|---------|-----------|---------------|--------|
| PostgreSQL | 5433 | 5432 | Database |
| Redis | 6380 | 6379 | Cache (dev only, prod internal) |
| Kafka | 29092 | 9092 | Message broker |
| Zookeeper | - | 2181 | Internal only |
| Prometheus | 9090 | 9090 | Metrics collector |
| Grafana | 3000 | 3000 | Dashboards |
| Kafka UI | 8084 | 8080 | Admin UI |
| Adminer | 8087 | 8080 | Database UI |

## 🔴 COMMON MISTAKES TO AVOID

1. **DON'T** put wallet-vault on 8089 - that's hotpath's API port
2. **DON'T** put monitoring-service on 8090 - that's wallet-vault's port
3. **DON'T** expect api-service on 8096 - it's 8092 (REST) and 8095 (WS)
4. **DON'T** use `WALLET_VAULT_PORT` - the code expects `VAULT_PORT`
5. **DON'T** use port 8091 for anything except monitoring-service

## ✅ CORRECT CONFIGURATION FILES

### `/etc/systemd/system/wallet-vault.service`
```ini
Environment=VAULT_PORT=8090
```

### `/etc/systemd/system/monitoring-service.service`
```ini
Environment=MONITORING_PORT=8091
```

### `/etc/mevana/wallet-vault.env`
```bash
VAULT_PORT=8090
WALLET_VAULT_AUTH_REQUIRED=true
WALLET_VAULT_SECRET="mevana-vault-secure-secret-2024"
```

### `/etc/mevana/monitoring.env`
```bash
MONITORING_PORT=8091
```

### `/etc/mevana/api-service.env`
```bash
PORT=8092
MEVANA_GATEWAY_ENABLE=true
MEVANA_WEBSOCKET_GATEWAY_PORT=8095
```

## 📍 MEVana-UI Configuration

The UI should connect to:
- API Gateway: `http://SERVER_IP:8092/api/v1`
- WebSocket: `ws://SERVER_IP:8095/gateway/ws`
- SSE Streams: `http://SERVER_IP:8092/sse`

**NOT** port 8096 (nothing listens there!)

## 🔧 Quick Verification Commands

```bash
# Check what's actually listening
netstat -tlnp | grep -E ':(808[89]|809[0125]|990[124]|9898)' | sort

# Expected output:
# 8088 - hotpath_arb_engine (metrics)
# 8089 - hotpath_arb_engine (API)
# 8090 - wallet-vault
# 8091 - monitoring-service
# 8092 - api-server
# 8095 - api-server (WS)
# 9901 - normalizer-service
# 9102 - discovery-service
# 9898 - executor-service (admin)
# 9904 - executor-service (metrics)
```

## 🚀 Deployment Checklist

Before any deployment:
1. ✓ Verify `/opt/MEVana/infrastructure/systemd/*.service` has correct ports
2. ✓ Check `/etc/mevana/*.env` files exist with correct ports
3. ✓ Ensure no service is trying to use 8089 except hotpath
4. ✓ Confirm monitoring is on 8091, NOT 8090
5. ✓ Update MEVana-UI to use 8092/8095, NOT 8096

## 📝 Historical Context

- **Original Problem**: wallet-vault was hardcoded to use 8089 (conflict with hotpath)
- **Wrong Fix Attempt**: Tried moving monitoring to free up 8090
- **Correct Fix**: wallet-vault → 8090, monitoring → 8091
- **Repository Status**: Already has correct ports in systemd files
- **Server Status**: Fixed as of 2025-09-27

## 🎯 Source of Truth Priority

1. **THIS DOCUMENT** (PORT_ASSIGNMENTS_MASTER.md)
2. infrastructure/systemd/*.service files in repo
3. Running service configurations on server
4. Never trust old documentation if it conflicts with this

---

**IF IN DOUBT, FOLLOW THIS DOCUMENT EXACTLY**

Contact: Deploy issues? Check this document first. Still confused? The services are configured correctly in the repo's `infrastructure/systemd/` directory.