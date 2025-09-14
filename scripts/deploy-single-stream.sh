#!/bin/bash

# =============================================================================
# MEVana Single-Stream Deployment (Shyft $199 Plan Compatible)
# =============================================================================
# Deploys 1 optimized Geyser stream within Shyft connection limits.
# Ready for easy upgrade to multi-stream when gRPC plan allows.
#
# SPEED IS LIFE - This script ensures stable connection within provider limits
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

# Stream Configuration (Single Stream)
STREAM1_PORT="8088"
STREAM1_ENDPOINT="https://grpc.ny.shyft.to"
STREAM1_REGION="NY"

# Shyft Token
SHYFT_TOKEN="efff2d23-97ae-4c5f-8b8b-9a6265001189"

# Database URL
DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb"

echo -e "${BLUE}🚀 MEVana Single-Stream Optimized Deployment${NC}"
echo -e "${BLUE}================================================================${NC}"
echo -e "${CYAN}⚡ SPEED IS LIFE - Connection Limit Compliant${NC}"
echo -e "${CYAN}🎯 Single Optimized Stream (Shyft $199 Plan Compatible)${NC}"
echo -e "${CYAN}💰 Ready for Multi-Stream Upgrade (see deploy-multi-stream-FUTURE.sh)${NC}"
echo ""

# Function to print banner
print_banner() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║               ⚡ OPTIMIZED SINGLE-STREAM MEVANA ⚡            ║"
    echo "║                                                              ║"
    echo "║  🌊 Stream 1: NY Geyser (Port 8088) - Optimized           ║"
    echo "║  💤 Stream 2: Reserved for Future Upgrade                   ║"
    echo "║  💤 Stream 3: Reserved for Future Upgrade                   ║"
    echo "║                                                              ║"
    echo "║  🎯 Connection Limit Compliance - Stable Performance        ║"
    echo "║  💰 Ready for Easy Multi-Stream Upgrade                    ║"
    echo "║  ⚡ Sub-millisecond Latency - Speed is Life!                ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Function to check prerequisites
check_prerequisites() {
    echo -e "${BLUE}🔍 Checking prerequisites...${NC}"

    if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
        echo -e "${RED}❌ Not in MEVana project root${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ In MEVana project root${NC}"

    if ! docker ps | grep -q "kafka\|redis\|postgres"; then
        echo -e "${RED}❌ Docker infrastructure not running${NC}"
        echo -e "${YELLOW}💡 Start with: cd infrastructure && docker compose up -d${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Docker infrastructure running${NC}"

    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}❌ Rust toolchain not available${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Rust toolchain available${NC}"

    if [ ! -d "$PROJECT_ROOT/crates/hotpath_arb_engine" ]; then
        echo -e "${RED}❌ hotpath_arb_engine crate not found${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ hotpath_arb_engine crate found${NC}"
}

# Function to kill existing processes
cleanup_existing() {
    echo -e "${BLUE}🧹 Cleaning up existing processes...${NC}"

    if pgrep -f "hotpath_arb_engine" > /dev/null; then
        echo -e "${YELLOW}⚠️  Killing existing hotpath_arb_engine processes...${NC}"
        pkill -f "hotpath_arb_engine" || true
        sleep 2
    fi

    rm -f "$PID_DIR"/stream*.pid

    if lsof -i :8088 > /dev/null 2>&1; then
        echo -e "${RED}❌ Port 8088 still in use${NC}"
        lsof -i :8088
        exit 1
    fi

    echo -e "${GREEN}✅ Cleanup completed - port 8088 free${NC}"
}

