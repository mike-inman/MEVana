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

# 4) Validate dashboard JSON files parse
dash_files=$(find infrastructure/grafana/dashboards -maxdepth 1 -type f -name '*.json' 2>/dev/null || true)
if [ -n "$dash_files" ]; then
  echo "Validating dashboard JSON files..."
  has_jq=$(command -v jq || true)
  for f in $dash_files; do
    if [ -n "$has_jq" ]; then
      jq empty "$f" || fail "Invalid JSON: $f"
    else
      python3 -c "import json,sys;json.load(open(sys.argv[1]))" "$f" || fail "Invalid JSON: $f"
    fi
  done
fi

# 5) Soft check: docs timestamp presence (warn only)
missing_ts=()
while IFS= read -r -d '' f; do
  if ! grep -Eq '^(Last updated:|Created:)' "$f"; then
    missing_ts+=("$f")
  fi
done < <(find docs -type f -name '*.md' -print0 2>/dev/null)

if [ ${#missing_ts[@]} -gt 0 ]; then
  echo "⚠️  WARNING: The following docs lack a timestamp (Last updated: or Created:):" >&2
  for f in "${missing_ts[@]}"; do echo " - $f" >&2; done
  echo "Add a timestamp near the top per CONTRIBUTING.md." >&2
fi

echo "✅ Repo hygiene passed"

# 6) Soft check: shell scripts live in standard locations
# Allow:
# - scripts/** (top-level operational scripts)
# - infrastructure/scripts/** (infra helpers)
# - crates/**/scripts/** (crate-local helpers)
# - infrastructure/*.sh (explicit allowlist for a few infra root scripts)
stray_scripts=()
while IFS= read -r -d '' s; do
  case "$s" in
    ./scripts/*|./infrastructure/scripts/*|./crates/*/scripts/*|./infrastructure/*.sh)
      : ;; # allowed
    *)
      stray_scripts+=("$s")
      ;;
  esac
done < <(find . -type f -name '*.sh' -not -path './.git/*' -not -path './target/*' -print0)

if [ ${#stray_scripts[@]} -gt 0 ]; then
  echo "⚠️  WARNING: Shell scripts outside standard locations detected:" >&2
  for s in "${stray_scripts[@]}"; do echo " - $s" >&2; done
  echo "Prefer placing scripts under 'scripts/**' or 'infrastructure/scripts/**' (or crate-local 'crates/**/scripts/**')." >&2
fi
