# Spec Delta — 2026-10-30 CDT

## MODIFIED Requirements

### Requirement: Fail-closed artifact loading
The API SHALL load its seed artifacts once before binding and SHALL refuse to start if any document fails strict parsing, canonical byte reproduction, request hash linkage, contract validation, the Simulated evidence rule, the safe integer rule, summary reconciliation, or the compiled SHA-256 pin for any of the eleven approved seed files. Strict parsing SHALL reject unknown, missing and duplicate properties.

#### Scenario: Tampered artifact
- **WHEN** a seed artifact is altered so any check fails
- **THEN** startup throws and nothing is served

#### Scenario: Unsafe number
- **WHEN** an artifact number is non-integer or outside ±(2^53 − 1)
- **THEN** startup throws `unsafe_integer`

#### Scenario: Altered approved seed bytes
- **WHEN** a seed file changes but passes every earlier syntax, contract, linkage and summary check
- **THEN** startup throws `seed_hash_mismatch` for that file before binding
