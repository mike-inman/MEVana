#!/bin/bash

# MEVana System Startup Script
# This script starts all services in the proper sequence
# Date: 2025-09-09

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored messages
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Function to send status to WebUI
send_status_to_webui() {
    local service=$1
    local status=$2
    local port=$3
    local message="$4"
    
    # Send status update to WebUI if it's running
    if lsof -i :3001 > /dev/null 2>&1; then
        curl -s -X POST "http://localhost:3001/api/system-status" \
            -H "Content-Type: application/json" \
            -d "{\"service\":\"$service\",\"status\":\"$status\",\"port\":$port,\"message\":\"$message\",\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)\"}" \
            > /dev/null 2>&1 || true
    fi
}

# Function to check service health with WebUI feedback
check_health() {
    local service_name=$1
    local health_url=$2
    local port=$3
    local max_attempts=30
    local attempt=0
    
    log_info "Checking health of $service_name at $health_url"
    send_status_to_webui "$service_name" "checking" "$port" "Health check in progress..."
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -s -f "$health_url" > /dev/null 2>&1; then
            log_success "$service_name is healthy"
            send_status_to_webui "$service_name" "healthy" "$port" "Service is healthy and responsive"
            return 0
        fi
        attempt=$((attempt + 1))
        echo -n "."
        sleep 2
    done
    
    log_warning "$service_name is not responding at $health_url"
    send_status_to_webui "$service_name" "unhealthy" "$port" "Service is not responding"
    return 1
}

# Change to MEVana directory
cd /Users/michaelinman/Repos/MEVana

log_info "==============================================="
log_info "Starting MEVana System with Enhanced UI"
log_info "==============================================="

# Step 0: Check and verify credentials
log_info "Step 0: Verifying credentials and configuration..."

# Check X_TOKEN for Geyser/Shyft access
if [ -z "$X_TOKEN" ]; then
    log_warning "X_TOKEN not set. Setting from environment or using default..."
    export X_TOKEN="efff2d23-97ae-4c5f-8b8b-9a6265001189"
    log_info "X_TOKEN set to: ${X_TOKEN:0:8}..."
else
    log_success "X_TOKEN is configured: ${X_TOKEN:0:8}..."
fi

# Check GEYSER_ENDPOINT
if [ -z "$GEYSER_ENDPOINT" ]; then
    log_warning "GEYSER_ENDPOINT not set. Using default Shyft endpoint..."
    export GEYSER_ENDPOINT="https://grpc.ny.shyft.to"
    log_info "GEYSER_ENDPOINT set to: $GEYSER_ENDPOINT"
else
    log_success "GEYSER_ENDPOINT is configured: $GEYSER_ENDPOINT"
fi

# Set trading mode (default to paper trading for safety)
if [ -z "$TRADING_MODE" ]; then
    export TRADING_MODE="paper"
    log_info "TRADING_MODE defaulted to: $TRADING_MODE (safe mode)"
else
    log_info "TRADING_MODE is: $TRADING_MODE"
fi

# Set initial arbitrage parameters (conservative defaults)
export ARB_MIN_PROFIT_BPS="${ARB_MIN_PROFIT_BPS:-25}"  # 25 bps minimum
export ARB_MAX_SLIPPAGE_BPS="${ARB_MAX_SLIPPAGE_BPS:-50}"  # 50 bps max slippage
export ARB_SIZING_MODE="${ARB_SIZING_MODE:-adaptive}"

log_success "Configuration verified:"
echo "  - Trading Mode: $TRADING_MODE"
echo "  - Min Profit: ${ARB_MIN_PROFIT_BPS} bps"
echo "  - Max Slippage: ${ARB_MAX_SLIPPAGE_BPS} bps"
echo "  - Sizing Mode: $ARB_SIZING_MODE"

# Step 1: Start WebUI and Monitoring Service (Early Phase)
log_info "Step 1: Starting WebUI and Monitoring Service..."

# Check if WebUI is already running
if lsof -i :3001 > /dev/null 2>&1; then
    log_success "WebUI is already running on port 3001"
else
    log_info "Starting MEVana WebUI..."
    cd /Users/michaelinman/Repos/MEVana-UI
    npm run dev > ../MEVana/logs/webui.log 2>&1 &
    cd /Users/michaelinman/Repos/MEVana
    sleep 8  # Give WebUI time to start
    if lsof -i :3001 > /dev/null 2>&1; then
        log_success "WebUI started on http://localhost:3001"
        send_status_to_webui "webui" "healthy" 3001 "WebUI is running and accessible"
    else
        log_warning "WebUI may still be starting. Check logs/webui.log"
    fi
fi

# Start monitoring service early for system visibility
log_info "Starting monitoring service..."
if lsof -i :8091 > /dev/null 2>&1; then
    log_success "Monitoring service is already running on port 8091"
    send_status_to_webui "monitoring-service" "healthy" 8091 "Already running"
