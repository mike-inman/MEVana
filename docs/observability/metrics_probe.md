# Metrics Probe Script
Last updated: 2025-09-15

Script: `scripts/metrics_probe.sh`

Quick sampling helper for local development. Shows structural gauges, pruning diagnostics, and histogram quantiles in a compact view.

## Usage

```bash
./scripts/metrics_probe.sh            # defaults to http://localhost:8080/metrics
./scripts/metrics_probe.sh http://host:port/metrics
```

## Output Sections
- Structural Gauges: edge & token counts, adjacency, max edge factor.
- Cycle Diagnostics: examined / pruned / profitable / pruning efficiency (requires `--features diagnostics`).
- Profit & Latency Quantiles: p50 / p90 / p99 approximations (exported as quantile series).
- Max Values: running max profit bps & latency ms.

## Notes
- If a metric is absent, it will simply not appear (e.g., diagnostics disabled).
- Extend by adding more `pick` lines in the script for new metrics.

## Next Ideas
- Add JSON output mode (`-j`) for programmatic scraping.
- Integrate into a CI smoke job to snapshot key metrics after synthetic load.
