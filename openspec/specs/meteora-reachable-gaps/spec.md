# meteora-reachable-gaps Specification

## Purpose
Provide bounded, reproducible read-only DLMM reserve walks and honest modeled reachable-gap outcomes without trading authority.

## Requirements

### Requirement: Bounded reachable-gap sample
The tool SHALL preflight before live reads and preserve older samples, using zero public requests, zero streams, at most 15 Tatum account calls, two rounds 120 seconds apart and under five minutes.
#### Scenario: Over-budget plan
- **WHEN** the upper bound exceeds 15 calls
- **THEN** the tool SHALL refuse before any read.

### Requirement: Verified modeled liquidity
The tool SHALL decode IDL-verified DLMM fields and walk real reserves with integer atoms and full modeled variable fees for $100 and $1,000 in both pool orders.
#### Scenario: Missing liquidity coverage
- **WHEN** required state or nearest liquidity is unavailable
- **THEN** the outcome SHALL be Not measured.

### Requirement: Honest reachable classification
A result SHALL be Reachable only when modeled output strictly exceeds input in the same round with slot spread at most 25; Indicative results SHALL never count as Reachable.
#### Scenario: Boundary
- **WHEN** output equals input
- **THEN** the result SHALL be NotReachable.

### Requirement: Reproducible evidence
The tool SHALL retain full pool identifiers, fee and average-price details, empty-bin distances and timestamp semantics, and reproduce identical JSON bytes offline.
#### Scenario: Review
- **WHEN** a repeated nominal gap cannot be explained from the sampled state
- **THEN** its cause SHALL remain unknown.
