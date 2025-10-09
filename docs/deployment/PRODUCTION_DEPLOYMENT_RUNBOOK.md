# MEVana Production Deployment Runbook

Last updated: 2025-10-09
Author: Codex CLI (automated)

> **Goal**: Deploy MEVana on `mevana-nyc-01` repeatably, without port/network drift, while keeping monitoring and wallet metrics online. This runbook captures the agreed production plan and required verification steps so future agents must follow it as written.

---

## 1. Roles & Responsibilities
- **Release Engineer / AI agent**: Executes this runbook end-to-end, records outcomes, raises deviations.
- **Infrastructure Owner**: Approves changes to server packages, Docker networking, and systemd units.
- **Trading Ops**: Confirms arbitrage preflight report before production trading is enabled.
- **Observability Owner**: Ensures Grafana dashboards and Prometheus alerts remain healthy.

> **Rule**: Do **not** deviate from this runbook without written approval. Document any variance in the deployment log.

---

## 2. Environment Overview
- Host: `root@149.28.56.125` (`mevana-nyc-01`, Ubuntu 22.04, kernel 5.15)
- Repo path: `/opt/MEVana`
- Binaries: `/opt/MEVana/bin`
- Systemd units: `/etc/systemd/system/*.service`
- Docker network: external bridge `mevana_net` (subnet `172.18.0.0/16`)
- Monitoring stack: Prometheus + Grafana inside Docker, scraping bare-metal services via `host-gateway`

### Service Port Matrix (host bindings)
| Service | Host Port | Notes |
|---------|-----------|-------|
| Grafana | 3000 | **Single** host binding; remove 3001 duplication before go-live |
| Prometheus | 9090 | Official host port; 9091 was previously used—keep free |
| Kafka (external) | 29092 | For host binaries |
| Redis | 6380 → container 6379 | Ensure only one binding is active |
| Postgres | 5433 | Internal DB |
| Hotpath metrics | 8088 | Bare-metal binary |
| Hotpath API | 8089 | Bare-metal binary |
| Wallet Vault | 8090 | Bare-metal binary |
| Monitoring Service | 8091 | Bare-metal binary |
| API Service | 8092 | Bare-metal binary |
| Executor metrics | 9904 | Bare-metal binary |
| Normalizer metrics | 9901 | Bare-metal binary |
| Wallet metrics | 9905 | Bare-metal exporter |
| Discovery metrics | 9102 | Must bind on host for Prometheus scrape |

> **Action**: Keep this table updated if any port changes. Update Prometheus targets simultaneously.

---

## 3. Data Retention & Hygiene Policy
- **PostgreSQL**: Retain application state and configuration tables only. Purge transactional / historical tables older than 7 days. Record cleanup in maintenance log.
- **Kafka**: Set topic retention to 7 days maximum; for ephemeral topics (e.g., discovery), recreation is acceptable.
- **Prometheus TSDB**: Retain 7 days; run `promtool tsdb clean-tombstones` monthly or during maintenance.
- **Logs**: Rotate systemd journal weekly; keep `journalctl --vacuum-time=7d` in maintenance cadence.

Maintenance tickets must track completion; update runbook when retention changes.

---

## 4. Pre-Deployment Checklist
1. **Access**
   - ✅ Confirm SSH root access.
   - ✅ Ensure repo mirror `/opt/MEVana` is up to date with target commit.
2. **Change Freeze**
   - ✅ Verify no unrelated shell sessions are modifying the server.
3. **Secrets**
   - ✅ Validate `/etc/mevana/*.env` contain required values:
     - `X_TOKEN`, `SHYFT_API_KEY`, RPC endpoints for executor/discovery.
4. **Backups**
   - ✅ Snapshot DB (`pg_dump` essential tables) and save AOF copy for Redis in `/opt/MEVana/backups/YYYYMMDD`.
5. **Deployment Log**
   - ✅ Open `docs/deployment/logs/DEPLOY_YYYYMMDD.md` (create if missing) and record each section outcome.

Deployment must not proceed unless every checkbox is satisfied.

---

## 5. Execution Phases

### Phase 0 – Document Baseline
- Run `docker ps`, `systemctl list-units '*mevana*'`, `ss -tulpn` and attach outputs to deployment log.
- Record current `/etc/mevana` env snapshots (mask secrets).

