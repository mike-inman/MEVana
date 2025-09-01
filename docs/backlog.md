# MEVana Arbitrage Engine Backlog

This backlog captures deferred "nice-to-have" enhancements. Core fundamentals are complete (edge ingestion, 2-leg + 3-leg w/ pruning, dynamic histograms, diagnostics metrics export). Items tagged *priority-early* slightly benefit from earlier integration due to potential interface changes.

## Observability & Metrics
- HTTP /metrics endpoint (serve `render_all()`) **(DONE)**
- Pruning efficiency ratio: pruned / (pruned + examined) **(DONE)**
- Export max theoretical factor observed **(DONE)**
- Gauge metrics: current edge count, token count, adjacency breadth **(DONE)**
- Edge staleness (time since last update) histogram / gauges **(TODO)**
- Structured JSON signal log (append-only) **(DONE)**
- WebSocket push channel for signals & metrics **(DONE)**
- AI integration endpoints (recent signals, advice, config update, persistence) **(DONE)**
- Tracing spans around scan phases (tokio-console or tracing subscriber integration) **(TODO)**
- Memory / allocator stats (jemalloc) (feature gated) **(TODO)**

## Performance / Algorithmic
- Per-edge or per-token adaptive sizing (*priority-early*)
- Depth/slippage-aware sizing solver (simulate along curve) (*priority-early*)
- Path cache + incremental invalidation
- Liquidity/volume-based edge filtering heuristic
- Multi-thread / sharded detector (partition graph by token groups)
- Additional pruning layers (per-token max factors, cached partial products)

## Reliability / Safety
- Circuit breakers on anomalous churn or stale edges
- Config hot-reload trigger (SIGHUP or admin HTTP endpoint)
- Per-venue health metrics & cooldown management
- Staleness-based disabling of edges (auto-prune) 

## Tooling / Testing
- Backtest & replay harness (deterministic edge delta feed)
- Scenario fuzzing for extreme fee / price combinations
- Bench suite (criterion) for 2-leg vs 3-leg scan latency
- Golden file tests for metrics render output

## Data & Persistence
- Optional persistence layer for latest edges (RocksDB / sled) on restart **(TODO)**
- Snapshot & restore of token index + edges **(TODO)**

## API / Integration
- gRPC or HTTP endpoint to query current best opportunities
- Subscription-based push for strategies (filter by token set)

## Housekeeping
- Remove deprecated helper `directed_tokens` after migration
- Inline docs for environment variables (README section)
- Add `--feature diagnostics` usage examples

---
Generated: 2025-08-31 (updated)
