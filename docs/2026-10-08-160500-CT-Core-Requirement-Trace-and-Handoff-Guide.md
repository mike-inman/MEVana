# Core requirement trace and smaller handoffs

Date: 2026-10-08 16:05 CT. Purpose: show where every original feature requirement remains and how complete selected Core context is supplied. Status: specifications prepared; no new product implementation or acceptance.

The original 2026-10-07 15:47 Experiment Request API specification remains unchanged. Its complete text and each selected slice specification are hash-pinned by AgentEngineering. Host-only original verification establishes provenance and never substitutes for a model's required reads. Slice documents define complete rules, public interfaces, tests and exclusions for their task; actual predecessor implementation is read where needed. The domain validator needs the passed syntax validator's complete public interface, not its implementation body; all solution checks still compile/run its tests.

| Original requirement | Owner/evidence still required |
| --- | --- |
| Raw request fields, exact quote integers, ordered pair, immutability | Request carrier and contract tests; invalid raw values retained |
| Identifier format/length, nulls, list counts/distinctness, positive min<=max, start<end and <=24h | Typed syntax validator and boundary/rejection tests |
| Immutable versioned catalog, metadata invariants, UTC coverage, decimals/bounds, references, empty default and honest availability | Catalog contracts/invariant tests; JSON loading and startup rejection remain later application integration |
| Dataset lookup/availability, strategy, known tokens, pool compatibility, coverage and quote/dataset bounds | Domain validator and semantic tests |
| UTC normalization, ordinal pool sorting, ordered pair, case/exact integer preservation, versioned canonical content | Catalog-independent canonicalization and tests; content equality retained for storage integration |
| Malformed/unknown/missing/wrong-type JSON and missing timezone | Future strict API parser/integration tests; typed dates cannot prove original timezone syntax |
| POST content type/16 KiB limit, key 8–128 ASCII characters/no trimming/one value, status and error responses | Future API handoff: 201/200/400/404/409/413/415, GUID/UTC/Location/Problem Details, no update/delete or execution transitions |
| Check retry key before current catalog, stable previously accepted metadata, canonical equality/conflict | Future idempotency/persistence/API integration; no availability lookup in canonicalization |
| Atomic UNIQUE key, race winner, parameterized SQL, schema/version, durable reopen, own connection, 5-second busy bounds, 503 storage_busy/500 and no success before commit | Future SQLite persistence/concurrency/failure acceptance |
| Loopback-only Development, no auth/exposure, ignore .local-data/SQLite artifacts, no real dataset/trading | Later local runtime verification and access-control decisions; all present slice scopes preserve these boundaries |
| Real dataset provenance/coverage/suitability; strategy implementation; production auth/database/messaging/workers/Rust | Original remaining decisions, not authorized by these slices |

Prepared sequential dependency order: request → catalog → syntax → domain validation → canonicalization. Each implementation owns its exact source/test files; only request owns approved test-project setup. Earlier broad profiles and failed terminal ledgers remain preserved. No partial checkpoints are treated as passed predecessors.

Each source has a local token ceiling. Model writing should proceed by descending file budget, with tests allowed before implementation; fixed checks run only after all assigned files have been written. The host then performs the same five real checks without another full-source Developer request. It stops after a failed/truncated build rather than testing stale binaries, and needs actual fresh nonzero passing TRX. Independent Tester/Reviewer still run, and no integration approval is inferred.

Offline capacity inspection reserves complete fixed documents, predecessor/output source ceilings, read/write framing and four modest failure calls. It estimates each pre-write envelope and independent source review, with output headroom for tool arguments. Reservations are planning data, never candidate source, fabricated tool success or provider acceptance. Failures/rewrites can still exceed this scenario; every actual request and source remains bounded. Preparing these documents does not establish that the model will implement them correctly.
