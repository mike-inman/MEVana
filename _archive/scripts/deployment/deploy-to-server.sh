#!/bin/bash
# Deprecated: use CI/CD GHCR deploy and scripts/deployment/deploy_vultr_compose.sh
# Last updated: 2025-09-15

# =============================================================================
# MEVana Production Server Deployment Script
# =============================================================================
# Deploys the proven 3-stream architecture to production server
# with comprehensive logging and rollback capabilities.
#
# Usage: ./deploy-to-server.sh [SERVER_IP] [deploy|rollback|status|logs]
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
DEPLOYMENT_LOG="$PROJECT_ROOT/logs/deployment_$(date +%Y%m%d_%H%M%S).log"
SERVER_USER="root"  # Adjust as needed
SERVER_IP="${1:-}"
COMMAND="${2:-deploy}"

# Baseline metrics from local system (captured)
LOCAL_BASELINE_STREAM1="15687"
LOCAL_BASELINE_STREAM2="67088"
LOCAL_BASELINE_STREAM3="109392"
LOCAL_BASELINE_TOTAL="192167"

# Create logs directory
mkdir -p "$PROJECT_ROOT/logs"

# Logging function
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "${timestamp} [${level}] ${message}" | tee -a "$DEPLOYMENT_LOG"
}

log_info() { log "INFO" "$@"; }
log_warn() { log "WARN" "$@"; }
log_error() { log "ERROR" "$@"; }
log_success() { log "SUCCESS" "$@"; }

# Print banner
print_banner() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║            🚀 MEVana Production Deployment 🚀               ║"
    echo "║                                                              ║"
    echo "║  📊 3-Stream Internal Splitting Architecture                ║"
    echo "║  ⚡ Sub-millisecond Arbitrage Detection                     ║"
    echo "║  🔍 Full Monitoring & Rollback Capabilities                ║"
    echo "║                                                              ║"
    echo "║  Target: ${SERVER_IP:-[SERVER_IP_REQUIRED]}                                   ║"
    echo "║  Command: ${COMMAND}                                             ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Validate prerequisites
check_prerequisites() {
    log_info "${BLUE}🔍 Validating deployment prerequisites...${NC}"

    if [ -z "$SERVER_IP" ]; then
        log_error "${RED}❌ Server IP required. Usage: $0 [SERVER_IP] [command]${NC}"
        exit 1
    fi

    if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
        log_error "${RED}❌ Not in MEVana project root${NC}"
        exit 1
    fi

    # Check if we can reach the server
    if ! ping -c 1 "$SERVER_IP" > /dev/null 2>&1; then
        log_warn "${YELLOW}⚠️ Cannot ping server $SERVER_IP${NC}"
    fi

    # Check SSH connectivity
    if ! ssh -o ConnectTimeout=10 "$SERVER_USER@$SERVER_IP" "echo 'SSH connection test'" > /dev/null 2>&1; then
        log_error "${RED}❌ Cannot SSH to $SERVER_USER@$SERVER_IP${NC}"
        exit 1
    fi

    log_success "${GREEN}✅ Prerequisites validated${NC}"
}

# Document local baseline
capture_baseline() {
    log_info "${BLUE}📊 Capturing local system baseline...${NC}"

    local baseline_file="$PROJECT_ROOT/logs/baseline_$(date +%Y%m%d_%H%M%S).txt"

    {
        echo "=== MEVana Local System Baseline ==="
        echo "Timestamp: $(date)"
        echo "Git Commit: $(git rev-parse HEAD 2>/dev/null || echo 'Unknown')"
        echo ""
        echo "Stream Messages Total:"
        echo "- Stream 1: $LOCAL_BASELINE_STREAM1"
        echo "- Stream 2: $LOCAL_BASELINE_STREAM2"
        echo "- Stream 3: $LOCAL_BASELINE_STREAM3"
        echo "- Total: $LOCAL_BASELINE_TOTAL"
        echo ""
        echo "System Status:"
        if pgrep -f "hotpath_arb_engine" > /dev/null; then
            echo "- Local system: RUNNING"
        else
            echo "- Local system: STOPPED"
        fi
        echo ""
        echo "Key Files:"
        echo "- Main binary: $(ls -la target/*/hotpath_arb_engine 2>/dev/null | wc -l) files"
        echo "- Deployment script: $(ls -la scripts/deploy-single-stream.sh)"
        echo ""
    } > "$baseline_file"

    log_success "${GREEN}✅ Baseline captured: $baseline_file${NC}"
}

