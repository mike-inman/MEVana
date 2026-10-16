Last updated: 2025-09-15

MEVana Grafana Dashboards — Contribution Guide

- Single source of truth: place all Grafana dashboard JSON files in `infrastructure/grafana/dashboards/`.
- Provisioning: Grafana mounts this directory into `/etc/grafana/provisioning/dashboards` and auto-loads dashboards via `dashboard.yml` in the same folder.
- Do not create or commit a top-level `dashboards/` directory. CI will fail if dashboards are placed outside `infrastructure/grafana/dashboards/`.
- Data sources are provisioned from `infrastructure/grafana/datasources/prometheus.yml`.

Quick test locally
- Run: `docker compose -f infrastructure/docker-compose.server.yml -f infrastructure/docker-compose.full.yml up -d grafana prometheus`
- Open: http://localhost:3000 (admin/<ADMIN_PASSWORD>)
- Dashboards should appear automatically under the configured folder.
