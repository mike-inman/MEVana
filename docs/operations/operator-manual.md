# MEVana Operator Manual
Last updated: 2025-09-15

**Document Version:** 2.0  
**Last Updated:** September 12, 2025  
**Target Audience:** System operators, trading desk personnel  
**System Status:** 75% ready - Controlled production deployment possible

---

## 🎯 Quick Reference

### **Emergency Contacts & Procedures**
- **Emergency Shutdown**: `pkill -f hotpath_arb_engine && docker-compose down`
- **System Health Check**: `curl http://localhost:8088/performance`
- **Database Check**: `docker exec postgres psql -U mev -d mevdb -c "SELECT COUNT(*) FROM arbitrage_opportunities WHERE timestamp > NOW() - INTERVAL '1 hour';"`

### **Key URLs**
- **Main Dashboard**: http://localhost:8088/performance
- **Grafana Monitoring**: http://localhost:3000 (admin/admin123)
- **Kafka UI**: http://localhost:8080
- **Database Admin**: http://localhost:8082

---

## 📊 Daily Operations Checklist

### **Morning Startup Checklist (Required)**
1. **Check Infrastructure Health**
   ```bash
   docker-compose ps  # All should show "healthy"
   ```

2. **Verify Database Connection**
   ```bash
   docker exec postgres psql -U mev -d mevdb -c "SELECT 1;"
   ```

3. **Start Monitoring Service**
   ```bash
   DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
   MONITORING_PORT=8091 \
   RUST_LOG=info \
   ./target/release/monitoring-service &
   ```

4. **Start HotPath Engine**
   ```bash
   DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
   RUST_LOG=info \
   GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
   X_TOKEN="your_shyft_token" \
   ORDERBOOK_HISTORY_ENABLED=true \
   PAPER_TRADING=true \
   METRICS_PORT=8088 \
   ./target/release/hotpath_arb_engine
   ```

5. **Verify System Health** (Wait 5 minutes after startup)
   ```bash
   curl http://localhost:8088/performance  # Should show active processing
   ```

### **Hourly Health Checks**
- **Opportunity Detection**: Should see 20-50 opportunities/hour during market hours
- **Processing Latency**: Should be <1500μs average
- **Memory Usage**: Should be <2GB total system
- **Error Rate**: Should be <1%

### **End of Day Checklist**
1. **Review Performance Metrics**
   - Check Grafana dashboards for any anomalies
   - Review total opportunities detected
   - Check for any error spikes

2. **Database Backup**
   ```bash
   ./scripts/daily_backup.sh
   ```

3. **Log Review**
   - Check application logs for errors
   - Review any alerts generated

---

## 🎛️ System Controls & Configuration

### **Smart Filter Configuration**
The system uses real-time database-driven configuration. Changes take effect within 50ms.

#### **View Current Configuration**
```bash
curl http://localhost:8088/api/v1/smart-filter/config
```

#### **Update Configuration Parameters**
```bash
# Increase minimum profit threshold
curl -X PUT http://localhost:8088/api/v1/smart-filter/config \
  -H "Content-Type: application/json" \
  -d '{
    "parameter": "min_profit_bps", 
    "value": 8,
    "applied_by": "operator"
  }'

# View configuration impact
curl http://localhost:8088/api/v1/smart-filter/performance
```

#### **Key Parameters for Operators**
| Parameter | Description | Safe Range | Notes |
|-----------|-------------|------------|-------|
| `min_profit_bps` | Minimum profit in basis points | 3-20 | Higher = fewer opportunities |
| `max_profit_bps` | Maximum profit (avoid "too good to be true") | 50-200 | Filter suspiciously high |
| `min_net_profit_sol` | Minimum net profit in SOL | 0.0005-0.01 | After all costs |
| `max_gas_budget_pct` | Maximum gas as % of profit | 10-50 | Lower = more selective |

### **System Performance Monitoring**

#### **Real-Time Performance Dashboard**
Access: http://localhost:8088/performance

**Key Metrics to Monitor:**
- **Opportunities/Minute**: Should be 15-50 during active market hours
- **Processing Latency**: Target <1000μs, alert >2000μs
- **Success Rate**: Detection accuracy >15%
- **Memory Usage**: Should be stable, no continuous growth

#### **Grafana Dashboards**
Access: http://localhost:3000 (admin/admin123)

