# Proposal — 2026-10-29 CDT

## Why

Research replay needs quotes that reproduce on-chain Whirlpool swap accounting. The current floating-point approximations cannot provide that evidence.

## What Changes

- Add integer-only exact-in and exact-out quotes for standard-fee Whirlpool pools, with explicit errors for adaptive fees and missing arrays.
- Decode captured fixed and dynamic tick arrays and evaluate offline golden swaps against observed vault changes.
- Pin the last Apache-2.0 Orca core and macros releases exactly, guard their lockfile versions, and commit a bounded real-case subset.
- Preserve legacy quoting behavior and all dependent crates.

## Capabilities

### New Capabilities

- `whirlpool-quoting`: Exact standard-fee swap quotes and offline evidence of correctness.

### Modified Capabilities

None.

## Impact

`whirlpool-core`, `geyser-capture`, `Cargo.lock`, and dated documentation. Capture processing is offline and trading remains disabled.
