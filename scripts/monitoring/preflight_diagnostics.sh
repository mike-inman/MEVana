#!/usr/bin/env bash
set -euo pipefail

DB_URL=${DATABASE_URL:-"postgresql://mev:mevpassword@localhost:5433/mevdb"}
PROM_URL=${PROMETHEUS_URL:-"http://localhost:9090"}

psql_cmd() {
  psql "$DB_URL" -v ON_ERROR_STOP=1 -Atc "$1"
}

promql() {
  curl -fsSL "${PROM_URL}/api/v1/query" --get --data-urlencode "query=$1" | jq -r '.data.result[]? | "\(.metric) => \(.value[1])"'
}

echo "== Latest Opportunities (realtime_preflight_metrics) =="
psql_cmd "SELECT timestamp, opportunity_id, status, estimated_profit_sol, total_estimated_cost_lamports/1e9 AS total_cost_sol, error_message FROM realtime_preflight_metrics ORDER BY timestamp DESC LIMIT 5;"

echo "\n== Hourly Summary (last 6 buckets) =="
psql_cmd "SELECT hour_bucket, preflight_passes, preflight_rejections, avg_net_sol FROM preflight_hourly_summary LIMIT 6;"

echo "\n== Top Routes (48h) =="
psql_cmd "SELECT token_pair, route, passes, rejections, avg_net_lamports/1e9 AS avg_net_sol FROM preflight_route_breakdown ORDER BY passes DESC LIMIT 10;"

echo "\n== Prometheus Rates (5m window) =="
promql 'rate(mevana_preflight_simulations_total{result="success"}[5m])'
promql 'rate(mevana_preflight_simulations_total{result="failure"}[5m])'
promql 'histogram_quantile(0.95, rate(mevana_preflight_latency_seconds_bucket[5m]))'

echo "\nDiagnostics complete."
