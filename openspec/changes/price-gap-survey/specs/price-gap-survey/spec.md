# Spec Delta

## Purpose

Provide bounded, reproducible Indicative evidence about mid-price gaps between verified public liquidity pools to guide later exact modeling.

## ADDED Requirements

### Requirement: Verified pool universe
The survey SHALL use at most 40 allowlisted public pool accounts, with verified owner programs, token mints, decimals, fee tiers and source provenance.

#### Scenario: Unverified pool
- **WHEN** a pool owner, mint or decoder layout cannot be verified against a real account
- **THEN** the survey SHALL stop before live capture and report the failed verification.

### Requirement: Bounded account capture
The survey SHALL subscribe only to allowlisted pool account updates at Confirmed commitment, with a required byte limit and a duration no greater than 3,600 seconds.

#### Scenario: Budget reached
- **WHEN** the configured byte or duration limit is reached
- **THEN** recording SHALL stop cleanly without leaving a provider stream.

### Requirement: Deterministic indicative analysis
The survey SHALL produce byte-identical output for identical inputs and SHALL label all observations Indicative.

#### Scenario: Missing fee or stale state
- **WHEN** a fee is unknown or the older pool price is more than 25 slots old
- **THEN** unknown net gaps SHALL remain NULL and stale observations SHALL be excluded and counted.

### Requirement: Fixed event and decision rules
The survey SHALL evaluate 5, 10, 25 and 50 bps net-gap events and SHALL apply the predeclared three-run segment gate unchanged.

#### Scenario: Gate evaluation
- **WHEN** a segment has at least three runs
- **THEN** advancement SHALL require at least six 10 bps events per hour, median duration at least two slots, known fees and stale exclusions below 20%.

### Requirement: Honest research presentation
The research tables and dashboard SHALL label all survey data Indicative and SHALL show no profit or executable-trade claim.

#### Scenario: Unknown value
- **WHEN** a measure is unknown
- **THEN** the dashboard SHALL show Not measured, preserving measured zero as zero.
