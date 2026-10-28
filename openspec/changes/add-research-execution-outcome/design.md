# 2026-10-28 CDT — Execution outcome design

## Context

See proposal.md. Ranking's route and internal validation/JSON helpers are now implemented and archived. The original hotpath F3 finding remains untouched; this contract prevents the future research interface from conflating estimates with reconciled profit.

## Goals / Non-Goals

Record terminal facts and permit pure idempotency conflict checks. No trade commands, persistence, signature authenticity checks or linked-record retrieval.

## Decisions

Require dataset ID, exact metadata version and strategy version for catalog semantics. Add schemaVersion 1, with canonicalizationVersion 1 in canonical JSON. Reuse ResearchRoute, EvidenceKind, PolicyKind, ValidationIssue and the shared rules rather than redefine them.

Require decisionAtUtc so every terminal record has a decision timestamp. Enforce monotonic present times, permitting equality. Bound detection by dataset coverage; other lifecycle times may extend beyond coverage because reconciliation can happen later.

Landed, Failed and Dropped are exactly the submitted states. Expired represents non-submitted expiry; a lost submission uses Dropped. Forbid submission time, signature and costs on other states. Landed slot is allowed only on Landed/Failed, and signatures only on submitted LiveSubmission records. Optional reason codes are also permitted on other states, but any provided reason must be a valid identifier.

Use long? for required input size and observed/landed slots to preserve invalid negative submissions for diagnostics. Fee/tip fields are ulong? because the contract explicitly requires unsigned costs; unsigned values beyond long.MaxValue still retain exact JSON precision. Use independent long? profit fields without computed properties, normalization or fallback mapping.

Ranking links require both ID and rank, but there is no artifact store here. The semantic validator checks only linkage syntax, not existence, policy matching or expiry of a separately supplied artifact. Unranked outcomes are valid for every policy so exploratory coverage remains available.

The pure idempotency helper canonicalizes both inputs and compares exact strings under equal ordinal keys. It does not reserve keys or promise durable exactly-once processing. Explicit JSON serialization preserves all null channels, costs and ordered routes. Independent Python fixtures cover Synthetic Rejected Shadow and Synthetic Landed Paper cases without fabricated market claims.

## Risks / Trade-offs

Syntactically valid transaction signatures are not authenticated → no decoding, RPC or landing verification is claimed. Terminal states/evidence are producer declarations → later workers must supply measured facts.

## Migration Plan

Additive contracts and tests; no migration or deployment. Pass tests/build/strict validation, archive with the CLI, and compare every merged requirement with the delta before final Stage 0 validation.
