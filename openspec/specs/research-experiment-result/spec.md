# research-experiment-result Specification

Date: 2026-10-27 CDT

## Purpose
Define reproducible experiment result evidence and fair policy comparisons with explicit provenance and independent profit quantities.

## Requirements

### Requirement: Raw immutable submissions
Result, policy and rejected-count DTOs SHALL retain nulls, undefined enum values, duplicates, invalid text, offsets, order and signed quantities without validation or normalization. Collections SHALL be defensive read-only snapshots.

#### Scenario: Raw input
- **WHEN** invalid values or duplicate and null members are submitted
- **THEN** construction preserves them for validation and subsequent source-list mutation has no effect

### Requirement: Identity and provenance
Results SHALL require valid result, dataset, metadata, strategy, engine and cost-model identifiers of 1–64 ASCII letters, digits, hyphens or underscores; creation time SHALL have zero UTC offset. Request provenance SHALL carry canonicalization version 1 and a 64-character lowercase hexadecimal SHA-256 hash of UTF-8 canonical request content.

#### Scenario: Missing or malformed provenance
- **WHEN** any required field is missing, invalid, has a nonzero creation offset, a malformed hash or unsupported canonicalization version
- **THEN** syntax validation returns field-specific required, invalid_identifier, invalid_utc, invalid_hash or unsupported_canonicalization_version issues

#### Scenario: Request binding
- **WHEN** a fingerprint is generated from a request or a result is validated against a supplied request
- **THEN** the hash is SHA-256 of canonical request content; differing hash, dataset, strategy or window returns request_mismatch and invalid request syntax returns invalid_request

### Requirement: Evidence and terminal status
Evidence SHALL explicitly be Synthetic, Simulated or Live, and status SHALL be Completed, Failed or Unavailable. Failed and Unavailable results SHALL require a valid error code and contain no policy metric entries. Completed results SHALL have no error code.

#### Scenario: Terminal states
- **WHEN** a failed or unavailable result has metrics or lacks an error code, or a completed result has an error code
- **THEN** syntax validation rejects it with metrics_not_allowed, required or error_not_allowed

#### Scenario: Enum validation
- **WHEN** evidence or status is missing or undefined
- **THEN** syntax validation returns required or invalid_enum

### Requirement: Independent signed profit quantities
Each policy SHALL retain independent nullable estimated, simulated and realized net profits in quote atoms as signed longs, including negative values. Realized profit SHALL be null unless evidence is Live; simulated profit SHALL be null for Synthetic. Constructors and canonicalization SHALL never copy a profit field into another.

#### Scenario: Evidence restrictions
- **WHEN** synthetic evidence has simulated or realized profit, or simulated evidence has realized profit
- **THEN** syntax validation returns profit_not_allowed

#### Scenario: Independent profits
- **WHEN** permitted profits are null, negative, zero or signed-long extremes
- **THEN** validation accepts them and canonicalization preserves each value and null independently

### Requirement: Complete common-scope comparison
Completed results SHALL contain exactly one FixedUniverse, SimpleRules and ModelRanking policy entry. Dataset, window and cost-model version SHALL exist only at the enclosing result scope. The window SHALL be increasing, at most 24 hours and zero-offset UTC.

#### Scenario: Comparison completeness
- **WHEN** policies are absent, duplicated, null, undefined or missing a kind
- **THEN** validation returns required, invalid_policy_set, invalid_enum or missing_policy

#### Scenario: Shared window
- **WHEN** a result window is missing, non-UTC, reversed, empty or longer than 24 hours
- **THEN** syntax validation returns required, invalid_utc or invalid_window

### Requirement: Candidate accounting
Each completed policy SHALL require nonnegative evaluated, detected and selected-or-executed counts and a rejection list with unique valid reason codes and nonnegative counts. Missed count SHALL be null when unknown or nonnegative when known. Total rejected plus selected-or-executed SHALL not exceed evaluated, with overflow-safe validation.

#### Scenario: Accounting boundaries
- **WHEN** counts are missing or negative, reasons invalid or duplicate, or rejection plus selection exceeds evaluated
- **THEN** syntax validation returns required, invalid_count, invalid_identifier, duplicate_reason or candidate_count_exceeded

#### Scenario: Valid accounting
- **WHEN** rejections plus selections equal evaluated including long.MaxValue, or missed is unknown
- **THEN** validation succeeds without integer overflow

### Requirement: Catalog semantics
Semantic validation SHALL return syntax issues before catalog lookup and reject a null catalog. Dataset identity, MetadataVersion and supported strategy SHALL match ordinally, with the window within dataset coverage. Historical result validation SHALL not depend on current VerifiedAvailable.

#### Scenario: Catalog mismatches
- **WHEN** the dataset is unknown, metadata version differs, strategy is unsupported or window exceeds coverage
- **THEN** validation returns unknown_dataset, metadata_version_mismatch, unsupported_strategy or invalid_window

#### Scenario: Syntax first
- **WHEN** the raw result is invalid
- **THEN** only its syntax issues are returned as immutable deterministic ValidationIssue records

### Requirement: Version one canonical results
Syntax-valid results SHALL canonicalize without catalog access to deterministic compact UTF-8 JSON, version 1, fixed property order, invariant UTC round-trip timestamps and exact integer/null values. Policies SHALL sort by enum declaration order and rejections by ordinal reason code. The dated golden fixture SHALL be Synthetic with synthetic-prefixed result and dataset IDs.

#### Scenario: Equivalent ordering
- **WHEN** policy or rejection input ordering or culture changes
- **THEN** canonical bytes are identical and raw input remains unchanged

#### Scenario: Invalid canonical input
- **WHEN** a null or syntax-invalid result is canonicalized
- **THEN** ArgumentNullException or ArgumentException is thrown

#### Scenario: Synthetic fixture
- **WHEN** the version-1 golden fixture is inspected
- **THEN** its bytes match canonical output, it is explicitly Synthetic and it is never presented as market evidence
