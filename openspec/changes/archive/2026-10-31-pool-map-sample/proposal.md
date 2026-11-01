# 2026-10-31 Pool map sample

## Why
A single bounded sample can identify less common pairs for later research without continuous collection.

## What Changes
Extend the existing research scripts with a standard-library Decimal collector, saved-response replay, mint checks, requirement sequence, transactional storage and a provisioned dashboard. Reuse the existing named credential loader through the public account snapshot helper.

## Capabilities
### New Capabilities
- `pool-map-sample`: Bounded public pool discovery and transparent first-failure reporting.
### Modified Capabilities
None.

## Impact
Research scripts and additive research tables only. Fixed thresholds: two pools, reported liquidity 20000, age seven days. Unknown fees remain review items.
