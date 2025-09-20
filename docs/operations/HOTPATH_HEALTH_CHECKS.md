# Hotpath Health Endpoint Rollout

Created: 2025-09-19

## What Changed
- `/health/live`, `/health/ready`, and `/health/summary` now ship from the auxiliary Axum server (metrics port + 1, default 8089).
- Docker health checks hit `/health/ready`, and port 8089 is published so local diagnostics can curl the new endpoints.
- `scripts/ci/post-deployment-diagnostics.sh` checks the readiness endpoint; keep this in the validation loop after each compose deploy.

## Next Helper TODOs
1. **Prometheus alerts** &mdash; add a scrape/blackbox rule so `/health/ready` failures alert operations.
2. **Dependency coverage** &mdash; extend `evaluate_database_health` to include Redis and Kafka connectivity once clients expose lightweight probes.
3. **Geyser observability** &mdash; surface stream lag (current slot vs. latest processed) in metrics, then wire into Grafana for the user’s “beautiful charts” goal.

This note is intentionally short: keep appending here as we iterate so future helpers land on the most recent breadcrumbs.