**Main Dashboards:**
1. **System Overview** - Infrastructure health, service uptime
2. **Trading Performance** - Opportunity detection, processing rates
3. **Database Performance** - Query times, connection health
4. **Business Intelligence** - Profit tracking, volume analysis

---

## ⚠️ Alert Conditions & Response

### **Critical Alerts (Immediate Action Required)**

#### **🚨 Service Down**
**Symptoms**: No response from http://localhost:8088/performance
**Response**:
```bash
# Check process status
ps aux | grep hotpath_arb_engine

# Check logs
tail -f logs/hotpath_arb_engine.log

# Restart if needed
./scripts/restart_hotpath.sh
```

#### **🚨 No Opportunities Detected (>10 minutes)**
**Symptoms**: Zero opportunities showing in dashboard
**Response**:
```bash
# Check Geyser connection
curl -H "x-token: your_token" https://grpc.ny.shyft.to

# Check smart filter settings
curl http://localhost:8088/api/v1/smart-filter/config

# Check database connectivity
docker exec postgres psql -U mev -d mevdb -c "SELECT 1;"
```

#### **🚨 High Processing Latency (>3000μs sustained)**
**Symptoms**: Latency metrics showing high values
**Response**:
```bash
# Check system resources
htop

# Check database performance
docker exec postgres psql -U mev -d mevdb -c "SELECT COUNT(*) FROM arbitrage_opportunities;"

# Consider reducing parameters if database is overloaded
curl -X PUT http://localhost:8088/api/v1/smart-filter/config \
  -H "Content-Type: application/json" \
  -d '{"parameter": "min_profit_bps", "value": 10, "applied_by": "latency_response"}'
```

### **Warning Alerts (Monitor Closely)**

#### **⚠️ Low Success Rate (<10%)**
**Symptoms**: Detection accuracy dropping
**Possible Causes**: Market conditions changed, configuration too restrictive
**Response**: Review and potentially adjust smart filter parameters

#### **⚠️ Memory Usage High (>1.5GB)**
**Symptoms**: Increasing memory usage
**Response**: Monitor for memory leaks, restart if necessary

#### **⚠️ Database Connection Issues**
**Symptoms**: Intermittent database errors in logs
**Response**: Check PostgreSQL health, verify connection pool settings

---

## 🛠️ Common Maintenance Tasks

### **Parameter Optimization**

#### **Market Condition Adjustments**
```bash
# High volatility periods - be more selective
curl -X PUT http://localhost:8088/api/v1/smart-filter/config \
  -H "Content-Type: application/json" \
  -d '{"parameter": "min_profit_bps", "value": 12, "applied_by": "volatility_adjustment"}'

# Low activity periods - be less selective
curl -X PUT http://localhost:8088/api/v1/smart-filter/config \
  -H "Content-Type: application/json" \
  -d '{"parameter": "min_profit_bps", "value": 5, "applied_by": "low_activity"}'
```

#### **Performance-Based Adjustments**
```bash
# If processing is slow, reduce load
curl -X PUT http://localhost:8088/api/v1/smart-filter/config \
  -H "Content-Type: application/json" \
  -d '{"parameter": "max_concurrent_opportunities", "value": 10, "applied_by": "performance_tuning"}'
```

### **Database Maintenance**

#### **Daily Database Statistics**
```bash
# Check table sizes
docker exec postgres psql -U mev -d mevdb -c "
SELECT 
  schemaname,
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
FROM pg_tables 
WHERE schemaname = 'public' 
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;"

# Check recent opportunity count
docker exec postgres psql -U mev -d mevdb -c "
SELECT 
  DATE(timestamp) as date,
  COUNT(*) as opportunities 
FROM arbitrage_opportunities 
WHERE timestamp > NOW() - INTERVAL '7 days'
GROUP BY DATE(timestamp) 
ORDER BY date DESC;"
```

#### **Weekly Database Cleanup**
```bash
# Archive old opportunities (older than 30 days)
docker exec postgres psql -U mev -d mevdb -c "
DELETE FROM arbitrage_opportunities 
WHERE timestamp < NOW() - INTERVAL '30 days';"

# Update database statistics
docker exec postgres psql -U mev -d mevdb -c "ANALYZE;"
```

### **Log Management**

#### **Log Rotation**
```bash
# Manual log rotation if needed
mv logs/hotpath_arb_engine.log logs/hotpath_arb_engine.log.$(date +%Y%m%d)
touch logs/hotpath_arb_engine.log
pkill -HUP hotpath_arb_engine  # Reopen log files
```

