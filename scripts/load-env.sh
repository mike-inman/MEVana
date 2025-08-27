#!/usr/bin/env bash
set -euo pipefail

# Local environment loader for MEVana
# Usage: source scripts/load-env.sh

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

ENV_CANDIDATES=(
  "${HOME}/.mevana.env"
  "${ROOT_DIR}/infrastructure/.env"
  "${ROOT_DIR}/.env"
)

FOUND_ENV_FILE=""
for candidate in "${ENV_CANDIDATES[@]}"; do
  if [[ -f "${candidate}" ]]; then
    FOUND_ENV_FILE="${candidate}"
    break
  fi
done

if [[ -z "${FOUND_ENV_FILE}" ]]; then
  echo "No env file found. Create ${HOME}/.mevana.env or infrastructure/.env with your tokens."
  return 1 2>/dev/null || exit 1
fi

# Export all variables defined in the env file without echoing values
set -a
source "${FOUND_ENV_FILE}"
set +a

# Provide sensible local defaults if not set
export KAFKA_BROKERS="${KAFKA_BROKERS:-localhost:29092}"
export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"

echo "Loaded environment from: ${FOUND_ENV_FILE}. Exported variables for current shell."


