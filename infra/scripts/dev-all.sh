#!/usr/bin/env bash
set -euo pipefail

# --- paths & logs ---
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT/logs"
mkdir -p "$LOG_DIR"

# --- env ---
# Optional per-dev overrides in .env.dev (not committed)
[[ -f "$ROOT/.env.dev" ]] && source "$ROOT/.env.dev"

: "${KAFKA_BROKER:=localhost:29092}"
: "${PRICES_TOPIC:=prices.latest}"
: "${RAW_TOPIC:=orca-swaps}"
: "${PARSED_TOPIC:=orca-swaps.parsed}"
: "${DLQ_TOPIC:=orca-swaps.dlq}"

# dex-stream-orca (needs these or it will be skipped)
: "${ENDPOINT:=}"         # e.g. https://grpc.ny.shyft.to
: "${X_TOKEN:=}"          # geyser API key
: "${DEX_GROUP_ID:=dex-stream-orca}"

# normalizer-orca env (matches your code)
: "${NORM_GROUP_ID:=normalizer-orca}"
: "${NORM_IN_TOPIC:=$RAW_TOPIC}"
: "${NORM_OUT_TOPIC:=$PARSED_TOPIC}"
: "${NORM_DLQ_TOPIC:=$DLQ_TOPIC}"

# price-aggregator env (matches your code)
: "${AGG_GROUP_ID:=price-aggregator}"
: "${AGG_IN_TOPIC:=$PARSED_TOPIC}"
: "${AGG_OUT_TOPIC:=$PRICES_TOPIC}"

# ui-gateway WS bridge
: "${WS_PORT:=8081}"

# --- helpers ---
cleanup() {
  echo -e "\n🧹 Stopping all child processes..."
  pkill -P $$ || true
  wait || true
  echo "Done."
}
trap cleanup EXIT

ensure_topic() {
  local topic="$1"
  local cfg="${2:-}"
  if command -v kafka-topics >/dev/null 2>&1; then
    kafka-topics --bootstrap-server "$KAFKA_BROKER" \
      --create --if-not-exists --topic "$topic" \
      --partitions 3 --replication-factor 1 ${cfg:+--config "$cfg"} \
      >/dev/null || true
  else
    echo "⚠️  kafka-topics not found in PATH; skipping topic creation for $topic"
  fi
}

run_svc() {
  local name="$1"
  local cmd="$2"
  echo "▶️  $name"
  bash -lc "$cmd" >"$LOG_DIR/$name.log" 2>&1 &
  echo $! > "$LOG_DIR/$name.pid"
}

# --- topics ---
echo "🔧 Ensuring Kafka topics on $KAFKA_BROKER"
ensure_topic "$RAW_TOPIC"
ensure_topic "$PARSED_TOPIC"
ensure_topic "$DLQ_TOPIC"
ensure_topic "$PRICES_TOPIC" "cleanup.policy=compact"

echo "🗂  Logs in: $LOG_DIR"
echo

# --- services ---
pushd "$ROOT" >/dev/null

# 1) Producer (dex-stream-orca) — start only if ENDPOINT & X_TOKEN are set
if [[ -n "$ENDPOINT" && -n "$X_TOKEN" ]]; then
  run_svc "dex-stream-orca" \
    "ENDPOINT='$ENDPOINT' X_TOKEN='$X_TOKEN' \
     KAFKA_BROKER='$KAFKA_BROKER' TOPIC='$RAW_TOPIC' \
     RUST_LOG=info cargo run -p dex-stream-orca"
else
  echo "⏭️  Skipping dex-stream-orca (set ENDPOINT and X_TOKEN in .env.dev to enable)"
fi

# 2) normalizer-orca
run_svc "normalizer-orca" \
  "KAFKA_BROKER='$KAFKA_BROKER' \
   IN_TOPIC='$NORM_IN_TOPIC' OUT_TOPIC='$NORM_OUT_TOPIC' DLQ_TOPIC='$NORM_DLQ_TOPIC' \
   GROUP_ID='$NORM_GROUP_ID' \
   RUST_LOG=info cargo run -p normalizer-orca"

# 3) price-aggregator
run_svc "price-aggregator" \
  "KAFKA_BROKER='$KAFKA_BROKER' \
   IN_TOPIC='$AGG_IN_TOPIC' OUT_TOPIC='$AGG_OUT_TOPIC' GROUP_ID='$AGG_GROUP_ID' \
   RUST_LOG=info cargo run -p price-aggregator"

# 4) ui-gateway (WS)
run_svc "ui-gateway" \
  "KAFKA_BROKER='$KAFKA_BROKER' PRICES_TOPIC='$PRICES_TOPIC' SWAPS_TOPIC='$PARSED_TOPIC' \
   RUST_LOG=info cargo run -p ui-gateway"

# 5) ui-web (Vite dev)
if [[ -d "$ROOT/crates/ui-web" ]]; then
  run_svc "ui-web" \
    "cd crates/ui-web && npm run dev -- --port 5173"
else
  echo "⏭️  Skipping ui-web (crates/ui-web not found)"
fi

popd >/dev/null

echo
echo "✅ All processes launched."
echo "   UI (Vite):          http://localhost:5173"
echo "   Gateway WS:         ws://localhost:$WS_PORT/ws/{prices,swaps,ping}"
echo
echo "📜 Tailing logs (Ctrl-C to stop all):"
echo "    tail -f $LOG_DIR/*.log"
echo
tail -f "$LOG_DIR"/*.log
