# Spec Delta — 2026-10-30 CDT

## ADDED Requirements

### Requirement: Version-two result path
A separate v2 result syntax check SHALL enforce requestCanonicalizationVersion 2 while applying every other v1 result syntax rule. A separate canonicalizer SHALL emit the same result property order and top-level canonicalizationVersion 1 with requestCanonicalizationVersion 2. The v1 syntax and canonical bytes SHALL remain unchanged.

#### Scenario: Version separation
- **WHEN** a version-one result is checked by the v2 syntax path
- **THEN** unsupported_canonicalization_version is returned

#### Scenario: Canonical linkage
- **WHEN** a v2 result names the exact canonical v2 request hash and matches catalog scope
- **THEN** its canonical bytes validate through the v2 request overload
