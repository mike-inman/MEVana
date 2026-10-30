# Proposal — 2026-10-30 CDT

## Why

Handoff 11 replay treated skipped slots as capture gaps and applied abandoned-fork writes, obscuring most comparisons. The existing capture also lacks initial tick-array state, so many quotes cannot be assessed.

## What Changes

- Reconstruct slot parent/status history and evaluate confirmed state by default, retaining processed mode for sensitivity.
- Classify true gaps separately from skipped slots and forks; report dropped writes.
- Add an opt-in, bounded Tatum tick-array snapshot to read-only capture, with versioned frames and explicit absence evidence.
- Replay one new snapshot-backed capture and validate three separate Simulated v1 results without changing contracts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `research-replay`: slot-correct commitment, gap and fork treatment; snapshot state and absent arrays.
- `market-data-capture`: optional bounded RPC snapshots and format v2 inspection.

## Impact

Only `crates/research-replay`, `crates/geyser-capture`, their locked dependencies, test-only .NET catalog/fixtures, and OpenSpec/docs. No trading, wallet, request-contract, UI, database, Kafka or provider access outside the one authorized capture.
