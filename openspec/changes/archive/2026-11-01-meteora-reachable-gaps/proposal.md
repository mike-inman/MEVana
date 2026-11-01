# Proposal — 2026-11-01 CST

## Why
Handoff 19b found nominal Meteora gaps above minimum fees without executable depth. Handoff 20 tests whether those gaps survive real bin liquidity and variable fees.

## What Changes
- Verify the published DLMM layout and implement read-only integer bin walking and fee modeling.
- Preflight and collect one bounded two-round account-only sample; preserve all older samples.
- Correct R5 to accept any passing pool pair and show honest, readable dashboard outcomes.

## Capabilities
### New Capabilities
- `meteora-reachable-gaps`: bounded account sampling and reproducible modeled round trips.
### Modified Capabilities
- `pool-map-sample`: accept any compared pool pair passing R5, and distinguish no failure from unknown.

## Impact
Python research tools, public account fixtures, research schema/dashboard, documentation. No trading paths, protected crates, UI or credential changes.
