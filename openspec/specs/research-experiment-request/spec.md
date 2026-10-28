# research-experiment-request

Date: 2026-10-27 CDT

## Purpose

Raw requests, validation and canonicalization, baselined from current Core code and its existing tests without behavior changes.

## Requirements

### Requirement: Raw request preservation
ExperimentRequest SHALL accept and retain nulls, invalid identifiers, duplicate members, list order, signed-long values and exact DateTimeOffset offsets. Lists SHALL be defensive read-only copies.

#### Scenario: Unvalidated submission
- **WHEN** invalid, duplicate or null values are submitted
- **THEN** they remain available without constructor rejection or normalization

#### Scenario: Snapshot
- **WHEN** a submitted list is subsequently mutated
- **THEN** the request remains unchanged

### Requirement: Syntax validation
Syntax validation SHALL return deterministic read-only ValidationIssue(Field, Code) records. A null request SHALL yield request/required; null fields SHALL yield required. 

#### Scenario: Deterministic errors
- **WHEN** the same invalid request is validated repeatedly
- **THEN** the same read-only issues are returned

### Requirement: Syntax boundaries
Identifiers SHALL use 1–64 ASCII letters, digits, hyphens or underscores. Token pairs SHALL have exactly two distinct valid identifiers; pools SHALL have 2–16 distinct valid identifiers, using ordinal comparisons. Trade bounds SHALL be positive and ordered. Windows SHALL be increasing and at most 24 hours by instant; nonzero offsets SHALL be accepted.

#### Scenario: Invalid fields
- **WHEN** identifiers, pair, pools, present trade bounds or present window violate these rules
- **THEN** invalid_identifier, invalid_token_pair, invalid_pool_ids, invalid_trade_size or invalid_window is returned for the corresponding field

#### Scenario: Missing fields
- **WHEN** all eight request fields are null
- **THEN** eight required issues are returned

#### Scenario: Boundaries
- **WHEN** case-distinct IDs, 16 pools, long.MaxValue bounds, a 24-hour window or nonzero offsets are submitted
- **THEN** syntax succeeds

### Requirement: Catalog validation
Semantic validation SHALL reject null catalogs, return syntax issues before lookup, use ordinal dataset and strategy identity, require VerifiedAvailable and supported strategy, bound the window by coverage, require both tokens and compatible selected pools, and bound trade sizes by the smaller dataset and second-token bound.

#### Scenario: Unknown or unavailable dataset
- **WHEN** syntax is valid but the dataset is absent or unverified
- **THEN** unknown_dataset or dataset_unavailable is returned

#### Scenario: Incompatible request
- **WHEN** strategy, window, tokens, pools or trade sizes are incompatible
- **THEN** unsupported_strategy, invalid_window, incompatible_pair, incompatible_pool or invalid_trade_size is returned

#### Scenario: Unknown pair short circuit
- **WHEN** a token is unknown
- **THEN** incompatible_pair is returned after independent availability, strategy and window checks; pool and trade-size checks are skipped

### Requirement: Version one canonical request
Canonicalization SHALL require valid syntax without catalog lookup and emit compact JSON with fixed property order: canonicalizationVersion, datasetId, strategyVersion, windowStartUtc, windowEndUtc, tokenPair, poolIds, minTradeSizeQuoteAtoms, maxTradeSizeQuoteAtoms. Version SHALL be 1, times invariant round-trip UTC with seven fractional digits and Z, pools sorted ordinally, and token pair order preserved. Integers SHALL retain full signed-long precision.

#### Scenario: Equivalent inputs
- **WHEN** pool ordering, culture or timestamp offsets differ but instants and other values match
- **THEN** canonical content is identical without mutating input

#### Scenario: Changed meaning
- **WHEN** dataset or strategy case, token-pair order, pool membership, window or trade bound changes
- **THEN** canonical content differs

#### Scenario: Invalid canonical input
- **WHEN** the request is null or syntax-invalid
- **THEN** ArgumentNullException or ArgumentException is thrown
