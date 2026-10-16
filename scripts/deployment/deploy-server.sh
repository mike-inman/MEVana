#!/bin/bash

# =============================================================================
# MEVANA SERVER DEPLOYMENT SCRIPT - Run this ON the server
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[$(date '+%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1"
}

error() {
    echo -e "${RED}[$(date '+%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1"
}

log "🚀 MEVana Server Deployment Starting..."

# =============================================================================
# PHASE 1: System Dependencies
# =============================================================================

log "📦 Installing system dependencies..."

# Update system
apt-get update -y

# Install essential packages
apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    bc \
    docker.io \
    docker-compose

# Start Docker service
systemctl start docker
systemctl enable docker

log "✅ System dependencies installed"

# =============================================================================
# PHASE 2: Rust Installation
# =============================================================================

log "🦀 Installing Rust..."

# Install Rust (if not already installed)
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    log "✅ Rust installed"
else
    log "✅ Rust already installed"
fi

# =============================================================================
# PHASE 3: Clone/Update MEVana Repository  
# =============================================================================

log "📂 Setting up MEVana repository..."

cd /opt

# Clone or update repository
if [[ -d "MEVana" ]]; then
    log "   • Updating existing repository..."
    cd MEVana
    git pull origin main
else
    log "   • Cloning repository..."
    git clone https://github.com/mike-inman/MEVana.git
    cd MEVana
fi

log "✅ Repository ready"

# =============================================================================
# PHASE 4: Build MEVana Services
# =============================================================================

log "🔨 Building MEVana services..."

# Build all services in release mode
source ~/.cargo/env 2>/dev/null || true
cargo build --release --workspace

if [[ $? -ne 0 ]]; then
    error "Failed to build MEVana services"
    exit 1
fi

log "✅ All services built successfully"

# =============================================================================
# PHASE 5: Docker Network and Infrastructure
# =============================================================================

log "🐋 Setting up Docker infrastructure..."

# Create Docker network
if ! docker network ls | grep -q "mevana_net"; then
    log "   • Creating Docker network..."
    docker network create mevana_net
else
    log "   • Docker network already exists"
fi

# Create directories
mkdir -p logs infrastructure

cd infrastructure

# Start infrastructure services
log "   • Starting Zookeeper and Redis..."
docker-compose up -d zookeeper redis

sleep 15

log "   • Starting Kafka and PostgreSQL..."
docker-compose up -d kafka postgres

sleep 25

log "   • Starting UI and monitoring services..."
docker-compose up -d kafka-ui redis-ui prometheus grafana adminer

sleep 10

log "✅ Infrastructure services started"

cd /opt/MEVana

# =============================================================================
# PHASE 6: Test Connectivity
# =============================================================================

log "🔍 Testing system connectivity..."

# Test Solana RPC
log "   • Testing Solana connectivity..."
START_TIME=$(date +%s%N)
curl -s -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
    https://api.mainnet-beta.solana.com > /dev/null
END_TIME=$(date +%s%N)
LATENCY_NS=$((END_TIME - START_TIME))
LATENCY_MS=$(echo "scale=3; $LATENCY_NS / 1000000" | bc -l)

log "   • Solana RPC latency: ${LATENCY_MS}ms"

# Test Docker services
log "   • Testing infrastructure services..."
if docker exec kafka kafka-topics --bootstrap-server localhost:9092 --list >/dev/null 2>&1; then
    log "   ✅ Kafka is healthy"
else
    warn "   Kafka health check failed - may still be starting"
fi

# =============================================================================
# PHASE 7: Start MEVana Trading Services
# =============================================================================

log "⚡ Starting MEVana trading services..."

# Create environment configuration
cat > venue_config.env << 'EOF'
# Multi-venue configuration
ORCA_PROGRAM_ID=whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc
RAYDIUM_PROGRAM_ID=675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
METEORA_PROGRAM_ID=Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB
PHOENIX_PROGRAM_ID=PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY

# Venue filtering (all enabled)
ENABLE_ORCA=true
ENABLE_RAYDIUM=true
ENABLE_METEORA=true
ENABLE_PHOENIX=true

# Jupiter integration
ENABLE_JUPITER=true
JUPITER_API_URL=https://quote-api.jup.ag/v6
JUPITER_MAX_HOPS=3

# Performance settings
ARB_MIN_PROFIT_BPS=5
ARB_MAX_SLIPPAGE_BPS=100
HOTPATH_LOG=true
EOF

# Kill any existing processes
pkill -f "whirlpool-live-stream" || true
pkill -f "arb-engine" || true
sleep 3

# Start discovery stream
log "   • Starting Orca discovery stream..."
nohup bash -c "
    source venue_config.env
    ENDPOINT=https://grpc.ny.shyft.to \
    X_TOKEN=<SHYFT_X_TOKEN> \
    KAFKA_BROKER=localhost:29092 \
    PROGRAM_IDS=whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc \
    ./target/release/whirlpool-live-stream
" > logs/orca-discovery.log 2>&1 &

ORCA_PID=$!
log "   • Orca discovery started (PID: $ORCA_PID)"

# Wait for stream to establish
sleep 8

# Start arbitrage engine with Jupiter integration
log "   • Starting arbitrage engine with Jupiter integration..."
nohup bash -c "
    source venue_config.env
    KAFKA_BROKER=localhost:29092 \
    HOTPATH_LOG=true \
    ARB_MIN_PROFIT_BPS=5 \
    ARB_MAX_SLIPPAGE_BPS=100 \
    WS_BIND=0.0.0.0:8085 \
    ENABLE_JUPITER=true \
    JUPITER_API_URL=https://quote-api.jup.ag/v6 \
    JUPITER_MAX_HOPS=3 \
    JUPITER_PAPER_TRADING=true \
    ./target/release/arb-engine
" > logs/arb-engine.log 2>&1 &

ARB_PID=$!
log "   • Arbitrage engine started (PID: $ARB_PID)"

# Create restart script
cat > restart_engine.sh << 'EOF'
#!/bin/bash
pkill -f "arb-engine" || true
sleep 2

source venue_config.env
nohup bash -c "
    KAFKA_BROKER=localhost:29092 \
    HOTPATH_LOG=true \
    ARB_MIN_PROFIT_BPS=5 \
    ARB_MAX_SLIPPAGE_BPS=100 \
    WS_BIND=0.0.0.0:8085 \
    ENABLE_JUPITER=true \
    JUPITER_API_URL=https://quote-api.jup.ag/v6 \
    JUPITER_MAX_HOPS=3 \
    JUPITER_PAPER_TRADING=true \
    ./target/release/arb-engine
" > logs/arb-engine.log 2>&1 &

echo "Arbitrage engine restarted (PID: $!)"
EOF

chmod +x restart_engine.sh

# =============================================================================
# PHASE 8: Final Status Report
# =============================================================================

sleep 5

log "📊 MEVANA DEPLOYMENT COMPLETE!"
echo
log "🏢 Infrastructure Services:"
log "   • Kafka UI:      http://$(hostname -I | awk '{print $1}'):8080"
log "   • Redis UI:      http://$(hostname -I | awk '{print $1}'):8083 (admin/<ADMIN_PASSWORD>)"
log "   • Grafana:       http://$(hostname -I | awk '{print $1}'):3000 (admin/<ADMIN_PASSWORD>)"
log "   • Prometheus:    http://$(hostname -I | awk '{print $1}'):9090"

log "⚡ Trading Services:"
log "   • Arbitrage API: http://$(hostname -I | awk '{print $1}'):8085/ai/config"
log "   • Metrics:       http://$(hostname -I | awk '{print $1}'):8085/metrics"

log "📡 Discovery Services:"
log "   • Orca Stream:   PID $ORCA_PID (logs/orca-discovery.log)"

log "🤖 AI Trading Engine:"
log "   • Main Engine:   PID $ARB_PID (logs/arb-engine.log)"

echo
log "📈 PAPER TRADING MODE ACTIVE!"
log "   • All venues: Orca, Raydium, Meteora, Phoenix"
log "   • Jupiter multi-hop routing: ENABLED"
log "   • Minimum profit threshold: 5 BPS"
log "   • Mode: PAPER TRADING (no wallet required)"

echo
log "🔧 Monitoring Commands:"
log "   • Live activity: tail -f logs/arb-engine.log"
log "   • Engine status: pgrep -f arb-engine"
log "   • Restart:       ./restart_engine.sh"
log "   • Check config:  curl http://localhost:8085/ai/config"

echo
log "✅ MEVana is now running in paper trading mode!"
log "🚨 Ready for weekend trading development!"

# Test the API
sleep 3
if curl -s http://localhost:8085/ai/config >/dev/null 2>&1; then
    log "🎯 API Test: SUCCESS - Arbitrage engine responding"
else
    warn "🎯 API Test: Engine still starting up (check logs/arb-engine.log)"
fi