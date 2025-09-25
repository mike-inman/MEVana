# Local Stack Overview

Last updated: 2025-02-14

## Quickstart for the next helper

1. **Read `docs/operations/ADMIN_CONTROL_PANEL.md` first.** It captures the Clerk/Grafana hand-off, the new wallet-label plumbing, and the button flows you must validate before calling local setup “done.”
2. Mirror production labels by setting `WALLET_LABELS` in your local `.env` or the generated `config/environments/executor.env`; without it the per-wallet Prometheus gauges will be useless to Grafana.
3. After you run `scripts/lifecycle/start_local_stack.sh`, hit `http://localhost:9904/metrics` and confirm `mevana_wallet_balance_sol{wallet="dev"}` (or your label) shows a real balance while any backups stay at `0`.

MEVana development now mirrors production: the Rust services run directly on the host while the shared infrastructure (Kafka, Redis, Postgres, Prometheus, Grafana) stays in Docker. This guide walks through the local workflow that matches the self-hosted runner.

## Architecture Summary
- **Host-managed binaries**
  - `hotpath_arb_engine`, `normalizer-service`, `discovery-service`, `executor-service`
  - Built with `cargo build --release` and stored under a local install prefix (defaults to `/opt/MEVana` in production).
- **Docker infrastructure (unchanged)**
  - `infrastructure/docker-compose.server.yml` brings up Kafka, Redis, Postgres, Adminer, Prometheus, Grafana, Kafka UI.
  - External network `mevana_net` keeps container DNS consistent (`kafka:9092`, `redis:6379`, etc.).
- **Observability**
  - Prometheus scrapes host-exposed metrics ports.
  - Grafana points at the Prometheus container and provisions dashboards from `infrastructure/grafana/dashboards/`.

## Quick Start Workflow
1. **Bootstrap a local install prefix (one time)**
   ```bash
   # Optional: create a local prefix without root/systemd
   sudo env INSTALL_PREFIX=$PWD/.local/mevana \
     MEVANA_USER=$USER SYSTEMCTL=false ENV_DIR=$PWD/.local/mevana-env \
     scripts/deployment/bootstrap-host.sh
   ```
   - Populates `config/flash_loan_providers.*.json` under `.local/mevana/config/`.
   - Copies env samples to `./.local/mevana-env/` if you set `ENV_DIR` (defaults to `/etc/mevana`; override when working locally).
   - You can keep using `/etc/mevana` with `sudo` if you prefer a single shared configuration.

2. **Start infrastructure containers**
   ```bash
   docker network create mevana_net 2>/dev/null || true
   docker compose -f infrastructure/docker-compose.server.yml up -d \
     zookeeper kafka redis postgres adminer prometheus grafana kafka-ui
   ```
   Ports:
   - Grafana `http://localhost:3000` (admin/admin123)
   - Prometheus `http://localhost:9090`
   - Adminer `http://localhost:8087`
   - Kafka UI `http://localhost:8084`

3. **Configure environment variables**
   - Shared values → `config/environments/common.env.sample`
   - Hotpath overrides → `config/environments/hotpath.env.sample`
   - Copy to a working location and export before running services:
     ```bash
     cp config/environments/common.env.sample .env.common
     cp config/environments/hotpath.env.sample .env.hotpath
     # edit .env.* with local tokens / port overrides
     export $(grep -v '^#' .env.common | xargs)
     export $(grep -v '^#' .env.hotpath | xargs)
     ```
   - Ensure `DATABASE_URL` points at the compose Postgres instance (`postgresql://mev:mevpassword@localhost:5433/mevdb`).
   - Mirror the production host wiring:
     ```env
     KAFKA_BROKER=localhost:29092
     REDIS_URL=redis://127.0.0.1:6380/0
     METRICS_PORT=8088           # hotpath → readiness automatically binds 8089
     EXECUTOR_METRICS_PORT=9904
     HEALTH_CHECK_PORT=9898
     ```
   - Provide an executor wallet at `infrastructure/keys/id.json` (devnet funds recommended) and copy any other required keys (`jito-keypair.json`, etc.) into the install prefix.

