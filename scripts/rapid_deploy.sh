#!/bin/bash

# Rapid Server Deployment Script
# Deploys MEVana multi-endpoint system with Docker for maximum speed

set -e

echo "🚀 MEVana Rapid Deployment Starting..."
echo "📊 This will deploy multi-endpoint Shyft streams with Grafana monitoring"

# Configuration
SERVER_HOST="${SERVER_HOST:-your-server-ip}"
DEPLOY_DIR="${DEPLOY_DIR:-/opt/mevana}"
DOCKER_COMPOSE_FILE="${DOCKER_COMPOSE_FILE:-config/multi-stream-docker.yml}"

# Pre-deployment checks
echo "🔍 Pre-deployment validation..."

# Check if Docker Compose file exists
if [ ! -f "$DOCKER_COMPOSE_FILE" ]; then
    echo "❌ Docker Compose file not found: $DOCKER_COMPOSE_FILE"
    exit 1
fi

# Check if we have the required environment variables
if [ -z "$X_TOKEN" ]; then
    echo "⚠️ X_TOKEN not set, using default from config"
fi

echo "✅ Pre-deployment checks passed"

# Step 1: Build Docker images locally (parallel building)
echo "🏗️ Building Docker images..."
start_time=$(date +%s)

# Build main application image
docker build -t mevana:latest . &
BUILD_PID=$!

# Create optimized Dockerfile if it doesn't exist
if [ ! -f "Dockerfile.deploy" ]; then
    cat > Dockerfile.deploy << 'EOF'
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build release version with optimizations
RUN cargo build --release --workspace

FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy built binaries
COPY --from=builder /app/target/release/ /usr/local/bin/

# Create non-root user
RUN useradd -r -s /bin/false mevana

USER mevana
EXPOSE 8080 8088 9090

CMD ["hotpath_arb_engine"]
EOF
fi

# Wait for build to complete
wait $BUILD_PID
build_time=$(($(date +%s) - start_time))
echo "✅ Docker build completed in ${build_time}s"

# Step 2: Deploy to server (if SERVER_HOST is specified)
if [ "$SERVER_HOST" != "your-server-ip" ] && [ -n "$SERVER_HOST" ]; then
    echo "🌐 Deploying to server: $SERVER_HOST"
    
    # Create deployment package
    echo "📦 Creating deployment package..."
    tar -czf mevana-deploy.tar.gz \
        $DOCKER_COMPOSE_FILE \
        infrastructure/ \
        scripts/health_check.sh \
        Dockerfile.deploy
    
    # Copy to server
    echo "📤 Copying to server..."
    scp mevana-deploy.tar.gz $SERVER_HOST:~/
    
    # Deploy on server
    ssh $SERVER_HOST << 'ENDSSH'
        echo "📥 Setting up MEVana on server..."
        
        # Create deployment directory
        sudo mkdir -p /opt/mevana
        cd ~
        tar -xzf mevana-deploy.tar.gz -C /opt/mevana
        cd /opt/mevana
        
        # Stop any existing services
        docker-compose -f config/multi-stream-docker.yml down || true
        
        # Start services
        echo "🚀 Starting MEVana services..."
        docker-compose -f config/multi-stream-docker.yml up -d
        
        # Wait for services to be ready
        echo "⏳ Waiting for services to start..."
        sleep 30
        
        # Health check
        ./scripts/health_check.sh
ENDSSH
    
    echo "✅ Server deployment completed!"
    
else
    # Local deployment
    echo "🏠 Starting local deployment..."
    
    # Stop any existing services
    docker-compose -f $DOCKER_COMPOSE_FILE down || true
    
    # Start services
    echo "🚀 Starting MEVana services locally..."
    docker-compose -f $DOCKER_COMPOSE_FILE up -d
    
    echo "⏳ Waiting for services to initialize..."
    sleep 30
fi

# Step 3: Verify deployment
echo "🔍 Verifying deployment..."

# Check metrics endpoints
ENDPOINTS=(
    "http://localhost:9101/metrics"  # NY stream
    "http://localhost:9102/metrics"  # VA stream  
    "http://localhost:9103/metrics"  # EU stream
    "http://localhost:8088/metrics"  # Main arbitrage engine
    "http://localhost:9090/targets"  # Prometheus
)

FAILED=0
for endpoint in "${ENDPOINTS[@]}"; do
    if curl -s --max-time 5 "$endpoint" > /dev/null; then
        echo "✅ $endpoint - OK"
    else
        echo "❌ $endpoint - FAILED"
        FAILED=1
    fi
done

# Check Grafana
if curl -s --max-time 5 "http://localhost:3000" > /dev/null; then
    echo "✅ Grafana dashboard - OK"
else
    echo "❌ Grafana dashboard - FAILED"
    FAILED=1
fi

total_time=$(($(date +%s) - start_time))

if [ $FAILED -eq 0 ]; then
    echo "🎉 Deployment successful in ${total_time}s!"
    echo ""
    echo "📊 Access your dashboards:"
    echo "   Grafana: http://localhost:3000 (admin/admin123)"
    echo "   Prometheus: http://localhost:9090"
    echo "   Metrics: http://localhost:8088/metrics"
    echo ""
    echo "🔄 Multi-endpoint streams:"
    echo "   NY: http://localhost:9101/metrics"
    echo "   VA: http://localhost:9102/metrics"  
    echo "   EU: http://localhost:9103/metrics"
    echo ""
    echo "📈 Expected: 3x data throughput from multiple Shyft endpoints"
else
    echo "⚠️ Deployment completed with errors in ${total_time}s"
    echo "🔍 Check logs: docker-compose -f $DOCKER_COMPOSE_FILE logs"
    exit 1
fi