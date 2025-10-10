# MEVana Session Handoff

_Last updated: 2025-10-10_

## Mission Snapshot
- Operate the MEVana hotpath in **dry-run** mode to evaluate swap opportunities, surface profitability metrics, and prepare for eventual live execution.
- Provide operators with realtime visibility into detection filters and the ability to tune them safely via the auxiliary API/Grafana.
- Compare legacy **heuristic** ingestion against the richer **edge detection** stream to identify higher-yield routes (focus on SOL/USDC/USDT/MSOL pairs so far).

## Key Artifacts & Endpoints
- **Grafana Dashboard:** `MEVana Dry-Run Insights` (includes new “Hotpath Property Controls” iframe).
- **Property Control UI:** `http://127.0.0.1:8089/ui/property-controls`  
  Allows live adjustment of:
  - `min_profit_bps`
  - `max_gas_budget_pct`
  - `max_edge_staleness_ms`
- **Prometheus Metrics:**  
  - `mevana_property_value{property="..."}` — reflects latest knob values.
  - `mevana_opportunities_passed_total`, `mevana_opportunity_rejections_total` — per token/venue flow, with `reason` labels (e.g., `competition_high`).
- **Property API:** `GET/PUT /properties/:key` (hotpath aux server, port 8089).

## Latest Local/Remote Status
- `hotpath_arb_engine` rebuilt from `origin/main` and deployed to `/opt/MEVana/bin/hotpath_arb_engine` (service restarted & running).
- Grafana container restarted (`docker restart mevana-grafana-1`) to pick up dashboard JSON changes.
- Repository clean and pushed (`2a136a61 Embed property control iframe…`).
- Control UI verified via `curl -I http://127.0.0.1:8089/ui/property-controls` (HTTP 200).
- Property change sanity check: `min_profit_bps` bumped to `3`, reflected in metrics.

## High-Value Observations
- Phoenix CLOB pairs (`USDT-SOL`, `USDC-SOL`) deliver the most consistent passes with minimal competition rejects—excellent dry-run candidates.
- Meteora/Raydium routes are heavily filtered by `competition_high`; relaxing competition thresholds or increasing `max_edge_staleness_ms` will surface more of them.
- Heuristics remain enabled (`heuristic_detection_enabled=true`) while edge detection is still off by default. When ready, flip `edge_detection_enabled=true` via the control UI, monitor `mevana_token_pair_evaluations_total`, and adjust competition knobs accordingly.

## Suggested Next Steps
1. **Embed live control iframe (done)** – Verify the Grafana panel renders; adjust values from within Grafana to confirm end-to-end workflow.
2. **Enable edge detection for comparison** – Using the UI, set `edge_detection_enabled=true` (optionally disable heuristics later). Track opportunity flow & rejections to gauge impact.
3. **Fine-tune competition heuristics** – Once edge data flows, lower thresholds incrementally and watch `competition_high` rejection counts to avoid overwhelming the pipeline.
4. **Document control workflow** – Update the operations guide with steps for using `/ui/property-controls` and interpreting the Grafana panels.
5. **Optional** – Extend the control page with additional properties (e.g., `min_venue_success_rate`) or integrate auth (use `ADMIN_API_TOKEN` header).

## Quick Reference Commands
- Restart hotpath:  
  `systemctl restart hotpath-arb-engine.service`
- Restart Grafana:  
  `docker restart mevana-grafana-1`
- View live metrics:  
  `curl -s http://127.0.0.1:8088/metrics | grep mevana_property_value`
- Stream executor dry-run results:  
  `/opt/MEVana/scripts/monitoring/stream_executor_results.sh --limit 10 --from-beginning`

Stay mindful of dry-run mode (`EXECUTION_DRY_RUN=true`) and circuit breaker thresholds—most “blocked execution” log messages are expected until live trading is authorized.
