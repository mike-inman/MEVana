# 2026-10-29 — Read-only Geyser capture proposal

## Why

Research needs real public Whirlpool account history before exact quoting and replay can be evaluated. No existing crate records the raw account stream durably.

## What Changes

- Add bounded, read-only Geyser recording, inspection, fixed-port metrics and integrity manifests.
- Derive account filters from the repository IDL and correct the verified TickArray layout error with real decoder fixtures.
- Require a successful dev/test connectivity preflight before recorder development; a failed preflight ends this slice with a blocker closeout.
- Preserve secrets and distinguish synthetic sources and dev/test evidence from production evidence.

## Capabilities

### New Capabilities

- `market-data-capture`: raw market-data acquisition, integrity, gap accounting, decoding and provenance.

### Modified Capabilities

None. DatasetMetadata integration is documented mapping only; existing research contracts do not change.

## Impact

New crates/geyser-capture, workspace membership and lockfile package additions; restricted whirlpool-core types/decode/tests repairs; OpenSpec context/specs, dated docs, capture ignores, allowlist and optional config/ports.toml development metrics key. No service, .NET, UI, infrastructure or trading changes. All Rust dependencies must already be locked. Live subscriptions use the authorized dev/test endpoint for at most ten minutes per run.
