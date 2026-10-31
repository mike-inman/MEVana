# Proposal — 2026-10-30 CDT

## Why

The Research API validates contracts and aggregate counts, but valid-looking changes to the local seed can still alter profit, units, or coverage without stopping startup. Pinning the exact approved seed bytes closes that evidence-integrity gap before the branch is merged.

## What Changes

- Pin SHA-256 for all eleven committed seed JSON files in the read-only artifact store.
- Verify every pin after the existing load checks so their specific failure codes remain stable.
- Add six independent tamper tests and record the new startup failure code in the closeout.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `research-api`: fail-closed artifact loading also requires the exact approved seed bytes.

## Impact

Only the Research API persistence store, new tests, the Handoff 15 closeout, and OpenSpec change. The seed files, Core contracts, HTTP response shape, Rust, and existing tests remain unchanged.
