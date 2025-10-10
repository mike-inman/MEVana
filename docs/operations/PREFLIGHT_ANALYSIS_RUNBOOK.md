# Preflight Analysis Runbook

Last updated: 2025-10-09

## Objective
Confirm the arbitrage pipeline (detection → validation → preflight simulation) is stable and profitable before enabling live trades. We stay in `STOP_AFTER_PREFLIGHT=true` mode until all checkpoints below pass for at least 24h of active monitoring.

## Data Sources
- **Grafana / Postgres**: `realtime_preflight_metrics` view (recent opportunity + preflight snapshot).
- **Grafana / Prometheus**: `mevana_preflight_simulations_total`, `mevana_preflight_latency_seconds`, `mevana_net_profit_after_costs_bps`.
- **Postgres**: `execution_results`, `arbitrage_opportunities`, `execution_reconciliation` (historical deep dives).

## Baseline Loop (per session)
1. Ensure services up: `curl http://localhost:8089/metrics | head` (executor) and Grafana dashboard loads without missing panels.
2. Confirm latest entries in `realtime_preflight_metrics`:
   ```sql
   SELECT timestamp,
          opportunity_id,
          status,
          estimated_profit_sol,
          total_estimated_cost_lamports / 1e9 AS total_cost_sol,
          error_message
   FROM realtime_preflight_metrics
   ORDER BY timestamp DESC
   LIMIT 10;
   ```
3. Verify Prometheus counters tick:
   ```promql
   rate(mevana_preflight_simulations_total{result="success"}[5m])
   rate(mevana_preflight_simulations_total{result="failure"}[5m])
   ```
   Success should be >= 80% once filters settle. Investigate if failure rate spikes > 0.2/min.
4. Inspect Grafana tables:
   - Panel 16 "Recent Arbitrage Opportunities" for latest per-opportunity status, profit, and error context.
   - Panel 17 "Hourly Preflight Summary" for pass/fail counts, average profit, cost, and net per hour.
   - Panel 18 "Route Performance (48h)" to spot venues or pairs dragging down net profit.
   - Panel 19 "Avg Net Profit (SOL) by Hour" should trend upward; flat/negative lines need investigation.

## Detailed Checks

### Profit & Cost Envelope
```sql
SELECT date_trunc('hour', executed_at) AS hour_bucket,
       COUNT(*) FILTER (WHERE status = 'PreflightSuccess') AS preflight_passes,
       AVG(estimated_profit_lamports) / 1e9 AS avg_profit_sol,
       AVG(total_estimated_cost_lamports) / 1e9 AS avg_cost_sol,
       AVG((estimated_profit_lamports - total_estimated_cost_lamports) / 1e9) AS avg_net_sol
FROM execution_results
WHERE executed_at >= NOW() - INTERVAL '6 hours'
GROUP BY hour_bucket
ORDER BY hour_bucket DESC;
```
Acceptable window: `avg_net_sol > 0.0001` (~0.1 mSOL) and `avg_cost_sol <= 0.0003`.

### Failure Review
```sql
SELECT status,
       error_message,
       COUNT(*) AS occurrences,
       AVG(estimated_cu_cost_lamports)::bigint AS avg_cu_cost,
       AVG(estimated_priority_fee_lamports)::bigint AS avg_priority
FROM execution_results
WHERE executed_at >= NOW() - INTERVAL '6 hours'
GROUP BY status, error_message
ORDER BY occurrences DESC;
```
Focus on: `simulation_failed`, `min_profit_not_met`, `priority_fee_exceeds_cap`. Tune filters/thresholds instead of forcing execution.

### Venue / Pair Breakdown
```sql
SELECT token_pair,
       venue_sell || '→' || venue_buy AS route,
       COUNT(*) FILTER (WHERE status = 'PreflightSuccess') AS passes,
       COUNT(*) FILTER (WHERE status != 'PreflightSuccess') AS rejections,
       AVG(estimated_profit_lamports - total_estimated_cost_lamports)::bigint AS avg_net_lamports
FROM realtime_preflight_metrics
WHERE timestamp >= NOW() - INTERVAL '12 hours'
GROUP BY token_pair, route
ORDER BY passes DESC
LIMIT 20;
```
Reject routes with negative `avg_net_lamports` or low pass count (< 3) to reduce noise.

### Latency Guard
PromQL:
```promql
histogram_quantile(0.95, rate(mevana_preflight_latency_seconds_bucket[10m]))
```
Target < 0.6s. If higher, inspect executor logs (`journalctl -u executor-service`).

## Pass Criteria Before Live Trades
- Preflight success rate ≥ 85% over a continuous 24h window (monitor gauge panel 8; confirm via PromQL).
- Median net profit per opportunity ≥ 0.00015 SOL after costs with standard deviation not exceeding profit (requires at least 30 passing samples).
- No persistent error category above 10% of total opportunities in the last 6h.
- Latency P95 < 600ms, P99 < 900ms.
- Flash-loan fee share < 25% of estimated profit for ≥ 90% of passes (check via custom SQL).

## Reporting & Actions
- Quick snapshot script: `scripts/monitoring/preflight_diagnostics.sh` (reads Postgres + Prometheus) for ad-hoc status dumps. Export `DATABASE_URL`/`PROMETHEUS_URL` if non-default.
- Record findings in `docs/deployment/logs/DEPLOY_YYYYMMDD.md` under "Preflight Analysis".
- If thresholds missed: adjust filters (`config_api` smart filter), priority fee caps, or min-profit thresholds before re-testing.
- Only after two consecutive green windows (>=48h) should `STOP_AFTER_PREFLIGHT` be reconsidered and dry-run toggles revisited.
