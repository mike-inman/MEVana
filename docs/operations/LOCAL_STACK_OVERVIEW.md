# Local Stack Overview

Last updated: 2025-09-22

This guide documents the reference local deployment that mirrors the self-hosted production stack. It explains the architecture, prerequisites, and the workflow used to assemble the full containerized environment that powers Prometheus/Grafana monitoring and the Solana trading services.

## Architecture Summary
- **Docker Compose core file**: `infrastructure/docker-compose.core.yml`
  - Infrastructure: Zookeeper, Kafka, Redis (+ Redis Insight), Postgres (+ Adminer), Prometheus, Grafana.
  - Trading services: `hotpath-arb-engine`, `normalizer-service`, `discovery-service`, `executor-service`.
- **Shared network**: `mevana_net` external bridge so services resolve with container DNS (e.g. `kafka:9092`).
- **Configuration**: `infrastructure/.env` holds Shyft tokens, Grafana port override, and any secrets consumed by compose.
- **Observability**: Prometheus scrapes each service container directly; Grafana reads from the Prometheus service and loads curated dashboards from `infrastructure/grafana/dashboards/curated/`.

## Prerequisites
1. Docker Desktop (or Docker Engine) with `mevana_net` network available (`docker network create mevana_net || true`).
2. Rust toolchain only needed if you plan to build binaries outside Docker (optional).
3. Populate `infrastructure/.env` (or root `.env`) with the required Shyft, Jito, and database settings. The compose file looks for `DATABASE_URL`, `GEYSER_ENDPOINT`, `X_TOKEN`, etc., and will fall back to safe local defaults if they are missing.
  ```bash
  cp infrastructure/env.example infrastructure/.env
  # edit X_TOKEN, ENDPOINT, optional Jupiter keys, GRAFANA_HOST_PORT etc.
  ```
4. Provide flash-loan provider overrides by pointing `FLASH_LOAN_PROVIDERS_CONFIG` at a JSON file (see `config/flash_loan_providers.sample.json` for the structure). Copy the sample and tweak per environment:
   ```bash
   cp config/flash_loan_providers.sample.json config/flash_loan_providers.local.json
   export FLASH_LOAN_PROVIDERS_CONFIG=$PWD/config/flash_loan_providers.local.json
   ```
   Each section lets you cap liquidity, override fees, and supply reserve accounts for Save/Port/Kamino/Marginfi. The executor and hotpath engine read this file on startup.
5. (Optional) Define `REMOTE_DATABASE_URL` in the env file if you plan to clone the production database locally using `scripts/maintenance/sync_remote_db.sh`. You can also override `DB_SYNC_TABLES` to list the tables that should be copied (defaults to the curated configuration tables such as `vault_config`, `dex_program_registry`, and `circuit_breaker_state`).
6. Generate a local Solana keypair for the executor (already created during setup):
   ```bash
   mkdir -p infrastructure/keys
   solana-keygen new --no-passphrase --outfile infrastructure/keys/executor-dev.json --force
   ```
   The compose file mounts this read-only at `/root/.config/solana/id.json` inside the executor container. Replace the key before enabling real execution.

## Boot Sequence
From the repo root:

```bash
docker compose -f infrastructure/docker-compose.core.yml up -d --build
```

This performs multi-stage builds for the Rust services (hotpath, normalizer, discovery, executor) and starts every container. First spin typically takes a few minutes while dependencies compile.

## Database Sync (optional)
- Ensure `REMOTE_DATABASE_URL` (and optional `LOCAL_DATABASE_URL`) are present in your env file. Adjust `DB_SYNC_TABLES` if you need to sync additional tables.
- Run `scripts/maintenance/sync_remote_db.sh` to take a snapshot from the remote Postgres instance and restore it into the local compose-managed database. The script stops dependent services, performs the import, and restarts the stack automatically.
- Snapshots are stored in `backups/remote-<db>-<timestamp>.sql.gz` for auditing or reuse.

### Health Checks
- `docker compose -f infrastructure/docker-compose.core.yml ps` → look for `Up (healthy)` on each service.
- Prometheus targets: `http://localhost:9091/targets` (should show `hotpath-arb-engine`, `discovery-service`, `executor-service`, `prometheus` as `UP`).
- Grafana: `http://localhost:${GRAFANA_HOST_PORT:-3001}` (default overriden to `3300`, login `admin` / `admin123`).
- Logs (example): `docker compose -f infrastructure/docker-compose.core.yml logs -f hotpath-arb-engine`.

### Shutdown
```bash
docker compose -f infrastructure/docker-compose.core.yml down
```

## Dashboard Provisioning
- Only curated dashboards in `infrastructure/grafana/dashboards/curated/` load automatically.
- Restart Grafana after edits: `docker compose -f infrastructure/docker-compose.core.yml restart grafana`.
- Key dashboards shipped:
  - **Hotpath Basic Overview** – live Geyser throughput, opportunity rates, stream errors.
  - **System Signals** (see below) – combined snapshot for discovery/executor health.

## Deploying to the Server
Once the local stack is validated:
1. Commit the curated dashboards and configuration changes.
2. Follow `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md` to trigger the self-hosted workflow; it syncs `infrastructure/` to `/opt/MEVana` and reuses the same compose definitions.
3. Confirm server Prometheus/Grafana health via the workflow diagnostics or by visiting the server Grafana instance (default `:3000`).

Keep this document up to date whenever compose topology or monitoring layout changes so new contributors can reproduce the environment without combing through previous instructions.
