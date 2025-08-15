#!/bin/bash

# =============================================================================
# SOLANA ARBITRAGE PLATFORM - BUILD SCRIPT
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
CACHE_PROFILE="cache"

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

# Check if Docker is running
check_docker() {
    if ! docker info > /dev/null 2>&1; then
        log_error "Docker is not running. Please start Docker and try again."
        exit 1
    fi
}

# Build cache layers
build_cache() {
    log_info "Building cache layers for faster rebuilds..."
    docker compose --profile $CACHE_PROFILE build --no-cache
    log_success "Cache layers built successfully"
}

# Build specific service
build_service() {
    local service=$1
    log_info "Building service: $service"
    
    # Build with cache
    docker compose build --cache-from arbitrage-$service:cache $service
    
    log_success "Service $service built successfully"
}

# Build all services
build_all() {
    log_info "Building all services..."
    
    for service in "${SERVICES[@]}"; do
        build_service $service
    done
    
    log_success "All services built successfully"
}

# Start services
start_services() {
    local services=$1
    if [ -z "$services" ]; then
        services="redis stream-orca swap-ingest pathfinder"
    fi
    
    log_info "Starting services: $services"
    docker compose up -d $services
    log_success "Services started successfully"
}

# Stop services
stop_services() {
    log_info "Stopping all services..."
    docker compose down
    log_success "Services stopped successfully"
}

# Clean up
cleanup() {
    log_info "Cleaning up Docker resources..."
    
    # Remove unused containers
    docker container prune -f
    
    # Remove unused images
    docker image prune -f
    
    # Remove unused networks
    docker network prune -f
    
    log_success "Cleanup completed"
}

# Show logs
show_logs() {
    local service=$1
    if [ -z "$service" ]; then
        log_info "Showing logs for all services..."
        docker compose logs -f
    else
        log_info "Showing logs for service: $service"
        docker compose logs -f $service
    fi
}

# Test heartbeats
test_heartbeats() {
    log_info "Testing service heartbeats..."
    ./test-heartbeats.sh test
}

# Monitor heartbeats
monitor_heartbeats() {
    log_info "Monitoring service heartbeats..."
    ./test-heartbeats.sh monitor
}

# Health check
health_check() {
    log_info "Checking service health..."
    docker compose ps
    log_info "Health check completed"
}

# Show usage
show_usage() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  build [SERVICE]     Build specific service or all services"
    echo "  cache               Build cache layers for faster rebuilds"
    echo "  start [SERVICES]    Start services (default: redis stream-orca swap-ingest pathfinder)"
    echo "  stop                Stop all services"
    echo "  restart [SERVICES]  Restart services"
    echo "  logs [SERVICE]      Show logs for service or all services"
    echo "  health              Check service health"
    echo "  heartbeats          Test service heartbeats"
    echo "  monitor             Monitor heartbeats in real-time"
    echo "  clean               Clean up Docker resources"
    echo "  full                Full build and start (cache + build + start)"
    echo ""
    echo "Examples:"
    echo "  $0 build stream-orca"
    echo "  $0 start"
    echo "  $0 logs swap-ingest"
    echo "  $0 heartbeats"
    echo "  $0 monitor"
    echo "  $0 full"
}

# Main script logic
main() {
    check_docker
    
    case "${1:-help}" in
        "build")
            if [ -n "$2" ]; then
                build_service $2
            else
                build_all
            fi
            ;;
        "cache")
            build_cache
            ;;
        "start")
            start_services "$2"
            ;;
        "stop")
            stop_services
            ;;
        "restart")
            stop_services
            start_services "$2"
            ;;
        "logs")
            show_logs "$2"
            ;;
        "health")
            health_check
            ;;
        "heartbeats")
            test_heartbeats
            ;;
        "monitor")
            monitor_heartbeats
            ;;
        "clean")
            cleanup
            ;;
        "full")
            build_cache
            build_all
            start_services
            health_check
            ;;
        "help"|*)
            show_usage
            ;;
    esac
}

# Run main function with all arguments
main "$@" 