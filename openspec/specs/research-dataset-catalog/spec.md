# research-dataset-catalog

Date: 2026-10-27 CDT

## Purpose

Local dataset catalog, baselined from current Core code and its existing tests without behavior changes.

## Requirements

### Requirement: Identifiers and token bounds
Identifiers SHALL contain 1–64 ASCII letters, digits, hyphens or underscores, with ordinal identity. Tokens SHALL have decimals from 0 through 18 and positive signed-long research bounds.

#### Scenario: Invalid token metadata
- **WHEN** a token has a null, blank, non-ASCII, dotted, slashed or overlength identifier, decimals outside 0–18, or a nonpositive bound
- **THEN** construction throws an ArgumentException

#### Scenario: Boundary metadata
- **WHEN** identifiers differ only by case, or bounds equal long.MaxValue
- **THEN** they remain distinct and valid

### Requirement: Dataset metadata
Datasets SHALL require valid IDs and metadata versions, nonblank provenance and suitability without slash, backslash or ://, strictly increasing zero-offset UTC coverage, at least one unique strategy, at least two unique tokens and pools, and a positive dataset bound. Pools SHALL reference exactly two distinct known tokens.

#### Scenario: Invalid catalog structure
- **WHEN** metadata, coverage, collection size, uniqueness or references violate these rules
- **THEN** construction throws an ArgumentException

#### Scenario: Unverified datasets
- **WHEN** VerifiedAvailable is false
- **THEN** the dataset is retained without claiming availability

### Requirement: Immutable catalog snapshots
Catalog, dataset and pool constructors SHALL defensively copy collections preserving order and expose read-only snapshots; null collections and null members SHALL be rejected. Catalogs MAY be empty and lookup SHALL be ordinal and return null when absent.

#### Scenario: Snapshots
- **WHEN** source lists are mutated after construction
- **THEN** stored metadata remains unchanged and exposed lists reject mutation

#### Scenario: Lookup
- **WHEN** an empty catalog or differently cased ID is queried
- **THEN** no dataset is returned
