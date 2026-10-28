# 2026-10-27 CDT — Result contract design

## Context

See proposal.md for motivation. Existing requests separate raw DTOs, syntax issues, catalog validation and canonical serialization. No worker or persistence layer exists.

## Goals / Non-Goals

Provide a contract that preserves bad submissions for inspection and permits trustworthy comparisons. Do not infer execution, generate market outcomes or implement API, persistence, ranking artifacts or execution outcomes.

## Decisions

- Keep common dataset, window and cost model only on the result, rather than duplicating per-policy scope that could disagree.
- Use nullable signed-long counts and profits in raw DTOs. Nullable counts distinguish missing values from zero. A selected-or-executed count makes the required accounting inequality testable. Reject duplicate rejection reasons rather than silently summing them. Subtract from an evaluated budget to avoid overflow.
- Failed/unavailable results have null or empty policies, rather than partly populated metrics. Completed results require all three policies; profits may remain unknown.
- Require zero-offset result timestamps including windows, 24-hour maximum duration, version-1 request provenance, lowercase SHA-256 hex and catalog-style identifiers. These choices keep the first contract bounded and unambiguous.
- Provide a fingerprint helper and a request-aware semantic validation overload. The two-argument catalog validator checks declared provenance shape and catalog compatibility; only the overload with the original request verifies content binding. Avoid embedding another complete request in each result.
- Catalog validation does not reject historical results because VerifiedAvailable later becomes false. It does require exact metadata and strategy identity and coverage bounds.
- Explicit Utf8JsonWriter serialization follows request style. Sort policies by enum numeric declaration order and reasons ordinally. Emit every nullable property explicitly, with seven-digit UTC timestamps, and preserve full integer precision. No mapper infers profit.
- Date the fixture using its filename and fixed createdAtUtc. It is synthetic contract test data only.

## Risks / Trade-offs

An arbitrary well-shaped hash alone cannot establish provenance → callers with the original request use request-aware validation; neither validation path proves market truth.

Version 1 accepts long atom quantities → future wider encodings need a new contract version.

## Migration Plan

Additive local Core contracts only; no deployment or data migration. Archive the spec delta after successful build, tests and strict validation. Rollback would revert the additive change without touching existing request/catalog behavior.
