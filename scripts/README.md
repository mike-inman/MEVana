MEVana Scripts Guide

Last updated: 2025-09-15

Preferred entrypoints
- Deployment: `scripts/deployment/deploy_vultr_compose.sh` (server compose), `scripts/deployment/build-and-ship.sh` (CI-assisted), `scripts/deployment/setup-live-trading.sh` (ops gating)
- Lifecycle: `scripts/lifecycle/start_system.sh`, `scripts/lifecycle/shutdown_system.sh`, `scripts/lifecycle/stop_system.sh`, `scripts/lifecycle/start_clean.sh`
- Monitoring: `scripts/monitoring/quick_health_check.sh`, `scripts/setup_grafana_production.sh`
- Maintenance: `scripts/maintenance/docker-data-purge.sh`, `scripts/maintenance/deploy-rollback.sh`, `scripts/maintenance/backup_postgres.sh`
- Testing: `scripts/testing/*` (observability, E2E, validation)

Local development helpers
- `scripts/development/local-cargo-check.sh` — Runs `cargo +nightly check --workspace` to surface compile errors fast without Docker.
- `scripts/development/build-images-locally.sh [tag]` — Builds Linux images for the 4 core app services using multi-stage Dockerfiles (ensures no exec-format issues). Defaults tag to the current commit SHA short hash.

Legacy/Deprecated (use GHCR workflow + compose deploy instead)
- scripts/simple-deploy.sh
- scripts/docker-deploy.sh
- scripts/rapid_deploy.sh
- scripts/deploy-multi-stream.sh, scripts/deploy-multi-stream-FUTURE.sh
- scripts/deploy-single-stream.sh
- scripts/deploy-to-server.sh

Notes
- Infrastructure dev helpers also exist under `infrastructure/scripts/` (for local/dev only).
- Prefer path-agnostic commands (use `git rev-parse --show-toplevel` and `-C`).
