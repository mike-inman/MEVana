#!/bin/bash

echo "🚀 Starting Ultra-Low Latency DEX Stream Integrated Service..."
echo "🔥 Single-process architecture for <1ms arbitrage detection"

# Load environment variables from .env file
if [ -f "../dex-stream-universal/.env" ]; then
    echo "📄 Loading environment from dex-stream-universal/.env file..."
    export $(cat ../dex-stream-universal/.env | grep -v '^#' | xargs)
fi

# Override for integrated service
export ENDPOINT="https://grpc.ny.shyft.to"
export X_TOKEN="${X_TOKEN}"
export DEX_FILTERING="ON"
export PROGRAM_IDS="whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
export HEALTH_PORT="8084"
export BIND_ADDRESS="0.0.0.0"
export RUST_LOG="info"
export MIN_PROFIT_BPS="50"
export MAX_SLIPPAGE_BPS="100"

echo "🔧 Configuration:"
echo "  Geyser Endpoint: $ENDPOINT"
echo "  Program ID: $PROGRAM_IDS"
echo "  Health Port: $HEALTH_PORT"
echo "  Min Profit: ${MIN_PROFIT_BPS} bps"
echo "  Max Slippage: ${MAX_SLIPPAGE_BPS} bps"
echo "  Token: ${X_TOKEN:0:8}..."

echo "⚡ Ultra-Low Latency Features:"
echo "  ✅ Single-process architecture"
echo "  ✅ In-memory communication"
echo "  ✅ No network bottlenecks"
echo "  ✅ Direct arbitrage detection"
echo "  ✅ Async persistence layer"

echo "✅ Starting integrated service..."
cargo run --release --bin dex-stream-integrated
