#!/bin/bash

# Deploy DEX Stream Universal to Docker (Master Infrastructure)
# NO HARDCODED NETWORK ADDRESSES - All configurable via environment variables
# Date: 2025-08-18

set -e

echo "🚀 Deploying DEX Stream Universal to Docker (Master Infrastructure)..."

# Load environment variables
if [ -f "env.dev" ]; then
    echo "📋 Loading development environment configuration..."
    export $(cat env.dev | grep -v '^#' | xargs)
else
    echo "⚠️  No env.dev file found, using defaults"
fi

# Check if mevana_net exists
if ! docker network ls | grep -q "mevana_net"; then
    echo "❌ mevana_net not found. Please create it first:"
    echo "   docker network create mevana_net"
    exit 1
fi

# Check if monitoring stack is running
if ! docker ps | grep -q "mevana_prometheus"; then
    echo "⚠️  Monitoring stack not running. Please deploy it first:"
    echo "   ./scripts/deploy-monitoring.sh"
    exit 1
fi

echo "🔧 Starting DEX Stream Universal..."

# Start the service from master infrastructure
cd ../../infrastructure
docker compose up -d dex-stream-universal

echo "⏳ Waiting for service to be healthy..."

# Wait for the service to be ready
echo "📊 Waiting for DEX Stream Universal..."
until curl -s http://localhost:${HEALTH_PORT:-8084}/health > /dev/null; do
    sleep 2
done
echo "✅ DEX Stream Universal is ready"

echo ""
echo "🎉 DEX Stream Universal Deployed Successfully!"
echo ""
echo "📊 Service:"
echo "   Health Check: http://localhost:${HEALTH_PORT:-8084}/health"
echo "   Metrics:      http://localhost:${HEALTH_PORT:-8084}/metrics"
echo ""
echo "🔗 Monitoring Integration:"
echo "   Prometheus:   http://localhost:9090"
echo "   Grafana:      http://localhost:3000"
echo ""
echo "📋 Verification Steps:"
echo "   1. Check Prometheus targets: http://localhost:9090/targets"
echo "   2. Verify metrics are flowing: http://localhost:9090/graph?query=dex_stream_v2_transactions_processed_total"
echo "   3. View dashboard in Grafana: http://localhost:3000"
echo ""
echo "💡 To view logs:"
echo "   cd ../../infrastructure && docker compose logs -f dex-stream-universal"
echo ""
