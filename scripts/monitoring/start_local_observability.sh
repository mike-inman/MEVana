#!/bin/bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
compose_file="$repo_root/infrastructure/docker-compose.core.yml"

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] docker command not found. Please install Docker Desktop or Docker Engine." >&2
  exit 1
fi

echo "[INFO] Ensuring shared Docker network exists (mevana_net)"
docker network create mevana_net >/dev/null 2>&1 || true

grafana_port=${GRAFANA_HOST_PORT:-3001}
if lsof -i tcp:$grafana_port >/dev/null 2>&1; then
  echo "[WARN] Port $grafana_port already in use; falling back to 3300" >&2
  grafana_port=3300
fi
export GRAFANA_HOST_PORT=$grafana_port

echo "[INFO] Starting Prometheus and Grafana from $compose_file (Grafana host port: $grafana_port)"
docker compose -f "$compose_file" up -d prometheus grafana

echo "[INFO] Prometheus UI: http://localhost:9091"
echo "[INFO] Grafana UI:    http://localhost:${grafana_port} (admin / admin123)"