#### **Log Analysis**
```bash
# Check error frequency
grep -c "ERROR" logs/hotpath_arb_engine.log

# Recent performance issues
tail -100 logs/hotpath_arb_engine.log | grep "latency\|slow\|timeout"

# Opportunity detection rate
grep "opportunities detected" logs/hotpath_arb_engine.log | tail -10
```

---

## 📈 Performance Optimization Guide

### **Optimal Configuration for Different Market Conditions**

#### **High Volatility Markets**
```json
{
  "min_profit_bps": 8,
  "max_profit_bps": 150,
  "min_net_profit_sol": 0.002,
  "max_gas_budget_pct": 20.0
}
```

#### **Low Volatility Markets** 
```json
{
  "min_profit_bps": 3,
  "max_profit_bps": 50,
  "min_net_profit_sol": 0.0005,
  "max_gas_budget_pct": 40.0
}
```

#### **High Activity Periods**
```json
{
  "max_concurrent_opportunities": 10,
  "competition_lookback_ms": 500
}
```

### **Performance Tuning Based on System Resources**

#### **High Memory Usage**
- Reduce `max_concurrent_opportunities` to 5-10
- Increase `min_profit_bps` to reduce processing load

#### **High CPU Usage**
- Increase `competition_lookback_ms` to reduce frequency
- Consider reducing `learning_rate` to reduce AI processing

#### **Database Performance Issues**
- Increase `min_profit_bps` to reduce database writes
- Consider archiving old opportunities more frequently

---

## 🚨 Emergency Procedures

### **Complete System Shutdown**
```bash
# Emergency shutdown sequence
echo "🚨 EMERGENCY SHUTDOWN INITIATED"

# 1. Stop trading engine immediately
pkill -f hotpath_arb_engine
pkill -f monitoring-service

# 2. Stop Docker infrastructure
docker-compose down

# 3. Create emergency backup
./scripts/emergency_backup.sh

echo "✅ Emergency shutdown complete"
```

### **System Recovery**
```bash
# Recovery sequence
echo "🔄 SYSTEM RECOVERY INITIATED"

# 1. Start infrastructure
docker-compose up -d

# 2. Wait for healthy status
sleep 60

# 3. Verify infrastructure
docker-compose ps

# 4. Start services with conservative settings
DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
MONITORING_PORT=8091 \
RUST_LOG=info \
./target/release/monitoring-service &

sleep 10

DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
RUST_LOG=info \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="your_shyft_token" \
ORDERBOOK_HISTORY_ENABLED=true \
PAPER_TRADING=true \
METRICS_PORT=8088 \
./target/release/hotpath_arb_engine &

# 5. Verify recovery
sleep 30
curl http://localhost:8088/performance

echo "✅ System recovery complete"
```

### **Data Recovery**
```bash
# Restore from backup if needed
gunzip -c backup_$(date +%Y%m%d).sql.gz | \
docker exec -i postgres psql -U mev -d mevdb
```

---

## 📞 Support & Escalation

### **Level 1: Operator Response**
- System monitoring alerts
- Parameter adjustments
- Service restarts
- Basic troubleshooting

### **Level 2: Technical Support**
- Database issues
- Configuration problems
- Performance optimization
- Log analysis

### **Level 3: Development Team** 
- Code issues
- Architecture changes
- New feature requests
- Complex system problems

### **Contact Information**
- **Emergency**: [Emergency contact procedures]
- **Technical Support**: [Support contact information]
- **Documentation**: See /docs-new/ directory for complete documentation

---

## 🔍 Troubleshooting Quick Reference

| Issue | Quick Check | Resolution |
|-------|-------------|------------|
| No opportunities | `curl localhost:8088/performance` | Check Geyser connection, smart filter config |
| High latency | `htop`, check database | Reduce concurrent opportunities |
| Service won't start | Check ports with `netstat -tlnp` | Kill conflicting processes |
| Database errors | `docker exec postgres psql -U mev -d mevdb -c "SELECT 1;"` | Restart PostgreSQL container |
| Memory issues | `free -h`, `ps aux --sort=-%mem` | Restart services, check for leaks |
| Config not updating | Check logs, verify database connection | Restart services |

---

*This operator manual reflects the current verified state of the MEVana system as of September 12, 2025. All procedures have been validated against the actual system.*
