# 2026-10-28 CDT — Ranking artifact delta

## Purpose

Define advisory Solana DEX WATCH rankings with point-in-time provenance and a shared route shape, without any authority to trade.

## ADDED Requirements

### Requirement: Advisory raw contract
Ranking artifacts SHALL be advisory WATCH records with no signing material, wallet identifiers, serialized transactions, instructions or trade-authorization fields, and no simulated or realized profit fields. Raw DTOs SHALL retain invalid/null values and immutable snapshots of ordered collections. All amounts and scores SHALL be integer types.

#### Scenario: Safe contract surface
- **WHEN** the public DTO and canonical JSON fields are inspected
- **THEN** only the specified advisory fields exist; no TRIGGER or trading authority is exposed

#### Scenario: Raw snapshots
- **WHEN** invalid values, offsets, null members or duplicate entries are submitted and source lists are mutated
- **THEN** submitted values and order remain unchanged for validation

### Requirement: Shared ordered cycle
Both rankings and execution outcomes SHALL use one route shape: 3–5 ordered token IDs forming a closed cycle and 2–4 distinct ordered pool IDs, with pools count equal to tokens count minus one. Token and pool identifiers SHALL follow catalog identifier rules. Syntax SHALL check shape without catalog lookup.

#### Scenario: Route shape
- **WHEN** tokens are not a closed cycle, counts mismatch, IDs are invalid, or a pool repeats
- **THEN** syntax returns required, invalid_identifier or invalid_route

#### Scenario: Ordered direction
- **WHEN** a syntactically valid route is reversed or rotated
- **THEN** its ordered token and pool sequences remain significant and are not sorted

### Requirement: Ranking provenance
Artifacts SHALL require catalog-style artifact, strategy, feature-set, dataset and metadata identifiers, schema version 1, defined policy and evidence kinds, increasing zero-offset UTC training times, and optional valid source experiment result ID. Model version SHALL be required only for ModelRanking and null otherwise.

#### Scenario: Provenance validation
- **WHEN** required fields are missing, IDs invalid, schema unsupported, enums undefined, times non-UTC or window not increasing
- **THEN** syntax returns field-specific required, invalid_identifier, unsupported_schema_version, invalid_enum, invalid_utc or invalid_window

#### Scenario: Policy model binding
- **WHEN** ModelRanking lacks a valid model or another policy supplies one
- **THEN** syntax returns required, invalid_identifier or model_not_allowed

### Requirement: Point in time validity
Artifacts SHALL require zero-offset UTC as-of, valid-from and expiry timestamps and nonnegative observed slot. Training end SHALL not exceed as-of; valid-from SHALL not precede as-of; expiry SHALL be after valid-from and within 24 hours. A validator accepting nowUtc SHALL reject a non-UTC clock and return not_yet_valid before valid-from or expired at or after expiry.

#### Scenario: Validity boundaries
- **WHEN** training uses future data, validity precedes as-of, TTL is nonpositive or exceeds 24 hours, or slot is negative
- **THEN** syntax rejects the artifact

#### Scenario: Clock check
- **WHEN** now is before valid-from, at valid-from, just before expiry or at expiry
- **THEN** validation reports not_yet_valid, success, success or expired respectively; a non-UTC clock yields invalid_utc

### Requirement: Ranked entries and integer metrics
Artifacts SHALL contain 1–1000 non-null entries with unique contiguous ranks from 1 and no duplicate ordered routes. Each entry SHALL require positive ordered long quote-atom bounds, priority 0–100, confidence 0–10000 basis points and nonnegative long sample count. Optional calibrated success probability SHALL independently be 0–10000; optional estimated net value SHALL be signed long.

#### Scenario: Entry validation
- **WHEN** size, rank, collection, priority, basis points, sample count or route uniqueness violates its bound
- **THEN** syntax returns required, invalid_entries, invalid_rank, duplicate_route, invalid_size, invalid_priority, invalid_basis_points or invalid_count

#### Scenario: Independent scores
- **WHEN** probability is absent or differs from confidence and net value is negative
- **THEN** validation and canonicalization preserve each value without copying confidence into probability

### Requirement: Catalog route semantics
Semantic validation SHALL reject null catalogs, return syntax errors before lookup, and require exact ordinal dataset, metadata version and supported strategy. Training SHALL fit coverage. Every route token and pool SHALL exist, each pool SHALL contain both adjacent tokens, and maximum size SHALL not exceed the dataset and first-token quote bounds. VerifiedAvailable SHALL not gate historical validation.

#### Scenario: Catalog mismatch
- **WHEN** dataset, version, strategy, training coverage, route membership, adjacency or quote size is incompatible
- **THEN** validation returns unknown_dataset, metadata_version_mismatch, unsupported_strategy, invalid_window, unknown_token, unknown_pool, incompatible_pool or invalid_trade_size

#### Scenario: First token quote
- **WHEN** the first-token maximum differs from the second-token maximum
- **THEN** validation uses the first-token bound, including at the exact allowed boundary

### Requirement: Version one canonical ranking
Syntax-valid artifacts SHALL produce compact deterministic version-1 JSON with fixed property order, explicit nulls, exact integers and invariant seven-digit UTC times. Entries SHALL sort by rank while route order stays intact. Invalid inputs SHALL not canonicalize. A dated Synthetic fixture SHALL use synthetic-prefixed identity and provenance IDs and SHALL never be described as market evidence.

#### Scenario: Canonical equivalence
- **WHEN** only entry ordering or culture differs
- **THEN** canonical UTF-8 bytes are identical without mutating input

#### Scenario: Golden fixture
- **WHEN** the independently authored Synthetic fixture is serialized
- **THEN** canonical bytes match exactly; malformed or null artifacts throw ArgumentException or ArgumentNullException
