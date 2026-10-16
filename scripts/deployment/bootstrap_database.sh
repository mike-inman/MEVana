#!/usr/bin/env bash
set -euo pipefail

log()  { printf '\033[0;34m[INFO]\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33m[WARN]\033[0m %s\n' "$1"; }
success(){ printf '\033[0;32m[SUCCESS]\033[0m %s\n' "$1"; }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCHEMA_FILE="$REPO_ROOT/../database/complete_schema.sql"
DB_URL=${DATABASE_URL:-postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb}
POSTGRES_CONTAINER=${POSTGRES_CONTAINER:-mevana-postgres-1}
POSTGRES_USER=${POSTGRES_USER:-mev}
POSTGRES_DB=${POSTGRES_DB:-mevdb}

exec_psql() {
  if command -v psql >/dev/null 2>&1; then
    psql "$DB_URL" "$@"
    return
  fi

  if docker ps --format '{{.Names}}' | grep -qx "$POSTGRES_CONTAINER"; then
    docker exec -i "$POSTGRES_CONTAINER" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" "$@"
    return
  fi

  warn "psql client not available and container $POSTGRES_CONTAINER not running; skipping database bootstrap"
  return 1
}

exec_sql_file() {
  if command -v psql >/dev/null 2>&1; then
    psql "$DB_URL" -f "$1"
  elif docker ps --format '{{.Names}}' | grep -qx "$POSTGRES_CONTAINER"; then
    cat "$1" | docker exec -i "$POSTGRES_CONTAINER" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB"
  else
    warn "Cannot apply schema – psql client and $POSTGRES_CONTAINER both unavailable"
    return 1
  fi
}

log "Ensuring database schema exists ($DB_URL)"

if exec_psql -tAc "SELECT to_regclass('public.vault_config')" 2>/dev/null | grep -q 'vault_config'; then
  log "vault_config table present"
else
  log "vault_config table missing – loading full schema"
  exec_sql_file "$SCHEMA_FILE"
fi

COUNT_RAW=$(exec_psql -tAc "SELECT COUNT(*) FROM vault_config" 2>/dev/null | tr -d '[:space:]') || COUNT_RAW=""
if [[ "$COUNT_RAW" =~ ^[0-9]+$ ]]; then
  COUNT="$COUNT_RAW"
  if [[ "$COUNT" == "0" ]]; then
    exec_psql -c "INSERT INTO vault_config (id, salt) VALUES (1, decode('placeholder_salt_will_be_replaced', 'hex')) ON CONFLICT (id) DO NOTHING;"
    log "Seeded vault_config"
  else
    log "vault_config already seeded ($COUNT rows)"
  fi
else
  warn "Unable to query vault_config count"
fi

success "Database bootstrap completed"
