#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
compose=(docker compose --env-file "$repo_root/infrastructure/research.env" -p mevana-research -f "$repo_root/infrastructure/docker-compose.research.yml")
temp_dir="$(mktemp -d /tmp/mevana-research-load.XXXXXX)"
trap 'if [[ -e "$temp_dir/seed.sql" ]]; then unlink "$temp_dir/seed.sql"; fi; rmdir "$temp_dir"' EXIT

# Validate all pins before changing the database.
dotnet run --project "$repo_root/dotnet/src/MEVana.Research.SqlExport/MEVana.Research.SqlExport.csproj" -- \
  "$repo_root/dotnet/data/research-artifacts" "$temp_dir/seed.sql"

"${compose[@]}" exec -T postgres sh -c 'exec psql -v ON_ERROR_STOP=1 -U postgres -d research -v reader_password="$RESEARCH_READER_PASSWORD"' <<'SQL'
SELECT format('CREATE ROLE research_reader LOGIN PASSWORD %L', :'reader_password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'research_reader') \gexec
SELECT format('ALTER ROLE research_reader WITH LOGIN PASSWORD %L', :'reader_password') \gexec
SQL
"${compose[@]}" exec -T postgres psql -v ON_ERROR_STOP=1 -U postgres -d research \
  < "$repo_root/infrastructure/research/schema.sql" >/dev/null
"${compose[@]}" exec -T postgres psql -v ON_ERROR_STOP=1 -U postgres -d research \
  < "$temp_dir/seed.sql" >/dev/null
printf 'Validated research seed loaded.\n'
