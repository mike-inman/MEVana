# Design — 2026-10-29 CDT

## Context

The handoff 09 capture contains pool, fixed-array, dynamic-array, and transaction updates. `whirlpool-core` currently exposes approximate legacy quoting; consumers must retain that behavior. The 2026-10-29 handoff 10 and its license decision control this change.

## Goals / Non-Goals

**Goals:** A separate exact API for standard fees, decoded captured arrays, offline golden extraction and reproducible tests.

**Non-Goals:** Adaptive-fee quoting, provider access, capture replay, trade execution, or replacement of legacy callers.

## Decisions

1. Pin `orca_whirlpools_core` and `orca_whirlpools_macros` at `=1.0.4` and disable defaults on both. These are the Apache-2.0 releases identified by the operator; a direct macros pin prevents semver from selecting a later non-commercial release. A lockfile test enforces the boundary. Implementing the swap math independently would add avoidable correctness risk.
2. Add a new `exact` module and retain existing `math`, `quoter`, and `tick_window` without edits. Adapters translate locally decoded pool and ticks into the 1.0.4 integer facades. No float helper enters the exact path.
3. Decode dynamic ticks by checking discriminator, length, pool key, bitmap, and each tick tag. The 88 tags account for the 148-byte minimum; each initialized tag adds 112 bytes. The quote facade stores both array kinds in one representation. Reject malformed or inconsistent bytes.
4. Read the six finalized capture files using existing frame decoding and Prost types. Build ordered pool/array histories by slot and write version. For each successful single-pool transaction, use the immediately preceding pool update and arrays observed by that point; map vault indexes through static and loaded addresses. Store versioned cases outside Git and commit only a bounded subset.
5. Compare exact-in and exact-out results against both vault deltas; require a matching mode and compare available post-state. Classify all nonmatches. Stop at the handoff's scarcity or unexplained thresholds, preserving diagnostics and avoiding math calibration.

## Risks / Trade-offs

- Processed capture gaps or missing initial array snapshots can leave too few evaluable cases → report the measured shortfall and stop at step 7.
- Multiple pool writes inside a transaction can confound vault-delta comparison → classify separately from unexplained mismatches.
- A dependency update could resolve a disallowed Orca version → exact direct pins and lockfile test.
- Historical on-chain data proves quote accounting for observed states only → document the limits in the closeout.

## Migration Plan

The new API has no existing callers and needs no data migration. A later handoff may adopt it for replay after golden validation. Reverting this branch removes the feature without changing legacy consumers.
