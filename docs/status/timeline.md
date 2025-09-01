# MEVana Arbitrage Engine Timeline

High-level milestones with completion status.

| Date (approx) | Milestone | Key Artifacts / Notes | Status |
|---------------|-----------|-----------------------|--------|
| 2025-08-?? | Initial edge ingestion scaffold | `edge_integration.rs` basic detector | Done |
| 2025-08-?? | 2-leg arbitrage detection | Profit calc, min profit filter | Done |
| 2025-08-?? | Dynamic histogram metrics | `stats.rs` profit & latency histograms | Done |
| 2025-08-?? | 3-leg (triangular) detection | Token-index seeded + brute force fallback | Done |
| 2025-08-?? | Performance refactor (Arc edges) | Removed heavy cloning | Done |
| 2025-08-?? | True re-binning & percentiles | Histogram resize & p50/p90/p99 | Done |
| 2025-08-?? | Pruning heuristics for 3-hop | Global max factor early cuts | Done |
| 2025-08-?? | Diagnostics counters | cycles_examined / pruned / profitable + metrics export | Done |
| 2025-08-31 | Backlog + timeline docs | `docs/backlog.md`, this file | Done |

## Current Phase
Core fundamentals complete; entering Observability/Optimization planning.

## Next Candidate Milestones
1. HTTP /metrics endpoint
2. Adaptive sizing & depth-aware solver
3. Backtest/replay harness
4. Advanced pruning layer (per-token max factors)

## Reference Locations
- Core detector: `crates/arb-engine/src/edge_integration.rs`
- Metrics & counters: `crates/arb-engine/src/stats.rs`
- Backlog list: `docs/backlog.md`

## Notes
Timeline uses approximate dates until version tagging is introduced. Consider tagging releases (e.g., `v0.1.0-core`, `v0.2.0-observability`) to harden milestone checkpoints.

Generated: 2025-08-31