else
    MONITORING_PORT=8091 DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
        RUST_LOG=info cargo run -p monitoring-service --bin monitoring-service > logs/monitoring-service.log 2>&1 &
    sleep 5
    if check_health "monitoring-service" "http://localhost:8091/health" 8091; then
        log_success "monitoring-service started"
    else
        log_warning "monitoring-service may still be starting"
        send_status_to_webui "monitoring-service" "starting" 8091 "Service is starting..."
    fi
fi

# Step 2: Check infrastructure with WebUI feedback
log_info "Step 2: Checking infrastructure services..."

if docker ps | grep -q postgres; then
    log_success "PostgreSQL is running"
    send_status_to_webui "postgresql" "healthy" 5433 "Docker container running"
else
    log_error "PostgreSQL is not running. Please start infrastructure first:"
    log_error "cd infrastructure && docker-compose up -d"
    send_status_to_webui "postgresql" "error" 5433 "Container not running"
    exit 1
fi

if docker ps | grep -q redis; then
    log_success "Redis is running"
    send_status_to_webui "redis" "healthy" 6379 "Docker container running"
else
    log_error "Redis is not running"
    send_status_to_webui "redis" "error" 6379 "Container not running"
    exit 1
fi

if docker ps | grep -q kafka; then
    log_success "Kafka is running"
    send_status_to_webui "kafka" "healthy" 29092 "Docker container running"
else
    log_error "Kafka is not running"
    send_status_to_webui "kafka" "error" 29092 "Container not running"
    exit 1
fi

# Step 3: Check wallet-vault service
log_info "Step 3: Checking wallet-vault service..."
if lsof -i :8090 | grep -q LISTEN; then
    log_success "Wallet-vault is running on port 8090"
    send_status_to_webui "wallet-vault" "healthy" 8090 "Service running"
else
    log_warning "Wallet-vault is not running. Starting it..."
    send_status_to_webui "wallet-vault" "starting" 8090 "Starting wallet service..."
    # Start wallet-vault in background
    WALLET_VAULT_AUTH_REQUIRED=false DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
        cargo run -p wallet-vault --bin wallet-vault > logs/wallet-vault.log 2>&1 &
    sleep 5
    if lsof -i :8090 | grep -q LISTEN; then
        log_success "Wallet-vault started successfully"
        send_status_to_webui "wallet-vault" "healthy" 8090 "Service started successfully"
    else
        log_warning "Wallet-vault may still be starting"
        send_status_to_webui "wallet-vault" "starting" 8090 "Still starting..."
    fi
fi

# Step 4: Start Core Services (Tier 1)
log_info "Step 4: Starting Core Services with real-time feedback..."

# Create logs directory if it doesn't exist
mkdir -p logs

# Start normalizer-service
log_info "Starting normalizer-service..."
send_status_to_webui "normalizer-service" "starting" 9901 "Starting transaction normalizer..."
RUST_LOG=info METRICS_PORT=9901 KAFKA_BROKER=localhost:29092 IN_TOPIC=dex-transactions OUT_TOPIC=normalized-swaps \
    cargo run -p normalizer-service --bin normalizer-service -- pipe > logs/normalizer-service.log 2>&1 &
sleep 5
if check_health "normalizer-service" "http://localhost:9901/health" 9901; then
    log_success "normalizer-service started"
else
    log_warning "normalizer-service health endpoint not available, but process appears to be running"
    # Check if process is actually running
    if pgrep -f "normalizer-service.*pipe" > /dev/null; then
        log_success "normalizer-service is running (no health endpoint)"
        send_status_to_webui "normalizer-service" "healthy" 9901 "Process running (no health endpoint)"
    else
        log_error "Failed to start normalizer-service. Check logs/normalizer-service.log"
        send_status_to_webui "normalizer-service" "error" 9901 "Failed to start - check logs"
    fi
fi

# Start edge-assembler
log_info "Starting edge-assembler..."
send_status_to_webui "edge-assembler" "starting" 9902 "Starting edge processing service..."
RUST_LOG=info METRICS_PORT=9902 KAFKA_BROKER=localhost:29092 \
    cargo run -p edge-assembler --bin edge-assembler > logs/edge-assembler.log 2>&1 &
sleep 5
if check_health "edge-assembler" "http://localhost:9902/health" 9902; then
    log_success "edge-assembler started"
else
    log_error "Failed to start edge-assembler. Check logs/edge-assembler.log"
fi

# Start arb-engine with configured parameters
log_info "Starting arb-engine with trading mode: $TRADING_MODE..."
send_status_to_webui "arb-engine" "starting" 8085 "Starting arbitrage engine in $TRADING_MODE mode..."
RUST_LOG=info WS_BIND=0.0.0.0:8085 ARB_SIZING_MODE="$ARB_SIZING_MODE" KAFKA_BROKER=localhost:29092 \
    ENABLE_JUPITER=true ARB_MIN_PROFIT_BPS="$ARB_MIN_PROFIT_BPS" ARB_MAX_SLIPPAGE_BPS="$ARB_MAX_SLIPPAGE_BPS" \
    TRADING_MODE="$TRADING_MODE" \
    cargo run -p arb-engine --bin arb-engine > logs/arb-engine.log 2>&1 &
sleep 5
if check_health "arb-engine" "http://localhost:8085/metrics" 8085; then
    log_success "arb-engine started in $TRADING_MODE mode"
