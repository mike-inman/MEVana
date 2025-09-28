Self‑Hosted Deployment Strategy & Workflows (Production Server)

Last updated: 2025-09-27

## Quickstart for the next helper

1. **Start with `docs/operations/ADMIN_CONTROL_PANEL.md`.** It explains the Clerk JWT setup, Grafana button requirements, per-wallet metrics, and audit expectations you must validate immediately after a deploy.
2. Ensure `/etc/mevana/common.env` and `/etc/mevana/executor.env` define `WALLET_LABELS=/opt/MEVana/keys/id.json=prod,...` so Prometheus exports `mevana_wallet_balance_sol{wallet="prod"}` and Grafana panels light up without guesswork.
3. After rolling new binaries, run `systemctl status executor-service` and curl the metrics endpoint (`curl -s http://127.0.0.1:9904/metrics | rg mevana_wallet_balance_sol`) to confirm the active wallet label reports the live balance and backups show `0`.

### 2025-09-27 operational notes

- Gateway groundwork complete: Kafka topics (`prices.latest`, `orca-swaps.parsed`, `orca-swaps.parsed.v2`) confirmed via `scripts/deployment/bootstrap_kafka.sh` and `docker exec mevana-kafka-1 kafka-topics ... --list`. `/etc/mevana/api-service.env` now enables `MEVANA_GATEWAY_ENABLE=true` with `PORT=8092` and `MEVANA_WEBSOCKET_GATEWAY_PORT=8095`; restart `api-service.service` after edits.
- New GitHub Action (`Rogue Process Cleanup (Self-Hosted)`) runs `scripts/maintenance/cleanup_rogue_processes.sh` to kill stale MEVana binaries and restart systemd units when the host accumulates orphaned processes.
- Pending follow-ups:
  * Update `/etc/mevana/ui.env` so MEVana-UI consumes `http://<host>:8095/gateway/*` (keep legacy URLs until the cutover is verified).
  * Run `scripts/ci/post-deployment-diagnostics.sh` after gateway restarts to ensure Prometheus/Grafana scrape the expected targets.
  * Execute `scripts/testing/gateway-fanout-smoke.sh` to push a sample opportunity/execution into Kafka and confirm data surfaces on `/gateway/ws/*` and `/gateway/sse/*` before cutting the UI to gateway-only mode.
  * Review firewall openings (8080, 3002, 3200, 8096) once the UI migration is complete and close any unused ports.

Overview
- Goal: Keep the hotpath, normalizer, discovery, and executor binaries as close to the metal as possible while leaving supporting infrastructure (Kafka, Redis, Postgres, Prometheus, Grafana) in Docker for easy parity with local development.
- What changed: the GitHub Actions runner now builds release binaries in-place and installs them under `/opt/MEVana/bin/`, managed by dedicated systemd units. No container rebuild loops for the Rust services.
- Benefits: deployments land in minutes, systemd restarts mirror local cargo runs, and production matches the host-based workflow you use during development.

Canonical Paths & Users
- Application root: `/opt/MEVana`
  - `bin/` compiled binaries (`hotpath_arb_engine`, `normalizer-service`, `discovery-service`, `executor-service`)
  - `config/` runtime JSON/config files (`flash_loan_providers.mainnet.json`, etc.)
  - `keys/` executor wallet keypair (`id.json`)
  - `state/`, `logs/`, `releases/` writable by the `mevana` user
- Environment files: `/etc/mevana/common.env` plus `/etc/mevana/{hotpath,normalizer,discovery,executor}.env`
- System user: `mevana` (created by the bootstrap helper)

Bootstrap (first run or whenever provisioning a new server)
```bash
# Run from the repository root on the target host
sudo scripts/deployment/bootstrap-host.sh
```
What it does:
1. Creates the `mevana` system user (home `/opt/MEVana`) and standard directories.
2. Seeds `/opt/MEVana/config/flash_loan_providers.*.json` and copies env samples into `/etc/mevana/*.env.sample`.
3. Installs/refreshes the four systemd unit files under `/etc/systemd/system/` and reloads systemd.
4. Copies `infrastructure/keys/id.json` to `/opt/MEVana/keys/id.json` if the key is missing (one-time convenience).

Secrets & Environment Configuration
1. Edit `/etc/mevana/common.env` using `config/environments/common.env.sample` as the template (shared values such as `DATABASE_URL`, `KAFKA_BROKER`, `FLASH_LOAN_PROVIDERS_CONFIG`, `WALLET_KEYPAIR_PATH`).
2. Keep `common.env` as the single source of truth for shared values; the copy in your repo root should match so future deploys don’t overwrite server edits. Key host mappings we set:
   - `KAFKA_BROKER=KAFKA_BOOTSTRAP_SERVERS=localhost:29092`
   - `REDIS_URL=redis://127.0.0.1:6380/0`
   - `FLASH_LOAN_PROVIDERS_CONFIG=/opt/MEVana/config/flash_loan_providers.mainnet.json`
   - `WALLET_KEYPAIR_PATH=/opt/MEVana/keys/id.json`
   - `JITO_AUTH_KEYPAIR_PATH=/opt/MEVana/keys/jito-keypair.json`
