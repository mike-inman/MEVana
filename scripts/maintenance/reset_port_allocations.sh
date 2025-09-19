#!/usr/bin/env bash

set -euo pipefail

# Path used by shared-core::port_manager for persisted allocations
PORT_FILE="/tmp/mevana_port_allocations.json"

echo "🔧 Resetting MEVana port allocations"

if [[ -f "${PORT_FILE}" ]]; then
  echo "📄 Found existing allocation file at ${PORT_FILE}, removing..."
  rm -f -- "${PORT_FILE}"
  echo "✅ Port allocation cache cleared"
else
  echo "ℹ️ No allocation file present; nothing to clear"
fi

echo "💡 Next: restart any running MEVana services so they bind to their configured ports"
