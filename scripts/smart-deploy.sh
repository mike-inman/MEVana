#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 MEVana Smart Deployment with Port Management${NC}"
echo "=============================================="

# Kill any conflicting processes on target ports
echo -e "${YELLOW}🔍 Checking for port conflicts...${NC}"

PORTS_TO_CHECK=(8085 8086 8087 8088 29092)

for port in "${PORTS_TO_CHECK[@]}"; do
    if lsof -i:$port >/dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  Port $port is in use${NC}"
        
        # Get process info
        PROCESS_INFO=$(lsof -i:$port | tail -1)
        echo "   Process: $PROCESS_INFO"
        
        # Ask if we should kill it (for local deployment)
        if [[ "${1:-}" == "--force-kill" ]]; then
            echo -e "${RED}💀 Killing process on port $port${NC}"
            lsof -ti:$port | xargs kill -9 2>/dev/null || true
        else
            echo -e "${YELLOW}   Use --force-kill to automatically kill conflicting processes${NC}"
        fi
    else
        echo -e "${GREEN}✅ Port $port is available${NC}"
    fi
done

echo ""

# Build with smart port allocation
echo -e "${BLUE}🔨 Building with smart port management...${NC}"
cargo build --release

# Start services with intelligent port allocation
echo -e "${BLUE}⚡ Starting services with conflict resolution...${NC}"

# Create logs directory
mkdir -p logs

# Function to start service with smart ports
start_service() {
    local service_name="$1"
    local service_type="$2"
    local binary_path="$3"
    local preferred_port="$4"
    local extra_env="$5"
    
    echo -e "${PURPLE}Starting $service_name...${NC}"
    
    # Use our Rust port manager to allocate port
    PORT_RESULT=$(cargo run -p shared-core --bin port-allocator -- \
        --service "$service_name" \
        --type "$service_type" \
        --preferred "$preferred_port" 2>/dev/null || echo "$preferred_port")
    
    ALLOCATED_PORT=$(echo "$PORT_RESULT" | tail -1)
    
    # Start the service with allocated port
    nohup bash -c "
        export WS_BIND=0.0.0.0:$ALLOCATED_PORT
        export ARB_HTTP_PORT=$ALLOCATED_PORT
        export METRICS_PORT=$ALLOCATED_PORT
        $extra_env
        $binary_path
    " > "logs/$service_name.log" 2>&1 &
    
    SERVICE_PID=$!
    echo -e "${GREEN}✅ $service_name started on port $ALLOCATED_PORT (PID: $SERVICE_PID)${NC}"
    
    # Save PID for cleanup
    echo "$SERVICE_PID" > "logs/$service_name.pid"
}

# Start infrastructure first
echo -e "${BLUE}🐋 Starting infrastructure...${NC}"
cd infrastructure
docker compose up -d
sleep 10
cd ..

# Start core services with smart port allocation
start_service "arbitrage-engine" "http" "./target/release/arb-engine" "8085" "KAFKA_BROKER=localhost:29092 ENABLE_JUPITER=true"
sleep 5

start_service "hotpath-engine" "metrics" "./target/release/hotpath_arb_engine" "8088" "GEYSER_ENDPOINT=wss://api.mainnet-beta.solana.com RUST_LOG=info"
sleep 5

# Health checks with retry logic
echo -e "${BLUE}🔍 Running health checks...${NC}"

check_service_health() {
    local service_name="$1"
    local port="$2"
    local endpoint="$3"
    
    echo -e "${YELLOW}Checking $service_name health...${NC}"
    
    for i in {1..10}; do
        if curl -s --connect-timeout 2 "http://localhost:$port$endpoint" >/dev/null 2>&1; then
            echo -e "${GREEN}✅ $service_name is healthy (attempt $i)${NC}"
            return 0
        else
            echo -e "${YELLOW}⏳ $service_name not ready (attempt $i/10)...${NC}"
            sleep 3
        fi
    done
    
    echo -e "${RED}❌ $service_name health check failed${NC}"
    echo "Log tail:"
    tail -10 "logs/$service_name.log" 2>/dev/null || echo "No log file found"
    return 1
}

# Check each service
SERVICES_HEALTHY=true

if check_service_health "arbitrage-engine" "8085" "/ai/config"; then
    echo -e "${GREEN}🎯 Arbitrage engine is operational${NC}"
else
    SERVICES_HEALTHY=false
fi

if check_service_health "hotpath-engine" "8088" "/metrics"; then
    echo -e "${GREEN}🚄 Hotpath engine is operational${NC}"
else
    SERVICES_HEALTHY=false
fi

# Generate deployment report
echo ""
echo -e "${BLUE}📊 Deployment Report${NC}"
echo "==================="

if [ "$SERVICES_HEALTHY" = true ]; then
    echo -e "${GREEN}🎉 All services deployed successfully!${NC}"
    
    echo ""
    echo "🌐 Service URLs:"
    echo "  • Arbitrage Engine: http://localhost:$(lsof -ti:8085 >/dev/null 2>&1 && echo "8085" || echo "$(cat logs/arbitrage-engine.port 2>/dev/null || echo "UNKNOWN")")"
    echo "  • HotPath Engine: http://localhost:$(lsof -ti:8088 >/dev/null 2>&1 && echo "8088" || echo "$(cat logs/hotpath-engine.port 2>/dev/null || echo "UNKNOWN")")"
    echo "  • Kafka UI: http://localhost:8080"
    echo ""
    echo -e "${GREEN}🏂 Ready for mountain deployment! No port conflicts detected.${NC}"
    
else
    echo -e "${RED}❌ Deployment completed with issues${NC}"
    echo "Check logs/ directory for details"
fi

# Cleanup function
cleanup_deployment() {
    echo -e "${YELLOW}🧹 Cleaning up deployment...${NC}"
    
    # Kill services
    for pidfile in logs/*.pid; do
        if [ -f "$pidfile" ]; then
            PID=$(cat "$pidfile")
            kill "$PID" 2>/dev/null && echo "Stopped $(basename "$pidfile" .pid)" || true
            rm "$pidfile"
        fi
    done
    
    # Stop infrastructure
    cd infrastructure
    docker compose down
    cd ..
}

# Register cleanup handler
trap cleanup_deployment EXIT