# MEVana Production Shutdown Procedures

**Version:** 1.0  
**Date:** 2025-09-09  
**Status:** 🔴 CRITICAL OPERATIONS DOCUMENT

## 🚨 Executive Summary

This document outlines the **safe and controlled shutdown procedures** for MEVana's trading infrastructure. **Improper shutdown can result in financial loss, data corruption, or system damage.**

### ⚠️ Critical Warnings
- **NEVER** force-kill trading services during active operations
- **ALWAYS** verify no pending trades before shutdown
- **BACKUP** critical state before major shutdowns
- **COORDINATE** with trading team before production shutdowns

---

## 🎯 Shutdown Sequence Overview

### Tiered Shutdown Approach
```
Tier 1: Client-Facing Services     (Stop new external connections)
├── hotpath_arb_engine             (External MEV detection)
└── monitoring-service             (External APIs/dashboards)

Tier 2: Core Trading Services      (Complete active trades)
├── arb-engine                     (Main arbitrage engine)
└── executor-service               (Trade execution)

Tier 3: Data Processing           (Process final messages)
├── edge-assembler                (Real-time processing)
└── normalizer-service            (Transaction parsing)

Tier 4: Security Services        (Last to shutdown)
└── wallet-vault                  (Wallet encryption/management)

Infrastructure: Docker Services   (Optional - can stay running)
├── PostgreSQL, Redis, Kafka      (Can persist across restarts)
└── Monitoring stack              (Grafana, Prometheus, etc.)
```

---

## 🛑 Pre-Shutdown Safety Checks

### 1. Trading Safety Assessment
```bash
# Check active arbitrage opportunities
curl -s http://localhost:8085/ai/config | jq '.active_strategies'

# Check pending transactions in wallet vault
curl -s http://localhost:8090/health | jq '.pending_operations'

# Check HotPath engine status
curl -s http://localhost:8088/performance | jq '.active_opportunities'
```

### 2. System Health Verification
```bash
# Verify all services are responsive
./scripts/health_check.sh

# Check for any error states
grep -i "error\|fatal\|panic" logs/*.log | tail -20

# Verify database connectivity
psql "postgresql://mev:mevpassword@localhost:5433/mevdb" -c "SELECT COUNT(*) FROM wallets;"
```

### 3. Data Backup Preparation
```bash
# Create safety backup
mkdir -p backups/shutdown_$(date +%Y%m%d_%H%M%S)

# Export current system metrics
curl -s http://localhost:8088/performance > backups/shutdown_$(date +%Y%m%d_%H%M%S)/hotpath_metrics.json
curl -s http://localhost:8085/ai/config > backups/shutdown_$(date +%Y%m%d_%H%M%S)/arb_config.json

# Database backup (if needed)
pg_dump "postgresql://mev:mevpassword@localhost:5433/mevdb" > backups/shutdown_$(date +%Y%m%d_%H%M%S)/database_backup.sql
```

---

## 🔧 Shutdown Execution

### Quick Shutdown (Development)
```bash
# Automated shutdown for development environments
./scripts/shutdown_system.sh
```

### Controlled Production Shutdown

#### Phase 1: Preparation (5-10 minutes)
1. **Notify stakeholders** of planned shutdown
2. **Stop accepting new trading opportunities** (if possible)
3. **Wait for current trades to complete**
4. **Backup critical state**

#### Phase 2: Client-Facing Shutdown (2-3 minutes)
```bash
# Stop HotPath Engine (external MEV detection)
log_info "Stopping external-facing MEV engine..."
pkill -TERM -f hotpath_arb_engine
# Wait up to 45 seconds for graceful shutdown
wait_for_shutdown "hotpath_arb_engine" 45

# Stop Monitoring APIs
log_info "Stopping monitoring APIs..."
pkill -TERM -f monitoring-service
wait_for_shutdown "monitoring-service" 30
```

#### Phase 3: Core Trading Shutdown (3-5 minutes)
```bash
# Stop Arbitrage Engine (allow current trades to finish)
log_info "Stopping arbitrage engine (finishing active trades)..."
pkill -TERM -f "arb-engine"
wait_for_shutdown "arb-engine" 60  # Extended timeout for trade completion

# Stop Executor Service  
log_info "Stopping trade executor..."
pkill -TERM -f executor-service
wait_for_shutdown "executor-service" 45
```

#### Phase 4: Data Processing Shutdown (2-3 minutes)
```bash
# Stop Edge Assembler
log_info "Stopping edge assembler..."
pkill -TERM -f edge-assembler
wait_for_shutdown "edge-assembler" 30

# Stop Normalizer Service
log_info "Stopping transaction normalizer..."
pkill -TERM -f normalizer-service
wait_for_shutdown "normalizer-service" 30
```

#### Phase 5: Security Shutdown (1-2 minutes)
```bash
# Stop Wallet Vault LAST (other services may need it)
log_info "Stopping wallet vault (final security shutdown)..."
pkill -TERM -f wallet-vault
wait_for_shutdown "wallet-vault" 30
```

---

## ⚙️ Production Considerations & Precautions

### 1. **Financial Risk Management**
- **Active Position Check**: Never shutdown during active arbitrage execution
- **Pending Transaction Verification**: Ensure no pending blockchain transactions
- **Wallet Lock Status**: Verify wallets are in safe state before shutdown
- **Flash Loan Monitoring**: Ensure no active flash loans (critical!)

### 2. **Data Integrity Safeguards**
- **State Persistence**: Ensure adaptive sizing state is saved to disk
- **Kafka Message Completion**: Allow message processing to complete
- **Database Transaction Completion**: Wait for DB writes to finish
- **Redis Cache Consistency**: Flush critical cached data if needed

