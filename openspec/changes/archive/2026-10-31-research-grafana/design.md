# Design — 2026-10-31 local Grafana research evidence

## Context

The pinned ResearchArtifactStore already validates all four committed experiments, including the v2 summary. The main compose stack and curated Grafana files are outside this slice. See proposal.md and the research-grafana spec.

## Goals / Non-Goals

**Goals:** isolated loopback stack; deterministic export and reload; exact numeric views; a provisioned dashboard checked through Grafana.

**Non-Goals:** changing research contracts or artifacts, provider access, trading, existing Grafana or public hosting.

## Decisions

- Use a second Compose file with its own project, network and named volumes. This avoids the main stack and its Prometheus dependency.
- Generate alphanumeric secrets once into ignored mode-600 `research.env`; provision the data source through Grafana environment expansion. Grafana uses a SELECT-only role.
- The .NET tool loads `ResearchArtifactStore.Load` before opening an output path. It formats invariant SQL literals and emits DELETE/INSERT statements within BEGIN/COMMIT. Rerunning schema and load gives the same rows.
- Relational tables keep atoms as bigint and absent measurements as NULL. Views divide bigint atoms through numeric arithmetic and carry the quote symbol. The dashboard selects one experiment for rejection, profit, shape and probe panels.
- Test against the actual local stack before closeout, then stop containers without removing volumes.

## Risks / Trade-offs

- [Anonymous Grafana Viewer] → Bind only to 127.0.0.1 and require a later authenticated handoff before exposure.
- [Eight-minute capture and missing state] → Show explicit evidence and limitation text, full evaluable counts and scoped shape labels.
- [Provisioning/query incompatibility] → Query every panel through the Grafana data source API and stop on any failure.

## Migration Plan

Start the dedicated stack, load the pinned seed, inspect the dashboard, then stop it. Re-run the loader to refresh the same local rows. Roll back by stopping this stack; named volumes remain until an operator separately removes them.
