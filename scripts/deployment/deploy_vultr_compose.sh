#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   scripts/deployment/deploy_vultr_compose.sh <ssh_host> [remote_dir] [compose_file]
# Examples:
#   scripts/deployment/deploy_vultr_compose.sh root@<VULTR_IP> /opt/MEVana
#   scripts/deployment/deploy_vultr_compose.sh root@<VULTR_IP> /opt/MEVana infrastructure/docker-compose.server.yml
#
# Notes:
# - Requires SSH access to the server with Docker + Docker Compose installed
# - Builds on the server (no registry needed)
# - Uses external network name 'mevana_net'

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <ssh_host> [remote_dir]" >&2
  exit 1
fi

SSH_HOST="$1"
REMOTE_DIR="${2:-/opt/MEVana}"
PROJECT_NAME="mevana"
COMPOSE_FILE_LOCAL="${3:-infrastructure/docker-compose.server.yml}"

echo "📍 Target: ${SSH_HOST}  Dir: ${REMOTE_DIR}"

echo "🔐 Checking SSH connectivity..."
ssh -o BatchMode=yes -o ConnectTimeout=8 "${SSH_HOST}" "echo 'SSH OK'" >/dev/null

echo "🗂️  Creating remote directory..."
ssh "${SSH_HOST}" "mkdir -p '${REMOTE_DIR}'"

echo "📦 Syncing repository to server (rsync preferred)..."
if command -v rsync >/dev/null 2>&1; then
  rsync -az --delete --exclude '.git' --exclude 'target' ./ "${SSH_HOST}:${REMOTE_DIR}/"
else
  echo "rsync not found locally; falling back to scp (slower)."
  tar czf - . --exclude .git --exclude target | ssh "${SSH_HOST}" "tar xzf - -C '${REMOTE_DIR}'"
fi

echo "🧱 Ensuring Docker network 'mevana_net' exists..."
ssh "${SSH_HOST}" "docker network ls --format '{{.Name}}' | grep -q '^mevana_net$' || docker network create mevana_net"

echo "🧹 Optional cleanup: remove exited containers and dangling artifacts"
ssh "${SSH_HOST}" "docker container prune -f >/dev/null || true; docker image prune -f >/dev/null || true; docker volume prune -f >/dev/null || true"

echo "🚀 Deploying stack on server using ${COMPOSE_FILE_LOCAL}..."
ssh "${SSH_HOST}" "cd '${REMOTE_DIR}' && docker compose -p '${PROJECT_NAME}' -f '${COMPOSE_FILE_LOCAL}' up -d --build"

echo "📋 Remote status:"
ssh "${SSH_HOST}" "cd '${REMOTE_DIR}' && docker compose -p '${PROJECT_NAME}' -f '${COMPOSE_FILE_LOCAL}' ps"

echo "🩺 Tailing early logs (Ctrl+C to stop)..."
ssh -t "${SSH_HOST}" "cd '${REMOTE_DIR}' && docker compose -p '${PROJECT_NAME}' -f '${COMPOSE_FILE_LOCAL}' logs -f --tail=120 zookeeper kafka postgres redis prometheus grafana hotpath-arb-engine"
