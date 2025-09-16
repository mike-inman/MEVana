#!/usr/bin/env bash
set -euo pipefail

echo "🔧 Ensuring Rust nightly is available..."
if ! command -v rustup >/dev/null 2>&1; then
  echo "Rustup not found. Install from https://rustup.rs first." >&2
  exit 1
fi

rustup toolchain install nightly >/dev/null 2>&1 || true

echo "🧪 Running cargo checks across the workspace (nightly)..."
export RUSTFLAGS="--cap-lints warn"
cargo +nightly check --workspace

echo "✅ cargo check completed for the entire workspace"

