#!/bin/bash
# Deprecated: superseded by compose-based deploy; see scripts/deployment
# Last updated: 2025-09-15

# =============================================================================
# MEVana Multi-Stream Ultra-Low Latency Deployment
# =============================================================================
# Deploys 3 separate Geyser streams on dedicated cores for maximum reliability
# and ultra-low latency arbitrage detection.
#
# SPEED IS LIFE - This script ensures clean hotpath with no port conflicts
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LOG_DIR="$PROJECT_ROOT/logs"
PID_DIR="$PROJECT_ROOT/pids"

# Create directories
mkdir -p "$LOG_DIR" "$PID_DIR"

# Stream Configuration
STREAM1_PORT="8088"
STREAM1_ENDPOINT="https://grpc.ny.shyft.to"
STREAM1_REGION="NY"

STREAM2_PORT="8089"
STREAM2_ENDPOINT="https://grpc.us1.shyft.to"
STREAM2_REGION="US1"

STREAM3_PORT="8090"
STREAM3_ENDPOINT="https://grpc.eu1.shyft.to"
STREAM3_REGION="EU1"

# Shyft Token (from your .env)
SHYFT_TOKEN="<SHYFT_X_TOKEN>"

# Database URL
DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb"

echo -e "${BLUE}🚀 MEVana Multi-Stream Ultra-Low Latency Deployment${NC}"
echo -e "${BLUE}================================================================${NC}"
echo -e "${CYAN}⚡ SPEED IS LIFE - Clean Hotpath Architecture${NC}"
echo -e "${CYAN}🎯 3 Dedicated Streams on Separate Cores/Ports${NC}"
echo -e "${CYAN}💰 Real Geyser Streaming with $200/month Shyft Access${NC}"
echo ""

# Function to print banner
print_banner() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║               ⚡ ULTRA-LOW LATENCY MEVANA ⚡                ║"
    echo "║                                                              ║"
    echo "║  🌊 Stream 1: NY Geyser (Port 8088) - Core 0              ║"
    echo "║  🌊 Stream 2: US1 Geyser (Port 8089) - Core 1              ║"
    echo "║  🌊 Stream 3: EU1 Geyser (Port 8090) - Core 2              ║"
    echo "║                                                              ║"
    echo "║  🎯 Clean Hotpath - No Port Conflicts                       ║"
    echo "║  💰 Real Trading - $200/month Shyft Investment             ║"
    echo "║  ⚡ Sub-millisecond Latency - Speed is Life!                ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Function to check prerequisites
check_prerequisites() {
    echo -e "${BLUE}🔍 Checking prerequisites...${NC}"
    
    # Check if we're in the right directory
    if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
        echo -e "${RED}❌ Not in MEVana project root${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ In MEVana project root${NC}"
    
    # Check Docker infrastructure
    if ! docker ps | grep -q "kafka\|redis\|postgres"; then
        echo -e "${RED}❌ Docker infrastructure not running${NC}"
        echo -e "${YELLOW}💡 Start with: cd infrastructure && docker compose up -d${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Docker infrastructure running${NC}"
    
    # Check Rust toolchain
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}❌ Rust toolchain not available${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Rust toolchain available${NC}"
    
    # Check if hotpath_arb_engine exists
    if [ ! -d "$PROJECT_ROOT/crates/hotpath_arb_engine" ]; then
        echo -e "${RED}❌ hotpath_arb_engine crate not found${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ hotpath_arb_engine crate found${NC}"
}

# Function to kill existing processes
cleanup_existing() {
    echo -e "${BLUE}🧹 Cleaning up existing processes...${NC}"
    
    # Kill any existing hotpath_arb_engine processes
    if pgrep -f "hotpath_arb_engine" > /dev/null; then
        echo -e "${YELLOW}⚠️  Killing existing hotpath_arb_engine processes...${NC}"
        pkill -f "hotpath_arb_engine" || true
        sleep 2
    fi
    
    # Clean up PID files
    rm -f "$PID_DIR"/stream*.pid
    
    # Verify ports are free
    for port in 8088 8089 8090; do
        if lsof -i :$port > /dev/null 2>&1; then
            echo -e "${RED}❌ Port $port still in use${NC}"
            lsof -i :$port
            exit 1
        fi
    done
    
    echo -e "${GREEN}✅ Cleanup completed - all ports free${NC}"
}

