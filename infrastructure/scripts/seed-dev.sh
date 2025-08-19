#!/usr/bin/env bash
set -euo pipefail

BROKER="${KAFKA_BROKER:-localhost:29092}"
PRICES_TOPIC="${PRICES_TOPIC:-prices.latest}"
SWAPS_TOPIC="${SWAPS_TOPIC:-orca-swaps.parsed}"

echo "Seeding to $BROKER → [$SWAPS_TOPIC, $PRICES_TOPIC]"

KAFKA_BROKER="$BROKER" \
PRICES_TOPIC="$PRICES_TOPIC" \
SWAPS_TOPIC="$SWAPS_TOPIC" \
cargo run -p dev-fixtures
