# Spec Delta — 2026-10-30 CDT

## Purpose

Offline multi-route replay evaluates the declared version-two route universe and reports reproducible, cost-adjusted simulated comparisons with explicit executable-state coverage.

## ADDED Requirements

### Requirement: Parity-locked route universe
V2 replay SHALL enumerate the same ordered routes as the version-two request contract: 120 triangles and 32 two-leg routes for the handoff 12 standard-fee allowlist and USDC quote. Every route SHALL be exact-in through its ordered token path, with the first failed leg determining route failure.

#### Scenario: Cross-language parity
- **WHEN** the committed full R2 route fixture is compared with Rust and .NET enumeration
- **THEN** both ordered 152-route lists match byte for byte

#### Scenario: Leg failure
- **WHEN** any leg fails during exact-in route quotation
- **THEN** that leg's reason is reported without quoting later legs

### Requirement: Shape-specific integer economics
V2 replay SHALL charge 400,000 assumed compute units for two legs and 600,000 for triangles, retaining base, priority and tip lamports and SOL-to-USDC reference conversion. Estimated and later-slot simulated net SHALL stay separate. Shape-specific cached costs SHALL not cross shapes.

#### Scenario: Cost distinction
- **WHEN** a two-leg route and triangle use the same boundary and reference pool
- **THEN** their cost conversion uses their respective compute-unit assumptions

### Requirement: Whole-universe policy decisions
FixedUniverse, SimpleRules and heuristic ModelRanking SHALL act on the same 152-route boundary universe. Pool conflicts SHALL include every leg pool; tie-breaks SHALL compare ordered pool sequences and preserve the existing two-leg ordering.

#### Scenario: Third-leg conflict
- **WHEN** a triangle's third pool conflicts with another selected route
- **THEN** the applicable policy rejects the conflicting route with pool_conflict

### Requirement: Offline v2 outputs and coverage
V2 profiles SHALL accept only format-v2 capture, refuse output overwrite, and emit canonical request/result, candidate JSONL, shape and pool-direction coverage summaries, and a run manifest. Each policy and shape SHALL report evaluated, detected, selected, rejections and separate estimated/simulated totals. Pool-direction attempts and reason outcomes SHALL include independent later-leg availability checks.

#### Scenario: Accounting
- **WHEN** a v2 run completes
- **THEN** each shape's buckets sum to its evaluated count and shape totals sum to policy totals, otherwise replay fails

#### Scenario: Ineligible input
- **WHEN** a v2 profile receives v1 capture or an existing output directory
- **THEN** it refuses the run without replacing prior results

### Requirement: Regression-gated evidence
No v2 result SHALL be reported until the route fixture and all three gates pass: v1 output byte parity against handoff 12 except manifest commit, two-leg per-size quote parity at each boundary, and byte-identical repeated v2 default outputs.

#### Scenario: Gate failure
- **WHEN** any parity or determinism comparison differs
- **THEN** work stops and reports the difference without changing the baseline
