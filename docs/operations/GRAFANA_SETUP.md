# Manual Steps to Get Solana Data in Grafana

## Current Status ✅
- **Laserstream service**: Running and generating metrics ✅
- **Prometheus**: Scraping successfully (target "UP") ✅
- **Data flowing**: 658+ transactions recorded ✅

## Issue 🔧
Grafana needs to be configured to connect to Prometheus and import the dashboard.

## Manual Setup Steps:

### 1. Configure Prometheus Data Source in Grafana
1. Open http://localhost:3000
2. Login with admin/admin
3. Go to **Configuration** → **Data Sources**
4. Click **Add data source**
5. Select **Prometheus**
6. Set URL to: `http://prometheus:9090`
7. Click **Save & Test** (should show "Data source is working")

### 2. Import the Solana Dashboard
1. Go to **Dashboards** → **Import**
2. Click **Upload JSON file**
3. Select: `/Users/michaelinman/Repos/MEVana/infrastructure/grafana/dashboards/solana-laserstream-dashboard.json`
4. In the import screen:
   - Set **Prometheus** data source to the one you just created
   - Click **Import**

### 3. View Live Data
The dashboard should immediately show:
- **Transactions per second** by program (Jupiter, Orca, Raydium)
- **Block height** updates
- **Success/error rates**
- **Latency metrics** (p50/p90/p99)
- **Transaction distribution** pie chart

## Test Queries (if needed):
If panels are empty, try these queries directly in Grafana Explore:

```promql
# Transaction rate by program
sum by (program) (rate(mevana_tx_total[1m]))

# Current block height
solana_block_height

# Success vs error rates
sum(rate(mevana_tx_total{status="ok"}[1m]))
sum(rate(mevana_tx_total{status="err"}[1m]))

# Latency percentiles
histogram_quantile(0.90, sum by (le) (rate(mevana_e2e_latency_ms_bucket[2m])))
```

For the new preflight decision table, verify Postgres has data:

```sql
SELECT status,
       estimated_profit_sol,
       total_estimated_cost_lamports / 1e9 AS total_cost_sol,
       error_message
FROM realtime_preflight_metrics
ORDER BY timestamp DESC
LIMIT 5;
```

## Current Live Data:
```bash
# Check metrics are flowing:
curl -s "http://localhost:9090/api/v1/query?query=mevana_tx_total"

# Current values as of now:
# - Jupiter transactions: 658+
# - Block height: 280000308+
# - Service uptime: 5+ minutes
```

## Preflight Diagnostics Row
- Panel 17: Hourly Preflight Summary (uses `preflight_hourly_summary`).
- Panel 18: Route Performance (48h window via `preflight_route_breakdown`).
- Panel 19: Avg Net Profit (SOL) by Hour trendline.
If panels are empty, ensure the reporting views exist (`psql -f database/migrations/create_preflight_summary_views.sql`).
