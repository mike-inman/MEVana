#!/bin/bash

# =============================================================================
# HEARTBEAT TEST SCRIPT
# =============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SERVICES=("stream-orca" "swap-ingest" "pathfinder")
HEARTBEAT_INTERVAL=35  # Slightly longer than 30s to account for timing

# Functions
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

# Check if services are running
check_services_running() {
    log_info "Checking if services are running..."
    
    for service in "${SERVICES[@]}"; do
        if docker compose ps | grep -q "$service.*Up"; then
            log_success "✅ $service is running"
        else
            log_error "❌ $service is not running"
            return 1
        fi
    done
    
    return 0
}

# Monitor heartbeats for a service
monitor_heartbeat() {
    local service=$1
    local duration=$2
    
    log_info "Monitoring heartbeats for $service for ${duration}s..."
    
    # Get the current timestamp
    local start_time=$(date +%s)
    local heartbeat_count=0
    local last_heartbeat_time=0
    
    # Monitor logs for heartbeat messages
    timeout $duration docker compose logs -f --tail=0 $service | while read -r line; do
        if echo "$line" | grep -q "💓.*heartbeat"; then
            heartbeat_count=$((heartbeat_count + 1))
            last_heartbeat_time=$(date +%s)
            local elapsed=$((last_heartbeat_time - start_time))
            log_success "💓 $service heartbeat #$heartbeat_count (${elapsed}s elapsed)"
        fi
    done
    
    # Check if we got any heartbeats
    if [ $heartbeat_count -gt 0 ]; then
        log_success "✅ $service sent $heartbeat_count heartbeats in ${duration}s"
        return 0
    else
        log_error "❌ $service sent no heartbeats in ${duration}s"
        return 1
    fi
}

# Test all services
test_all_heartbeats() {
    log_info "Testing heartbeats for all services..."
    
    local all_passed=true
    
    for service in "${SERVICES[@]}"; do
        log_info "Testing $service..."
        
        if monitor_heartbeat $service $HEARTBEAT_INTERVAL; then
            log_success "✅ $service heartbeat test passed"
        else
            log_error "❌ $service heartbeat test failed"
            all_passed=false
        fi
        
        # Small delay between tests
        sleep 2
    done
    
    if $all_passed; then
        log_success "🎉 All heartbeat tests passed!"
        return 0
    else
        log_error "💥 Some heartbeat tests failed"
        return 1
    fi
}

# Quick status check
quick_status() {
    log_info "Quick status check..."
    
    echo ""
    echo "Service Status:"
    docker compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"
    
    echo ""
    echo "Recent Heartbeats:"
    for service in "${SERVICES[@]}"; do
        echo -n "$service: "
        if docker compose logs --tail=10 $service | grep -q "💓.*heartbeat"; then
            echo -e "${GREEN}✅ Recent heartbeat detected${NC}"
        else
            echo -e "${RED}❌ No recent heartbeat${NC}"
        fi
    done
}

# Show usage
show_usage() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  status     Quick status check of all services"
    echo "  test       Test heartbeats for all services (takes ~2 minutes)"
    echo "  monitor    Monitor heartbeats in real-time"
    echo "  help       Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 status"
    echo "  $0 test"
    echo "  $0 monitor"
}

# Monitor heartbeats in real-time
monitor_real_time() {
    log_info "Monitoring heartbeats in real-time (Ctrl+C to stop)..."
    
    # Monitor all services simultaneously
    docker compose logs -f --tail=0 "${SERVICES[@]}" | while read -r line; do
        if echo "$line" | grep -q "💓.*heartbeat"; then
            local timestamp=$(date '+%H:%M:%S')
            local service=$(echo "$line" | grep -o 'stream-orca\|swap-ingest\|pathfinder' | head -1)
            echo -e "${GREEN}[$timestamp] 💓 $service heartbeat${NC}"
        fi
    done
}

# Main script logic
main() {
    case "${1:-help}" in
        "status")
            check_services_running && quick_status
            ;;
        "test")
            check_services_running && test_all_heartbeats
            ;;
        "monitor")
            check_services_running && monitor_real_time
            ;;
        "help"|*)
            show_usage
            ;;
    esac
}

# Run main function with all arguments
main "$@" 