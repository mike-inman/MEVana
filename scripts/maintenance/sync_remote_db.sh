#!/usr/bin/env bash
set -euo pipefail

# Synchronise the local Postgres container with a remote instance referenced by
# REMOTE_DATABASE_URL. Remote credentials are loaded from the first available
# env file (MEVANA_ENV_FILE, infrastructure/.env, .env, or ~/.mevana.env).
#
# Usage:
#   scripts/maintenance/sync_remote_db.sh
#
# Requirements:
#   - Docker and docker compose available
#   - Remote Postgres reachable from this machine
#   - REMOTE_DATABASE_URL defined in the env file chosen below
#   - Optional overrides: LOCAL_DATABASE_URL, POSTGRES_USER, POSTGRES_PASSWORD

log() {
  printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$1"
}

fail() {
  printf '[%s] ERROR: %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$1" >&2
  exit 1
}

repo_root=$(git rev-parse --show-toplevel)
compose_file="$repo_root/infrastructure/docker-compose.core.yml"
backup_dir="$repo_root/backups"

# Locate env file
if [[ -n "${MEVANA_ENV_FILE:-}" ]]; then
  env_file="$MEVANA_ENV_FILE"
elif [[ -f "$repo_root/infrastructure/.env" ]]; then
  env_file="$repo_root/infrastructure/.env"
elif [[ -f "$repo_root/.env" ]]; then
  env_file="$repo_root/.env"
elif [[ -f "$HOME/.mevana.env" ]]; then
  env_file="$HOME/.mevana.env"
else
  fail "No environment file found (set MEVANA_ENV_FILE or create infrastructure/.env)."
fi

log "Loading secrets from: $env_file"
set -a
source "$env_file"
set +a

[[ -n "${REMOTE_DATABASE_URL:-}" ]] || fail "REMOTE_DATABASE_URL must be set in the env file."

# Default local connection maps to the docker-compose Postgres service exposure.
default_local_url="postgresql://${POSTGRES_USER:-mev}:${POSTGRES_PASSWORD:-<POSTGRES_PASSWORD>}@localhost:5433/${POSTGRES_DB:-mevdb}"
LOCAL_DATABASE_URL="${LOCAL_DATABASE_URL:-$default_local_url}"
export REMOTE_DATABASE_URL
export LOCAL_DATABASE_URL

# Resolve which tables to synchronise (space or comma separated list). If empty,
# the script falls back to a curated set of configuration tables.
if [[ -n "${DB_SYNC_TABLES:-}" ]]; then
  raw_table_list="$DB_SYNC_TABLES"
else
  raw_table_list="vault_config dex_program_registry circuit_breaker_state service_levels smart_filter_config_history parameter_performance_impact smart_filter_performance_snapshots venue_performance_history"
fi

requested_tables=()
if [[ -n "$raw_table_list" ]]; then
  normalized_tables=$(RAW_TABLE_LIST="$raw_table_list" python3 <<'PY'
import os

entries = []
for chunk in os.environ.get("RAW_TABLE_LIST", "").replace(',', ' ').split():
    item = chunk.strip()
    if item and item not in entries:
        entries.append(item)

print('\n'.join(entries))
PY
  )

  if [[ -n "$normalized_tables" ]]; then
    while IFS= read -r entry; do
      [[ -n "$entry" ]] || continue
      requested_tables+=("$entry")
    done <<< "$normalized_tables"
  fi
fi

eval "$(python3 <<'PY'
import os
import shlex
from urllib.parse import urlparse


def parse(name):
    value = os.environ[name]
    parsed = urlparse(value)
    if parsed.scheme not in {"postgres", "postgresql"}:
        raise SystemExit(f"Unsupported scheme in {name}: {parsed.scheme}")
    if parsed.hostname is None:
        raise SystemExit(f"Missing hostname in {name}")
    db_name = parsed.path.lstrip('/')
    if not db_name:
        raise SystemExit(f"Missing database name in {name}")
    return {
        'user': parsed.username or '',
        'password': parsed.password or '',
        'host': parsed.hostname,
        'port': str(parsed.port or 5432),
        'name': db_name,
    }


remote = parse('REMOTE_DATABASE_URL')
local = parse('LOCAL_DATABASE_URL')


for prefix, values in [('REMOTE_DB_', remote), ('LOCAL_DB_', local)]:
    for key, value in values.items():
        var = prefix + key.upper()
        print(f"{var}={shlex.quote(value)}")
PY
)"

