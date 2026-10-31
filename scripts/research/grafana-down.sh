#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
docker compose --env-file "$repo_root/infrastructure/research.env" -p mevana-research -f "$repo_root/infrastructure/docker-compose.research.yml" down
