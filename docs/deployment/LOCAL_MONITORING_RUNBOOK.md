# Local Monitoring & Deploy Runbook
Last updated: 2025-09-19

This runbook mirrors the self-hosted production stack on Docker Desktop so you can build a fresh `hotpath-arb-engine`, verify Prometheus and Grafana wiring, and iterate quickly before pushing through the self-hosted deploy workflow.

## 1. Prerequisites
- Docker Desktop (or equivalent) running with at least 8 GB RAM
- `.env` copied from `infrastructure/env.example` (used by the compose files)
- Rust toolchain for local editing/building (unchanged from dev requirements)

```bash
cp infrastructure/env.example infrastructure/.env
# update credentials as needed (Shyft, database, etc.)
```

## 2. Build the latest hotpath container
Every compose run will rebuild from the local sources, but explicitly rebuilding ensures the image matches what you just changed.

```bash
# From repo root
docker compose \
  -f infrastructure/docker-compose.core.yml \
  build hotpath-arb-engine
```

## 3. Start the observability-aligned stack
This stack matches the ports we use on the host server (`docker-compose.server.yml`) but publishes Grafana/Prometheus on non-conflicting host ports.

```bash
# Start everything (Kafka, Redis, Postgres, Prometheus, Grafana, hotpath)
docker compose \
  -f infrastructure/docker-compose.core.yml \
  up -d

# See container status
docker compose -f infrastructure/docker-compose.core.yml ps
```

## 4. Verify metrics and dashboards locally
Use curl to confirm each endpoint before opening Grafana. These match the host deploy checks.

```bash
# Prometheus readiness
curl -f http://localhost:9091/-/ready

# Ensure Prometheus sees hotpath target
curl -s http://localhost:9091/api/v1/targets | grep 'hotpath-arb-engine'

# Hotpath metrics should include build info / counters
curl -s http://localhost:8088/metrics | head

# Grafana health
curl -s http://localhost:${GRAFANA_HOST_PORT:-3001}/api/health
```

Then log into Grafana (default credentials: `admin` / `<ADMIN_PASSWORD>`) and verify dashboards:
```
http://localhost:${GRAFANA_HOST_PORT:-3001}
```

## 5. Update code and restart the service fast
Whenever you change Rust code, rebuild the single service and restart only that container.

```bash
# Rebuild hotpath only
docker compose -f infrastructure/docker-compose.core.yml build hotpath-arb-engine

# Restart the container
docker compose -f infrastructure/docker-compose.core.yml up -d hotpath-arb-engine

# Tail logs if needed
docker compose -f infrastructure/docker-compose.core.yml logs -f hotpath-arb-engine
```

## 6. Tear down
```bash
docker compose -f infrastructure/docker-compose.core.yml down
# Remove persisted volumes if you need a clean start
# docker compose -f infrastructure/docker-compose.core.yml down -v
```

## 7. Production parity checklist
- Metrics ports in the compose file match `prometheus.prod.yml` targets (`8088`, `8085`, `9904`, etc.).
- Prometheus configuration uses service names internally (`hotpath-arb-engine:8088`) just like the server stack.
- Grafana dashboards are provisioned from `infrastructure/grafana/dashboards/curated`, keeping local in sync with production.
- Self-hosted deploy (see `.github/workflows/self-hosted-deploy-hotpath.yml`) now runs the same readiness checks used above, so CI/CD will fail if Prometheus or Grafana stop responding after a deploy.

Following these steps ensures the Docker Desktop setup behaves like the host server, and the same checks the workflow runs can be executed locally before pushing.
