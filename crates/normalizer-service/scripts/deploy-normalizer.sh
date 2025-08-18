#!/bin/bash

# Deployment script for Normalizer Service
# NO HARDCODED NETWORK ADDRESSES - All configurable via environment variables
# Date: 2025-08-18

set -e

echo "🚀 Deploying Normalizer Service..."

# Load environment variables
if [ -f "env.dev" ]; then
    echo "📁 Loading development environment variables..."
    export $(cat env.dev | grep -v '^#' | xargs)
else
    echo "⚠️  No env.dev file found, using system defaults"
fi

# Check if mevana-network exists
echo "🔍 Checking Docker network..."
if ! docker network ls | grep -q "mevana-network"; then
    echo "❌ mevana-network not found. Please create it first:"
    echo "   docker network create mevana-network"
    exit 1
fi

echo "✅ mevana-network found"

# Build and deploy the service
echo "🔨 Building and deploying normalizer service..."
docker compose --profile dev up -d --build

# Wait for service to start
echo "⏳ Waiting for service to start..."
sleep 10

# Check service health
echo "🏥 Checking service health..."
if curl -f "http://localhost:${HEALTH_PORT:-8085}/health" > /dev/null 2>&1; then
    echo "✅ Service is healthy!"
else
    echo "❌ Service health check failed"
    echo "📋 Checking service logs..."
    docker compose --profile dev logs normalizer_service_dev
    exit 1
fi

# Check service metrics
echo "📊 Checking service metrics..."
if curl -f "http://localhost:${HEALTH_PORT:-8085}/metrics" > /dev/null 2>&1; then
    echo "✅ Metrics endpoint is accessible!"
else
    echo "❌ Metrics endpoint check failed"
fi

echo "🎉 Normalizer Service deployed successfully!"
echo "📍 Health endpoint: http://localhost:${HEALTH_PORT:-8085}/health"
echo "📊 Metrics endpoint: http://localhost:${HEALTH_PORT:-8085}/metrics"
echo "🐳 Container: normalizer_service_dev"
echo "🌐 Network: mevana-network"

# Show running containers
echo ""
echo "📋 Running containers:"
docker compose --profile dev ps
