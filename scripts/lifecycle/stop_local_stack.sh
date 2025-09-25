#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PID_DIR="$REPO_ROOT/.local/mevana/pids"
COMPOSE_FILE="$REPO_ROOT/infrastructure/docker-compose.server.yml"
PROJECT_NAME="mevana"

if [[ -d "$PID_DIR" ]]; then
  for pidfile in "$PID_DIR"/*.pid; do
    [[ -e "$pidfile" ]] || continue
    pid=$(cat "$pidfile")
    if kill -0 "$pid" 2>/dev/null; then
      echo "Stopping $(basename "$pidfile" .pid) (pid $pid)"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
    rm -f "$pidfile"
  done
fi

echo "Stopping Docker infrastructure (project=$PROJECT_NAME)"
docker compose -p "$PROJECT_NAME" -f "$COMPOSE_FILE" down

echo "Local MEVana stack stopped"
