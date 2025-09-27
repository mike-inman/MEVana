#!/bin/bash
# Deprecated: Use GHCR-based deploy (Actions) and scripts/deployment/deploy_vultr_compose.sh
# Last updated: 2025-09-15

# MEVana Simple Production Deployment
# Transfers code and builds on server with optimized configuration

set -e

SERVER_IP="149.28.56.125"
SERVER_USER="root"
DEPLOYMENT_PATH="/opt/MEVana"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

main() {
    log_info "🚀 MEVana Simple Deployment to $SERVER_IP"

    # 1. Create deployment archive
    log_info "📦 Creating deployment archive..."
    tar --exclude='.git' --exclude='target' --exclude='*.tar.gz' --exclude='logs' -czf mevana-deploy.tar.gz .

    # 2. Transfer to server
    log_info "📡 Transferring code to server..."
    scp mevana-deploy.tar.gz $SERVER_USER@$SERVER_IP:/tmp/

    # 3. Deploy on server
    log_info "🎯 Deploying on server..."
    ssh $SERVER_USER@$SERVER_IP << 'DEPLOY_SCRIPT'
        set -e

        echo "🔍 Setting up deployment..."
        mkdir -p /opt/MEVana
        cd /opt/MEVana

        # Stop existing processes
        echo "🛑 Stopping existing processes..."
        pkill -f "hotpath_arb_engine" || true
        pkill -f "arb-engine" || true
        pkill -f "whirlpool-live-stream" || true
        sleep 3

        # Extract new code
        echo "📦 Extracting deployment..."
        tar -xzf /tmp/mevana-deploy.tar.gz

        # Set up production Docker infrastructure
        echo "🐳 Configuring Docker infrastructure for production..."
        cd infrastructure
        cp prometheus.prod.yml prometheus.yml  # Use production Prometheus config

        # Set up Prometheus alert rules
        echo "🚨 Configuring Prometheus alert rules..."
        mkdir -p prometheus/rules
        if [ -f "prometheus/rules/geyser_alerts.yml" ]; then
            echo "✅ Geyser alert rules found and ready"
        else
            echo "⚠️ Alert rules not found - monitoring without alerts"
        fi

        # Restart Docker containers with robust fallback approach
        if command -v docker >/dev/null 2>&1; then
            echo "📦 Using Docker Compose..."
            docker compose down 2>/dev/null || true
            docker compose up -d
        elif command -v docker >/dev/null 2>&1; then
            echo "📦 Using native Docker commands (fallback)..."
            docker restart $(docker ps -q --filter name=prometheus) 2>/dev/null || echo "⚠️ Prometheus not found"
            docker restart $(docker ps -q --filter name=grafana) 2>/dev/null || echo "⚠️ Grafana not found"
        else
            echo "⚠️ Docker not available, skipping container restart"
        fi

        # Wait for containers to be ready
        echo "⏳ Waiting for Docker services..."
        sleep 15

        cd ..

        # Build with optimized flags (allows warnings)
        echo "🔨 Building with optimized configuration..."
        export RUSTFLAGS="--cap-lints warn -C target-cpu=native"
        export CARGO_BUILD_JOBS=$(nproc)

        # Clean build for reliability
        cargo clean
        cargo build --release -p hotpath_arb_engine --bin hotpath_arb_engine || {
            echo "❌ Build failed, checking logs..."
            exit 1
        }

        echo "✅ Build completed successfully"

        # Create logs directory
        mkdir -p logs
        chmod 755 logs

        # Start the service
        echo "🚀 Starting hotpath_arb_engine..."
        nohup bash -c "
            export PAPER_TRADING=true
            export DATABASE_URL='postgresql://mev:mevpassword@localhost:5433/mevdb'
            export GEYSER_ENDPOINT='https://grpc.ny.shyft.to'
            export X_TOKEN='efff2d23-97ae-4c5f-8b8b-9a6265001189'
            export METRICS_PORT=8088
            export RUST_LOG=info
            ./target/release/hotpath_arb_engine
        " > logs/hotpath_production.log 2>&1 &

        HOTPATH_PID=$!
        echo "Started hotpath_arb_engine (PID: $HOTPATH_PID)"

        # Wait for startup
        echo "⏳ Waiting for service startup..."
        sleep 15

        # Health check
        echo "🔍 Health check..."
        for i in {1..10}; do
            if curl -s --connect-timeout 5 http://localhost:8088/metrics > /dev/null 2>&1; then
                echo "✅ Service is healthy! (attempt $i)"
                break
            else
                echo "⏳ Waiting for service... (attempt $i/10)"
                sleep 5
            fi
        done

        # Process check
        if pgrep -f "hotpath_arb_engine" > /dev/null; then
            echo "✅ Process is running"
        else
            echo "❌ Process not found"
            exit 1
        fi

        echo "🎉 Deployment completed successfully!"
        echo ""
        echo "📊 Service Endpoints:"
        echo "   • Metrics: http://149.28.56.125:8088/metrics"
        echo "   • Performance: http://149.28.56.125:8088/performance"
        echo "   • AI Config: http://149.28.56.125:8088/ai/config"
        echo "   • Grafana: http://149.28.56.125:3000 (admin/admin123)"
        echo "   • Prometheus: http://149.28.56.125:9090"
        echo ""
        echo "📋 Management:"
        echo "   • Logs: tail -f /opt/MEVana/logs/hotpath_production.log"
        echo "   • Status: pgrep -f hotpath_arb_engine"
        echo "   • Kill: pkill -f hotpath_arb_engine"

DEPLOY_SCRIPT

    # 4. Final validation
    log_info "🔍 Final validation..."
    sleep 5

    if curl -s --connect-timeout 10 http://$SERVER_IP:8088/metrics | head -3; then
        log_success "✅ Deployment successful!"
        log_success "🎯 MEVana HotPath Engine is live at http://$SERVER_IP:8088/metrics"
    else
        log_error "⚠️ Service may not be fully ready yet. Check logs on server."
    fi

    # Clean up
    rm -f mevana-deploy.tar.gz

    log_success "🎉 Simple deployment complete!"
}

# Run with error handling
trap 'log_error "Deployment failed on line $LINENO"' ERR
main "$@"
