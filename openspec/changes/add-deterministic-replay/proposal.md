# Proposal — 2026-10-30 CDT

## Why

The captured Whirlpool accounts and exact quoter can now answer the Stage 0 route-selection question on a bounded, reproducible historical sample. Revision 1 limits each result to one token pair so its canonical v1 request truthfully describes all evaluated pools.

## What Changes

- Add an offline, deterministic replay crate over the finalized allowlist capture.
- Evaluate ordered same-pair two-leg routes separately for SOL/USDC, SOL/USDT and USDC/USDT with exact integer quotes and one declared cost model.
- Emit per-candidate evidence, three `Simulated` request/result pairs, manifests and policy summaries; validate the requests/results through unchanged C# contracts.
- Add a capture-backed test catalog entry and bounded committed artifacts. No live service or trading behavior changes.

## Capabilities

### New Capabilities

- `research-replay`: Offline event-time state reconstruction, exact route evaluation, policy accounting and reproducible simulated evidence.

### Modified Capabilities

None. The existing dataset and experiment contracts remain unchanged.

## Impact

New `crates/research-replay`, a workspace member and lockfile entry, test-only .NET catalog/validation artifacts, OpenSpec, and a dated closeout. Captures and full replay outputs stay outside Git. No network, wallet, UI, database, Kafka or trading path is involved.
