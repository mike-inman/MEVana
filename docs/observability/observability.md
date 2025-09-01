# Observability & SLOs

## Metrics
- Decision latency histograms (geyser→extractor, extractor→arb, arb decision)
- Counters: decode hits, extraction success, DLQ, arb attempts/accepts/fills, PnL
- Dynamic histograms:
	- arb_signal_profit_bps (adaptive buckets)
	- arb_signal_latency_ms (edge→signal latency)
- Gauges / Diagnostics (feature: diagnostics):
	- arb_edge_staleness_ms (freshness of last edge delta)
	- arb_process_resident_bytes (RSS)
	- arb_edge_pairs_total / arb_tokens_indexed_total / arb_avg_adjacency
	- arb_max_edge_factor (pruning upper bound snapshot)
	- arb_pruning_efficiency (pruned / (pruned + examined))
- Counters (diagnostics): arb_cycles_examined_total, arb_cycles_profitable_total, arb_cycles_pruned_total

## Dashboards
- Latency p50/p90/p99 over time; PnL by venue/time; acceptance rates

## Alerts
- SLO breach: p99 latency above threshold
- Negative net-PnL over rolling window
- Crash loops / heartbeat failures
- Edge staleness > threshold (e.g. >500ms) indicates upstream gap
- Memory growth: arb_process_resident_bytes > expected envelope

## Kill-Switch
- Disable submission when SLO or PnL conditions fail until manual review
 - Optionally trigger if arb_edge_staleness_ms sustained high / lost feed
