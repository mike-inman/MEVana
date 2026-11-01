# 2026-11-01 Check all eligible pool pairs

## Why
The previous cap left most eligible pairs unread and conflated unchecked state with rejection.

## What Changes
Plan all eligible saved-map pairs and all controls before live reads. Read mints once, collect three batched pool rounds, separate NotChecked and label within-bin gaps without changing any threshold or list rule.

## Capabilities
### Modified Capabilities
- `pool-map-sample`: Complete bounded eligibility checks and bin labels.
### New Capabilities
None.

## Impact
Research scripts and dashboard only; reuse saved public map and existing verified decoder helper. Preserve both earlier samples.