### 3. **Service Dependencies**
```mermaid
graph TD
    A[HotPath Engine] --> B[Arbitrage Engine]
    B --> C[Executor Service] 
    C --> D[Wallet Vault]
    B --> E[Edge Assembler]
    E --> F[Normalizer Service]
    F --> G[Kafka]
    D --> H[PostgreSQL]
    
    style A fill:#ff6b6b
    style D fill:#51cf66
    style G fill:#339af0
    style H fill:#339af0
```

### 4. **Monitoring During Shutdown**
```bash
# Watch for graceful shutdown signals
tail -f logs/*.log | grep -E "(shutdown|SIGTERM|graceful)"

# Monitor process termination
watch "ps aux | grep -E '(arb-engine|executor|normalizer|edge|hotpath|wallet-vault)'"

# Check port availability after shutdown
watch "lsof -i :8085 -i :8088 -i :8090 -i :9901 -i :9902 -i :9904"
```

---

## 🚨 Emergency Shutdown Procedures

### Immediate Emergency Stop
```bash
# EMERGENCY ONLY - May cause data loss
pkill -KILL -f "arb-engine|executor-service|hotpath_arb_engine|normalizer-service|edge-assembler|wallet-vault|monitoring-service"

# Verify all stopped
ps aux | grep -E "(arb-engine|executor|normalizer|edge|hotpath|wallet-vault)" | grep -v grep
```

### Emergency Recovery Checklist
- [ ] Check database for corrupted transactions
- [ ] Verify wallet balances against backups
- [ ] Restart with recovery mode if available
- [ ] Review logs for forced termination impacts
- [ ] Validate system state before resuming trading

---

## 🔄 Post-Shutdown Verification

### Service Status Check
```bash
# Verify all services stopped
echo "Service Status Check:"
echo "  arb-engine: $(pgrep -f 'arb-engine' >/dev/null && echo '⚠️ Still Running' || echo '✅ Stopped')"
echo "  executor-service: $(pgrep -f executor-service >/dev/null && echo '⚠️ Still Running' || echo '✅ Stopped')"
echo "  hotpath_arb_engine: $(pgrep -f hotpath_arb_engine >/dev/null && echo '⚠️ Still Running' || echo '✅ Stopped')"
echo "  normalizer-service: $(pgrep -f normalizer-service >/dev/null && echo '⚠️ Still Running' || echo '✅ Stopped')"
echo "  edge-assembler: $(pgrep -f edge-assembler >/dev/null && echo '⚠️ Still Running' || echo '✅ Stopped')"
echo "  wallet-vault: $(pgrep -f wallet-vault >/dev/null && echo '⚠️ Still Running' || echo '✅ Stopped')"
echo "  monitoring-service: $(pgrep -f monitoring-service >/dev/null && echo '⚠️ Still Running' || echo '✅ Stopped')"
```

### Port Availability Check
```bash
# Verify ports are released
netstat -an | grep -E ":808[58]|:809[04]|:990[1-4]" | grep LISTEN || echo "✅ All trading ports released"
```

### Infrastructure Status  
```bash
# Check infrastructure services (should remain running)
echo "Infrastructure Status:"
echo "  PostgreSQL: $(docker ps | grep -q postgres && echo '✅ Running' || echo '⚠️ Stopped')"
echo "  Redis: $(docker ps | grep -q redis && echo '✅ Running' || echo '⚠️ Stopped')"  
echo "  Kafka: $(docker ps | grep -q kafka && echo '✅ Running' || echo '⚠️ Stopped')"
```

---

## 🔄 Restart Procedures

### Standard Restart
```bash
# After graceful shutdown, restart with:
./scripts/start_system.sh
```

### Recovery Mode Restart (After Issues)
```bash
# Start with additional safety checks
RECOVERY_MODE=true ./scripts/start_system.sh
```

---

## 📝 Production Shutdown Checklist

### Pre-Shutdown (15-30 minutes)
- [ ] **Stakeholder notification** sent
- [ ] **Trading team approval** obtained  
- [ ] **Active positions verified** as closed/safe
- [ ] **System health check** completed
- [ ] **Backup procedures** executed
- [ ] **Emergency contact list** ready

### During Shutdown (10-15 minutes)
- [ ] **Follow tiered shutdown** sequence exactly
- [ ] **Monitor logs** for graceful shutdown confirmations
- [ ] **Verify each service** stops cleanly
- [ ] **Check for error messages** during shutdown
- [ ] **Document any issues** encountered

### Post-Shutdown (5-10 minutes)
- [ ] **All services confirmed** stopped
- [ ] **Ports released** verification
- [ ] **Infrastructure status** checked
- [ ] **Logs reviewed** for shutdown completion
- [ ] **Backup integrity** verified
- [ ] **Restart readiness** confirmed

---

## 📚 Additional Resources

- **Startup Procedures**: `scripts/start_system.sh`
- **Health Monitoring**: `scripts/health_check.sh` 
- **Port Configuration**: `PORT_MAPPING.md`
- **Emergency Contacts**: `EMERGENCY_CONTACTS.md`
- **System Architecture**: `CLAUDE.md`

---

## ⚠️ Important Notes

1. **Production shutdowns** should always be coordinated with the trading team
2. **Emergency shutdowns** may result in financial losses - use only when absolutely necessary
3. **Infrastructure services** (PostgreSQL, Redis, Kafka) can typically remain running across application restarts
4. **Always test shutdown procedures** in staging environments before production use
5. **Document any deviations** from standard procedures for post-incident analysis

---

*This document should be reviewed and updated whenever shutdown procedures change*