# Tasks — 2026-10-31 research Grafana

## 1. Isolated stack and data model

- [x] 1.1 Add loopback-only research Compose stack, port allocations, generated secret scripts and example; verify Compose config and port binds.
- [x] 1.2 Add rerunnable research SQL schema, read-only role grants and exact numeric views; verify reapplication and role access against local Postgres.

## 2. Validated export and loader

- [x] 2.1 Add .NET SQL export tool to the solution, loading through ResearchArtifactStore; verify four experiment and twelve policy rows, nulls, escaping and tamper refusal.
- [x] 2.2 Add seed loader using container psql and document operator commands in a dated runbook; verify two loads give identical queried rows.

## 3. Grafana evidence presentation

- [x] 3.1 Provision SELECT-only data source and six evidence-aware dashboard panel groups; verify dashboard JSON guard test and file provisioning.
- [x] 3.2 Query every panel through anonymous Grafana data source API; verify expected coverage, profits, nulls and scoped diagnostic rows.

## 4. Integration closeout

- [x] 4.1 Run full handoff validation, scope/secret checks and dated closeout; verify stack stopped with named volumes kept.

## Workflow follow-up

- Archive the OpenSpec change after all tracked tasks and validation pass.
- Commit and push the branch without merging.
