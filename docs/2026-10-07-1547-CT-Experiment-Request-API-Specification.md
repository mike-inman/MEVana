# Experiment Request API — First Feature Specification

Date: 2026-10-07 15:47 CT. Status: proposed specification for human review; not implemented. Purpose: turn the planner draft into precise, bounded coding tasks.

## What we are building

A local .NET 10 API that validates and persists a research experiment request and retrieves it by ID. Accepted means validation passed and a database transaction committed. No experiment, replay, arbitrage calculation, trade, wallet operation, or signing occurs. There is no worker, queue, or status progression in this milestone.

AgentEngineering contains the coding-agent tooling. MEVANA owns this application under dotnet/MEVana.slnx. Existing Rust code is outside all initial coding-task boundaries.

## Proposed implementation choices

Use ASP.NET Core minimal endpoints, a small Core project for domain rules/storage interfaces, Microsoft.Data.Sqlite in a separate Persistence project, and a test project. Choose SQLite for real, repeatable local persistence without another running database server. Keep database-specific code out of Core. This is not the chosen production Azure database: a later decision will evaluate Azure storage against the actual workload. Pin compatible package versions when scaffolding/implementing dependencies; no package is installed by this document.

The initial application runs in Development on loopback only. No application authentication is implemented; this is a local learning API and must not be exposed through a tunnel, LAN bind, or Azure deployment until access control is designed. The controller's Azure login is not the application's authentication.

## Request contract

POST /api/experiment-requests requires Content-Type application/json and an Idempotency-Key header. Maximum request body: 16 KiB. Reject unknown JSON properties, missing required fields, wrong types, nulls, and malformed JSON with 400; oversized body with 413; unsupported content type with 415.

Required body fields:

| Field | Type and rule |
| --- | --- |
| datasetId | Case-sensitive ASCII identifier, 1–64 characters, letters/digits/hyphen/underscore only; catalog entry must exist and be verifiedAvailable. |
| strategyVersion | Same identifier syntax; must appear in the selected dataset's supported research-strategy versions. This does not claim an implemented strategy exists. |
| windowStartUtc / windowEndUtc | ISO 8601 timestamps with Z or explicit UTC offset; normalize to UTC. Start strictly before end, duration at most 24 hours, and entire half-open interval inside dataset coverage. Reject missing timezone. |
| tokenPair | Array of exactly two distinct token identifiers present in the dataset's catalog; base and quote order is meaningful and preserved. IDs are opaque catalog identifiers; no new Solana address parser is assumed. |
| poolIds | Array of 2–16 distinct case-sensitive catalog pool IDs, each supporting the selected pair. No duplicates. |
| minTradeSizeQuoteAtoms / maxTradeSizeQuoteAtoms | Positive JSON integers, min <= max, both <= catalog maxResearchTradeSizeQuoteAtoms. The quote asset is tokenPair[1]; units are its smallest recorded denomination, defined by catalog quote-token decimals. No floating point, price conversion, wallet funds, or execution is involved. |

Idempotency-Key: 8–128 ASCII letters/digits/hyphen/underscore, case-sensitive; one value only. No trimming or case folding. Bounds here are proposed first-exercise resource limits, not profitable strategy parameters or validated trading limits.

Schematic example (the IDs are placeholders, not an available dataset):

```json
{
  "datasetId": "example-fixture-only",
  "strategyVersion": "research-v1",
  "windowStartUtc": "2026-01-01T00:00:00Z",
  "windowEndUtc": "2026-01-01T01:00:00Z",
  "tokenPair": ["fixture-base", "fixture-quote"],
  "poolIds": ["fixture-pool-a", "fixture-pool-b"],
  "minTradeSizeQuoteAtoms": 1000,
  "maxTradeSizeQuoteAtoms": 10000
}
```

## Dataset catalog and honest availability

A versioned local JSON catalog is loaded and validated at startup. Each immutable dataset version has an ID, metadata version, provenance, availability flag, suitability description, UTC coverage, token metadata including decimals, compatible pools/pairs, supported strategy versions, and positive research trade-size bounds. Catalog entries must not contain credentials or user-supplied paths/URLs. Invalid catalog structure prevents startup rather than silently accepting data.

The shipped catalog has zero datasets because no verified dataset exists yet. Consequently the initial running API cannot accept an experiment request until an operator supplies a reviewed, available dataset entry. Tests use explicitly labeled fixtures with temporary storage; passing those tests is not live dataset acceptance. The API never downloads data or trusts a client's assertion of availability. A real catalog entry needs operator verification of underlying data, coverage, and suitability; metadata alone is not proof of data correctness. Changing catalog metadata requires a new dataset/metadata version and an application restart for this milestone.

## Responses and status

New valid submission: 201 Created with Location /api/experiment-requests/{id}. Return server-generated GUID id, status Accepted, createdAtUtc, datasetId, catalogMetadataVersion, and normalized request. Accepted means recorded only. No Completed, Running, Validated, or PendingExecution transitions are implemented.

GET existing GUID: 200 with the same saved record. Unknown GUID: 404. Invalid GUID syntax: 400. Responses use UTC timestamps. No update/delete endpoint is introduced.

