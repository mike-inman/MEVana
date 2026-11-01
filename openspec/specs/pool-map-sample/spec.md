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