### Phase 1 – Infrastructure Cleanup (Mandatory)
1. `cd /opt/MEVana`
2. Execute cleanup sequence (mirrors workflow lines 252-273):
   ```bash
   docker compose -f infrastructure/docker-compose.yml down --remove-orphans || true
   docker compose -f infrastructure/docker-compose.core.yml down --remove-orphans || true
   docker compose -p mevana -f infrastructure/docker-compose.server.yml -f infrastructure/docker-compose.full.yml down --remove-orphans || true
   docker compose -p mevana -f infrastructure/docker-compose.server.yml down --remove-orphans || true
   docker stop mevana-hotpath mevana-hotpath-arb-engine-1 hotpath_arb_engine || true
   docker rm mevana-hotpath mevana-hotpath-arb-engine-1 hotpath_arb_engine || true
   for port in 9090 3000 8080 8082 29092 5433 6380 6379 2181; do
     ids=$(docker ps --filter "publish=${port}" --format '{{.ID}}')
     if [ -n "$ids" ]; then docker stop $ids && docker rm $ids; fi
   done
   ```
3. Verify `docker network ls` still lists `mevana_net`. If missing, recreate with `docker network create mevana_net`.

### Phase 2 – Repair Stateful Services
1. **Redis**
   - Copy `/var/lib/docker/volumes/mevana_redis_data` to backup.
   - Run inside redis container image:
     ```bash
     docker run --rm -v mevana_redis_data:/data redis:7-alpine redis-check-aof --fix /data/appendonly.aof
     ```
   - Set `vm.overcommit_memory=1`:
     ```bash
     sysctl -w vm.overcommit_memory=1
     echo 'vm.overcommit_memory = 1' >> /etc/sysctl.conf
     ```
2. **Kafka**
   - Ensure volumes clean (`docker volume inspect mevana_kafka_data`).
   - Drop lingering recovery logs if necessary.
3. **Prometheus**
   - Remove blocks older than 7 days if disk pressure.

Document actions and results.

### Phase 3 – Bootstrap Infrastructure
1. Start core stack:
   ```bash
   docker compose -p mevana -f infrastructure/docker-compose.server.yml up -d \
     zookeeper kafka redis postgres adminer prometheus grafana kafka-ui
   ```
2. Wait 30s; check `docker compose ... ps` for healthy status.
3. Confirm single Grafana binding (`ss -tulpn | grep 3000`). If 3001 still present, fix compose file before proceeding.
4. Validate Prometheus container resolves `host.docker.internal` to reachable host IP (`docker exec mevana-prometheus-1 curl -I http://host.docker.internal:8088/metrics`).

### Phase 4 – Seed Database, Kafka, Prometheus
1. Database migration:
   ```bash
   ./scripts/deployment/bootstrap_database.sh
   ```
   - Post-run: remove historical tables (>7d) via SQL script (document queries).
2. Kafka topics:
   ```bash
   ./scripts/deployment/bootstrap_kafka.sh localhost:29092
   ```
   - Manually ensure required topics exist: `unknown-programs`, `dex-transactions`, etc.
   - Set retention: `kafka-configs --alter --entity-type topics --entity-name <topic> --add-config retention.ms=604800000`.
3. Prometheus retention: verify `--storage.tsdb.retention.time=7d` in compose/environment.
4. Ensure Grafana Postgres datasource (`infrastructure/grafana/datasources/postgres.yml`) points to `postgres:5432` and reload Grafana before dashboards are accessed.

### Phase 5 – Build & Install Services
1. Ensure Rust toolchain available (`cargo --version`).
2. Run release script with services list (include wallet-metrics exporter):
   ```bash
   sudo INSTALL_PREFIX=/opt/MEVana ./scripts/deployment/release-services.sh hotpath normalizer discovery executor wallet-vault monitoring wallet-metrics api-service
   ```
   - Before restarting the exporter, populate `/etc/mevana/wallet-metrics.env` with the latest `HELIUS_API_KEY` (primary price provider) and `X_TOKEN` values.
