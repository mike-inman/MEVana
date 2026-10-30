# research-experiment-request-v2 Specification

## Purpose

A version-two research request describes a multi-token route universe and cost-model price inputs so future replay can report its evaluated scope truthfully.

## Requirements

### Requirement: Raw v2 request
ExperimentRequestV2 SHALL preserve raw nulls, invalid values, list order, duplicates, signed-long values and exact DateTimeOffset offsets in defensive read-only copies. It SHALL carry dataset, strategy, window, quote token, token IDs, route-leg pool IDs, route shapes, price-input pool IDs and trade bounds.

#### Scenario: Invalid submission remains inspectable
- **WHEN** a caller supplies null, duplicate or invalid values and later mutates input lists
- **THEN** the request retains the original submitted values without constructor rejection or mutation

### Requirement: V2 syntax
Syntax validation SHALL return deterministic read-only issues. Identifiers SHALL use 1–64 ASCII letters, digits, hyphens or underscores; windows SHALL increase by at most 24 hours; trade bounds SHALL be positive and ordered. Token IDs SHALL be 2–8 distinct IDs including the quote, pool IDs 2–32 distinct IDs, shapes 1–2 distinct values from `two_leg` and `triangle`, and price-input pools 0–4 distinct IDs.

#### Scenario: Required and shared rules
- **WHEN** a request or required field is null, or an identifier, window or bound violates v1's rule
- **THEN** request/required, field/required, invalid_identifier, invalid_window or invalid_trade_size is returned as applicable

#### Scenario: Token and quote rules
- **WHEN** token IDs violate their bounds, uniqueness or syntax, or the quote token is absent from otherwise valid token IDs
- **THEN** invalid_token_ids or invalid_quote_token is returned respectively

#### Scenario: Pools and shapes
- **WHEN** route pool IDs, route shapes or price-input pool IDs violate their bounds, uniqueness or allowed values
- **THEN** invalid_pool_ids, invalid_route_shapes or invalid_price_input_pools is returned respectively

### Requirement: V2 catalog scope
Catalog validation SHALL apply v1 dataset, availability, strategy and coverage checks. All named tokens and pools SHALL exist; route-leg pool tokens SHALL belong to the token universe. Each requested shape SHALL yield a route, every route-leg pool SHALL be used by a route, and trade bounds SHALL not exceed the smaller dataset and quote-token bound.

#### Scenario: Dataset and strategy
- **WHEN** a dataset is unknown or unavailable, strategy unsupported or window outside coverage
- **THEN** unknown_dataset, dataset_unavailable, unsupported_strategy or invalid_window is returned as applicable

#### Scenario: Incompatible members and bound
- **WHEN** a token or pool is unknown, a route-leg pool has an out-of-universe token, or trade size exceeds the quote bound
- **THEN** incompatible_token, incompatible_pool or invalid_trade_size is returned as applicable

#### Scenario: Unused leg or impossible shape
- **WHEN** a named route-leg pool belongs to no route, or a requested shape yields no route
- **THEN** unused_pool or shape_unsatisfiable is returned respectively

### Requirement: Deterministic route universe
A pure enumerator SHALL return an ordinally ordered list of pool sequences and token paths for a valid request. A two-leg route SHALL use distinct same-pair pools containing the quote token. A triangle SHALL use distinct pools and three distinct tokens in quote → X → Y → quote order.

#### Scenario: R1 triangles
- **WHEN** six SOL/USDC, five SOL/USDT and two USDC/USDT pools are requested with quote USDC and triangle only
- **THEN** exactly 120 routes are enumerated, including both directions of every three-pool cycle

#### Scenario: R2 combined shapes
- **WHEN** those pools request both two_leg and triangle with quote USDC
- **THEN** exactly 152 routes are enumerated: 30 SOL/USDC two-leg, 2 USDC/USDT two-leg and 120 triangles

#### Scenario: R3 unused pool
- **WHEN** two_leg alone is requested and a SOL/USDT pool is included with quote USDC
- **THEN** that pool is reported unused_pool

### Requirement: Version-two canonical identity
Canonicalization SHALL require valid v2 syntax and emit compact JSON with canonicalizationVersion 2 and the declared field order. Token IDs, pool IDs, shapes and price-input pool IDs SHALL be sorted ordinally; timestamps SHALL use invariant UTC round-trip format. The SHA-256 fingerprint SHALL hash canonical UTF-8 bytes and carry version 2.

#### Scenario: Golden deterministic bytes
- **WHEN** equivalent valid inputs differ only in list ordering, culture or timestamp offset
- **THEN** canonical bytes and SHA-256 match the golden v2 request for each R1, R2 and R3 scope

#### Scenario: Invalid canonical input
- **WHEN** a null or syntax-invalid request is canonicalized
- **THEN** ArgumentNullException or ArgumentException is thrown respectively

### Requirement: Version-separated result linkage
The v2 result validator SHALL apply existing result and catalog checks, require requestCanonicalizationVersion 2, and compare the v2 fingerprint, dataset, strategy and window with the original request. It SHALL not alter the v1 result path.

#### Scenario: Cross-version mismatch
- **WHEN** a version-1 result is checked against an equivalent v2 request
- **THEN** `request/request_mismatch` is returned by the v2 overload

#### Scenario: Valid v2 linkage
- **WHEN** a result has version 2 and the exact canonical v2 fingerprint and common scope
- **THEN** it passes result linkage validation
