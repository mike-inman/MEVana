# Proposal — 2026-10-30 CDT

## Why

The one-pair v1 request cannot truthfully describe a SOL–USDC–USDT triangle or a cost-model pool that is not a route leg. A separate v2 contract is needed before later multi-pair replay.

## What Changes

- Add a raw v2 request, syntax and catalog validation, canonicalization, fingerprinting and deterministic route enumeration.
- Add v2 result linkage through the existing result type, while preserving v1 behavior.
- Freeze v1 request behavior explicitly in its existing specification.
- Add contract-only tests and golden canonical artifacts; no replay or provider use.

## Capabilities

### New Capabilities

- `research-experiment-request-v2`: Multi-pair request scope, route shapes, price inputs, validation, canonical identity and result linkage.

### Modified Capabilities

- `research-experiment-request`: State that v1 remains unchanged and independent of v2.

## Impact

Only new .NET Core and test files, necessary overloads in `ExperimentResultValidator.cs`, OpenSpec, and a dated closeout. No Rust, provider, trading, UI, or persistence changes.
