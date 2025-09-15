#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

PROJECT_NAME="mevana"
COMPOSE_FILE="docker-compose.core.yml"
NETWORK_NAME="mevana_net"

echo "🔎 Checking Docker network '${NETWORK_NAME}'..."
if ! docker network ls --format '{{.Name}}' | grep -q "^${NETWORK_NAME}$"; then
  echo "🧱 Creating network '${NETWORK_NAME}'"
  docker network create "${NETWORK_NAME}"
else
  echo "✅ Network '${NETWORK_NAME}' exists"
fi

echo "🚀 Building and starting core stack (infra + hotpath)..."
docker compose -p "${PROJECT_NAME}" -f "${COMPOSE_FILE}" up -d --build

echo "⏳ Waiting for core services to report healthy..."
sleep 5

echo "📋 Current status:"
docker compose -p "${PROJECT_NAME}" -f "${COMPOSE_FILE}" ps

echo "🩺 Tailing logs for early feedback (Ctrl+C to stop)"
docker compose -p "${PROJECT_NAME}" -f "${COMPOSE_FILE}" logs -f --tail=100 \
  zookeeper kafka redis postgres prometheus grafana hotpath-arb-engine 2>&1 | sed -u -e 's/^/[core] /'

