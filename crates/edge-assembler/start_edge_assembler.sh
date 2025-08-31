#!/bin/bash

# Start Edge Assembler Service
echo "🚀 Starting Edge Assembler Service..."

# Set environment variables
export KAFKA_BROKER="localhost:29092"
export ORCA_POOLS_TOPIC="orca-swaps.parsed.v2"
export ORCA_TICKARRAYS_TOPIC="orca-tickarrays.parsed.v2"
export EDGE_DELTA_TOPIC="edge.delta.v1"
export EDGE_SNAPSHOT_TOPIC="edge.snapshot.v1"
export SNAPSHOT_INTERVAL_MS="5000"

# Set log level
export RUST_LOG="info"

# Start the service
echo "📡 Connecting to Kafka at: $KAFKA_BROKER"
echo "📥 Input topics: $ORCA_POOLS_TOPIC, $ORCA_TICKARRAYS_TOPIC"
echo "📤 Output topics: $EDGE_DELTA_TOPIC, $EDGE_SNAPSHOT_TOPIC"
echo "⏱️  Snapshot interval: ${SNAPSHOT_INTERVAL_MS}ms"

cargo run --release
