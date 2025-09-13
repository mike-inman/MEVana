# MEVana Startup Troubleshooting Guide

**Last Updated:** 2025-09-09  
**Purpose:** Comprehensive troubleshooting guide for MEVana system startup

## Quick Reference - Binary Specifications Required

Many MEVana crates contain multiple binaries. **Always specify the correct binary** when running services:

```bash
# ✅ CORRECT - Binary specifications
cargo run -p normalizer-service --bin normalizer-service -- pipe
cargo run -p edge-assembler --bin edge-assembler  
cargo run -p arb-engine --bin arb-engine
cargo run -p executor-service --bin executor-service

# ❌ INCORRECT - Will fail with "could not determine which binary to run"
cargo run -p normalizer-service  # Missing --bin flag
cargo run -p executor-service    # Missing --bin flag
```

## Environment Variables - Critical Details

### Port Configuration Issues

**Problem:** arb-engine ignores `ARB_HTTP_PORT` variable  
**Solution:** Use `WS_BIND` environment variable instead

```bash
# ❌ WRONG - Will default to port 8080 (conflicts with Kafka-UI)
ARB_HTTP_PORT=8085 cargo run -p arb-engine --bin arb-engine

# ✅ CORRECT - Uses port 8085 as specified  
WS_BIND=0.0.0.0:8085 cargo run -p arb-engine --bin arb-engine
```

### Service-Specific Environment Variables

| Service | Port Variable | Default Port | Notes |
|---------|---------------|--------------|-------|
| arb-engine | `WS_BIND` | 8080 | **Use 8085 to avoid Kafka-UI conflict** |
| normalizer-service | `METRICS_PORT` | - | Use 9901 |
| edge-assembler | `METRICS_PORT` | - | Use 9902 |  
| executor-service | `METRICS_PORT` | - | Use 9904 |
| hotpath_arb_engine | `METRICS_PORT` | - | Use 8088 |
| wallet-vault | Built-in | 8090 | No configuration needed |

## Health Check Patterns

**Many services don't expose `/health` endpoints.** Use fallback checks:

```bash
# Method 1: Port check (for services with HTTP servers)
lsof -i :8085 2>/dev/null | grep -q LISTEN && echo "Running" || echo "Not running"

# Method 2: Process check (for services without HTTP servers)  
pgrep -f "normalizer-service.*pipe" > /dev/null && echo "Running" || echo "Not running"

# Method 3: Combined check
if lsof -i :9902 2>/dev/null | grep -q LISTEN; then
    echo "edge-assembler: ✅ Running"
elif pgrep -f "edge-assembler" > /dev/null; then  
    echo "edge-assembler: ✅ Running (no HTTP endpoint)"
else
    echo "edge-assembler: ❌ Not running"
fi
```

## Common Startup Issues & Solutions

### 1. Port Conflicts

**Symptom:** `Address already in use (os error 48)`  
**Solutions:**
- Check PORT_MAPPING.md for correct port assignments
- Use `lsof -i :PORT` to identify conflicts  
- Kill conflicting processes: `pkill -f "service-name"`

### 2. Binary Selection Errors

**Symptom:** `cargo run` could not determine which binary to run  
**Solution:** Add `--bin binary-name` to all cargo run commands

### 3. Service Dependencies

**Required startup order:**
1. Infrastructure (Docker): PostgreSQL, Redis, Kafka
2. Core Services: wallet-vault, normalizer-service, edge-assembler, arb-engine, executor-service  
3. Supporting Services: token-registry-service, discovery-service
4. HotPath Services: hotpath_arb_engine

### 4. Database Connection Issues

**Ensure PostgreSQL URL matches:**
```bash
DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb"
```

**Note:** Port 5433 (external) maps to 5432 (internal Docker port)

## Idle Service Behavior (Normal)

**This is expected behavior:**
```
kafka_idle_no_messages_yet interval_secs=5
```

Services will show idle warnings when no DEX transactions are being streamed. This is normal during startup and testing phases.

## Service Status Verification

Use this comprehensive status check:

```bash
#!/bin/bash
echo "=== MEVana System Status ==="
echo ""
echo "Infrastructure Services:"
echo "  PostgreSQL: $(docker ps | grep -q postgres && echo '✅ Running' || echo '❌ Not running')"
echo "  Redis: $(docker ps | grep -q redis && echo '✅ Running' || echo '❌ Not running')"  
echo "  Kafka: $(docker ps | grep -q kafka && echo '✅ Running' || echo '❌ Not running')"
echo ""
echo "Core Services:"
echo "  wallet-vault (8090): $(lsof -i :8090 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"
echo "  normalizer-service: $(pgrep -f 'normalizer-service.*pipe' >/dev/null && echo '✅ Running' || echo '❌ Not running')"
echo "  edge-assembler: $(pgrep -f 'edge-assembler' >/dev/null && echo '✅ Running' || echo '❌ Not running')"  
echo "  arb-engine (8085): $(lsof -i :8085 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"
echo "  executor-service: $(lsof -i :9904 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '🔄 Starting')"
```

## Log File Locations

All service logs are stored in `logs/` directory:
- `logs/normalizer-service.log`
- `logs/edge-assembler.log` 
- `logs/arb-engine.log`
- `logs/executor-service.log`
- `logs/wallet-vault.log`
- `logs/hotpath_arb_engine.log`

## Key Reference Documents

- **PORT_MAPPING.md** - Master port allocation registry
- **database/complete_schema.sql** - Master database schema  
- **scripts/start_system.sh** - Automated startup script
- **CLAUDE.md** - Development guidance and architecture

## Emergency Recovery Commands

```bash
# Stop all MEVana services
pkill -f "arb-engine|normalizer-service|edge-assembler|executor-service|wallet-vault"

# Restart infrastructure
cd infrastructure && docker-compose restart

# Check all ports in use
lsof -i :8080,8085,8090,9901,9902,9904,8088

# View recent logs  
tail -f logs/*.log
```

## Production Deployment Notes

1. **Always use the startup script:** `./scripts/start_system.sh`
2. **Verify PORT_MAPPING.md before deployment**  
3. **Test health checks for all services**
4. **Monitor logs during initial startup phase**
5. **Expect 2-5 minute compilation time for executor-service**

---

*This guide captures all troubleshooting knowledge from system startup sessions to ensure smooth production deployments.*