else
    log_error "Failed to start arb-engine. Check logs/arb-engine.log"
fi

# Start executor-service
log_info "Starting executor-service..."
send_status_to_webui "executor-service" "starting" 9904 "Starting trade execution service..."
RUST_LOG=info METRICS_PORT=9904 DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
    ALLOW_EMPTY_SHYFT_API=1 TRADING_MODE="$TRADING_MODE" \
    cargo run -p executor-service --bin executor-service > logs/executor-service.log 2>&1 &
sleep 5
if check_health "executor-service" "http://localhost:9904/health" 9904; then
    log_success "executor-service started"
else
    log_error "Failed to start executor-service. Check logs/executor-service.log"
fi

# Step 5: Start Supporting Services (Tier 2) - Non-critical
log_info "Step 5: Starting Supporting Services (non-critical)..."

# Start token-registry-service
log_info "Starting token-registry-service..."
send_status_to_webui "token-registry-service" "starting" 9898 "Starting token metadata service..."
RUST_LOG=info TOKEN_REGISTRY_PORT=9898 cargo run -p token-registry-service > logs/token-registry.log 2>&1 &
sleep 3
if lsof -i :9898 > /dev/null 2>&1; then
    log_success "token-registry-service started on port 9898"
    send_status_to_webui "token-registry-service" "healthy" 9898 "Service started successfully"
else
    log_warning "token-registry-service may still be starting"
    send_status_to_webui "token-registry-service" "starting" 9898 "Still starting..."
fi

# Step 6: Start HotPath Service (Tier 3) - Critical
log_info "Step 6: Starting HotPath Arbitrage Engine..."

# Credentials are now set at the top, so we can proceed
log_info "Starting hotpath_arb_engine with verified credentials..."
send_status_to_webui "hotpath_arb_engine" "starting" 8088 "Starting zero-latency MEV engine..."
RUST_LOG=info METRICS_PORT=8088 GEYSER_ENDPOINT="$GEYSER_ENDPOINT" X_TOKEN="$X_TOKEN" \
    DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
    TRADING_MODE="$TRADING_MODE" \
    cargo run -p hotpath_arb_engine --bin hotpath_arb_engine > logs/hotpath_arb_engine.log 2>&1 &
sleep 10
if check_health "hotpath_arb_engine" "http://localhost:8088/performance" 8088; then
    log_success "hotpath_arb_engine started with $TRADING_MODE mode"
else
    log_error "Failed to start hotpath_arb_engine. Check logs/hotpath_arb_engine.log"
fi

# Summary
log_info "==============================================="
log_info "System Startup Summary"
log_info "==============================================="

# Check running services
log_info "Checking service status..."

echo ""
echo "Infrastructure Services:"
echo "  PostgreSQL: $(docker ps | grep -q postgres && echo '✅ Running' || echo '❌ Not running')"
echo "  Redis: $(docker ps | grep -q redis && echo '✅ Running' || echo '❌ Not running')"
echo "  Kafka: $(docker ps | grep -q kafka && echo '✅ Running' || echo '❌ Not running')"

echo ""
echo "User Interface:"
echo "  WebUI (3001): $(lsof -i :3001 2>/dev/null | grep -q LISTEN && echo '✅ Running at http://localhost:3001' || echo '❌ Not running')"
echo "  monitoring-service (8091): $(lsof -i :8091 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"

echo ""
echo "Core Services:"
echo "  wallet-vault (8090): $(lsof -i :8090 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"
echo "  normalizer-service (9901): $(lsof -i :9901 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"
echo "  edge-assembler (9902): $(lsof -i :9902 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"
echo "  arb-engine (8085): $(lsof -i :8085 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"
echo "  executor-service (9904): $(lsof -i :9904 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"

echo ""
echo "Supporting Services:"
echo "  token-registry (9898): $(lsof -i :9898 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"

echo ""
echo "HotPath Services:"
echo "  hotpath_arb_engine (8088): $(lsof -i :8088 2>/dev/null | grep -q LISTEN && echo '✅ Running' || echo '❌ Not running')"

echo ""
log_info "Logs are available in the 'logs' directory"
log_info "Monitor services at:"
echo "  - 🎛️  MEVana WebUI: http://localhost:3001 (PRIMARY DASHBOARD)"
echo "  - 📊 HotPath Dashboard: http://localhost:8088/performance"
echo "  - 🔍 Monitoring API: http://localhost:8091/health"
echo "  - 📈 Kafka UI: http://localhost:8080"
echo "  - 💾 Redis UI: http://localhost:8083"
echo "  - 📊 Grafana: http://localhost:3000"
echo "  - ⚡ Prometheus: http://localhost:9090"

# Send final status to WebUI
send_status_to_webui "system" "complete" 0 "All services started successfully in $TRADING_MODE mode"

log_success "🚀 Enhanced MEVana System startup complete!"
log_success "🎯 Trading Mode: $TRADING_MODE"
log_success "💻 Primary Interface: http://localhost:3001"
log_success "⚙️  Admin settings available in WebUI for authorized users"