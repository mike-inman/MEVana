#!/usr/bin/env bash
set -euo pipefail

echo "🚀 Creating Kafka topics for trading platform..."

# Configurable bootstrap (inside the container)
BOOTSTRAP="${BOOTSTRAP:-localhost:9092}"

# Find a Kafka container (name contains kafka or broker)
KAFKA_CONTAINER="$(docker ps --format '{{.Names}}' | grep -E 'kafka|broker' | grep -v ui | head -n1 || true)"
if [[ -z "${KAFKA_CONTAINER}" ]]; then
  echo "❌ Kafka container not found!"
  echo "Available containers:"
  docker ps --format "table {{.Names}}\t{{.Status}}"
  exit 1
fi

echo "📋 Using Kafka container: ${KAFKA_CONTAINER}"

# Figure out which CLI is available inside
if docker exec "${KAFKA_CONTAINER}" bash -lc 'command -v kafka-topics >/dev/null 2>&1'; then
  KCLI="kafka-topics"
elif docker exec "${KAFKA_CONTAINER}" bash -lc 'command -v kafka-topics.sh >/dev/null 2>&1'; then
  KCLI="kafka-topics.sh"
else
  echo "❌ Neither kafka-topics nor kafka-topics.sh found in the container."
  exit 1
fi
echo "🛠  Using CLI: ${KCLI}"

# Wait for broker to be up
echo "🔍 Checking if Kafka is ready..."
if ! docker exec "${KAFKA_CONTAINER}" "${KCLI}" --bootstrap-server "${BOOTSTRAP}" --list >/dev/null 2>&1; then
  echo "⚠️  Kafka not ready yet. Waiting 30 seconds..."
  sleep 30
  if ! docker exec "${KAFKA_CONTAINER}" "${KCLI}" --bootstrap-server "${BOOTSTRAP}" --list >/dev/null 2>&1; then
    echo "❌ Kafka still not ready. Check logs:"
    echo "docker logs ${KAFKA_CONTAINER}"
    exit 1
  fi
fi
echo "✅ Kafka is ready!"

create_topic () {
  local topic="$1"
  local partitions="${2:-3}"
  shift 2 || true
  local extra_cfg=("$@")

  echo "📦 Creating topic: ${topic} (partitions=${partitions})"
  docker exec "${KAFKA_CONTAINER}" "${KCLI}" \
    --create \
    --if-not-exists \
    --topic "${topic}" \
    --bootstrap-server "${BOOTSTRAP}" \
    --partitions "${partitions}" \
    --replication-factor 1 \
    ${extra_cfg[@]+"${extra_cfg[@]}"} \
    >/dev/null 2>&1 || true

  echo "✅ Ensured topic ${topic}"
}

# ----- Choose a consistent topic layout -----
# Raw/decoded per DEX:
create_topic "raw.swaps.whirlpool" 3
create_topic "raw.swaps.raydium" 3
create_topic "raw.swaps.meteora" 3
create_topic "raw.swaps.pumpfun" 3

# Normalized (output from normalizer-service)
create_topic "normalized.swaps" 6

# Enriched/derived
create_topic "enriched.swaps.all" 6
create_topic "enriched.swaps.large" 3

# Alerts
create_topic "alerts.arbitrage.opportunities" 3
create_topic "alerts.high.impact" 3

echo ""
echo "📋 All topics:"
docker exec "${KAFKA_CONTAINER}" "${KCLI}" --list --bootstrap-server "${BOOTSTRAP}"

echo ""
echo "🎉 Topic creation complete!"
echo "💡 If you run Kafka UI, it is commonly at http://localhost:8080"
