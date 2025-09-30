Last updated: 2025-10-01

## Hotpath Real Detection Migration — Current Status
- Shared swap decoder landed in `normalizer-runtime::transaction::extract_swaps_from_update`; normalizer-service now delegates to it (removes duplicated parsing).
- PropertyStore exposes two feature flags:
  - `heuristic_detection_enabled` (defaults true)
  - `edge_detection_enabled` (defaults false)
  Both persist across restarts and appear in property snapshots.
- Hotpath now consumes `edge.delta.v1` via `spawn_edge_consumer`, pushes `ArbSignal`s through `run_signal_consumer`, and reuses the smart filter/execution path.
  - Legacy heuristic path still present behind the property toggle.
  - New Prometheus counters: `hotpath_signals_{received,processed,rejected}_total{source}`, `hotpath_opportunities_{evaluated,accepted,rejected,blocked}_total{source}`.
- Documentation: `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md` updated to mention the toggles.

## Not Yet Done
- Quote feed integration still unused (FastEngine::upsert_quotes path dormant).
- Executor still receives heuristic-style opportunities; need to confirm real edge signals hit the execution queue end-to-end in staging.
- No replay/integration test yet for the hotpath edge pipeline (next logical task: fixture replay harness).

## Next Steps Recommendation
1. In staging, enable `edge_detection_enabled=true`, keep heuristics on; monitor `hotpath_signals_*` counters for a clean stream.
2. Once satisfied, set `heuristic_detection_enabled=false` to rely solely on edge data.
3. Add replay/integration tests or CI job ingesting sample `edge.delta.v1` frames to guard the path long-term.
4. Wire metrics dashboards/alerts for new counters.

## References
- `crates/hotpath_arb_engine/src/main.rs`: edge consumer + signal handler (`spawn_edge_consumer`, `run_signal_consumer`).
- `crates/hotpath_arb_engine/src/property_store.rs`: new property toggles and persistence.
- `crates/normalizer-runtime/src/transaction.rs`: shared swap extraction logic and unit tests.
- `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md`: toggle guidance post-deploy.