3. Layer service-specific overrides only where needed:
   - `/etc/mevana/hotpath.env` → `METRICS_PORT=8088` (API is `8089` automatically) and `PROPERTY_STORE_PERSISTENCE_PATH=/opt/MEVana/state/property_store.json`.
   - `/etc/mevana/normalizer.env` → Kafka topics (`dex-transactions` → `orca-swaps.parsed.v2`), `REDIS_URL=redis://127.0.0.1:6380/0`, metrics port `9901` (default).
   - `/etc/mevana/executor.env` → `METRICS_PORT=9904`, `HEALTH_CHECK_PORT=9898`, and set `ALLOW_EMPTY_SHYFT_API=1` only if you intentionally leave the API key blank.
   - `/etc/mevana/discovery.env` can stay empty unless you want non-default Kafka groups or topics.
   - `/etc/mevana/api-service.env` → `PORT=8092`, `MEVANA_GATEWAY_ENABLE=true`, and gateway topic overrides when the UI requires a different Kafka fan-out.
4. Prime the data stores (idempotent; safe to run before every deploy):
   ```bash
   sudo ./scripts/deployment/bootstrap_database.sh             # applies schema + seeds vault_config
   sudo ./scripts/deployment/bootstrap_kafka.sh kafka:9092     # creates trading topics if missing
   ```
   Both helpers are invoked automatically by `start_system.sh`/`release-services.sh`, but running them during initial provisioning guarantees Postgres and Kafka are ready before services start.
5. Permissions: keep every env file `root:mevana` with mode `640` so systemd can read while unprivileged users cannot.
6. Ensure `/opt/MEVana/keys/id.json` (executor wallet) and `/opt/MEVana/keys/jito-keypair.json` exist and are owned by `mevana:mevana` with mode `600` before enabling live services.
7. Copy `config/environments/alerts.env.sample` to `/etc/mevana/alerts.env` and populate `SLACK_WEBHOOK_URL` (or other contact points) so the Grafana/Prometheus alerting stack can notify ops.

Infrastructure Endpoints & Firewall Rules
- Application services listen on the host network while infrastructure stays in Docker. Open these ports locally (UFW rules shown):
  ```bash
  sudo ufw allow 8088/tcp  # hotpath metrics
  sudo ufw allow 8089/tcp  # hotpath API
  sudo ufw allow 9901/tcp  # normalizer metrics
  sudo ufw allow 9102/tcp  # discovery metrics
  sudo ufw allow 9904/tcp  # executor metrics
  sudo ufw allow 8092/tcp  # REST gateway (api-service)
  sudo ufw allow 8095/tcp  # WebSocket gateway (api-service)
  ```
- Prometheus runs in Docker but scrapes host services at `host.docker.internal:<port>`. The bundled configs already target those endpoints (`infrastructure/prometheus.{core,prod}.yml`).

Kafka Topics (bare metal services)
- Managed by `scripts/deployment/bootstrap_kafka.sh`. For manual validation:
  ```bash
  docker exec mevana-kafka-1 kafka-topics --bootstrap-server localhost:9092 --list
  ```
  You should see `dex-transactions`, `orca-swaps.parsed.v2`, `orca-tickarrays.parsed.v2`, `edge.delta.v1`, `edge.snapshot.v1`, `normalized-swaps.v1`, `unknown-programs`, `arb.opportunities.v1`.

Infrastructure Services (Docker, once per host)
```bash
# Keep Kafka/Redis/Postgres/Grafana in Docker for parity
cd /opt/MEVana
sudo docker compose -p mevana -f infrastructure/docker-compose.server.yml up -d \
  zookeeper kafka redis postgres adminer prometheus grafana kafka-ui
```
- Grafana: `http://<server-ip>:3000` (admin/admin123)
- Prometheus: `http://<server-ip>:9090`
- Kafka broker exposed internally as `kafka:9092` (compose network `mevana_net`)
- Database forwarded on `localhost:5433` via compose

Systemd Services (hotpath + companions)
```bash
# Once env files are in place
sudo systemctl enable --now hotpath-arb-engine.service normalizer-service.service \
  discovery-service.service executor-service.service api-service.service

# Common operations
sudo systemctl restart hotpath-arb-engine.service
sudo systemctl status hotpath-arb-engine.service
sudo journalctl -u hotpath-arb-engine.service -n 200 --no-pager
```
- Units run from `/opt/MEVana`, inherit shared env files, and write to `/opt/MEVana/logs` & `/opt/MEVana/state`.
- `hotpath-arb-engine` exposes readiness on `8089` and metrics on `8088` (set `METRICS_PORT` to change both).
- `normalizer-service` executes `normalizer-service pipe ...` under systemd; adjust `/etc/mevana/normalizer.env` to change topics or the consumer group.
- `executor-service` listens for admin/health on `9898` and metrics on `9904`; keep `ALLOW_EMPTY_SHYFT_API=1` only if you intentionally skip Shyft credentials.
- `api-service` binds to `8092` (REST) and `8095` (gateway). Make sure the UI’s `NEXT_PUBLIC_GATEWAY_BASE_URL` matches and that Kafka topics configured in `/etc/mevana/api-service.env` align with normalizer output.
- `release-services.sh` (see below) automatically enables and restarts the units when deploying.

