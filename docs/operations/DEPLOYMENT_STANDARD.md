# Production Deployment Standard - MANDATORY

## ⚠️ CRITICAL: This is the ONLY approved deployment method for production

**Last Updated**: 2025-01-09
**Status**: MANDATORY - Deviations require explicit approval
**Enforcement**: All AI assistants and operators MUST follow this standard

---

## Executive Summary

**THE RULE**: Production server MUST ALWAYS use `docker-compose.server.yml` with project name `mevana`

**WHY**: Inconsistent deployment methods cause:
- ❌ Port conflicts (Grafana on 3000 vs 3001)
- ❌ Orphaned containers (mevana-* vs infrastructure-*)
- ❌ Volume name mismatches
- ❌ Configuration drift
- ❌ Wasted time debugging "why did the port change again?"

---

## MANDATORY Production Deployment Commands

### Infrastructure Deployment

**ALWAYS use this exact command**:
```bash
cd /opt/MEVana/infrastructure
docker compose -p mevana -f docker-compose.server.yml up -d
```

**NEVER use**:
- ❌ `docker compose up -d` (uses wrong file)
- ❌ `docker compose -f docker-compose.yml up -d` (deprecated)
- ❌ `docker compose -f docker-compose.core.yml up -d` (development only)
- ❌ Any command without `-p mevana` flag

### Infrastructure Shutdown

**ALWAYS use this exact command**:
```bash
cd /opt/MEVana/infrastructure
docker compose -p mevana -f docker-compose.server.yml down
```

### Check Current State

**ALWAYS verify container names start with `mevana-`**:
```bash
docker ps --format 'table {{.Names}}\t{{.Status}}'
```

**Expected output**:
```
NAMES                 STATUS
mevana-grafana-1      Up X minutes (healthy)
mevana-kafka-1        Up X minutes (healthy)
mevana-zookeeper-1    Up X minutes (healthy)
mevana-postgres-1     Up X minutes (healthy)
mevana-redis-1        Up X minutes (healthy)
mevana-prometheus-1   Up X minutes (healthy)
```

**If you see `infrastructure-*` prefix**: ❌ WRONG - Redeploy immediately

---

## Docker Compose File Reference

### Production (Server)
- **File**: `docker-compose.server.yml`
- **Purpose**: Production deployment configuration
- **Project Name**: `mevana` (set via `-p` flag)
- **Container Prefix**: `mevana-*`
- **When to Use**: ALL production deployments
- **Extends**: `docker-compose.core.yml` with production overrides

**Key Production Overrides**:
```yaml
# Grafana port - PRODUCTION STANDARD
grafana:
  ports:
    - "3000:3000"  # NOT 3001

# Prometheus port - PRODUCTION STANDARD
prometheus:
  ports:
    - "9090:9090"  # NOT 9091
```

### Core (Development/Base)
- **File**: `docker-compose.core.yml`
- **Purpose**: Base configuration + local development
- **Project Name**: `infrastructure` (default)
- **Container Prefix**: `infrastructure-*`
- **When to Use**: ONLY local development on laptop
- **DO NOT USE**: On production server