table_args=()
truncate_targets=()
if (( ${#requested_tables[@]} )); then
  log "Checking remote database for requested tables"
  existing_raw=$(docker run --rm \
    -e PGPASSWORD="$REMOTE_DB_PASSWORD" \
    postgres:15 \
    psql --host="$REMOTE_DB_HOST" --port="$REMOTE_DB_PORT" \
         --username="$REMOTE_DB_USER" --dbname="$REMOTE_DB_NAME" \
         -Atc "SELECT table_schema || '.' || table_name FROM information_schema.tables WHERE table_type='BASE TABLE' AND table_schema NOT IN ('pg_catalog','information_schema');")

  existing_tables=()
  if [[ -n "$existing_raw" ]]; then
    while IFS= read -r line; do
      [[ -n "$line" ]] || continue
      existing_tables+=("$line")
    done <<< "$existing_raw"
  fi

  if (( ${#existing_tables[@]} == 0 )); then
    log "No tables reported by remote database"
  fi

  for entry in "${requested_tables[@]}"; do
    schema_table="$entry"
    [[ "$schema_table" == *.* ]] || schema_table="public.$schema_table"
    found=false
    for candidate in "${existing_tables[@]}"; do
      if [[ "$candidate" == "$schema_table" ]]; then
        found=true
        break
      fi
    done
    if $found; then
      table_args+=("$schema_table")
      truncate_targets+=("$schema_table")
    else
      log "Skipping missing table: $schema_table"
    fi
  done

  if (( ${#table_args[@]} == 0 )); then
    log "No matching tables found; nothing to sync"
    exit 0
  fi
fi

mkdir -p "$backup_dir"
backup_path="$backup_dir/remote-${REMOTE_DB_NAME}-$(date '+%Y%m%d-%H%M%S').sql.gz"

log "Ensuring Postgres container is running"
docker compose -f "$compose_file" up -d postgres >/dev/null

MEVANA_APP_SERVICES=(hotpath-arb-engine normalizer-service discovery-service executor-service)
log "Stopping application containers to avoid active DB connections"
docker compose -f "$compose_file" stop "${MEVANA_APP_SERVICES[@]}" >/dev/null 2>&1 || true

log "Dumping remote database ${REMOTE_DB_NAME}@${REMOTE_DB_HOST}:${REMOTE_DB_PORT}"
tmp_dump=$(mktemp)
trap 'rm -f "$tmp_dump"' EXIT

pg_dump_args=(
  --no-owner
  --no-privileges
  --host="$REMOTE_DB_HOST"
  --port="$REMOTE_DB_PORT"
  --username="$REMOTE_DB_USER"
  --dbname="$REMOTE_DB_NAME"
  --format=p
  --verbose
)

if (( ${#table_args[@]} )); then
  log "Syncing tables: ${table_args[*]}"
  pg_dump_args+=(--data-only)
  for table in "${table_args[@]}"; do
    pg_dump_args+=("--table=${table}")
  done
fi

docker run --rm \
  -e PGPASSWORD="$REMOTE_DB_PASSWORD" \
  postgres:15 \
  pg_dump "${pg_dump_args[@]}" > "$tmp_dump"

if [[ ! -s "$tmp_dump" ]]; then
  fail "Remote dump failed or empty output"
fi

gzip -c "$tmp_dump" > "$backup_path"
log "Remote dump stored at: $backup_path"

log "Preparing local database ${LOCAL_DB_NAME}@${LOCAL_DB_HOST}:${LOCAL_DB_PORT}"
if (( ${#table_args[@]} )); then
  log "Truncating target tables before restore"
  truncate_sql="TRUNCATE TABLE $(IFS=', '; echo "${truncate_targets[*]}") RESTART IDENTITY CASCADE;"
  docker compose -f "$compose_file" exec -T \
    -e PGPASSWORD="$LOCAL_DB_PASSWORD" \
    postgres \
    psql -v ON_ERROR_STOP=1 -U "$LOCAL_DB_USER" -d "$LOCAL_DB_NAME" -c "$truncate_sql" >/dev/null
else
  docker compose -f "$compose_file" exec -T \
    -e PGPASSWORD="$LOCAL_DB_PASSWORD" \
    postgres \
    psql -v ON_ERROR_STOP=1 -U "$LOCAL_DB_USER" -d "$LOCAL_DB_NAME" -c 'DROP SCHEMA public CASCADE' >/dev/null 2>&1 || true

  docker compose -f "$compose_file" exec -T \
    -e PGPASSWORD="$LOCAL_DB_PASSWORD" \
    postgres \
    psql -v ON_ERROR_STOP=1 -U "$LOCAL_DB_USER" -d "$LOCAL_DB_NAME" -c 'CREATE SCHEMA public' >/dev/null
fi

log "Restoring data into local Postgres"
gunzip -c "$backup_path" | docker compose -f "$compose_file" exec -T \
  -e PGPASSWORD="$LOCAL_DB_PASSWORD" \
  postgres \
  psql -v ON_ERROR_STOP=1 -U "$LOCAL_DB_USER" -d "$LOCAL_DB_NAME" >/dev/null

log "Local database sync complete"

log "Restarting application containers"
docker compose -f "$compose_file" up -d "${MEVANA_APP_SERVICES[@]}" >/dev/null

log "Database sync finished successfully"