# Function to start a single stream
start_stream() {
    local stream_name="$1"
    local port="$2"
    local endpoint="$3"
    local region="$4"
    local core_id="$5"
    
    echo -e "${BLUE}🌊 Starting $stream_name on port $port (Core $core_id)...${NC}"
    
    # Set CPU affinity for dedicated core
    local cpu_affinity=""
    if command -v taskset &> /dev/null; then
        cpu_affinity="taskset -c $core_id"
    fi
    
    # Environment variables for this stream
    local env_vars=(
        "MULTI_ENDPOINT_MODE=true"
        "PAPER_TRADING=true"
        "ORDERBOOK_HISTORY_ENABLED=true"
        "DATABASE_URL=$DATABASE_URL"
        "X_TOKEN=$SHYFT_TOKEN"
        "ENDPOINT=$endpoint"
        "METRICS_PORT=$port"
        "RUST_LOG=info"
        "STREAM_ID=$stream_name"
        "STREAM_REGION=$region"
    )
    
    # Start the stream in background
    local log_file="$LOG_DIR/${stream_name}.log"
    local pid_file="$PID_DIR/${stream_name}.pid"
    
    # Build command
    local cmd="cd $PROJECT_ROOT && ${env_vars[*]} $cpu_affinity cargo run -p hotpath_arb_engine --bin hotpath_arb_engine"
    
    echo -e "${CYAN}📝 Command: $cmd${NC}"
    echo -e "${CYAN}📁 Log: $log_file${NC}"
    echo -e "${CYAN}🆔 PID: $pid_file${NC}"
    
    # Start process
    nohup bash -c "$cmd" > "$log_file" 2>&1 &
    local pid=$!
    echo $pid > "$pid_file"
    
    # Wait for startup
    echo -e "${YELLOW}⏳ Waiting for $stream_name to start...${NC}"
    sleep 5
    
    # Verify process is running
    if kill -0 $pid 2>/dev/null; then
        echo -e "${GREEN}✅ $stream_name started successfully (PID: $pid)${NC}"
        
        # Test metrics endpoint
        if curl -s "http://localhost:$port/metrics" > /dev/null 2>&1; then
            echo -e "${GREEN}✅ $stream_name metrics endpoint responding${NC}"
        else
            echo -e "${YELLOW}⚠️  $stream_name metrics endpoint not yet ready${NC}"
        fi
    else
        echo -e "${RED}❌ $stream_name failed to start${NC}"
        return 1
    fi
}

# Function to start all streams
start_all_streams() {
    echo -e "${BLUE}🚀 Starting all 3 Geyser streams...${NC}"
    
    start_stream "stream1" "$STREAM1_PORT" "$STREAM1_ENDPOINT" "$STREAM1_REGION" "0"
    sleep 2
    start_stream "stream2" "$STREAM2_PORT" "$STREAM2_ENDPOINT" "$STREAM2_REGION" "1"
    sleep 2
    start_stream "stream3" "$STREAM3_PORT" "$STREAM3_ENDPOINT" "$STREAM3_REGION" "2"
}

