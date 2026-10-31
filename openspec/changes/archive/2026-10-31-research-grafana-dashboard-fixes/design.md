# Design — 2026-10-31 research dashboard display fixes

## Context

The Handoff 17 schema and API values passed, but a browser showed rendering faults. The provisioned dashboard is a static JSON file; its SQL panels and field display options determine the visible meaning.

## Goals / Non-Goals

**Goals:** readable, precise browser presentation; select a universe by default; explicit v1 absence; selective Grafana secrets; second-checkout compatibility.

**Non-Goals:** changing seed values, contract validation, trading, provider access, public hosting, or the existing Grafana stack.

## Decisions

- Use a horizontal bar chart for coverage. It consumes one SQL row per experiment and policy directly, avoiding a bar gauge reduction. Pair it with a full-width table that formats counts as display strings while views stay numeric.
- Replace the time-based heatmap with a table grouped by pool and direction, showing one column per missing-state reason. A one-row SQL fallback explains v1 absence. Shape coverage uses the same fallback pattern.
- Use field overrides for distinct fixed neutral rejection colors; reserve the dashboard's height and width for full labels and twelve coverage rows.
- Apply `trim_scale` in the numeric views. SQL table queries append quote symbols to exact numeric text and use two-decimal share formatting.
- Pass selective environment variables to Grafana through Compose interpolation with the script's explicit research env file. Existing Postgres and Grafana volumes keep their initialized superuser/admin passwords; the loader resets the reader role password to the checkout's generated value.

## Risks / Trade-offs

- [Variable provisioning may reset selection] → Verify the default through the anonymous dashboard API and in a rendered browser.
- [Second checkout uses fresh local secrets] → Verify loading against retained volumes, state that the Grafana admin password remains from the first initialization, and do not delete volumes.
- [Display formatting obscures raw types] → Retain exact bigint/numeric views and test exact values before formatting.

## Migration Plan

Reprovision the same local dashboard and data source by restarting the isolated stack. Reapply the rerunnable schema and seed, validate both universe and v1 panels, then stop containers while keeping volumes.
