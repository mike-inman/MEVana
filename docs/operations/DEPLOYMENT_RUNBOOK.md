# MEVana Deployment Runbook (GHCR CI/CD)

Last updated: 2025-09-15 00:00 UTC

This document describes the automated, image‑based deployment process that builds on GitHub Actions, pushes images to GHCR, and restarts only the app services on the production server.

## Overview

- Build: GitHub Actions compiles Linux images (Rust nightly) for changed services only.
- Push: Images are pushed to GHCR and tagged with the commit SHA.
- Deploy: The workflow SSHes to the server, optionally writes hotpath secrets to `infrastructure/.env`, sets `MEVANA_TAG` to the commit SHA, and restarts only the changed services via Docker Compose.
- Infra (Kafka, Zookeeper, Redis, Postgres, Prometheus, Grafana) stays up.

## Components

- Workflow: `.github/workflows/ghcr-build-deploy.yml`
- Compose (server + app overlay):
  - `infrastructure/docker-compose.server.yml`
  - `infrastructure/docker-compose.full.yml`
- Images (GHCR):
  - `ghcr.io/mike-inman/mevana-hotpath-arb-engine:${MEVANA_TAG}`
  - `ghcr.io/mike-inman/mevana-normalizer-service:${MEVANA_TAG}`
  - `ghcr.io/mike-inman/mevana-discovery-service:${MEVANA_TAG}`
  - `ghcr.io/mike-inman/mevana-executor-service:${MEVANA_TAG}`

## One‑time GitHub setup

Add GitHub Actions repository secrets (Settings → Secrets and variables → Actions):

- Required for server SSH:
  - `SSH_HOST` = `149.28.56.125`
  - `SSH_USER` = `root`
  - `SERVER_DIR` = `/opt/MEVana`
  - `VULTR_SERVER_SSH_KEY` = private key that can SSH into the server

- Optional (server pulls private images):
  - `GHCR_USERNAME` = your GitHub username (e.g., `mike-inman`)
  - `GHCR_TOKEN` = PAT with `read:packages` (server pulls only)

- Optional (hotpath secrets managed by Actions):
  - `HOTPATH_ENDPOINT`
  - `HOTPATH_X_TOKEN`

> Note: Build/push uses the built‑in `GITHUB_TOKEN` (no PAT needed) and build cache is enabled (Buildx).

## Running a deploy

Two ways:

1) Push to `main` — auto runs and deploys only changed services.
2) Manual — Actions → “Build and Deploy (GHCR)” → Run workflow on `main`.

The workflow:

- Detects changed services (hotpath/normalizer/discovery/executor).
- Builds/pushes only those images to GHCR using Docker Buildx cache.
- Syncs the `infrastructure/` directory to the server to ensure compose is up‑to‑date.
- Optionally writes hotpath secrets to `infrastructure/.env` (if `HOTPATH_*` secrets provided).
- Sets `MEVANA_TAG=${GITHUB_SHA}` and runs `docker compose up -d` for only the changed services.

## Verifying a deploy

On the server:

```bash
ssh root@149.28.56.125 'cd /opt/MEVana && \
  docker compose -p mevana -f infrastructure/docker-compose.server.yml -f infrastructure/docker-compose.full.yml ps'
```

Hotpath health (once up):

```bash
ssh root@149.28.56.125 'curl -sf http://localhost:8088/metrics | head -n 10'
```

UIs:

- Grafana: `http://149.28.56.125:3000`
- Prometheus: `http://149.28.56.125:9090`
- Kafka‑UI: `http://149.28.56.125:8080`

## Rollback

SSH to server and set a prior tag:

```bash
ssh root@149.28.56.125 'cd /opt/MEVana && \
  MEVANA_TAG=<old_sha> docker compose -p mevana \
  -f infrastructure/docker-compose.server.yml \
  -f infrastructure/docker-compose.full.yml \
  up -d hotpath-arb-engine normalizer-service discovery-service executor-service'
```

> Optional future enhancement: add a rollback input (SHA) to the workflow.

## Troubleshooting

1) Exec format error (container restarts: 255)
   - Cause: running macOS‑built binaries in Linux. Fixed by building images in Actions (Linux) only.

2) Edition 2024 errors (e.g., `base64ct-1.8.0` requires edition2024)
   - Fix: Builder stages install Rust nightly and use `cargo +nightly` for all service builds.

3) Missing runtime libs (librdkafka / libssl)
   - Fix: Runtime images install `librdkafka1` and `curl` where needed; add `libssl3` if logs show SSL loader errors.

4) Zookeeper “No space left on device”
   - Fix: Stop Kafka/ZK, truncate Docker JSON logs, prune, remove/recreate only ZK/Kafka containers, then `up -d` in order (ZK → Kafka → Kafka‑UI). Optionally run `create-topics.sh`.

5) Secrets management
   - If `HOTPATH_ENDPOINT/X_TOKEN` are set as GH Secrets, the workflow writes `infrastructure/.env` automatically. Otherwise it leaves the file unchanged.

## Service health & metrics

- Hotpath: health is verified by the workflow (curl `/metrics` on localhost:8088).
- Recommendation: add `/health` endpoints to discovery and executor; update Prometheus targets to scrape these services by container name.

## Server layout

- Code & compose: `/opt/MEVana`

---

Legacy notes
- Additional historical notes were migrated from the root repository into `docs/operations/DEPLOYMENT_RUNBOOK_legacy.md` on 2025-09-15 for reference.
- Network: `mevana_net` (external)
- Compose files: `infrastructure/`

---

## Recent Changes (2025‑09‑15)

1) CI/CD overhaul:
   - Added `.github/workflows/ghcr-build-deploy.yml` with:
     - Selective builds (changed-files detection)
     - Docker Buildx cache
     - Concurrency (cancel in-progress)
     - Targeted deploy (restart only changed services)
     - Optional secret‑managed hotpath `.env`

2) Docker/compose:
   - Compose switched to GHCR images (by commit SHA via `MEVANA_TAG`).
   - Runtime images for discovery, normalizer, executor now include `librdkafka1` (and `curl`).
   - Hotpath runtime includes `curl`; healthcheck uses `curl /metrics`.

3) Reliability fixes:
   - All service builder Dockerfiles compile with Rust nightly (`cargo +nightly`) to support edition2024.
   - Deploy syncs `infrastructure/` to the server before compose, so compose stays in sync.

4) Operational guidance:
   - Zookeeper/Kafka reset procedure and disk cleanup documented above.