# Function to verify all streams
verify_streams() {
    echo -e "${BLUE}🔍 Verifying all streams...${NC}"
    
    local all_healthy=true
    
    # Check stream1
    local pid_file="$PID_DIR/stream1.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}✅ stream1: Running (PID: $pid, Port: $STREAM1_PORT)${NC}"
            if curl -s "http://localhost:$STREAM1_PORT/metrics" > /dev/null 2>&1; then
                echo -e "${GREEN}   📊 Metrics endpoint: OK${NC}"
            else
                echo -e "${YELLOW}   📊 Metrics endpoint: Not ready${NC}"
                all_healthy=false
            fi
        else
            echo -e "${RED}❌ stream1: Not running${NC}"
            all_healthy=false
        fi
    else
        echo -e "${RED}❌ stream1: No PID file${NC}"
        all_healthy=false
    fi
    
    # Check stream2
    local pid_file="$PID_DIR/stream2.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}✅ stream2: Running (PID: $pid, Port: $STREAM2_PORT)${NC}"
            if curl -s "http://localhost:$STREAM2_PORT/metrics" > /dev/null 2>&1; then
                echo -e "${GREEN}   📊 Metrics endpoint: OK${NC}"
            else
                echo -e "${YELLOW}   📊 Metrics endpoint: Not ready${NC}"
                all_healthy=false
            fi
        else
            echo -e "${RED}❌ stream2: Not running${NC}"
            all_healthy=false
        fi
    else
        echo -e "${RED}❌ stream2: No PID file${NC}"
        all_healthy=false
    fi
    
    # Check stream3
    local pid_file="$PID_DIR/stream3.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}✅ stream3: Running (PID: $pid, Port: $STREAM3_PORT)${NC}"
            if curl -s "http://localhost:$STREAM3_PORT/metrics" > /dev/null 2>&1; then
                echo -e "${GREEN}   📊 Metrics endpoint: OK${NC}"
            else
                echo -e "${YELLOW}   📊 Metrics endpoint: Not ready${NC}"
                all_healthy=false
            fi
        else
            echo -e "${RED}❌ stream3: Not running${NC}"
            all_healthy=false
        fi
    else
        echo -e "${RED}❌ stream3: No PID file${NC}"
        all_healthy=false
    fi
    
    if [ "$all_healthy" = true ]; then
        echo -e "${GREEN}🎉 All streams healthy!${NC}"
        return 0
    else
        echo -e "${RED}❌ Some streams unhealthy${NC}"
        return 1
    fi
}

# Function to show status
show_status() {
    echo -e "${BLUE}📊 MEVana Multi-Stream Status${NC}"
    echo -e "${BLUE}================================================================${NC}"
    
    # Stream status
    echo -e "${PURPLE}🌊 Stream Status:${NC}"
    
    # Stream1
    local pid_file="$PID_DIR/stream1.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}  ✅ stream1: Port $STREAM1_PORT, PID $pid, Region $STREAM1_REGION${NC}"
        else
            echo -e "${RED}  ❌ stream1: Dead (PID $pid)${NC}"
        fi
    else
        echo -e "${RED}  ❌ stream1: No PID file${NC}"
    fi
    
    # Stream2
    local pid_file="$PID_DIR/stream2.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}  ✅ stream2: Port $STREAM2_PORT, PID $pid, Region $STREAM2_REGION${NC}"
        else
            echo -e "${RED}  ❌ stream2: Dead (PID $pid)${NC}"
        fi
    else
        echo -e "${RED}  ❌ stream2: No PID file${NC}"
    fi
    
    # Stream3
    local pid_file="$PID_DIR/stream3.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}  ✅ stream3: Port $STREAM3_PORT, PID $pid, Region $STREAM3_REGION${NC}"
        else
            echo -e "${RED}  ❌ stream3: Dead (PID $pid)${NC}"
        fi
    else
        echo -e "${RED}  ❌ stream3: No PID file${NC}"
    fi
    
    # Metrics endpoints
    echo -e "${PURPLE}📊 Metrics Endpoints:${NC}"
    if curl -s "http://localhost:$STREAM1_PORT/metrics" > /dev/null 2>&1; then
        echo -e "${GREEN}  ✅ Port $STREAM1_PORT: Responding${NC}"
    else
        echo -e "${RED}  ❌ Port $STREAM1_PORT: Not responding${NC}"
    fi
    
    if curl -s "http://localhost:$STREAM2_PORT/metrics" > /dev/null 2>&1; then
        echo -e "${GREEN}  ✅ Port $STREAM2_PORT: Responding${NC}"
    else
        echo -e "${RED}  ❌ Port $STREAM2_PORT: Not responding${NC}"
    fi
    
    if curl -s "http://localhost:$STREAM3_PORT/metrics" > /dev/null 2>&1; then
        echo -e "${GREEN}  ✅ Port $STREAM3_PORT: Responding${NC}"
    else
        echo -e "${RED}  ❌ Port $STREAM3_PORT: Not responding${NC}"
    fi
    
    # Grafana access
    echo -e "${PURPLE}📈 Monitoring:${NC}"
    echo -e "${CYAN}  Grafana: http://localhost:3000${NC}"
    echo -e "${CYAN}  Prometheus: http://localhost:9090${NC}"
    echo -e "${CYAN}  Kafka UI: http://localhost:8080${NC}"
    
    # Quick commands
    echo -e "${PURPLE}🚀 Quick Commands:${NC}"
    echo -e "${YELLOW}  Stop all: $0 stop${NC}"
    echo -e "${YELLOW}  Status: $0 status${NC}"
    echo -e "${YELLOW}  Logs: tail -f $LOG_DIR/stream1.log${NC}"
}

