# 2026-10-28 CDT — Ranking artifact proposal

## Why

A versioned advisory WATCH artifact must preserve point-in-time training provenance, bounded validity and route monitoring priorities before ranking can be evaluated.

## What Changes

- Add one shared closed-cycle Solana DEX route DTO and validators.
- Add raw ranking artifacts, syntax/catalog/time validation and deterministic canonical JSON.
- Provide contract tests and a dated Synthetic golden fixture; no trading authority.

## Capabilities

### New Capabilities

- `research-ranking-artifact`: advisory WATCH rankings, shared routes, provenance and bounded validity.

### Modified Capabilities

None.

## Impact

Additive Core contracts and tests only. Existing contracts, runtime, API host, persistence and UI remain unchanged. The outcome change will reuse the route type after this change is archived.
