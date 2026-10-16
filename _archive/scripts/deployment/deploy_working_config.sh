#!/bin/bash
#
# Deploy MEVana using the EXACT working local dev configuration
# This ensures server mirrors local dev environment exactly
#

set -e

echo "🚀 MEVana Working Config Deployment"
echo "📊 Deploying exact copy of local dev environment"

# Ensure we're in the right directory
cd /opt/MEVana

# Ensure Docker network exists
echo "🔧 Setting up Docker network..."
docker network create mevana_net 2>/dev/null || true

# Stop any existing services
echo "🛑 Stopping existing services..."
docker compose -f infrastructure/docker-compose.yml down || true

# Build all services
echo "🏗️ Building MEVana services..."
docker compose -f infrastructure/docker-compose.yml build

# Start infrastructure services first
echo "🏗️ Starting infrastructure services..."
docker compose -f infrastructure/docker-compose.yml up -d postgres redis kafka zookeeper prometheus grafana kafka-ui adminer redis-ui

# Wait for infrastructure to be ready
echo "⏳ Waiting for infrastructure to be ready..."
sleep 15

# Start MEVana application services
echo "🚀 Starting MEVana application services..."
docker compose -f infrastructure/docker-compose.yml up -d hotpath-arb-engine normalizer-service discovery-service executor-service

echo "⏳ Waiting for all services to initialize..."
sleep 20

# Show status
echo "📊 Service status:"
docker compose -f infrastructure/docker-compose.yml ps

echo ""
echo "✅ Deployment completed!"
echo ""
echo "📊 Access your services (matching local dev):"
echo "   Grafana: http://localhost:3300 (admin/<ADMIN_PASSWORD>)"
echo "   Prometheus: http://localhost:9091"
echo "   Hotpath metrics: http://localhost:8088/metrics"
echo "   Kafka UI: http://localhost:8084"
echo "   Redis UI: http://localhost:8083"
echo "   Adminer: http://localhost:8087"
echo ""
echo "🎯 This deployment exactly mirrors your working local dev environment!"