# Function to stop all streams
stop_streams() {
    echo -e "${BLUE}🛑 Stopping all streams...${NC}"
    
    # Stop stream1
    local pid_file="$PID_DIR/stream1.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${YELLOW}⏹️  Stopping stream1 (PID: $pid)...${NC}"
            kill $pid
            sleep 1
            if kill -0 $pid 2>/dev/null; then
                echo -e "${YELLOW}⚠️  Force killing stream1...${NC}"
                kill -9 $pid
            fi
            echo -e "${GREEN}✅ stream1 stopped${NC}"
        fi
        rm -f "$pid_file"
    fi
    
    # Stop stream2
    local pid_file="$PID_DIR/stream2.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${YELLOW}⏹️  Stopping stream2 (PID: $pid)...${NC}"
            kill $pid
            sleep 1
            if kill -0 $pid 2>/dev/null; then
                echo -e "${YELLOW}⚠️  Force killing stream2...${NC}"
                kill -9 $pid
            fi
            echo -e "${GREEN}✅ stream2 stopped${NC}"
        fi
        rm -f "$pid_file"
    fi
    
    # Stop stream3
    local pid_file="$PID_DIR/stream3.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${YELLOW}⏹️  Stopping stream3 (PID: $pid)...${NC}"
            kill $pid
            sleep 1
            if kill -0 $pid 2>/dev/null; then
                echo -e "${YELLOW}⚠️  Force killing stream3...${NC}"
                kill -9 $pid
            fi
            echo -e "${GREEN}✅ stream3 stopped${NC}"
        fi
        rm -f "$pid_file"
    fi
    
    echo -e "${GREEN}🎉 All streams stopped${NC}"
}

# Main execution
main() {
    case "${1:-deploy}" in
        "deploy")
            print_banner
            check_prerequisites
            cleanup_existing
            start_all_streams
            sleep 5
            verify_streams
            show_status
            echo -e "${GREEN}🎉 Multi-stream deployment complete!${NC}"
            echo -e "${CYAN}💡 Check Grafana at http://localhost:3000 for real-time data${NC}"
            ;;
        "status")
            show_status
            ;;
        "stop")
            stop_streams
            ;;
        "restart")
            stop_streams
            sleep 2
            main deploy
            ;;
        *)
            echo "Usage: $0 [deploy|status|stop|restart]"
            echo ""
            echo "Commands:"
            echo "  deploy  - Deploy all 3 streams (default)"
            echo "  status  - Show current status"
            echo "  stop    - Stop all streams"
            echo "  restart - Restart all streams"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
