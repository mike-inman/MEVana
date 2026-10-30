# Proposal — 2026-10-30 CDT

## Why

The version-2 request can declare a three-token route universe, but replay currently evaluates only same-pair two-leg routes. This change measures whether triangles add opportunities in the bounded handoff 12 capture without changing v1 behavior.

## What Changes

- Add offline v2 multi-leg replay, shape-specific costs, whole-universe policies and detailed coverage.
- Add v2 result syntax and canonicalization while retaining the unchanged v1 result path.
- Add a shared cross-language route parity fixture and strict v1, quote-parity and determinism gates.

## Capabilities

### New Capabilities

- `research-multi-route-replay`: Deterministic offline multi-pair and triangle replay with coverage and reproducibility.

### Modified Capabilities

- `research-experiment-result`: Add a version-2 request linkage syntax/canonicalization path without altering v1 result behavior.

## Impact

`research-replay`, new .NET Core/test files and one permitted helper extraction, OpenSpec, bounded committed fixtures and dated closeout. Capture and existing replay files remain read-only. No provider or trading path.
