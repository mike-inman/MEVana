#!/usr/bin/env bash
set -euo pipefail
BOOTSTRAP="${BOOTSTRAP:-kafka:9092}"

topics=(
  "orca.raw.channel:1:1"
  "orca.raw.stream:1:1"
  "orca.processed.channel:1:1"
  "orca.processed.stream:1:1"
  "orca.normalized-swap.stream:1:1"
)

for t in "${topics[@]}"; do
  IFS=: read -r name partitions rf <<< "$t"
  echo "Creating topic $name"
  kafka-topics.sh --bootstrap-server "$BOOTSTRAP" \
    --create --if-not-exists --topic "$name" \
    --partitions "${partitions:-1}" --replication-factor "${rf:-1}" || true
done

echo "Topic creation done."
