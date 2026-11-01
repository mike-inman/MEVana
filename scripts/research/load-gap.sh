#!/usr/bin/env bash
# 2026-10-31: Load a validated Indicative gap survey in one transaction.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [[ $# -lt 2 ]]; then
  printf 'usage: load-gap.sh GATE.json ANALYSIS.json [ANALYSIS.json ...]\n' >&2
  exit 2
fi
gate="$1"
shift
compose=(docker compose --env-file "$root/infrastructure/research.env" -p mevana-research -f "$root/infrastructure/docker-compose.research.yml")
"${compose[@]}" exec -T postgres psql -v ON_ERROR_STOP=1 -U postgres -d research < "$root/infrastructure/research/schema.sql" >/dev/null
python3 "$root/scripts/research/gap-sql.py" --allowlist "$root/crates/gap-survey/allowlist.json" --gate "$gate" --analyses "$@" |
  "${compose[@]}" exec -T postgres psql -v ON_ERROR_STOP=1 -U postgres -d research >/dev/null
printf 'Loaded Indicative gap survey aggregates.\n'
