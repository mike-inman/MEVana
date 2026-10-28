# 2026-10-28 08:10 CDT — Handoff 08 implementation plan

Implement ranking, archive it, then implement and archive outcomes. Each change gets its mandated OpenSpec proposal, design, tasks and spec delta, tests and independently authored synthetic canonical fixtures. Verify the 79-test baseline, zero-warning builds, five strictly valid main specs and unchanged prior contracts before publication.

Use one immutable raw route type; syntax checks shape, catalog semantics check adjacent tokens, first-token quote bounds and exact provenance. Reuse existing enums, ValidationIssue and CatalogRules identifier grammar.

Ranking: schema version 1, fixed WATCH kind, zero-offset UTC, exclusive expiry (`now >= expiresAt` is expired), inclusive valid-from. Training coverage must fit dataset coverage; training end cannot exceed as-of. Required numeric fields are nullable until validation. Models are present only for ModelRanking. Confidence and calibrated probability stay separate.

Outcomes: add dataset/metadata/strategy provenance for catalog checks, schema version 1, required decision time, and restrict submission facts to Landed/Failed/Dropped. Expired means non-submitted expiry; an unconfirmed submission uses Dropped. Landed slot is required only for Landed/Failed, forbidden otherwise. Transaction signatures are allowed only for live submitted states. Fees use nullable ulong; all other atom quantities use long. Only LiveSubmission Landed/Failed plus reconciliation permits realized profit. Never infer profit channels.

Canonicalizers are explicit, deterministic version-1 writers; preserve route order and sort only ranking entries. An idempotency helper compares validated canonical bytes under equal keys; it performs no storage or mutation.

No provider operations or trading activation. Preserve the existing request's second-token bound and non-UTC-offset acceptance; propose separate follow-ups in the closeout. Fetch and inspect remote-only commits before a fast-forward push; merge remote history if needed, never force.
