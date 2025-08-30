#!/bin/bash

# Test Environment Deployment Script
# This script deploys the dex-stream-universal service in test mode

set -e

echo "🧪 Deploying DEX Stream Universal V2 - Test Environment"

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker and try again."
    exit 1
fi

# Load environment variables
if [ -f "env.test" ]; then
    echo "📋 Loading test environment configuration..."
    export $(cat env.test | grep -v '^#' | xargs)
else
    echo "❌ Error: env.test file not found. Please create it with test configuration."
    exit 1
fi

# Validate required environment variables
if [ -z "$TEST_KAFKA_BROKER" ] || [ -z "$TEST_X_TOKEN" ]; then
    echo "❌ Error: Required environment variables not set:"
    echo "   TEST_KAFKA_BROKER: $TEST_KAFKA_BROKER"
    echo "   TEST_X_TOKEN: $TEST_X_TOKEN"
    exit 1
fi

# Build and start test service
echo "🔨 Building and starting test service..."
docker compose --profile test up -d --build

# Wait for service to be healthy
echo "⏳ Waiting for service to be healthy..."
sleep 30

# Check service health
echo "🏥 Checking service health..."
if curl -f http://:8081/health > /dev/null 2>&1; then
    echo "✅ Test service is healthy!"
else
    echo "❌ Test service health check failed!"
    docker compose --profile test logs dex_stream_universal_test
    exit 1
fi

echo "🎉 Test deployment completed successfully!"
echo ""
echo "📋 Service Information:"
echo "   Health Check: http://:8081/health"
echo "   Metrics: http://:8081/metrics"
echo "   Kafka Broker: $TEST_KAFKA_BROKER"
echo "   Topic: ${TEST_TOPIC:-dex-transactions}"
echo ""
echo "🔍 To view logs: docker compose --profile test logs -f dex_stream_universal_test"
echo "🛑 To stop: docker compose --profile test down"
