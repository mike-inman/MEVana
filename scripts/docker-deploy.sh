#!/bin/bash

# MEVana Docker Production Deployment Script
# Eliminates architecture compatibility and build issues

set -e

# Configuration
SERVER_IP="149.28.56.125"
SERVER_USER="root"
IMAGE_NAME="mevana/hotpath"
CONTAINER_NAME="mevana-hotpath"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

main() {
    log_info "🚀 Starting MEVana Docker Deployment"
    echo "================================================"
    echo "Target: $SERVER_IP"
    echo "Image: $IMAGE_NAME"
    echo "Container: $CONTAINER_NAME"
    echo "================================================"

    # Step 1: Build Docker image locally
    log_info "📦 Building Docker image locally..."
    docker build -f Dockerfile.production -t $IMAGE_NAME . || {
        log_error "Docker build failed"
        exit 1
    }
    log_success "Docker image built successfully"

    # Step 2: Save and transfer image
    log_info "💾 Saving Docker image to tarball..."
    docker save $IMAGE_NAME | gzip > mevana-hotpath.tar.gz
    log_success "Image saved to mevana-hotpath.tar.gz"

    # Step 3: Transfer to server
    log_info "📡 Transferring image to server..."
    scp mevana-hotpath.tar.gz $SERVER_USER@$SERVER_IP:/tmp/ || {
        log_error "Failed to transfer image to server"
        exit 1
    }
    log_success "Image transferred to server"

    # Step 4: Deploy on server
    log_info "🎯 Deploying on server..."
    ssh $SERVER_USER@$SERVER_IP << 'DEPLOY_SCRIPT'
        set -e

        echo "🔍 Loading Docker image..."
        docker load < /tmp/mevana-hotpath.tar.gz

        echo "🛑 Stopping existing container..."
        docker stop mevana-hotpath 2>/dev/null || echo "No existing container"
        docker rm mevana-hotpath 2>/dev/null || echo "No existing container to remove"

        echo "🚀 Starting new container..."
        docker run -d \
            --name mevana-hotpath \
            --restart unless-stopped \
            -p 8088:8088 \
            -e PAPER_TRADING=true \
            -e DATABASE_URL="postgresql://mev:mevpassword@host.docker.internal:5433/mevdb" \
            -e GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
            -e X_TOKEN="efff2d23-97ae-4c5f-8b8b-9a6265001189" \
            -e METRICS_PORT=8088 \
            -e RUST_LOG=info \
            --add-host host.docker.internal:host-gateway \
            mevana/hotpath

        echo "⏳ Waiting for service to start..."
        sleep 10

        echo "🔍 Health check..."
        for i in {1..10}; do
            if curl -s http://localhost:8088/metrics > /dev/null; then
                echo "✅ Service is healthy!"
                break
            else
                echo "⏳ Attempt $i/10 - waiting..."
                sleep 5
            fi
        done

        echo "📊 Container status:"
        docker ps | grep mevana-hotpath

        echo "📝 Recent logs:"
        docker logs --tail 20 mevana-hotpath

        echo "🎉 Docker deployment complete!"
DEPLOY_SCRIPT

    log_success "Deployment completed!"

    # Step 5: Validation
    log_info "🔍 Final validation..."

    # Test metrics endpoint
    if curl -s --connect-timeout 5 http://$SERVER_IP:8088/metrics | head -5; then
        log_success "✅ Metrics endpoint responding"
    else
        log_warning "⚠️ Metrics endpoint not responding yet"
    fi

    # Clean up local files
    log_info "🧹 Cleaning up..."
    rm -f mevana-hotpath.tar.gz

    log_success "==============================================="
    log_success "🎯 MEVana Docker Deployment Complete!"
    log_success "==============================================="
    echo ""
    echo "🔗 Access Points:"
    echo "   • Metrics: http://$SERVER_IP:8088/metrics"
    echo "   • Performance: http://$SERVER_IP:8088/performance"
    echo "   • AI Config: http://$SERVER_IP:8088/ai/config"
    echo ""
    echo "📋 Management Commands:"
    echo "   • View logs: ssh $SERVER_USER@$SERVER_IP 'docker logs -f mevana-hotpath'"
    echo "   • Restart: ssh $SERVER_USER@$SERVER_IP 'docker restart mevana-hotpath'"
    echo "   • Stop: ssh $SERVER_USER@$SERVER_IP 'docker stop mevana-hotpath'"
}

# Error handling
trap 'log_error "Deployment failed on line $LINENO"' ERR

# Run main function
main "$@"