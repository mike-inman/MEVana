# Spec Delta

## ADDED Requirements

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
