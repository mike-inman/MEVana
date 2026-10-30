# Design — 2026-10-30 CDT

## Context

V1 replay and its manifest are the regression baseline. Its config and candidate row must not change. The capture is a read-only format-v2 snapshot session. The version-two route enumerator provides the ordering contract.

## Goals / Non-Goals

**Goals:** One USDC-quoted 152-route replay, exact-in two/three-leg quotation, shape-specific costs, shared policy competition, coverage diagnostics and reproducible v2 artifacts.

**Non-Goals:** Provider access, capture modification, adaptive-fee pools, trade execution, fill/inclusion modeling, a learned ranker, or any v1 behavior change.

## Decisions

- Keep v1 replay/config/output entry points intact. Put v2 route, policy, replay and serialization behind v2 profiles so the v1 manifest remains unchanged except its supplied commit.
- Commit one full route fixture generated from the established .NET enumerator. Rust reads it for parity; Rust never writes it.
- Reuse the existing slot graph and snapshot application semantics. V2 route cache tracks all route pool generations plus the reference generation; v2 cost cache keys both shape and reference generation.
- Independently probe each pool direction at all three declared raw input-atom ladder sizes for coverage diagnostics, while route outcome still uses first failing leg. These probes reveal missing arrays on later legs without inventing an executable preceding output; their input atoms are diagnostic, not a chained route amount.
- Extract the existing result JSON writer into an internal helper with a parameterized validation result; prove original bytes unchanged with existing golden tests. V2 syntax validates a version-normalized copy plus an explicit version-two check.

## Risks / Trade-offs

- [Large candidate output] → Check disk before the run; write outside Git and fail on existing output paths.
- [Coverage probes cannot know later-leg actual input when a previous leg fails] → Label the probe input and keep diagnostics separate from route outcomes.
- [V1 regression drift] → Compare exact files against the immutable handoff 12 baseline before reporting results; stop on any mismatch.

## Migration Plan

Additive offline research path only. No data migration. Remove v2 profile and helper additions to roll back; v1 remains intact.
