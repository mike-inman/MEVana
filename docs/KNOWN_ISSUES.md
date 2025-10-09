# Known Issues - January 9, 2025

## Active Issues

### 1. Wallet Metrics Not Displaying in Grafana
**Status**: IDENTIFIED - NOT FIXED
**Severity**: Medium
**Impact**: Cannot view wallet balance in Grafana dashboard

**Symptoms**:
- Wallet-metrics-exporter service running correctly (port 9905)
- Metrics endpoint responding with data
- Prometheus configured to scrape (in prometheus.prod.yml)
- Query returns empty: `mevana_wallet_balance_sol` → no results
- Dashboard loaded but panels show "No data"

**Root Cause**: Prometheus container NOT using prometheus.prod.yml
- Config file exists in source: `infrastructure/prometheus.prod.yml`
- Config includes wallet-metrics scrape job (lines 70-77)
- Container may be using different config file or missing volume mount

**Evidence**:
```bash
# Service is working
systemctl status wallet-metrics-exporter  # ✅ Active
curl http://localhost:9905/metrics        # ✅ Returns data

# Prometheus not scraping
curl 'http://localhost:9090/api/v1/query?query=mevana_wallet_balance_sol'
# Returns: {"status":"success","data":{"resultType":"vector","result":[]}}
```

**Next Steps to Fix**:
1. Verify Prometheus volume mount in docker-compose.server.yml
2. Confirm prometheus.prod.yml is mounted at /etc/prometheus/prometheus.yml
3. Check Prometheus targets API to see if wallet-metrics job exists
4. Restart Prometheus container after verifying config
5. Test query again

**Workaround**: Direct access to metrics endpoint works:
```bash
curl http://149.28.56.125:9905/metrics
```

---

### 2. Executor Kafka Consumer Status Unknown
**Status**: NEEDS INVESTIGATION
**Severity**: High
**Impact**: Cannot test preflight simulation (blocks main test objective)

**Symptoms**:
- Executor service running
- Kafka configuration correct (KAFKA_BROKER=localhost:29092)
- KAFKA_ENABLE_CONSUMER=true set
- No Kafka consumption logs visible
- No evidence of opportunities being consumed

**Root Cause**: Unknown - possible causes:
1. Binary built without Kafka consumer feature
2. Consumer code not logging initialization
3. Additional config flag needed
4. Environment variable name mismatch

**Next Steps**:
1. Review executor source code for Kafka consumer implementation
2. Check binary build flags/features
3. Add debug logging to consumer initialization
4. Test locally with verbose logging
5. Rebuild if necessary

**Blocked Tasks**:
- Preflight simulation testing
- STOP_AFTER_PREFLIGHT verification
- Complete end-to-end flow validation

---

## Resolved Issues (This Session)

### ✅ Container Naming Inconsistency
**Fixed**: 2025-01-09 05:42 UTC
**Solution**: Created mandatory deployment standard, redeployed with correct method
**Documentation**: `docs/operations/DEPLOYMENT_STANDARD.md`

### ✅ Kafka Cluster ID Mismatch
**Fixed**: 2025-01-09 05:15 UTC
**Solution**: Clean shutdown/restart of infrastructure with persistent volumes
**Documentation**: `docs/operations/CONTAINER_MANAGEMENT.md`

### ✅ Services Using Wrong Kafka Port
**Fixed**: 2025-01-09 05:28 UTC
**Solution**: Added KAFKA_BOOTSTRAP_SERVERS=localhost:29092 to common.env
**Impact**: Discovery and normalizer services now connect successfully

### ✅ Kafka Publishing Pipeline Not Working
**Fixed**: 2025-01-08 (previous session)
**Solution**: Modified EVENT_BUS to read environment variables
**Documentation**: `docs/CHANGES_2025-01-08.md`
**Verification**: Message retrieved from Kafka topic ✅

---

## Non-Issues (Working Correctly)

### ✅ HotPath Arbitrage Detection
- 5,333+ opportunities detected from real blockchain data
- SmartFilter validation working (90% pass rate)
- Opportunities published to Kafka successfully
- Safety gates functioning correctly

### ✅ Infrastructure Services
- All Docker containers healthy
- Kafka, Zookeeper, PostgreSQL, Redis operational
- No restart loops or failures

### ✅ Wallet Metrics Exporter Service
- Service running for 2+ hours
- Correctly tracking 0.1999 SOL balance
- Metrics endpoint responding on port 9905
- Only issue: Prometheus not scraping (see Issue #1)

---

## Deferred Issues (Low Priority)

### Jupiter Price API Failures
**Impact**: Low - balance tracking works without USD pricing
**Status**: Monitoring
**Details**: wallet-metrics-exporter shows "Failed to fetch prices from Jupiter"
- Network connectivity issue or rate limiting
- Does not affect SOL/token balance tracking
- USD value shows $0.00 but balances are correct

---

## Issue Tracking

For new issues discovered:
1. Add to this document with status "IDENTIFIED"
2. Include symptoms, root cause analysis, next steps
3. Update when resolved with solution and documentation links
4. Move to "Resolved Issues" section when fixed

**Document Location**: `docs/KNOWN_ISSUES.md`
**Last Updated**: 2025-01-09 05:50 UTC