**Port Differences** (WHY we don't use this in production):
```yaml
# Grafana - Development uses 3001 to avoid conflicts
grafana:
  ports:
    - "${GRAFANA_HOST_PORT:-3001}:3000"

# Prometheus - Development uses 9091 to avoid conflicts
prometheus:
  ports:
    - "9091:9090"
```

---

## Standard Port Assignments - PRODUCTION

These ports are FIXED and MUST NOT change between deployments:

| Service | Port | Access | URL |
|---------|------|--------|-----|
| Grafana | 3000 | External | http://SERVER:3000 |
| Prometheus | 9090 | External | http://SERVER:9090 |
| Kafka | 29092 | External | localhost:29092 |
| Kafka (Internal) | 9092 | Internal | kafka:9092 |
| PostgreSQL | 5433 | External | localhost:5433 |
| Redis | 6380 | External | localhost:6380 |
| Kafka-UI | 8084 | External | http://SERVER:8084 |
| Adminer | 8087 | External | http://SERVER:8087 |
| Redis-UI | 8083 | External | http://SERVER:8083 |
| Zookeeper | 2181 | Internal | zookeeper:2181 |

**Application Services** (systemd, not Docker):
| Service | Port | Access |
|---------|------|--------|
| HotPath | 8088 | External |
| Executor | 9904 | External |
| Normalizer | 9901 | External |
| Discovery | 9102 | External |
| Monitoring | 8091 | External |
| Wallet Vault | 8090 | External |
| Wallet Metrics | 9905 | External |
| API Service | 8092 | External |

---

## Pre-Deployment Checklist

Before running deployment:

- [ ] SSH to production server: `ssh root@149.28.56.125`
- [ ] Navigate to directory: `cd /opt/MEVana/infrastructure`
- [ ] Check for orphaned containers: `docker ps -a | grep -E '(mevana|infrastructure)'`
- [ ] If orphans exist, clean up first (see cleanup section)
- [ ] Verify docker-compose.server.yml exists: `ls -la docker-compose.server.yml`
- [ ] Review command syntax one more time
- [ ] Execute with `-p mevana -f docker-compose.server.yml`

---

## Post-Deployment Verification

After deployment, ALWAYS verify:

### 1. Container Names
```bash
docker ps --format '{{.Names}}' | sort
```

**Must see ONLY**:
- mevana-adminer-1
- mevana-grafana-1
- mevana-kafka-1
- mevana-kafka-ui-1
- mevana-postgres-1
- mevana-prometheus-1
- mevana-redis-1
- mevana-redis-ui-1
- mevana-zookeeper-1

**If you see `infrastructure-*`**: ❌ FAILED - Redeploy

### 2. Port Mappings
```bash
docker ps --format 'table {{.Names}}\t{{.Ports}}' | grep grafana
```

**Must show**: `0.0.0.0:3000->3000/tcp`
**NOT**: `0.0.0.0:3001->3000/tcp`

### 3. Service Health
```bash
docker ps --format 'table {{.Names}}\t{{.Status}}'
```

All services must show: `Up X minutes (healthy)`

### 4. Grafana Access
```bash
curl -s http://localhost:3000/api/health
```

Must return: `{"database":"ok","version":"10.4.5"}`

### 5. No Orphaned Containers
```bash
docker ps -a --filter "name=infrastructure-" --format '{{.Names}}'
```

Must return: *empty* (no output)

---

## Cleanup Procedure (When Orphans Exist)

If you deployed with wrong method and have `infrastructure-*` containers:

```bash
# Stop ALL infrastructure containers
docker compose -p mevana -f docker-compose.server.yml down || true
docker compose -f docker-compose.core.yml down || true
docker compose -f docker-compose.yml down || true

# Remove any orphaned containers explicitly
docker ps -a --filter "name=infrastructure-" --format '{{.ID}}' | xargs -r docker rm -f
docker ps -a --filter "name=mevana-" --format '{{.ID}}' | xargs -r docker rm -f

# Verify clean state
docker ps -a --filter "name=infrastructure-"
docker ps -a --filter "name=mevana-"

# Redeploy with correct method
docker compose -p mevana -f docker-compose.server.yml up -d
```

---

## GitHub Actions Deployment Workflow

The deployment workflow at `.github/workflows/self-hosted-deploy.yml` uses the CORRECT method:

**Line 278**:
```yaml
docker compose -p mevana -f infrastructure/docker-compose.server.yml up -d \
  zookeeper kafka redis postgres adminer prometheus grafana kafka-ui
```

**This is the REFERENCE implementation** - manual deployments must match this exactly.

---

## Why This Matters - Real Example

**What Happened Today** (2025-01-09):

1. ❌ Manual fix used: `docker compose -f docker-compose.core.yml up -d`
2. ❌ Created containers: `infrastructure-grafana-1` on port 3001
3. ❌ User tried to access: http://SERVER:3000 (expected production port)
4. ❌ Grafana not accessible (actually on 3001)
5. ❌ Confusion: "why are we doing this over and over? changing ports"
6. ✅ Root cause identified: Wrong compose file used
7. ✅ Fixed: Redeployed with correct method
8. ✅ Grafana now on port 3000 as expected

**Time Wasted**: 30+ minutes debugging
**Solution**: THIS DOCUMENT - follow it ALWAYS

---

## Local Development vs Production

### Local Development (Your Laptop)
```bash
# Use docker-compose.core.yml
cd infrastructure
docker compose -f docker-compose.core.yml up -d

# Containers: infrastructure-*
# Grafana: localhost:3001
# Prometheus: localhost:9091
```

### Production Server (149.28.56.125)
```bash
# Use docker-compose.server.yml with -p mevana
cd /opt/MEVana/infrastructure
docker compose -p mevana -f docker-compose.server.yml up -d

# Containers: mevana-*
# Grafana: localhost:3000
# Prometheus: localhost:9090
```

**NEVER mix these up**

---

## Emergency Override Procedure

If you MUST deviate from this standard:

1. Document WHY in deployment log
2. Get explicit approval from system owner
3. Create GitHub issue documenting the exception
4. Plan to return to standard ASAP
5. Update this document if permanent change needed

**Valid reasons for override**:
- Testing new infrastructure configuration
- Debugging port conflicts
- Disaster recovery scenario

**Invalid reasons**:
- "It's faster this way"
- "I forgot the command"
- "The other file works too"

---

## Enforcement

### For AI Assistants
- ✅ ALWAYS read this document before infrastructure operations
- ✅ ALWAYS use exact commands from this document
- ✅ ALWAYS verify post-deployment
- ❌ NEVER use alternative docker-compose files on production
- ❌ NEVER skip verification steps

### For Human Operators
- Add this checklist to deployment runbook
- Review this document quarterly
- Update when infrastructure changes
- Report violations immediately

---

## Quick Reference Card

**PRODUCTION INFRASTRUCTURE DEPLOYMENT**

```bash
# Deploy
cd /opt/MEVana/infrastructure
docker compose -p mevana -f docker-compose.server.yml up -d

# Verify
docker ps --format '{{.Names}}' | head -1  # Should be "mevana-*"
curl -s http://localhost:3000/api/health   # Grafana on 3000

# Access
Grafana:    http://149.28.56.125:3000
Prometheus: http://149.28.56.125:9090
```

**IF ANYTHING ELSE**: ❌ YOU'RE DOING IT WRONG

---

## Version History

| Date | Change | Author |
|------|--------|--------|
| 2025-01-09 | Initial standard created after port inconsistency incident | Claude Code |

---

## Related Documentation

- `docs/operations/CONTAINER_MANAGEMENT.md` - Container lifecycle and troubleshooting
- `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md` - Complete deployment guide
- `.github/workflows/self-hosted-deploy.yml` - Automated deployment workflow
- `infrastructure/docker-compose.server.yml` - Production compose file
- `infrastructure/docker-compose.core.yml` - Base/development compose file

---

**REMEMBER**: Consistency > Cleverness. Use the standard method EVERY time.
