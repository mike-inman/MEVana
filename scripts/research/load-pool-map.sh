#!/usr/bin/env bash
# 2026-10-31: Transactional pool-map loader; existing local credentials stay private.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
compose=(docker compose --env-file "$root/infrastructure/research.env" -p mevana-research -f "$root/infrastructure/docker-compose.research.yml")
"${compose[@]}" exec -T postgres psql -v ON_ERROR_STOP=1 -U postgres -d research < "$root/infrastructure/research/schema.sql" >/dev/null
python3 "$root/scripts/research/pool_map/load.py" "$1" | "${compose[@]}" exec -T postgres psql -v ON_ERROR_STOP=1 -U postgres -d research >/dev/null
