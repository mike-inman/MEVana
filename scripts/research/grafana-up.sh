#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
env_file="$repo_root/infrastructure/research.env"
if [[ ! -e "$env_file" ]]; then
  umask 077
  temp_file="$(mktemp "$repo_root/infrastructure/.research-env.XXXXXX")"
  trap 'rm -f "$temp_file"' EXIT
  {
    printf 'POSTGRES_PASSWORD=%s\n' "$(openssl rand -hex 32)"
    printf 'RESEARCH_READER_PASSWORD=%s\n' "$(openssl rand -hex 32)"
    printf 'GF_SECURITY_ADMIN_PASSWORD=%s\n' "$(openssl rand -hex 32)"
  } > "$temp_file"
  chmod 600 "$temp_file"
  if [[ -e "$env_file" ]]; then
    echo 'research.env appeared during setup; refusing to overwrite' >&2
    exit 1
  fi
  mv -n "$temp_file" "$env_file"
  trap - EXIT
fi
chmod 600 "$env_file"
if lsof -nP -iTCP:5434 -sTCP:LISTEN >/dev/null || lsof -nP -iTCP:3010 -sTCP:LISTEN >/dev/null; then
  echo 'Research port is occupied; refusing to start' >&2
  exit 1
fi
docker compose --env-file "$env_file" -p mevana-research -f "$repo_root/infrastructure/docker-compose.research.yml" up -d
