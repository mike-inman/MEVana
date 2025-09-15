#!/usr/bin/env bash
set -euo pipefail

echo "🔎 Repo hygiene checks..."

fail() { echo "❌ $*" >&2; exit 1; }

# 1) Forbid root-level dashboards or stray dashboards
if ls dashboards/*.json >/dev/null 2>&1; then
  fail "Found dashboards/ at repo root. Move JSON files to infrastructure/grafana/dashboards/."
fi

stray=$(find . -type f -name '*.json' -path '*/dashboards/*' ! -path './infrastructure/grafana/dashboards/*' | wc -l | xargs)
if [ "$stray" != "0" ]; then
  echo "Stray dashboard files:" >&2
  find . -type f -name '*.json' -path '*/dashboards/*' ! -path './infrastructure/grafana/dashboards/*' >&2 || true
  fail "Dashboard JSON must live in infrastructure/grafana/dashboards/."
fi

# 2) Disallow docs-new (all docs must live in docs/)
if [ -d docs-new ]; then
  fail "'docs-new/' detected. Consolidate into 'docs/' and remove 'docs-new/'."
fi

# 3) Forbid OS files and archives
bad=$(git ls-files -z | tr '\0' '\n' | grep -E '\.(tgz|tar\.gz|zip)$|(^|/)\.DS_Store$|(^|/)\._' || true)
if [ -n "$bad" ]; then
  echo "$bad" >&2
  fail "Remove archives and OS files from the repo."
fi

echo "✅ Repo hygiene passed"

