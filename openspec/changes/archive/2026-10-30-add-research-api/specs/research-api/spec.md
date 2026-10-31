# Spec Delta — 2026-10-30 CDT

## Purpose

The Research API serves validated research artifacts read-only on loopback, keeping evidence strength visible to its clients.

## ADDED Requirements

### Requirement: Fail-closed artifact loading
The API SHALL load its seed artifacts once before binding and SHALL refuse to start if any document fails strict parsing, canonical byte reproduction, request hash linkage, contract validation, the Simulated evidence rule, the safe integer rule, or summary reconciliation. Strict parsing SHALL reject unknown, missing and duplicate properties.

#### Scenario: Tampered artifact
- **WHEN** a seed artifact is altered so any check fails
- **THEN** startup throws and nothing is served

#### Scenario: Unsafe number
- **WHEN** an artifact number is non-integer or outside ±(2^53 − 1)
- **THEN** startup throws `unsafe_integer`

### Requirement: Contract-separated request parsing
A v1 request document SHALL carry `canonicalizationVersion` 1 and a v2 document SHALL carry 2. Neither parser SHALL accept the other's document.

#### Scenario: Wrong parser
- **WHEN** a v2 document is given to the v1 parser, or the reverse
- **THEN** parsing fails with `contract_version_mismatch`

### Requirement: Read-only HTTP surface
The API SHALL expose only GET and HEAD on `/health`, `/research/v1/datasets`, `/research/v1/datasets/{datasetId}`, `/research/v1/experiments` and `/research/v1/experiments/{resultId}`. Every other method SHALL return 405, and an unknown path or id SHALL return 404 with body `{"code":"not_found"}` that does not echo the id. Request logs SHALL contain only method, path template and status.

#### Scenario: Write attempt
- **WHEN** a client sends POST, PUT, PATCH or DELETE to any path
- **THEN** the response is 405 with `Allow: GET, HEAD`

#### Scenario: Malformed id
- **WHEN** a client requests a malformed or unknown experiment id
- **THEN** the response is the plain 404 body, and no filesystem access occurs

### Requirement: Evidence stays visible
Every experiment SHALL report its evidence kind, quote token, quote decimals and per-policy coverage. Coverage SHALL count `state_gap`, `insufficient_state` and `tick_array_absent` rejections as unevaluable, using integers only. Unmeasured `missedCandidates` and realized profit SHALL remain null. The API SHALL NOT sum or rank profit across experiments, and SHALL NOT return a verdict field.

#### Scenario: Different quote assets
- **WHEN** experiments are listed
- **THEN** each reports its own quote asset, and no cross-experiment total exists

#### Scenario: Not measured
- **WHEN** a policy has no measured missed count or realized profit
- **THEN** the served value is JSON null, not 0

#### Scenario: Diagnostic probes
- **WHEN** the v2 experiment's pool-direction rows are served
- **THEN** they are labeled `diagnosticProbe: true`

### Requirement: Verifiable artifacts
An experiment detail SHALL serve `request` and `result` byte-unchanged, and repeated starts on the same seed SHALL return byte-identical responses.

#### Scenario: Hash recomputation
- **WHEN** a client hashes the served `request` value with SHA-256
- **THEN** it equals `requestSha256`

### Requirement: Loopback-only hosting
The default address SHALL be `http://127.0.0.1:8098`. Startup SHALL fail unless the configured address is a literal loopback http address. With no authentication, loopback SHALL be the only access control, and exposure beyond loopback SHALL require a later change that adds real authentication.

#### Scenario: Non-loopback bind
- **WHEN** the configured address is `0.0.0.0`, a wildcard, a public address or a hostname
- **THEN** startup fails

### Requirement: Restricted seed scope
The seed SHALL contain exactly the three handoff 12 snapshot pair results and the handoff 14 universe result. Superseded handoff 11 results, synthetic artifacts, ranking artifacts and execution outcomes SHALL NOT be served.

#### Scenario: Served experiments
- **WHEN** the experiment list is requested
- **THEN** exactly those four results are returned, ordered ordinally by `resultId`
