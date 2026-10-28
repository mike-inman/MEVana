# 2026-10-27 CDT — Experiment result proposal

## Why

Stage 0 needs reproducible policy comparisons with explicit provenance and evidence labels. Profit estimates must remain distinct from simulated and realized outcomes, including failed and unavailable experiments.

## What Changes

- Add raw result, policy and rejection-count DTOs, syntax and catalog validation, request-hash verification and deterministic version-1 canonical result JSON.
- Test the result contract and provide one explicitly synthetic golden fixture.

## Capabilities

### New Capabilities

- `research-experiment-result`: immutable raw experiment results, evidence/status semantics, common comparison scope, independent profit fields and canonical provenance.

### Modified Capabilities

None. Dataset and request behavior stays as baselined.

## Impact

Only .NET Core contracts and tests, OpenSpec artifacts and dated documentation. No new dependencies, market data, external runtime services, API/persistence implementation, UI, Rust, deployment or trading changes.
