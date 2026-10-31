# Proposal — 2026-10-31 research dashboard display fixes

## Why

Handoff 17's database values are correct, but browser review found panels that collapse rows, hide missing-state data, and format evidence poorly. These display failures can misrepresent the Stage 0 result despite successful API queries.

## What Changes

- Render all 12 coverage rows with readable scopes and precise shares, and make the universe experiment the default.
- Replace the time-based probe heatmap with a scoped, time-free diagnostic table; explain absent v1 shape and probe data.
- Format counts, route shapes and exact numeric profit; distinguish rejection reasons with neutral fixed colors.
- Limit Grafana's environment to its two needed secrets and verify a second checkout against retained volumes.
- Extend static/live checks and update the runbook and closeout.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `research-grafana`: Strengthen dashboard display and local stack secret boundaries while preserving its evidence rules.

## Impact

Research Compose, schema views, dashboard JSON, research validation scripts, repository hygiene, runbook and closeout. No API, Core, seed, Rust, UI or main compose changes.
