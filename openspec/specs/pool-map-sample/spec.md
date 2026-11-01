# pool-map-sample Specification

## Purpose
Provide one bounded public pool-market sample with reproducible requirements, distinct evidence labels and transparent unknowns for later investigation.

## Requirements

### Requirement: Bounded public sample
Collection SHALL finish under 20 minutes using at most 300 public requests and 20 Tatum account calls.

#### Scenario: Limit reached
- **WHEN** a request or deadline limit would be exceeded
- **THEN** collection SHALL stop and report the limit.

### Requirement: Deterministic requirement sequence
The tool SHALL apply R1 through R5 in order using exact money arithmetic and retain the first failure.

#### Scenario: Unknown fee
- **WHEN** combined fees are unavailable
- **THEN** R5 SHALL remain Not measured for review.

### Requirement: Transparent presentation
The store and dashboard SHALL retain Reported and OnChain evidence and unknown values.

#### Scenario: Missing value
- **WHEN** a field is unknown
- **THEN** it SHALL remain NULL and display Not measured.

### Requirement: Pool map coverage and on-chain comparisons
The sample SHALL expand per-token coverage, exclude bonding curves from requirements, and keep on-chain gaps separate from reported gaps.

#### Scenario: Fee evidence
- **WHEN** two pools are compared within 25 slots
- **THEN** R5 SHALL use fees for those same pools and distinguish Measured, MinimumFeeOnly and Not measured outcomes.

#### Scenario: New bounded sample
- **WHEN** the follow-up sample is collected
- **THEN** it SHALL preserve the original run, finish under 10 minutes, use at most 220 public requests and 10 Tatum calls, and record skipped and capped tokens.

### Requirement: Readable pool map evidence
The dashboard SHALL show symbols, readable timestamps, formatted numeric clues, plain mint checks and per-pair deepest and shallowest depth estimates with venues.

#### Scenario: Review
- **WHEN** controls and candidates are displayed
- **THEN** reported gap, on-chain gap, fee kind and full mint identifiers SHALL remain available without truncating the clue cells at 1500 pixels.
