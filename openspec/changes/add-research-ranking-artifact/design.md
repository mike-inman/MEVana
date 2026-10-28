# 2026-10-28 CDT — Ranking artifact design

## Context

See proposal.md. The handoff unifies WATCH and ranking output. Existing Core contracts preserve raw input and use explicit canonical writers. The dated implementation plan is in docs/2026-10-28-0810-CDT-research-stage0-handoff-08-implementation-plan.md.

## Goals / Non-Goals

Provide inspectable advisory contracts and reusable route validation. No runtime ingestion, trading commands or inferred confidence/profit values.

## Decisions

Use ResearchRoute once, with defensive read-only token/pool lists. Reuse CatalogRules.Id through a new internal issue collector; do not alter any existing validators. Check syntactic route equality by exact ordinal sequences, never rotation or reversal normalization.

Use nullable long for sizes/slots/samples, nullable int for ranks/priorities/basis points and schema version, plus existing PolicyKind and EvidenceKind. Only canonicalization adds canonicalizationVersion 1; schemaVersion must already equal 1. Kind is a fixed WATCH constant, not an extensible message selector.

Require training within coverage but do not impose the request's 24-hour training limit; training may be longer. No future training, zero-offset UTC throughout, inclusive valid-from and exclusive expiry, with maximum TTL 24 hours. Compare time differences without adding time to avoid overflow. Invalid nowUtc is an issue rather than an exception.

Use the first route token for quote bounds. Check even syntactically valid same-token adjacent hops as incompatible with the catalog pool's distinct pair. Historical validation does not depend on VerifiedAvailable, matching result conventions. Source-result IDs are syntax-checked only; no result store exists.

Explicit JSON writes preserve route order and nulls and sort entries only by rank. Independent Python JSON fixture construction protects against serializer self-consistency errors. Field-surface allowlist tests prevent accidental authority or floating-point fields, including nested DTOs.

## Risks / Trade-offs

Evidence labels and calibration claims are declarations → contract tests validate structure, not ranking quality or authenticity. Large raw inputs are preserved → the caller still needs transport-level limits later.

## Migration Plan

Additive local Core and tests only. Strict validation and passing tests precede CLI archive; compare the merged spec against every delta requirement afterward. No deployment or data migration.
