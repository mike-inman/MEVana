#!/usr/bin/env bash
set -euo pipefail

URL="${1:-http://localhost:8080/metrics}"
echo "Fetching metrics from: $URL" >&2
data=$(curl -fsSL "$URL") || { echo "Failed to fetch metrics" >&2; exit 1; }

pick() { echo "$data" | grep -E "^$1" || true; }

echo "--- Structural Gauges ---"
pick "arb_edge_pairs_total" | tail -n1
pick "arb_tokens_indexed_total" | tail -n1
pick "arb_avg_adjacency" | tail -n1
pick "arb_max_edge_factor" | tail -n1

echo "--- Cycle Diagnostics (if diagnostics feature enabled) ---"
pick "arb_cycles_examined_total"
pick "arb_cycles_pruned_total"
pick "arb_cycles_profitable_total"
pick "arb_pruning_efficiency"

echo "--- Profit Histogram Quantiles ---"
pick "arb_signal_profit_bps_quantile" | sort

echo "--- Latency Histogram Quantiles ---"
pick "arb_signal_latency_ms_quantile" | sort

echo "--- Latest Profit Max ---"
pick "arb_signal_profit_bps_max" | tail -n1

echo "--- Latest Latency Max (ms) ---"
pick "arb_signal_latency_ms_max" | tail -n1

echo "Done." >&2