# Design — 2026-10-30 CDT

## Context

The API project was an empty host and Persistence had no source. Core contracts, canonicalizers and validators exist and must not change. Request and result parsing existed only in tests.

## Goals / Non-Goals

**Goals:** Serve exactly four validated Simulated experiments, keep evidence kind, quote asset, coverage and null-versus-zero visible, and refuse to run on any integrity failure or non-loopback bind.

**Non-Goals:** Writes, authentication, CORS, deployment, a database, dynamic discovery, synthetic artifacts, ranking artifacts, execution outcomes, or cross-experiment totals.

## Decisions

- Seed files are byte-identical copies of the fixtures plus the handoff 14 summary. Tests compare the copies with the fixtures and the catalog with `CaptureReplayCatalog.CreateSnapshot()`, so duplication cannot drift silently.
- The store validates everything before Kestrel binds, then handlers read only in-memory bytes. No URL value reaches the filesystem, and manifest file names must be plain names.
- Responses are built once with `Utf8JsonWriter`, so bytes are deterministic. `request` and `result` are written raw so clients can recompute `requestSha256`.
- All artifact numbers must be JavaScript-safe integers, so a browser client cannot lose precision.
- Bind safety has two layers: the URL must be a literal loopback http address, and the started server's addresses are rechecked. `localhost` is refused because it depends on name resolution. An explicit Kestrel endpoint overrides `ASPNETCORE_URLS`.
- A middleware returns 405 before routing outcomes matter. A catch-all route would otherwise turn a wrong method on a known path into 404.
- Tests run the real host on port 0. No package is added.

## Risks / Trade-offs

- [No authentication] → Loopback is the only access control. Exposure beyond loopback requires a later handoff with real authentication.
- [Static seed] → New results need a seed change and restart, which keeps every served artifact reviewed.
- [Coverage counts could be mistaken for a verdict] → The API returns integer counts per policy and no verdict field.

## Migration Plan

Additive and local. Removing the API source and seed directory rolls it back. Nothing is deployed.