4. **Bootstrap data stores (handled automatically)**
   - `./scripts/lifecycle/start_system.sh` now invokes two idempotent helpers:
     - `scripts/deployment/bootstrap_database.sh` – applies `database/complete_schema.sql` if required and seeds `vault_config`.
     - `scripts/deployment/bootstrap_kafka.sh` – creates the standard topics (`dex-transactions`, `orca-swaps.parsed.v2`, `orca-tickarrays.parsed.v2`, `edge.delta.v1`, `edge.snapshot.v1`, `normalized-swaps.v1`, `unknown-programs`, `arb.opportunities.v1`).
   - If you need to run them manually (e.g., during host provisioning) they can be executed directly and safely re-run as many times as needed.

5. **Run services directly (Cargo)**
   ```bash
   # Hotpath (API on 8089, metrics on 8088)
   cargo run -p hotpath_arb_engine --bin hotpath_arb_engine

   # Normalizer (metrics on 9901)
   cargo run -p normalizer-service --bin normalizer-service

   # Discovery (metrics on 9102)
   cargo run -p discovery-service --bin discovery-service

   # Executor (metrics on 9904)
   cargo run -p executor-service --bin executor-service
   ```
   Each binary pulls configuration from the exported environment variables; logs stream to stdout for quick iteration.

6. **Production-parity installs (optional)**
   - To rehearse the systemd deployment flow locally without root, reuse the release script:
     ```bash
   sudo env INSTALL_PREFIX=$PWD/.local/mevana MEVANA_USER=$USER \
     SYSTEMCTL=false scripts/deployment/release-services.sh hotpath executor
     ```
   - Binaries land in `.local/mevana/bin/` and the script writes the current commit hash to `.local/mevana/.mevana_tag`.
   - Restart your locally managed processes manually after copying binaries.

## Files & Directories to Know
| Path | Purpose |
|------|---------|
| `infrastructure/keys/id.json` | Local executor wallet (copy to `/opt/MEVana/keys/id.json` in production) |
| `config/flash_loan_providers.local.json` | Editable template for flash-loan settings used by hotpath/executor |
| `config/environments/*.env.sample` | Canonical env templates for shared + per-service configs |
| `scripts/deployment/bootstrap-host.sh` | Creates install prefix, env samples, and systemd units |
| `scripts/deployment/release-services.sh` | Builds release binaries and (optionally) restarts services |
| `infrastructure/prometheus.core.yml` | Prometheus scrape config that targets the host services via `host.docker.internal` |

## Health & Diagnostics
- `scripts/ci/post-deployment-diagnostics.sh` works locally; it exercises HTTP health endpoints, Prometheus target status, and logs in to Grafana (admin/admin123) to verify the datasource. With `SYSTEMCTL=false`, systemd checks are skipped.
- Prometheus targets should include `host.docker.internal:8088`, `host.docker.internal:9901`, `host.docker.internal:9102`, and `host.docker.internal:9904` — matching the production scrape configuration.
- Grafana dashboards reside in `infrastructure/grafana/dashboards/`; live reload is easiest by restarting the Grafana container: `docker compose -f infrastructure/docker-compose.server.yml restart grafana`.

## Shutdown & Cleanup
```bash
docker compose -f infrastructure/docker-compose.server.yml down
```
- Stop any `cargo run` processes or host-managed binaries manually.
- Remove the local install prefix if desired: `rm -rf .local/mevana`

## Tips for Fast Iteration
- Use `cargo watch -x "run -p hotpath_arb_engine --bin hotpath_arb_engine"` to rebuild on file changes when testing hotpath logic.
- Keep `config/environments/*.env` under version control with safe defaults but no secrets—source them per terminal session instead of editing system copies so the host and local stacks stay aligned.
- When tweaking executor logic, run it with `EXECUTION_DRY_RUN=true` and tail metrics: `curl -s http://localhost:9904/metrics | head`.
- Before dispatching the GitHub workflow, run `scripts/ci/post-deployment-diagnostics.sh` locally to catch regressions fast.

By following this host-first workflow locally, you stay aligned with production: the exact binaries you compile are the ones systemd runs, while Docker continues to provide the shared observability and data backbone.
