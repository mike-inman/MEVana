#!/usr/bin/env bash
set -euo pipefail

# Backup full Postgres database from running docker-compose stack.
# Output directory: backups/ (created if missing)
# Uses compose service name 'postgres' and default creds from docker-compose.
# Adjust env vars if you changed credentials.

DB_USER="${POSTGRES_USER:-mev}"
DB_PASSWORD="${POSTGRES_PASSWORD:-<POSTGRES_PASSWORD>}"
DB_NAME="${POSTGRES_DB:-mevdb}"
CONTAINER_NAME="postgres"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
OUT_DIR="backups"
OUT_FILE="${OUT_DIR}/pg-backup-${DB_NAME}-${TIMESTAMP}.sql.gz"

mkdir -p "${OUT_DIR}"

echo "[INFO] Creating logical dump of ${DB_NAME} from container ${CONTAINER_NAME}" >&2
if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
  echo "[ERROR] Postgres container '${CONTAINER_NAME}' not running. Start stack first." >&2
  exit 1
fi

echo "[INFO] Dumping... (this may take a while)" >&2
# Use env passthrough for password inside container context
PGPASSWORD="${DB_PASSWORD}" docker exec -e PGPASSWORD="${DB_PASSWORD}" "${CONTAINER_NAME}" \
  pg_dump -U "${DB_USER}" -d "${DB_NAME}" -F p | gzip > "${OUT_FILE}"

if [[ ! -s "${OUT_FILE}" ]]; then
  echo "[ERROR] Backup file empty or missing: ${OUT_FILE}" >&2
  exit 2
fi

echo "[INFO] Backup complete: ${OUT_FILE}" >&2

echo "[INFO] To restore (destructive):" >&2
echo "       gunzip -c ${OUT_FILE} | PGPASSWORD=... psql -h localhost -p 5433 -U ${DB_USER} -d ${DB_NAME}" >&2
