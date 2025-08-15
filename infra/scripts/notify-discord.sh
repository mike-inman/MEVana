#!/usr/bin/env bash
set -euo pipefail

WEBHOOK="${DISCORD_WEBHOOK:-}"
MESSAGE="${1:-AI Intelligence notification}"

if [[ -n "$WEBHOOK" ]]; then
  curl -H "Content-Type: application/json" -X POST -d "{\"content\":\"$MESSAGE\"}" "$WEBHOOK"
else
  echo "No DISCORD_WEBHOOK set, skipping notification"
fi