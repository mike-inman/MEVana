# 🌅 Quick Start Commands for Tomorrow Morning
Last updated: 2025-09-15

## 🎯 GOAL: Get Grafana Dashboards Working with ISP Workaround Data

### Step 1: Clean Up Port Conflicts (2 minutes)
```bash
# Kill all conflicting processes
pkill -f "hotpath_arb_engine"

# Verify cleanup
ps aux | grep cargo | grep hotpath
# Should show no results

# Keep database replay running (process 07a150 should still be active)
ps aux | grep replay_database
```

### Step 2: Start Single Clean Service (1 minute)
```bash
# Start one fake Geyser process on port 8088
FAKE_GEYSER=true PAPER_TRADING=true \
DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
METRICS_PORT=8088 RUST_LOG=info \
cargo run -p hotpath_arb_engine --bin hotpath_arb_engine
```

### Step 3: Verify Metrics Endpoint (30 seconds)
```bash
# Test metrics endpoint
curl http://localhost:8088/metrics | head -10

# Test performance endpoint
curl http://localhost:8088/performance

# Both should return data immediately
```

### Step 4: Check Grafana Dashboard (1 minute)
```bash
# Open Grafana in browser
open http://localhost:3000

# Login: admin/admin123
# Check if dashboards now show data from localhost:8088
```

## 🚨 IF GRAFANA STILL EMPTY:

### Check Prometheus Configuration
```bash
# Verify Prometheus is scraping the correct endpoint
open http://localhost:9090/targets

# Look for localhost:8088/metrics in the targets list
# Should show "UP" status
```

### Check Grafana Data Source
```bash
# In Grafana UI:
# 1. Go to Configuration → Data Sources
# 2. Verify Prometheus URL is http://localhost:9090
# 3. Test connection - should be successful
```

## 📊 EXPECTED SUCCESS INDICATORS

1. ✅ **Metrics endpoint responds**: `curl localhost:8088/metrics` returns Prometheus format data
2. ✅ **Database replay continues**: Process 07a150 shows transaction updates
3. ✅ **Fake Geyser generates opportunities**: Log shows "SYNTH-XXXXXX" arbitrage events
4. ✅ **Grafana displays data**: Charts populate with arbitrage opportunities and system metrics
5. ✅ **No port conflicts**: Only one process using port 8088

## 🔧 TROUBLESHOOTING

### If metrics endpoint fails:
```bash
# Check if port 8088 is already in use
lsof -i :8088

# If occupied by another process, kill it
sudo kill -9 <PID>
```

### If database replay stopped:
```bash
# Restart database replay
./scripts/maintenance/replay_database.sh &
```

### If Grafana shows no data:
```bash
# Check Prometheus scrape status
curl http://localhost:9090/api/v1/targets

# Check if metrics are being collected
curl http://localhost:9090/api/v1/query?query=up
```

## 📝 CURRENT BACKGROUND PROCESSES TO MAINTAIN

- **Database Replay (07a150)**: Keep running - provides realistic transaction patterns
- **New Fake Geyser**: Single process on port 8088 for Grafana metrics

## 🎯 SUCCESS = Pretty Charts with Data

When working correctly, you should see:
- Arbitrage opportunity counts per minute
- Profit distribution charts  
- System performance metrics
- Real-time transaction processing rates

**The ISP workaround technology works - it's just a process management issue preventing Grafana access to the generated metrics.**
