#!/bin/bash

echo "🚀 Starting DEX Stream Universal for Whirlpool..."
echo "📡 Configuring for Whirlpool program ID..."

# Load environment variables from .env file
if [ -f ".env" ]; then
    echo "📄 Loading environment from .env file..."
    export $(cat .env | grep -v '^#' | xargs)
fi

# Override for Whirlpool streaming
export ENDPOINT="https://grpc.ny.shyft.to"
export X_TOKEN="${X_TOKEN}"
export DEX_FILTERING="ON"
export PROGRAM_IDS="whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
export KAFKA_BROKER="localhost:29092"
export KAFKA_TOPIC="raw.dex.transactions"
export DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb"
export HEALTH_PORT="8084"
export BIND_ADDRESS="0.0.0.0"
export RUST_LOG="info"

echo "🔗 Geyser Endpoint: $ENDPOINT"
echo "🎯 Program ID: $PROGRAM_IDS"
echo "📊 Kafka Broker: $KAFKA_BROKER"
echo "📝 Kafka Topic: $KAFKA_TOPIC"
echo "🗄️  Database: $DATABASE_URL"
echo "🏥 Health Port: $HEALTH_PORT"
echo "📋 Log Level: $RUST_LOG"
echo "🔑 Token: ${X_TOKEN:0:8}..."

echo "✅ Starting service..."
cargo run --release --bin dex-stream-universal
