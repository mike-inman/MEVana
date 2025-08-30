#!/bin/bash

# Development Environment Deployment Script
# This script deploys the dex-stream-universal service in development mode

set -e

echo "🚀 Deploying DEX Stream Universal V2 - Development Environment"

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker and try again."
    exit 1
fi

# Load environment variables
if [ -f "env.dev" ]; then
    echo "📋 Loading development environment configuration..."
    export $(cat env.dev | grep -v '^#' | xargs)
else
    echo "⚠️  Warning: env.dev file not found. Using default values."
fi

# Build and start services
echo "🔨 Building and starting development services..."
docker compose --profile dev up -d --build

# Wait for services to be healthy
echo "⏳ Waiting for services to be healthy..."
sleep 30

# Check service health
echo "🏥 Checking service health..."
if curl -f http://:8080/health > /dev/null 2>&1; then
    echo "✅ Service is healthy!"
else
    echo "❌ Service health check failed!"
    docker compose --profile dev logs dex_stream_universal_dev
    exit 1
fi

# Check Kafka connectivity
echo "📊 Checking Kafka connectivity..."
if docker exec kafka-dev kafka-topics --bootstrap-server --list | grep -q "dex-transactions"; then
    echo "✅ Kafka topic created successfully!"
else
    echo "⚠️  Kafka topic not found. Creating..."
    docker exec kafka-dev kafka-topics --bootstrap-server --create --topic dex-transactions --partitions 1 --replication-factor 1
fi

echo "🎉 Development deployment completed successfully!"
echo ""
echo "📋 Service Information:"
echo "   Health Check: http://:8080/health"
echo "   Metrics: http://:8080/metrics"
echo "   Prometheus: http://:9090"
echo "   Grafana: http://:3000 (admin/admin)"
echo "   Kafka: [configured via environment]"
echo "   Redis: [configured via environment]"
echo ""
echo "🔍 To view logs: docker compose --profile dev logs -f dex_stream_universal_dev"
echo "🛑 To stop: docker compose --profile dev down"
