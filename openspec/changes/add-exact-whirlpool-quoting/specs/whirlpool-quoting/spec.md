# Spec Delta — 2026-10-29 CDT

## Purpose

Provide reproducible integer Whirlpool swap quotes for captured standard-fee pools and verify them against public on-chain swap outcomes.

## ADDED Requirements

### Requirement: Exact standard-fee quotes
The system SHALL quote exact-input and exact-output swaps in integer token atoms using the complete observed pool and tick-array state, returning input, output, fee, and available next pool state.

#### Scenario: Both directions
- **WHEN** a standard-fee pool has all required tick arrays and either swap direction is requested
- **THEN** the quote follows initialized ticks with integer arithmetic and returns a deterministic result

### Requirement: Explicit unsupported and incomplete-state errors
The system SHALL refuse adaptive-fee pools and missing tick arrays with distinct typed errors.

#### Scenario: Adaptive fee
- **WHEN** a pool's fee-tier index differs from its tick spacing
- **THEN** the quote returns `AdaptiveFeeUnsupported`

#### Scenario: Missing array
- **WHEN** an array needed by the swap is absent
- **THEN** the quote returns a missing-array error instead of assuming zero liquidity

### Requirement: Captured tick-array decoding
The system SHALL decode fixed and dynamic tick arrays into one quote-facing representation and reject malformed dynamic lengths.

#### Scenario: Real dynamic account
- **WHEN** captured dynamic-array bytes have a valid discriminator and length of 148 plus 112 times an integer from 0 through 88
- **THEN** initialized ticks and pool identity are decoded for quoting

#### Scenario: Invalid dynamic length
- **WHEN** the dynamic-array length does not fit the declared layout
- **THEN** decoding fails

### Requirement: Offline golden evaluation
The system SHALL extract candidate swaps only from existing captures, require a successful single-allowlisted-pool transaction and a continuous observed pre-state, and classify unusable or nonmatching cases.

#### Scenario: Exact observed accounting
- **WHEN** a candidate has pre-state, sufficient arrays and observed vault deltas
- **THEN** both quote modes are evaluated and a pass requires one mode to reproduce both vault amounts exactly, with post-state compared when available

#### Scenario: Scarcity or mismatch threshold
- **WHEN** fewer than 200 cases are evaluable, fewer than five cross a tick, or unexplained mismatches exceed one percent
- **THEN** evaluation stops and reports counts and examples without tuning the math

### Requirement: Bounded reproducible evidence
The system SHALL retain a versioned offline corpus outside Git and commit at most 256 KB compressed of real public swap cases covering both directions, five standard-fee pools across tick spacings, and five tick crossings.

#### Scenario: Golden regression
- **WHEN** the committed golden tests run
- **THEN** at least twenty cases reproduce observed vault amounts exactly and identify themselves as quote-correctness evidence only

### Requirement: License boundary
The system SHALL resolve Orca core and macros only at version 1.0.4 with default float features disabled and fail validation if any locked `orca_*` package has another version.

#### Scenario: Lockfile drift
- **WHEN** `Cargo.lock` contains an `orca_*` package at any version other than 1.0.4
- **THEN** the license guard test fails
