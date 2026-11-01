# 2026-11-01 Pool map fixes

## Why
The first sample's pool coverage, reported gaps, unknown fees and unreadable panels limit interpretation.

## What Changes
Add per-token discovery, bonding-curve classification, same-slot decoded prices and fees, per-pool depth and readable panels. Collect one new bounded sample.

## Capabilities
### Modified Capabilities
- `pool-map-sample`: Coverage and on-chain fee evidence.
### New Capabilities
None.

## Impact
Research scripts, a read-only adapter to the existing verified decoder and the pair discovery dashboard. Preserve the original sample and all fixed thresholds.
