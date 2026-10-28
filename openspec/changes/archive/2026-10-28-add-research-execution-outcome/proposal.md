# 2026-10-28 CDT — Execution outcome proposal

## Why

Intent and estimated profit must remain distinct from submitted, landed and reconciled outcomes. Advisory research also needs rejected and unranked candidate records to avoid biased comparisons.

## What Changes

- Add raw execution outcomes with mode/evidence/state constraints, monotonic timing and optional ranking links.
- Reuse the shared route and catalog semantics from the archived ranking capability.
- Add independent profit and unsigned cost fields, canonical version 1, idempotency-conflict detection and Synthetic fixtures for rejection and paper landing.

## Capabilities

### New Capabilities

- `research-execution-outcome`: terminal candidate facts, linkage, reconciliation and idempotency conflict detection.

### Modified Capabilities

None; the shared route is reused unchanged.

## Impact

Additive Core DTOs, validators, canonicalization, helper and tests only. No trading logic repair, submission, provider lookup, API, persistence, UI or deployment changes. This supplies a trustworthy contract for later F3 remediation; it does not repair the Rust hotpath.
