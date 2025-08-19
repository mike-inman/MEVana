#!/bin/bash

# Deploy Monitoring Stack for MEVana
# NO HARDCODED NETWORK ADDRESSES - All configurable via environment variables
# Date: 2025-08-18

set -e

echo "🚀 Deploying MEVana Monitoring Stack..."

# Load environment variables
if [ -f "env.monitoring" ]; then
    echo "📋 Loading monitoring environment configuration..."
    export $(cat env.monitoring | grep -v '^#' | xargs)
else
    echo "⚠️  No env.monitoring file found, using defaults"
fi

# Check if mevana_net exists
if ! docker network ls | grep -q "mevana_net"; then
    echo "❌ mevana_net not found. Please create it first:"
    echo "   docker network create mevana_net"
    exit 1
fi

echo "🔧 Starting monitoring services..."

# Start the monitoring stack
docker compose -f docker-compose.monitoring.yml up -d

echo "⏳ Waiting for services to be healthy..."

# Wait for PostgreSQL
echo "📊 Waiting for PostgreSQL..."
until docker exec mevana_postgres pg_isready -U ${POSTGRES_USER:-mevana_user} > /dev/null 2>&1; do
    sleep 2
done
echo "✅ PostgreSQL is ready"

# Wait for Prometheus
echo "📈 Waiting for Prometheus..."
until curl -s http://localhost:${PROMETHEUS_PORT:-9090}/-/healthy > /dev/null; do
    sleep 2
done
echo "✅ Prometheus is ready"

# Wait for Grafana
echo "📊 Waiting for Grafana..."
until curl -s http://localhost:${GRAFANA_PORT:-3000}/api/health > /dev/null; do
    sleep 2
done
echo "✅ Grafana is ready"

echo ""
echo "🎉 Monitoring Stack Deployed Successfully!"
echo ""
echo "📊 Services:"
echo "   PostgreSQL: localhost:${POSTGRES_PORT:-5432}"
echo "   Prometheus: http://localhost:${PROMETHEUS_PORT:-9090}"
echo "   Grafana:   http://localhost:${GRAFANA_PORT:-3000}"
echo ""
echo "🔑 Grafana Login:"
echo "   Username: ${GRAFANA_ADMIN_USER:-admin}"
echo "   Password: ${GRAFANA_ADMIN_PASSWORD:-admin}"
echo ""
echo "📋 Next Steps:"
    echo "   1. Deploy dex-stream-universal to Docker"
echo "   2. Verify metrics are flowing in Prometheus"
echo "   3. Check dashboard in Grafana"
echo ""
