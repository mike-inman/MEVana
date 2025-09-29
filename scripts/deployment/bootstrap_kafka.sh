#!/usr/bin/env bash
set -euo pipefail

BROKER=${1:-localhost:9092}
KAFKA_CONTAINER=${KAFKA_CONTAINER:-mevana-kafka-1}
TOPICS=(
  dex-transactions
  orca-swaps.parsed.v2
  orca-tickarrays.parsed.v2
  edge.delta.v1
  edge.snapshot.v1
  normalized-swaps.v1
  unknown-programs
  arb.opportunities.v1
  alerts.arbitrage.opportunities
  prices.latest
  execution.results.v1
)

log(){ printf '\033[0;34m[INFO]\033[0m %s\n' "$1"; }
success(){ printf '\033[0;32m[SUCCESS]\033[0m %s\n' "$1"; }
warn(){ printf '\033[1;33m[WARN]\033[0m %s\n' "$1"; }

run_kafka_topics() {
  if command -v kafka-topics >/dev/null 2>&1; then
    kafka-topics --bootstrap-server "$BROKER" "$@"
    return
  fi

  if docker ps --format '{{.Names}}' | grep -qx "$KAFKA_CONTAINER"; then
    docker exec "$KAFKA_CONTAINER" kafka-topics --bootstrap-server "$BROKER" "$@"
    return
  fi

  warn "kafka-topics CLI not available and container $KAFKA_CONTAINER not running"
  return 1
}

log "Ensuring Kafka topics on broker $BROKER"
for topic in "${TOPICS[@]}"; do
  run_kafka_topics --create --if-not-exists --topic "$topic" --partitions 3 --replication-factor 1 >/dev/null
  success "Topic $topic ready"
done
