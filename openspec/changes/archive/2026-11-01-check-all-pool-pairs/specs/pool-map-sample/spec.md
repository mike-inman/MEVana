# Spec Delta

## ADDED Requirements

### Requirement: All eligible saved-map pairs checked
The sample SHALL plan every pair passing R1 through R3 and every control without a shortlist cap, retaining up to three pools per pair by the existing rule.

#### Scenario: Preflight exceeds budget
- **WHEN** the conservative account plan exceeds 15 RPC calls
- **THEN** collection SHALL stop before any live read without trimming pairs.

#### Scenario: Unchecked state
- **WHEN** a required mint was never requested
- **THEN** the pair SHALL be NotChecked separately from rejection; a requested unreadable mint SHALL remain mint_unreadable.

### Requirement: Bin step label
The sample SHALL retain a nullable gap_within_bin_step label, true when the compared on-chain gap is at or below the largest known bin step of those two pools.

#### Scenario: Exact boundary
- **WHEN** gap equals bin step
- **THEN** the label SHALL be true while R5 and selection rules remain unchanged.
