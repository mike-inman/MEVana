#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

ENV_FILE="${ROOT}/.env.dev"
if [[ -f "$ENV_FILE" ]]; then
  # export everything from .env.dev
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
fi

# Defaults
KAFKA_BROKER="${KAFKA_BROKER:-localhost:29092}"
TOPIC="${TOPIC:-orca-swaps}"                      # raw geyser stream
SWAPS_PARSED_TOPIC="${SWAPS_PARSED_TOPIC:-orca-swaps.parsed}"
DLQ_TOPIC="${DLQ_TOPIC:-orca-swaps.dlq}"
PRICES_TOPIC="${PRICES_TOPIC:-prices.latest}"

mkdir -p "${ROOT}/infra/logs"

mask() {
  local s="${1:-}"
  local n=${#s}
  if (( n == 0 )); then
    echo "(unset)"
  elif (( n <= 10 )); then
    echo "***${s: -2}"
  else
    echo "${s:0:6}...${s: -4}"
  fi
}

echo "== MEVana dev launcher =="
echo "KAFKA_BROKER=${KAFKA_BROKER}"
echo "ENDPOINT=${ENDPOINT:-"(unset)"}"
echo "X_TOKEN=$(mask "${X_TOKEN:-}")"
echo "TOPICS: in=${TOPIC} parsed=${SWAPS_PARSED_TOPIC} prices=${PRICES_TOPIC} dlq=${DLQ_TOPIC}"
echo

# Ensure compacted prices topic exists (no-op if present)
kafka-topics --bootstrap-server "$KAFKA_BROKER" \
  --create --if-not-exists --topic "$PRICES_TOPIC" \
  --partitions 3 --replication-factor 1 \
  --config cleanup.policy=compact >/dev/null 2>&1 || true

# 1) dex-stream-orca
(
  cd "$ROOT"
  ENDPOINT="${ENDPOINT:-}" \
  X_TOKEN="${X_TOKEN:-}" \
  KAFKA_BROKER="$KAFKA_BROKER" \
  TOPIC="$TOPIC" \
  cargo run -p dex-stream-orca
) >> "${ROOT}/infra/logs/dex-stream-orca.log" 2>&1 & echo $! > "${ROOT}/infra/logs/dex-stream-orca.pid"

# 2) normalizer-orca  (env names match your current main.rs: IN_TOPIC/OUT_TOPIC/DLQ_TOPIC)
(
  cd "$ROOT"
  KAFKA_BROKER="$KAFKA_BROKER" \
  IN_TOPIC="$TOPIC" \
  OUT_TOPIC="$SWAPS_PARSED_TOPIC" \
  DLQ_TOPIC="$DLQ_TOPIC" \
  GROUP_ID="normalizer-orca" \
  cargo run -p normalizer-orca
) >> "${ROOT}/infra/logs/normalizer-orca.log" 2>&1 & echo $! > "${ROOT}/infra/logs/normalizer-orca.pid"

# 3) price-aggregator
(
  cd "$ROOT"
  KAFKA_BROKER="$KAFKA_BROKER" \
  IN_TOPIC="$SWAPS_PARSED_TOPIC" \
  OUT_TOPIC="$PRICES_TOPIC" \
  GROUP_ID="price-aggregator" \
  cargo run -p price-aggregator
) >> "${ROOT}/infra/logs/price-aggregator.log" 2>&1 & echo $! > "${ROOT}/infra/logs/price-aggregator.pid"

# 4) ui-gateway
(
  cd "$ROOT"
  KAFKA_BROKER="$KAFKA_BROKER" \
  PRICES_TOPIC="$PRICES_TOPIC" \
  SWAPS_TOPIC="$SWAPS_PARSED_TOPIC" \
  cargo run -p ui-gateway
) >> "${ROOT}/infra/logs/ui-gateway.log" 2>&1 & echo $! > "${ROOT}/infra/logs/ui-gateway.pid"

# 5) ui-web (Vite dev)
(
  cd "$ROOT/crates/ui-web"
  npm run dev
) >> "${ROOT}/infra/logs/ui-web.log" 2>&1 & echo $! > "${ROOT}/infra/logs/ui-web.pid"

echo "Launched. Logs in: ${ROOT}/infra/logs"
echo "- UI:       http://localhost:5173"
echo "- Gateway:  ws://localhost:8081/ws/{prices,swaps,ping}"
echo
echo "Tip: tail -f infra/logs/*.log"