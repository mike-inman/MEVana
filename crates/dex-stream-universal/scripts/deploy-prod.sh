#!/bin/bash

# Production Environment Deployment Script
# This script deploys the dex-stream-universal service in production mode

set -e

echo "🚀 Deploying DEX Stream Universal V2 - Production Environment"

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker and try again."
    exit 1
fi

# Load environment variables
if [ -f "env.prod" ]; then
    echo "📋 Loading production environment configuration..."
    export $(cat env.prod | grep -v '^#' | xargs)
else
    echo "❌ Error: env.prod file not found. Please create it with production configuration."
    exit 1
fi

# Validate required environment variables
if [ -z "$PROD_KAFKA_BROKER" ] || [ -z "$PROD_X_TOKEN" ]; then
    echo "❌ Error: Required environment variables not set:"
    echo "   PROD_KAFKA_BROKER: $PROD_KAFKA_BROKER"
    echo "   PROD_X_TOKEN: $PROD_X_TOKEN"
    exit 1
fi

# Production safety checks
echo "🔒 Running production safety checks..."

# Check if running as root
if [ "$EUID" -eq 0 ]; then
    echo "⚠️  Warning: Running as root. Consider using a non-root user for production."
fi

# Check Docker daemon configuration
if docker info | grep -q "User Namespace"; then
    echo "✅ User namespace enabled (good for security)"
else
    echo "⚠️  Warning: User namespace not enabled"
fi

# Build and start production service
echo "🔨 Building and starting production service..."
docker compose --profile prod up -d --build

# Wait for service to be healthy
echo "⏳ Waiting for service to be healthy..."
sleep 30

# Check service health
echo "🏥 Checking service health..."
if curl -f http://:8082/health > /dev/null 2>&1; then
    echo "✅ Production service is healthy!"
else
    echo "❌ Production service health check failed!"
    docker compose --profile prod logs dex_stream_universal_prod
    exit 1
fi

# Production monitoring setup
echo "📊 Setting up production monitoring..."

echo "🎉 Production deployment completed successfully!"
echo ""
echo "📋 Service Information:"
echo "   Health Check: http://:8082/health"
echo "   Metrics: http://:8082/metrics"
echo "   Kafka Broker: $PROD_KAFKA_BROKER"
echo "   Topic: ${PROD_TOPIC:-dex-transactions}"
echo "   Security Level: ${PROD_SECURITY_LEVEL:-standard}"
echo ""
echo "🔍 To view logs: docker compose --profile prod logs -f dex_stream_universal_prod"
echo "🛑 To stop: docker compose --profile prod down"
echo ""
echo "⚠️  IMPORTANT: Monitor the service closely in production!"
echo "   - Check health endpoints regularly"
echo "   - Monitor Prometheus metrics"
echo "   - Review logs for errors"
echo "   - Verify Kafka message production"
