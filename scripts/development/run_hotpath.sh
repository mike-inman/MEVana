#!/bin/bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

export METRICS_PORT=${METRICS_PORT:-8088}
export WS_BIND=${WS_BIND:-0.0.0.0:8085}
export RUST_LOG=${RUST_LOG:-info}

echo "[INFO] Running hotpath_arb_engine (METRICS_PORT=$METRICS_PORT, WS_BIND=$WS_BIND, RUST_LOG=$RUST_LOG)"
cargo run -p hotpath_arb_engine --bin hotpath_arb_engine "$@"
