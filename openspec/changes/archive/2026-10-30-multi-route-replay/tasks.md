# Tasks — 2026-10-30 CDT

## 1. Contract and parity

- [x] 1.1 Commit full .NET-generated R2 route fixture and verify byte-identical .NET and Rust enumeration with 120 triangles and 32 two-leg routes.
- [x] 1.2 Add v2 result syntax and canonicalizer; verify v1 golden bytes unchanged and v2 fixture validation.

## 2. Replay

- [x] 2.1 Add exact-in multi-leg quotes, first-failure attribution, shape-specific cost and cache tests.
- [x] 2.2 Add whole-universe policy decisions, third-pool conflicts, deterministic tie-breaks and accounting tests.
- [x] 2.3 Add v2 replay profiles, per-shape and per-pool-direction coverage, canonical artifacts and manifest; verify refusal of v1 capture and overwrite.

## 3. Evidence and closeout

- [x] 3.1 Pass route parity and all three real-data regression gates before interpreting a v2 result.
- [x] 3.2 Run default twice and three sensitivities; validate request/result with .NET and write dated closeout with hashes, policy/coverage tables and limitations.
- [x] 3.3 Run pinned Rust, .NET, OpenSpec, hygiene, diff and non-printing credential checks; record the pre-existing workspace-wide format failure and passing scoped format check.

## Workflow follow-up

- Sync specs, archive the OpenSpec change, commit and synchronize the branch under the operator's network instruction; do not merge.