# Function to start the stream
start_stream() {
    echo -e "${BLUE}🌊 Starting optimized single stream on port 8088...${NC}"

    # Environment variables for this stream
    local env_vars=(
        "PAPER_TRADING=true"
        "ORDERBOOK_HISTORY_ENABLED=true"
        "DATABASE_URL=$DATABASE_URL"
        "X_TOKEN=$SHYFT_TOKEN"
        "GEYSER_ENDPOINT=$STREAM1_ENDPOINT"
        "METRICS_PORT=$STREAM1_PORT"
        "RUST_LOG=info"
        "STREAM_ID=single-optimized"
        "STREAM_REGION=$STREAM1_REGION"
    )

    # Start the stream in background
    local log_file="$LOG_DIR/single-stream.log"
    local pid_file="$PID_DIR/single-stream.pid"

    # Build command
    local cmd="cd $PROJECT_ROOT && ${env_vars[*]} cargo run -p hotpath_arb_engine --bin hotpath_arb_engine"

    echo -e "${CYAN}📝 Command: $cmd${NC}"
    echo -e "${CYAN}📁 Log: $log_file${NC}"
    echo -e "${CYAN}🆔 PID: $pid_file${NC}"

    # Start process
    nohup bash -c "$cmd" > "$log_file" 2>&1 &
    local pid=$!
    echo $pid > "$pid_file"

    # Wait for startup
    echo -e "${YELLOW}⏳ Waiting for stream to start...${NC}"
    sleep 5

    # Verify process is running
    if kill -0 $pid 2>/dev/null; then
        echo -e "${GREEN}✅ Single stream started successfully (PID: $pid)${NC}"

        # Test metrics endpoint
        if curl -s "http://localhost:$STREAM1_PORT/metrics" > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Metrics endpoint responding${NC}"
        else
            echo -e "${YELLOW}⚠️  Metrics endpoint not yet ready${NC}"
        fi
    else
        echo -e "${RED}❌ Single stream failed to start${NC}"
        return 1
    fi
}

# Function to show status
show_status() {
    echo -e "${BLUE}📊 MEVana Single-Stream Status${NC}"
    echo -e "${BLUE}================================================================${NC}"

    echo -e "${PURPLE}🌊 Stream Status:${NC}"

    local pid_file="$PID_DIR/single-stream.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}  ✅ single-stream: Port $STREAM1_PORT, PID $pid, Region $STREAM1_REGION${NC}"
        else
            echo -e "${RED}  ❌ single-stream: Dead (PID $pid)${NC}"
        fi
    else
        echo -e "${RED}  ❌ single-stream: No PID file${NC}"
    fi

    echo -e "${PURPLE}📊 Metrics Endpoint:${NC}"
    if curl -s "http://localhost:$STREAM1_PORT/metrics" > /dev/null 2>&1; then
        echo -e "${GREEN}  ✅ Port $STREAM1_PORT: Responding${NC}"
    else
        echo -e "${RED}  ❌ Port $STREAM1_PORT: Not responding${NC}"
    fi

    echo -e "${PURPLE}📈 Monitoring:${NC}"
    echo -e "${CYAN}  Grafana: http://localhost:3000${NC}"
    echo -e "${CYAN}  Prometheus: http://localhost:9090${NC}"
    echo -e "${CYAN}  Kafka UI: http://localhost:8080${NC}"

    echo -e "${PURPLE}🚀 Quick Commands:${NC}"
    echo -e "${YELLOW}  Stop: $0 stop${NC}"
    echo -e "${YELLOW}  Status: $0 status${NC}"
    echo -e "${YELLOW}  Logs: tail -f $LOG_DIR/single-stream.log${NC}"
    echo -e "${YELLOW}  Multi-stream (future): ./deploy-multi-stream-FUTURE.sh${NC}"
}

# Function to stop stream
stop_stream() {
    echo -e "${BLUE}🛑 Stopping single stream...${NC}"

    local pid_file="$PID_DIR/single-stream.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${YELLOW}⏹️  Stopping single-stream (PID: $pid)...${NC}"
            kill $pid
            sleep 1
            if kill -0 $pid 2>/dev/null; then
                echo -e "${YELLOW}⚠️  Force killing single-stream...${NC}"
                kill -9 $pid
            fi
            echo -e "${GREEN}✅ Single stream stopped${NC}"
        fi
        rm -f "$pid_file"
    fi
}

# Main execution
main() {
    case "${1:-deploy}" in
        "deploy")
            print_banner
            check_prerequisites
            cleanup_existing
            start_stream
            sleep 3
            show_status
            echo -e "${GREEN}🎉 Single-stream deployment complete!${NC}"
            echo -e "${CYAN}💡 Check Grafana at http://localhost:3000 for real-time data${NC}"
            echo -e "${CYAN}🚀 Multi-stream available when you upgrade: ./deploy-multi-stream-FUTURE.sh${NC}"
            ;;
        "status")
            show_status
            ;;
        "stop")
            stop_stream
            ;;
        "restart")
            stop_stream
            sleep 2
            main deploy
            ;;
        *)
            echo "Usage: $0 [deploy|status|stop|restart]"
            echo ""
            echo "Commands:"
            echo "  deploy  - Deploy single optimized stream (default)"
            echo "  status  - Show current status"
            echo "  stop    - Stop stream"
            echo "  restart - Restart stream"
            echo ""
            echo "Future upgrade: ./deploy-multi-stream-FUTURE.sh (when gRPC plan allows)"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"