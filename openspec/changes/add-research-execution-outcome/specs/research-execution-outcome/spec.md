# 2026-10-28 CDT — Execution outcome delta

## Purpose

Record terminal candidate and reconciliation facts independently from intent and profit estimates, including unranked and rejected coverage.

## ADDED Requirements

### Requirement: Advisory raw outcome
Execution outcomes SHALL retain raw nulls, invalid values and offsets without constructor normalization and SHALL reuse the immutable shared route. They SHALL contain only candidate/outcome facts, integer amounts and defined evidence labels, with no signing material, wallet identifiers, serialized transactions, instructions or trade-authorization fields. A transaction signature is an identifier of a submission, never a transaction payload.

#### Scenario: Safe raw surface
- **WHEN** the public fields and canonical JSON are inspected
- **THEN** only specified outcome facts and identifiers exist; no authority or floating-point fields are exposed

#### Scenario: Raw invalid submission
- **WHEN** invalid, null or non-UTC values are constructed
- **THEN** all values remain unchanged for validation and route lists remain read-only snapshots

### Requirement: Outcome identity and linkage
Outcomes SHALL require schema version 1, valid outcome/idempotency/dataset/metadata/strategy identifiers and defined policy kind. Ranking artifact ID and entry rank SHALL be both absent or both present, with valid ID and rank 1–1000. Experiment result ID SHALL be optional and valid when present. Unranked candidates SHALL remain representable.

#### Scenario: Link validation
- **WHEN** one ranking link field is missing or an identifier/rank is invalid
- **THEN** syntax returns invalid_ranking_link, invalid_identifier, required or invalid_rank

#### Scenario: Exploratory candidate
- **WHEN** both ranking fields are null for any policy
- **THEN** syntax accepts the unranked candidate without inventing a ranking link

### Requirement: Mode and evidence
Mode SHALL be Shadow, Paper or LiveSubmission; evidence SHALL reuse Synthetic, Simulated or Live. Live evidence SHALL occur exactly with LiveSubmission. Synthetic evidence SHALL be restricted to Shadow or Paper. Shadow SHALL never represent a submitted state.

#### Scenario: Evidence matrix
- **WHEN** any mode/evidence pair is submitted for validation
- **THEN** only Shadow/Paper with Synthetic/Simulated or LiveSubmission with Live is accepted; others yield incompatible_evidence

#### Scenario: Unknown enum
- **WHEN** mode, policy, evidence or state is missing or undefined
- **THEN** syntax returns required or invalid_enum

### Requirement: Terminal and submission facts
State SHALL be Rejected, NotSubmitted, Landed, Failed, Dropped or Expired. Rejected/Failed/Dropped/Expired SHALL require reason codes. Landed/Failed/Dropped SHALL require submittedAtUtc and Paper or LiveSubmission mode. Landed/Failed SHALL require landedSlot at least observedSlot; all other states SHALL forbid landedSlot. Non-submitted states SHALL forbid submittedAtUtc. Live submitted states SHALL require a 64–88-character base58 signature, forbidden elsewhere.

#### Scenario: Submission matrix
- **WHEN** terminal state, reason, submission time, mode, landed slot or signature violate these rules
- **THEN** syntax rejects the outcome with required, invalid_identifier, submission_not_allowed, invalid_slot, signature_not_allowed or invalid_signature

#### Scenario: Signature syntax only
- **WHEN** a live submitted outcome has a 64–88-character string from the base58 alphabet
- **THEN** syntax accepts its shape without decoding or querying any service

### Requirement: Candidate timing
Outcomes SHALL require a shared valid closed-cycle route, positive long quote input, nonnegative observed slot and zero-offset UTC detection and decision times. Optional submission and reconciliation times SHALL also be zero-offset UTC. Present detection, decision, submission and reconciliation timestamps SHALL never decrease.

#### Scenario: Timing boundaries
- **WHEN** a required fact is missing, size/slot invalid, any timestamp has an offset or event order decreases
- **THEN** syntax returns required, invalid_size, invalid_count, invalid_utc or non_monotonic_time

#### Scenario: Equal timestamps
- **WHEN** all present timestamps are equal
- **THEN** monotonic validation succeeds without inferring an event

### Requirement: Independent profit and costs
Estimated, simulated and realized net profits SHALL be independent nullable signed-long quote atoms. Synthetic evidence SHALL forbid simulated profit. Realized profit SHALL require LiveSubmission, Landed or Failed state and reconciledAtUtc. Network fee, priority fee and tip SHALL be independent nullable unsigned-long quote atoms and forbidden unless submitted. Constructors, helpers and canonicalization SHALL never derive realized profit from any other value.

#### Scenario: Profit matrix
- **WHEN** forbidden simulated/realized profit or non-submitted costs are supplied
- **THEN** syntax returns profit_not_allowed, required or costs_not_allowed

#### Scenario: Failed live fees
- **WHEN** a reconciled Failed live outcome realizes a negative amount
- **THEN** validation and canonicalization retain that loss independently of estimates, simulation and costs

#### Scenario: No inferred profit
- **WHEN** only estimated or simulated profit is populated
- **THEN** realized profit stays null through construction, helper calls and canonicalization

### Requirement: Outcome catalog semantics
Semantic validation SHALL reject null catalogs and return syntax issues before lookup. Dataset, metadata version and strategy SHALL match ordinally. Route membership, adjacency and first-token quote size limits SHALL use shared ranking semantics. Detection SHALL lie within dataset coverage; VerifiedAvailable SHALL not gate historical validation. Link IDs SHALL not imply lookup or validation of unavailable linked records.

#### Scenario: Catalog checks
- **WHEN** provenance, detection coverage, route membership/adjacency or first-token size is incompatible
- **THEN** validation returns unknown_dataset, metadata_version_mismatch, unsupported_strategy, invalid_window, unknown_token, unknown_pool, incompatible_pool or invalid_trade_size

### Requirement: Canonical outcome and idempotency
Syntax-valid outcomes SHALL emit deterministic compact version-1 JSON with fixed order, exact integers, explicit nulls, invariant seven-digit UTC timestamps and preserved route order. A pure helper SHALL detect equal valid idempotency keys with different canonical bytes, accept identical content and distinguish different keys. Invalid outcomes SHALL not canonicalize or enter conflict comparison.

#### Scenario: Conflict detection
- **WHEN** equal keys carry different canonical content, identical content is repeated, or keys differ
- **THEN** the helper returns conflict, no conflict or no conflict respectively without mutation

#### Scenario: Synthetic golden cases
- **WHEN** dated Synthetic Rejected and Paper Landed fixtures are canonicalized
- **THEN** each matches independently authored bytes; fixtures never constitute market evidence

#### Scenario: Invalid canonicalization
- **WHEN** a null or invalid outcome is canonicalized or compared
- **THEN** ArgumentNullException or ArgumentException is thrown