Fast Deploy Cycle (manual)
```bash
# Build and deploy selected services (run from repo root on the server)
sudo scripts/deployment/release-services.sh hotpath executor
# Build everything
sudo scripts/deployment/release-services.sh
```
- Builds `cargo --release` for each requested service, installs binaries into `/opt/MEVana/bin/`, writes the short commit hash to `/opt/MEVana/.mevana_tag`, and restarts the relevant systemd units.
- Set `SKIP_RESTART=1` to install binaries without touching running services (useful for staging or manual restarts).
- Override `INSTALL_PREFIX` or `MEVANA_USER` if you are targeting a non-standard path/user (e.g., local experiments): `sudo INSTALL_PREFIX=/srv/mevana scripts/deployment/release-services.sh hotpath`.

CI/CD Workflow (`self-hosted-deploy.yml`)
- Trigger via GitHub Actions (“Build and Deploy (Self-Hosted)”) with optional `services` input (`hotpath normalizer`, etc.).
- Steps performed:
  1. Check out code on the self-hosted runner.
  2. Re-run `bootstrap-host.sh` (idempotent) to refresh unit files and config samples.
  3. Execute `release-services.sh` with the resolved service list.
  4. Run `scripts/ci/post-deployment-diagnostics.sh` against systemd + HTTP endpoints.
  5. Always print `systemctl --no-pager status ...`; on failure, tail the last 200 journal entries per service.
- Expected runtime: ~3–4 minutes for hotpath-only changes; ~6–7 minutes when rebuilding all binaries from scratch.

-Validation & Diagnostics
- `scripts/ci/post-deployment-diagnostics.sh` now checks:
  - Grafana/Prometheus/Kafka/Adminer health (HTTP)
  - Application metrics endpoints (`8088`, `8089`, `9901`, `9102`, `9904`, `8092`, `8095`)
  - Prometheus target coverage against `host.docker.internal:<port>`
  - Grafana datasource connectivity (script logs in with admin/admin123) and warns if dashboards are missing
  - Systemd unit status for the bare-metal services, including the API gateway
- Run manually on the host after changes:
  ```bash
  cd /opt/MEVana
  ./scripts/ci/post-deployment-diagnostics.sh
  ```

Local Development Parity
- Dependencies: run the same Docker Compose infrastructure stack locally (`docker compose -f infrastructure/docker-compose.server.yml up -d ...`).
- Application binaries: run with Cargo for hot reload:
  ```bash
  # Example: hotpath
  source config/environments/common.env.sample   # or export selectively
  source config/environments/hotpath.env.sample  # override ports/tokens locally
  cargo run -p hotpath_arb_engine --bin hotpath_arb_engine
  ```
- For faster rebuilds mirroring production, you can point the release script at a local prefix:
  ```bash
  sudo INSTALL_PREFIX=/opt/MEVana-local MEVANA_USER=$USER \
    scripts/deployment/bootstrap-host.sh
  sudo env INSTALL_PREFIX=/opt/MEVana-local SYSTEMCTL=false SKIP_RESTART=1 \
    scripts/deployment/release-services.sh hotpath
  ```
  (When `SYSTEMCTL=false`, the script will still build/install but you restart the binary manually.)

Operational Checklist Before Enabling Live Execution
- Ensure `/opt/MEVana/keys/id.json` is the intended wallet (permissions `600`, owner `mevana`).
- Populate `/etc/mevana/executor.env` with `EXECUTION_DRY_RUN=false` only when ready and keep `EXECUTION_ALLOW_LIVE=false` until final go/no-go.
- Confirm Prometheus target `executor-service:9904` is `UP` and Grafana can query Prometheus.
- Run `journalctl -u executor-service.service -f` while flipping execution flags to observe any runtime errors.

Rollback & Troubleshooting
- Older binaries remain available under `/opt/MEVana/releases/` (if you copy them manually before overwriting).
- To roll back quickly:
  ```bash
  sudo cp /opt/MEVana/releases/<timestamp>/hotpath_arb_engine /opt/MEVana/bin/
  sudo systemctl restart hotpath-arb-engine.service
  ```
- Reset a failing service:
  ```bash
  sudo systemctl stop hotpath-arb-engine.service
  sudo rm -rf /opt/MEVana/state/*
  sudo systemctl start hotpath-arb-engine.service
  ```
- If diagnostics flag Grafana/Prometheus connectivity, re-run the Docker infrastructure compose stack; for executor restarts, verify the wallet key path first.

Summary
- Host-based systemd units deliver the speed boost required for tight development loops.
- Docker is still used for shared infra to keep the monitoring pipeline identical across environments.
- The new scripts (`bootstrap-host.sh`, `release-services.sh`) are the single source of truth for provisioning and deploys—both locally and in CI.
