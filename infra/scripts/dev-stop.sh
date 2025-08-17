#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PIDS_DIR="$ROOT/infra/pids"

echo "Stopping MEVana dev stack…"

# 1) If we started inside tmux, kill the session
if command -v tmux >/dev/null 2>&1; then
  if tmux has-session -t mevana-dev 2>/dev/null; then
    echo " - killing tmux session: mevana-dev"
    tmux kill-session -t mevana-dev || true
  fi
fi

# 2) If PID files exist (future-proof), stop those cleanly
kill_pid_file () {
  local file="$1"
  [[ -f "$file" ]] || return 0
  local pid
  pid="$(cat "$file" 2>/dev/null || true)"
  [[ -n "${pid:-}" ]] || return 0
  if kill -0 "$pid" 2>/dev/null; then
    echo " - stopping pid $pid ($(basename "$file"))"
    kill "$pid" 2>/dev/null || true
    sleep 1
    if kill -0 "$pid" 2>/dev/null; then
      echo "   force killing $pid"
      kill -9 "$pid" 2>/dev/null || true
    fi
  fi
  rm -f "$file" || true
}

if [[ -d "$PIDS_DIR" ]]; then
  for f in "$PIDS_DIR"/*.pid; do
    [[ -e "$f" ]] && kill_pid_file "$f"
  done
fi

# 3) Fallback: kill by binary names (harmless if not running)
pkill -f '/target/debug/dex-stream-orca'      2>/dev/null || true
pkill -f '/target/debug/normalizer-orca'      2>/dev/null || true
pkill -f '/target/debug/price-aggregator'     2>/dev/null || true
pkill -f '/target/debug/ui-gateway'           2>/dev/null || true

# 4) Close common dev ports (UI/Vite + gateway)
if command -v lsof >/dev/null 2>&1; then
  lsof -ti tcp:5173 | xargs -r kill -9 2>/dev/null || true  # ui-web (Vite)
  lsof -ti tcp:8081 | xargs -r kill -9 2>/dev/null || true  # ui-gateway
fi

echo "Done. (If anything lingers, re-run or kill manually with ps/grep.)"