Validation errors: 400 Problem Details with stable field error codes, including unknown_dataset, dataset_unavailable, incompatible_pair, incompatible_pool, unsupported_strategy, invalid_window, and invalid_trade_size. Do not echo secrets, database paths, or internal exceptions. Missing dataset is a real rejection, not fallback to sample data.

## Idempotency and concurrent requests

After syntax/type validation and canonicalization, check an existing idempotency key before current catalog validation. A key with the same normalized payload returns the original record with 200 and Location, including its original timestamp and metadata version. Thus a committed request remains retrievable by retry after catalog availability changes. Same key with different normalized payload returns 409; it never modifies the original record. Different keys may intentionally record the same research parameters separately.

Canonicalization normalizes timestamps to UTC and sorts poolIds (order has no meaning); tokenPair order is retained. Preserve integer values and case-sensitive identifiers. Compare a deterministic, versioned canonical representation; if a hash is used, retain and compare the canonical content as well. Do not hash raw JSON property order or whitespace. Invalid attempts do not reserve a key.

Persist request and key atomically with a database UNIQUE constraint on the key, not a check-then-insert alone. On an insert race, read the winner and apply the same equality/conflict behavior. Each operation owns its connection; do not share a mutable connection across concurrent requests. Use a bounded database command timeout (5 seconds); exhausted busy/locked handling returns 503 with a stable storage_busy code. Other database failures return 500 with a generic error and internal diagnostic logging. Never return Accepted before commit or substitute in-memory success if storage fails.

## Persistence and repository hygiene

Store a schema-versioned table containing ID, unique idempotency key, canonical request, canonicalization version, status Accepted, server UTC creation timestamp, and catalog metadata version. Use parameterized SQL. Initialization creates/migrates the first schema and fails explicitly on unsupported future schema versions. Requests survive close/reopen of the database.

Default database and operational files belong in dotnet/.local-data/, ignored by Git along with SQLite journal/WAL/SHM files. Do not store databases, tokens, or keys in tracked configuration. Existing credentials and local passwords must not be removed. Catalog examples must be unmistakably empty or test-only. Keep deployment, cloud resources, Rust changes, and unrelated repository cleanup out of this feature.

## Acceptance evidence

- Contract/domain tests cover malformed input, unknown properties, required fields, boundary values, timezone handling, dataset availability/coverage, ordered pair semantics, and pool compatibility.
- Temporary file-backed SQLite tests prove persistence after closing/reopening, parameterized storage, failed validation not reserving a key, and no false success on storage failure.
- Concurrent same-key/same-payload submissions create exactly one row and return one ID; same-key/different-payload races produce one winner and conflicts for different content.
- HTTP integration tests prove 201/200/400/404/409/413/415 semantics and stable errors. The busy/locked path verifies bounded failure without invented success.
- Retry tests cover reordered JSON/pools and equivalent timestamp offsets, plus a previously accepted request retried after catalog metadata changes.
- Shipped empty catalog rejects unknown datasets in a real local run. Test fixtures do not prove real research data exists.

## Small coding tasks and file ownership

Execute these sequentially for the first exercise. Review each diff and evidence before the next task; no parallel agent execution is needed yet.

1. Scaffold projects only: create Api, Core, Persistence, and Tests under dotnet/, reference them, add them to MEVana.slnx, and set up ignored local data paths. Do not implement endpoints, install database packages, or add placeholder success responses. Api may contain only a minimal application host without endpoints. Verify build and project listing; do not invent a test just to claim tests passed.
2. Implement Core contracts, catalog validation, and request canonicalization with focused Core tests. Own Core and its test files only.
3. Implement SQLite persistence and atomic idempotency, with file-backed persistence/concurrency tests. Own Persistence and its tests; explicitly include approved project dependency files.
4. Implement the two API endpoints and HTTP integration tests, using the established Core/Persistence contracts. Own Api and its tests. No changes to earlier shared contracts without review.
5. Review the complete feature, run required checks, and document local usage and limitations. No Azure deployment or real research execution is part of acceptance.

Before task 1, create an exact developer-task instruction in AgentEngineering specifying target checkout, permitted paths, command allowlist, expected output, and human review point. Developer-agent tool execution is not implemented by this specification; choose and implement that mechanism deliberately rather than confusing direct assistant edits with a demonstrated autonomous agent run.

## Remaining decisions

Review these proposed limits and contracts with the user before treating the specification as final. The source/provenance of a real dataset, actual research strategy implementation, production authentication, Azure database, messaging, worker lifecycle, and integration with Rust remain future decisions. They do not authorize live trading or assume operational readiness.

Sources checked on 2026-10-07: [Microsoft.Data.Sqlite overview](https://learn.microsoft.com/en-us/dotnet/standard/data/sqlite/), [transactions](https://learn.microsoft.com/en-us/dotnet/standard/data/sqlite/transactions), [database errors and concurrency](https://learn.microsoft.com/en-us/dotnet/standard/data/sqlite/database-errors), [async limitations](https://learn.microsoft.com/en-us/dotnet/standard/data/sqlite/async). The API contract and task sequence above are our proposed design, not requirements from those sources.
