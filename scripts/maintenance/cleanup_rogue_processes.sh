#!/usr/bin/env bash
set -euo pipefail

log(){ printf "%s\n" "$*"; }
warn(){ printf "[WARN] %s\n" "$*"; }
info(){ printf "[INFO] %s\n" "$*"; }

if [[ $EUID -ne 0 ]]; then
  warn "Run as root (sudo ./cleanup_rogue_processes.sh)"
  exit 1
fi

# unit -> primary binary name
declare -A UNITS=(
  ["api-service.service"]="api-server"
  ["monitoring-service.service"]="monitoring-service"
  ["hotpath-arb-engine.service"]="hotpath_arb_engine"
  ["normalizer-service.service"]="normalizer-service"
  ["discovery-service.service"]="discovery-service"
  ["executor-service.service"]="executor-service"
  ["wallet-vault.service"]="wallet-vault"
  ["mevana-ui.service"]="node"
)

cleanup_unit(){
  local unit=$1
  local binary=$2
  local main_pid
  if ! systemctl list-unit-files "$unit" >/dev/null 2>&1; then
    warn "Skipping $unit (not installed)"
    return
  fi

  main_pid=$(systemctl show "$unit" -p MainPID --value 2>/dev/null || echo 0)
  [[ -z "$main_pid" ]] && main_pid=0

  mapfile -t pids < <(pgrep -f "$binary" || true)
  if [[ ${#pids[@]} -eq 0 ]]; then
    info "No processes found for $binary"
    return
  fi

  info "Inspecting $unit ($binary)"
  for pid in "${pids[@]}"; do
    [[ -z "$pid" ]] && continue
    if [[ "$pid" == "$main_pid" ]]; then
      info "  Keeping main PID $pid"
      continue
    fi
    if ! ps -p "$pid" >/dev/null 2>&1; then
      continue
    fi
    warn "  Killing rogue PID $pid ($binary)"
    kill "$pid" 2>/dev/null || true
    sleep 1
    if ps -p "$pid" >/dev/null 2>&1; then
      warn "  Force killing PID $pid"
      kill -9 "$pid" 2>/dev/null || true
    fi
  done
}

log "=== MEVana rogue process cleanup ==="
for unit in "${!UNITS[@]}"; do
  cleanup_unit "$unit" "${UNITS[$unit]}"
  systemctl reset-failed "$unit" >/dev/null 2>&1 || true
  systemctl start "$unit" >/dev/null 2>&1 || true
  systemctl status "$unit" --no-pager >/dev/null 2>&1 || warn "  $unit did not start"
  echo
done

info "Active listeners after cleanup:"
ss -tlnp || true
