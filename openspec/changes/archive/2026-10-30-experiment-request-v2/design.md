# Design — 2026-10-30 CDT

## Context

The v1 request and result syntax paths are independent classes. `ResearchContractJson` only writes JSON; it does not parse requests. Existing catalog metadata contains two-token pool edges. See proposal and delta specs.

## Goals / Non-Goals

**Goals:** A separate immutable v2 DTO; deterministic validation, canonicalization and route enumeration; v2 result linkage without changing v1 behavior.

**Non-Goals:** Triangle replay, provider reads, result schema changes, a JSON parser, or changes to handoff 12 fixtures/tests.

## Decisions

- Use a distinct class and validators rather than branching v1 classes. This keeps v1 error ordering and canonical bytes unchanged.
- Enumerate routes from catalog pool token pairs. Sort shape, pool sequence and token path ordinally for a stable public definition. A two-leg route traverses two distinct pools on one quote-containing pair; a triangle traverses three distinct token edges and pools, with both directions.
- Validate scope against the enumerated universe. Check shape feasibility before reporting unused leg pools; price-input pools need catalog existence but never count as route legs.
- Add a v2 overload to the existing result validator. To reuse all established result syntax and catalog checks without widening the v1-only syntax validator, validate an otherwise identical copy whose canonicalization version is normalized to 1, then check the original version-2 fingerprint and scope. This is internal to the v2 overload; the v1 overload remains byte-for-byte intact.
- The 2026-10-30 13:19 CDT amendment freezes the two asymmetric rejection codes: a version-1 result against a v2 request receives `request/request_mismatch`; a version-2 result against a v1 request receives the existing `requestCanonicalizationVersion/unsupported_canonicalization_version`. Keep an optional v2 overload parameter so literal-null calls remain bound to the original v1 methods.
- Use `Utf8JsonWriter` and the v1 UTC formatting convention for canonical v2 JSON. No new dependency.

## Risks / Trade-offs

- [A v2 result syntactically passes via a normalized copy] → Assert valid v2 linkage and cross-version rejection; keep the normalization local and documented.
- [Combinatorial enumeration grows with 32 pools] → Scope is bounded and enumeration is pure; tests cover exact 120/152 counts and deterministic order.
- [Unknown token/pool errors can mask route checks] → Return catalog identity issues deterministically and enumerate only when all required referenced members exist.

## Migration Plan

Additive contract only. No data migration. Reverting v2 files and overload leaves v1 intact.
