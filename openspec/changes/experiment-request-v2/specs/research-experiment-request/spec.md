# Spec Delta — 2026-10-30 CDT

## ADDED Requirements

### Requirement: V1 behavior is frozen
ExperimentRequest v1 validation, canonicalization, fingerprinting and result linkage SHALL retain their existing behavior and bytes when v2 requests are introduced. Version-2 requests and results SHALL not be accepted through v1-only paths.

#### Scenario: Existing fingerprints
- **WHEN** existing handoff 11 v1 request fixtures are fingerprinted after v2 is introduced
- **THEN** their canonical bytes and SHA-256 fingerprints remain unchanged

#### Scenario: Version separation
- **WHEN** a version-2 result is passed to a v1 request linkage check
- **THEN** it receives request_mismatch or the existing v1 syntax version issue without v1 behavior changing
