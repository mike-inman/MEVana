# Observability & SLOs

## Metrics
- Decision latency histograms (geyser→extractor, extractor→arb, arb decision)
- Counters: decode hits, extraction success, DLQ, arb attempts/accepts/fills, PnL

## Dashboards
- Latency p50/p90/p99 over time; PnL by venue/time; acceptance rates

## Alerts
- SLO breach: p99 latency above threshold
- Negative net-PnL over rolling window
- Crash loops / heartbeat failures

## Kill-Switch
- Disable submission when SLO or PnL conditions fail until manual review