3. Confirm binaries installed in `/opt/MEVana/bin` and permissions are `mevana:mevana`.
4. Reload systemd (`systemctl daemon-reload`).
5. Restart services in dependency order:
   ```bash
   systemctl restart discovery-service.service
   systemctl restart normalizer-service.service
   systemctl restart hotpath-arb-engine.service
   systemctl restart executor-service.service
   systemctl restart wallet-vault.service
   systemctl restart monitoring-service.service
   systemctl restart wallet-metrics-exporter.service
   systemctl restart api-service.service
   ```
6. Ensure secrets loaded: `systemctl show executor-service -p Environment` (mask output in log).

### Phase 6 – Verification & Preflight
1. Prometheus targets:
   ```bash
   curl -s http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | {job, health, lastError}'
   ```
   - All host scrape targets must be `"health":"up"`.
2. Metrics spot checks:
   - `curl -s http://localhost:8088/metrics | grep hotpath_opportunities`
   - `curl -s http://localhost:9905/metrics | head`
   - `curl -s http://localhost:9102/metrics | head`
3. Grafana login (`admin`/`admin123` or secret). Confirm dashboards load wallet/gateway data.
4. Run `scripts/ci/post-deployment-diagnostics.sh` and archive output.
5. Execute arbitrage preflight tool (when available) and capture summary.

> **If any check fails**, stop the deployment and remediate before enabling trading.

### Phase 7 – Enable Live Trading (Manual Approval)
- Confirm `executor-service` has valid API key and blockhash refresh stable.
- Switch `PAPER_TRADING=false` only after Ops sign-off.
- Monitor for 30 minutes; track metrics.

---

## 6. Post-Deployment Tasks
- Update deployment log with:
  - Commit hash deployed
  - Redis repair status
  - Kafka/DB cleanup actions
  - Prometheus retention confirmation
  - Diagnostics summary (pass/fail)
- Notify stakeholders (Grafana dashboard URL, snapshot of wallet metrics).
- Schedule next maintenance window for retention pruning.

---

## 7. Partial Redeployments

When only a subset of services changes, run the relevant phases in isolation instead of the full pipeline:

1. **Code or binary-only updates**: skip Phases 1–4; execute Phase 5 for the specific crates (e.g., `cargo build --release -p discovery-service`) and restart the matching systemd unit. Record the restart in the deployment log.
2. **Infrastructure-only changes**: run Phases 1–3 (cleanup, repair, stack bootstrap) and re-validate monitoring endpoints before touching binaries.
3. **Monitoring adjustments**: update Prometheus/Grafana assets, reload Prometheus via `/-/reload`, and capture the new target list. No systemd restarts required.
4. **Topic/schema tweaks**: execute Phase 4 with the new migrations/retention config, then run Phase 6 diagnostics to confirm data flow.

Always update the deployment log with which phases were executed and why. If the scope expands mid-run, revert to the full sequence to avoid drift.

---

## 8. Maintenance Plan (Data Retention)
1. **Weekly Maintenance Script** (cron/systemd timer TBD):
   - `psql` cleanup stored procs (truncate historical tables > 7 days).
   - `kafka-run-class kafka.tools.GetOffsetShell` to confirm log size.
   - `kafka-configs` ensure retention not drifting.
   - `prometheus` container: `promtool tsdb clean-tombstones --tsdb.path /prometheus`.
   - `journalctl --vacuum-time=7d`.
2. Record maintenance execution in `/opt/MEVana/logs/maintenance/MAINT_YYYYMMDD.md`.

---

## 9. Appendices
- **A. Required Topics**: `dex-transactions`, `unknown-programs`, `orca-swaps.parsed.v2`, `edge.delta.v1`, `edge.snapshot.v1`.
- **B. Critical Environment Files**: `/etc/mevana/common.env`, `hotpath.env`, `executor.env`, `wallet-metrics.env`, `wallet-vault.env`, `monitoring.env`, `api-service.env`.
  - `wallet-metrics.env` must include `HELIUS_API_KEY` (primary price feed) and will fall back to Jupiter automatically if Helius data is unavailable.
- **C. Useful Commands**:
  - `docker exec mevana-prometheus-1 wget -qO- http://host.docker.internal:9905/metrics | head`
  - `systemctl status hotpath-arb-engine.service --no-pager`
  - `journalctl -u wallet-metrics-exporter.service -n 50 --no-pager`

Keep this runbook synchronized with actual automation (`.github/workflows/self-hosted-deploy.yml`). Update immediately after any process change.
