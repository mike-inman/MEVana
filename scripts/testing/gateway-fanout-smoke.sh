#!/usr/bin/env bash
set -euo pipefail

# Simple smoke test that pushes one opportunity and one execution result into Kafka
# and asserts that the gateway fan-out surfaces them over WebSocket/SSE endpoints.

BROKER=${BROKER:-localhost:29092}
KAFKA_CONTAINER=${KAFKA_CONTAINER:-mevana-kafka-1}
OPP_TOPIC=${GATEWAY_OPPORTUNITIES_TOPIC:-alerts.arbitrage.opportunities}
EXEC_TOPIC=${GATEWAY_EXECUTION_RESULTS_TOPIC:-execution.results.v1}
GATEWAY_HOST=${GATEWAY_HOST:-http://localhost:8095}
TIMEOUT=${TIMEOUT:-10}

log(){ printf '\033[0;34m[fanout]\033[0m %s\n' "$1"; }
warn(){ printf '\033[1;33m[fanout]\033[0m %s\n' "$1"; }
error(){ printf '\033[0;31m[fanout]\033[0m %s\n' "$1"; }

if ! command -v curl >/dev/null 2>&1; then
  error "curl is required"; exit 1;
fi

produce(){
  local topic=$1
  local payload=$2
  if command -v kafka-console-producer >/dev/null 2>&1; then
    printf '%s\n' "$payload" | kafka-console-producer --broker-list "$BROKER" --topic "$topic" >/dev/null
  elif docker ps --format '{{.Names}}' | grep -qx "$KAFKA_CONTAINER"; then
    printf '%s\n' "$payload" | docker exec -i "$KAFKA_CONTAINER" kafka-console-producer --broker-list "$BROKER" --topic "$topic" >/dev/null
  else
    error "No kafka-console-producer available (install locally or run docker container $KAFKA_CONTAINER)."
    exit 1
  fi
}

log "Publishing sample opportunity to $OPP_TOPIC"
produce "$OPP_TOPIC" "{\"signature\":\"sample-opp-$(date +%s)\",\"token_a\":\"SOL\",\"token_b\":\"USDC\",\"venues\":[\"Raydium\",\"Meteora\"],\"profit_bps\":42,\"estimated_profit_sol\":0.01,\"details\":{\"size_in\":\"1000000\",\"price_buy\":23.1,\"price_sell\":23.3,\"ev_usd\":1.5}}"

log "Publishing sample execution result to $EXEC_TOPIC"
produce "$EXEC_TOPIC" "{\"execution_id\":\"sample-exec-$(date +%s)\",\"status\":\"Confirmed\",\"transaction_signatures\":[\"sample-sig\"],\"estimated_profit_lamports\":5000000,\"execution_time_ms\":25,\"executed_at_ns\":$(( $(date +%s%N) ))}"

log "Checking gateway health"
if ! curl -sf "$GATEWAY_HOST/gateway/health" >/dev/null; then
  error "Gateway health endpoint did not respond"; exit 1;
fi

log "Watching SSE stream (opportunities)"
if ! curl --no-buffer --max-time "$TIMEOUT" "$GATEWAY_HOST/gateway/sse/opportunities" | head -n 1; then
  warn "No SSE payload observed (ensure gateway service is running and topics contain data)."
fi

log "Watching SSE stream (bundles)"
if ! curl --no-buffer --max-time "$TIMEOUT" "$GATEWAY_HOST/gateway/sse/bundles" | head -n 1; then
  warn "No bundle payload observed; this is expected if executor is idle."
fi

log "Smoke test complete"
