# 2026-10-29 — Read-only Geyser capture proposal

## Why

Research needs real public Whirlpool account history before exact quoting and replay can be evaluated. No existing crate records the raw account stream durably.

## What Changes

- Add bounded, read-only Geyser recording, inspection, fixed-port metrics and integrity manifests.
- Derive account filters from the repository IDL and correct the verified TickArray layout error with real decoder fixtures.
- Use the already passed Sodae and Tatum preflights; do not repeat them. Shyft remains blocked.
- Preserve secrets and distinguish synthetic sources and dev/test evidence from production evidence.

## Capabilities

### New Capabilities

- `market-data-capture`: raw market-data acquisition, integrity, gap accounting, decoding and provenance.

### Modified Capabilities

None. DatasetMetadata integration is documented mapping only; existing research contracts do not change.

## Impact

New crates/geyser-capture, workspace membership and lockfile package additions; restricted whirlpool-core types/decode/tests repairs; OpenSpec context/specs, dated docs, capture ignores, allowlist and optional config/ports.toml development metrics key. No service, .NET, UI, infrastructure or trading changes. All Rust dependencies must already be locked. Live subscriptions use the authorized dev/test endpoint for at most ten minutes per run.

## Revision 3 details — 2026-10-29

Provider profiles select endpoint, token and auth header atomically. Sodae uses https://ams.tls.sodae.io:443 and SODAE.IO_API_KEY/x-token; Tatum uses https://solana-mainnet-grpc.gateway.tatum.io:443 and TATUM_MAINNET/x-api-key. Refuse plaintext and endpoint overrides crossing profile hosts; never use Shyft. Read only those two token keys, environment first, without sourcing .env.

Identify the dynamic tick array and 254-byte family by discriminator from current Orca program source when the local IDL is older; capture fixed and dynamic arrays with separate pool offsets. Report dynamic decoding as unsupported if necessary. Position families are excluded.

Revision 3 overrides earlier workflow text: all development is offline; live purposes are only the 60-second filter check, five-minute discovery, ten-minute allowlist and optional bounded kill test. Never subscribe owner-wide. Use Sodae first; Tatum only for filter-check fallback. Maximum five consecutive failed reconnects, capped backoff and hard duration. Total received bytes across the slice must remain below 0.5 GB. After each run record resolved provider IPs, pgrep/lsof evidence of no remaining streams and manifest bytes. Maintain per-provider totals. Stop on rejected filters, exhausted quota or live failures needing code changes.