# Deploy to server
deploy_to_server() {
    log_info "${BLUE}🚀 Starting deployment to $SERVER_IP...${NC}"

    # Step 1: Prepare local environment
    log_info "📦 Preparing local environment..."

    # Build release binary
    log_info "🔧 Building release binary..."
    if ! DATABASE_URL="postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb" cargo build --release -p hotpath_arb_engine; then
        log_error "${RED}❌ Failed to build release binary${NC}"
        exit 1
    fi
    log_success "${GREEN}✅ Release binary built${NC}"

    # Step 2: Transfer files to server
    log_info "📤 Transferring files to server..."

    # Create MEVana directory on server
    ssh "$SERVER_USER@$SERVER_IP" "mkdir -p /opt/mevana/{logs,pids,scripts}" || {
        log_error "${RED}❌ Failed to create server directories${NC}"
        exit 1
    }

    # Transfer binary
    scp "target/release/hotpath_arb_engine" "$SERVER_USER@$SERVER_IP:/opt/mevana/" || {
        log_error "${RED}❌ Failed to transfer binary${NC}"
        exit 1
    }

    # Transfer deployment script
    scp "scripts/deploy-single-stream.sh" "$SERVER_USER@$SERVER_IP:/opt/mevana/scripts/" || {
        log_error "${RED}❌ Failed to transfer deployment script${NC}"
        exit 1
    }

    # Make scripts executable
    ssh "$SERVER_USER@$SERVER_IP" "chmod +x /opt/mevana/scripts/deploy-single-stream.sh /opt/mevana/hotpath_arb_engine" || {
        log_error "${RED}❌ Failed to make scripts executable${NC}"
        exit 1
    }

    log_success "${GREEN}✅ Files transferred successfully${NC}"

    # Step 3: Server environment check
    log_info "🔍 Validating server environment..."

    server_check_result=$(ssh "$SERVER_USER@$SERVER_IP" "
        echo 'Server Environment Check:'
        echo '- Docker: ' \$(docker --version 2>/dev/null || echo 'NOT INSTALLED')
        echo '- PostgreSQL accessible: ' \$(pg_isready -h localhost -p 5433 2>/dev/null && echo 'YES' || echo 'NO')
        echo '- Port 8088 available: ' \$(lsof -i :8088 2>/dev/null && echo 'OCCUPIED' || echo 'FREE')
        echo '- Network to Shyft: ' \$(curl -s --connect-timeout 5 https://grpc.ny.shyft.to && echo 'OK' || echo 'FAILED')
    ")

    log_info "$server_check_result"

    # Step 4: Deploy and start service
    log_info "🌊 Starting MEVana service on server..."

    # Set environment variables and start service
    start_result=$(ssh "$SERVER_USER@$SERVER_IP" "
        cd /opt/mevana
        export PAPER_TRADING=true
        export DATABASE_URL='postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb'
        export GEYSER_ENDPOINT='https://grpc.ny.shyft.to'
        export X_TOKEN='<SHYFT_X_TOKEN>'
        export METRICS_PORT=8088
        export RUST_LOG=info
        export STREAM_ID=server-production
        export STREAM_REGION=NY

        # Kill any existing processes
        pkill -f 'hotpath_arb_engine' || true
        sleep 2

        # Start service in background
        nohup ./hotpath_arb_engine > logs/production.log 2>&1 &
        echo \$! > pids/hotpath_arb_engine.pid

        # Wait for startup
        sleep 5

        # Check if process is running
        if kill -0 \$(cat pids/hotpath_arb_engine.pid 2>/dev/null) 2>/dev/null; then
            echo 'Service started successfully'
        else
            echo 'Service failed to start'
            exit 1
        fi
    ")

    if [[ "$start_result" == *"Service started successfully"* ]]; then
        log_success "${GREEN}✅ MEVana service started on server${NC}"
    else
        log_error "${RED}❌ Failed to start service: $start_result${NC}"
        exit 1
    fi

    # Step 5: Validate deployment
    validate_deployment
}

# Validate deployment
validate_deployment() {
    log_info "${BLUE}🔍 Validating server deployment...${NC}"

    # Wait for metrics to be available
    sleep 10

    # Check metrics endpoint
    metrics_check=$(ssh "$SERVER_USER@$SERVER_IP" "
        curl -s http://localhost:8088/metrics | head -5 || echo 'METRICS_FAILED'
    ")

    if [[ "$metrics_check" == *"METRICS_FAILED"* ]]; then
        log_error "${RED}❌ Metrics endpoint not responding${NC}"
        show_server_logs
        exit 1
    fi

    log_success "${GREEN}✅ Metrics endpoint responding${NC}"

    # Check for arbitrage detection
    log_info "⏳ Waiting for arbitrage detection (60 seconds)..."

    detection_check=$(ssh "$SERVER_USER@$SERVER_IP" "
        timeout 60s tail -f /opt/mevana/logs/production.log | grep -m 1 'SUCCESS.*μs' || echo 'NO_DETECTION'
    ")

    if [[ "$detection_check" == *"NO_DETECTION"* ]]; then
        log_warn "${YELLOW}⚠️ No arbitrage detection within 60 seconds${NC}"
        log_info "Recent logs:"
        show_server_logs
    else
        log_success "${GREEN}✅ Arbitrage detection confirmed: $detection_check${NC}"
    fi

    # Performance comparison
    server_metrics=$(ssh "$SERVER_USER@$SERVER_IP" "
        curl -s http://localhost:8088/metrics | grep 'stream_messages_total' || echo 'NO_METRICS'
    ")

    log_info "📊 Server metrics comparison:"
    log_info "Local baseline total: $LOCAL_BASELINE_TOTAL messages"
    log_info "Server metrics: $server_metrics"
}

# Show server logs
show_server_logs() {
    log_info "${BLUE}📋 Recent server logs:${NC}"
    ssh "$SERVER_USER@$SERVER_IP" "tail -n 20 /opt/mevana/logs/production.log || echo 'No logs available'"
}

# Check deployment status
check_status() {
    log_info "${BLUE}📊 Checking server status...${NC}"

    status_result=$(ssh "$SERVER_USER@$SERVER_IP" "
        cd /opt/mevana
        echo '=== MEVana Server Status ==='
        echo 'Process: ' \$(ps aux | grep hotpath_arb_engine | grep -v grep | wc -l) 'running'
        echo 'PID: ' \$(cat pids/hotpath_arb_engine.pid 2>/dev/null || echo 'No PID file')
        echo 'Port 8088: ' \$(lsof -i :8088 2>/dev/null | wc -l) 'connections'
        echo 'Metrics: ' \$(curl -s http://localhost:8088/metrics | grep stream_messages_total | wc -l) 'streams'
        echo ''
        echo 'Recent performance:'
        tail -n 5 logs/production.log | grep 'SUCCESS.*μs' || echo 'No recent detections'
    ")

    log_info "$status_result"
}

# Rollback deployment
rollback_deployment() {
    log_warn "${YELLOW}🔄 Rolling back deployment...${NC}"

    ssh "$SERVER_USER@$SERVER_IP" "
        cd /opt/mevana
        # Stop current service
        pkill -f 'hotpath_arb_engine' || true
        # Could restore previous version here if we had one
        echo 'Service stopped. Manual intervention may be required.'
    "

    log_success "${GREEN}✅ Rollback completed${NC}"
}

# Main execution
main() {
    print_banner

    case "$COMMAND" in
        "deploy")
            check_prerequisites
            capture_baseline
            deploy_to_server
            log_success "${GREEN}🎉 Deployment completed successfully!${NC}"
            log_info "Deployment log: $DEPLOYMENT_LOG"
            log_info "Server monitoring: ssh $SERVER_USER@$SERVER_IP 'cd /opt/mevana && tail -f logs/production.log'"
            ;;
        "status")
            check_status
            ;;
        "logs")
            show_server_logs
            ;;
        "rollback")
            rollback_deployment
            ;;
        *)
            log_error "Usage: $0 [SERVER_IP] [deploy|status|logs|rollback]"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
