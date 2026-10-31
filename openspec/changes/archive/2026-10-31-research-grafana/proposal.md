# Proposal — 2026-10-31 research evidence Grafana

## Why

The four validated Stage 0 experiments need a local SQL and Grafana presentation that preserves evidence scope, missing values and quote assets. Handoff 17 establishes a separate loopback research stack without altering the Research API or the trading stack.

## What Changes

- Add an isolated Postgres/Grafana stack with generated local credentials and a read-only Grafana database role.
- Export the pinned seed through `ResearchArtifactStore.Load` into an idempotent transaction, schema and exact numeric views.
- Provision one dashboard with scoped coverage, rejection, policy, shape and diagnostic probe panels.
- Add validation, a runbook and a dated closeout.

## Capabilities

### New Capabilities

- `research-grafana`: Local, read-only presentation of validated research evidence with explicit scope, coverage and null semantics.

### Modified Capabilities

None.

## Impact

New files under `infrastructure/research`, `scripts/research`, `dotnet/src/MEVana.Research.SqlExport`, `docs` and OpenSpec. `config/ports.toml` and `dotnet/MEVana.slnx` add entries. No existing seed, API, Core, Rust, UI, provider or main compose behavior changes.
