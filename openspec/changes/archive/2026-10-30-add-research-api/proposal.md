# Proposal — 2026-10-30 CDT

## Why

Validated research results exist only as fixtures and local replay directories. The MEVana-UI research page needs a truthful read-only source that cannot present weak evidence as stronger than it is.

## What Changes

- Add a loopback-only, read-only API in `MEVana.Research.Api` serving one dataset and four Simulated experiments.
- Add a validated in-memory artifact store in `MEVana.Research.Persistence`, with strict parsers that fail closed at startup.
- Add committed seed artifacts in `dotnet/data/research-artifacts/` and reserve port 8098.

## Capabilities

### New Capabilities

- `research-api`: Read-only, loopback-only serving of validated research artifacts with evidence, quote-asset and coverage labels.

### Modified Capabilities

None. Core contracts are unchanged.

## Impact

New .NET source and tests, one test-project reference edit, one `config/ports.toml` line and OpenSpec. No Core, Rust, MEVana-UI, Docker, database, authentication, provider or trading change.
