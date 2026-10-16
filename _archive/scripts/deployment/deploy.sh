#!/bin/bash

# MEVana Universal Deployment Script
# Usage: ./scripts/deploy.sh [environment]
# Example: ./scripts/deploy.sh production

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }

# Deployment state tracking
STEP_COUNTER=1
FAILED_CHECKS=()

step() {
    log_info "Step $STEP_COUNTER: $1"
    STEP_COUNTER=$((STEP_COUNTER + 1))
}

check_prerequisite() {
    local name=$1
    local command=$2
    local expected_output=$3

    log_info "Checking prerequisite: $name"

    if eval "$command" >/dev/null 2>&1; then
        if [ -n "$expected_output" ]; then
            local output=$(eval "$command" 2>/dev/null)
            if echo "$output" | grep -q "$expected_output"; then
                log_success "$name is available and functional"
                return 0
            else
                log_error "$name available but output unexpected: $output"
                FAILED_CHECKS+=("$name")
                return 1
            fi
        else
            log_success "$name is available"
            return 0
        fi
    else
        log_error "$name is not available"
        FAILED_CHECKS+=("$name")
        return 1
    fi
}

verify_data_flow() {
    local service_name=$1
    local url=$2
    local data_pattern=$3

    log_info "Verifying data flow for $service_name"

    local response=$(curl -s --connect-timeout 5 "$url" 2>/dev/null || echo "CURL_FAILED")

    if [ "$response" = "CURL_FAILED" ]; then
        log_error "$service_name: Connection failed to $url"
        FAILED_CHECKS+=("$service_name data flow")
        return 1
    fi

    if [ -n "$data_pattern" ]; then
        if echo "$response" | grep -q "$data_pattern"; then
            log_success "$service_name: Data flow verified (found: $data_pattern)"
            return 0
        else
            log_error "$service_name: Expected data pattern '$data_pattern' not found"
            log_warning "Response preview: $(echo "$response" | head -3)"
            FAILED_CHECKS+=("$service_name data pattern")
            return 1
        fi
    else
        log_success "$service_name: Endpoint accessible"
        return 0
    fi
}

check_port_conflicts() {
    local ports=("$@")
    log_info "Checking for port conflicts on critical ports: ${ports[*]}"

    for port in "${ports[@]}"; do
        local usage=$(lsof -i ":$port" 2>/dev/null || echo "")
        if [ -n "$usage" ]; then
            log_warning "Port $port is in use:"
            echo "$usage"
            # Don't fail deployment for port usage, just warn
        else
            log_info "Port $port is available"
        fi
    done
}

install_missing_prerequisites() {
    local target_server=$1

    if [ "$target_server" = "localhost" ]; then
        log_info "Local deployment - assuming prerequisites are available"
        return 0
    fi

    log_info "Installing missing prerequisites on $target_server"

    # Install Rust if missing
    if ! ssh "$SERVER_USER@$target_server" "source ~/.cargo/env 2>/dev/null && cargo --version" >/dev/null 2>&1; then
        log_info "Installing Rust on $target_server"
        ssh "$SERVER_USER@$target_server" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
    fi

    # Install Docker Compose if missing
    if ! ssh "$SERVER_USER@$target_server" "docker compose version" >/dev/null 2>&1; then
        log_info "Installing Docker Compose on $target_server"
        ssh "$SERVER_USER@$target_server" "
            curl -fsSL https://download.docker.com/linux/ubuntu/gpg | apt-key add - &&
            echo 'deb [arch=amd64] https://download.docker.com/linux/ubuntu jammy stable' | tee /etc/apt/sources.list.d/docker.list &&
            apt update && apt install -y docker-compose-plugin
        "
    fi
}

main() {
    local environment=${1:-production}

    # Load environment configuration
    local env_file="config/environments/$environment.env"
    if [ ! -f "$env_file" ]; then
        log_error "Environment configuration file not found: $env_file"
        exit 1
    fi

    log_info "🚀 MEVana Deployment Started - Environment: $environment"
    source "$env_file"

    # Step 1: Process Cleanup (Critical for Single Instance Policy)
    step "Process Cleanup and Conflict Prevention"

    local cleanup_cmd="
        pkill -f 'hotpath_arb_engine' || true &&
        pkill -f 'cargo run.*hotpath' || true &&
        sleep 3 &&
        echo 'Background processes cleaned up'
    "

    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        ssh "$SERVER_USER@$SERVER_IP" "$cleanup_cmd"
    else
        eval "$cleanup_cmd"
    fi
    log_success "Process cleanup completed"

    # Step 2: Prerequisites Check
    step "Infrastructure Prerequisites Check"

    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        # Remote deployment prerequisites
        check_prerequisite "SSH Access" "ssh -o ConnectTimeout=5 $SERVER_USER@$SERVER_IP 'echo ok'" "ok"
        check_prerequisite "Remote Docker" "ssh $SERVER_USER@$SERVER_IP 'docker --version'" "Docker"
        check_prerequisite "Remote Rust" "ssh $SERVER_USER@$SERVER_IP 'source ~/.cargo/env && cargo --version'" "cargo"
    else
        # Local deployment prerequisites
        check_prerequisite "Docker" "docker --version" "Docker"
        check_prerequisite "Docker Compose" "docker compose version" "Docker Compose"
        check_prerequisite "Rust" "cargo --version" "cargo"
    fi

    # Install missing prerequisites if needed
    if [ ${#FAILED_CHECKS[@]} -gt 0 ] && [ "$DEPLOYMENT_TARGET" = "production" ]; then
        log_warning "Installing missing prerequisites..."
        install_missing_prerequisites "$SERVER_IP"
        FAILED_CHECKS=() # Reset after attempting fixes
    elif [ ${#FAILED_CHECKS[@]} -gt 0 ]; then
        log_error "Prerequisites missing. Please install: ${FAILED_CHECKS[*]}"
        exit 1
    fi

    # Step 3: Port Conflict Check
    step "Port Conflict Analysis"
    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        ssh "$SERVER_USER@$SERVER_IP" "$(declare -f check_port_conflicts log_info log_warning); check_port_conflicts $CRITICAL_PORTS"
    else
        check_port_conflicts $CRITICAL_PORTS
    fi

    # Step 4: Code Deployment
    step "Code Deployment"

    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        # Create deployment archive
        log_info "Creating deployment archive..."
        tar --exclude='.git' --exclude='target' --exclude='*.tar.gz' --exclude='logs' -czf mevana-deploy.tar.gz .

        # Transfer to server
        log_info "Transferring code to $SERVER_IP..."
        scp mevana-deploy.tar.gz "$SERVER_USER@$SERVER_IP:/tmp/"

        # Extract on server
        ssh "$SERVER_USER@$SERVER_IP" "
            mkdir -p $DEPLOYMENT_PATH &&
            cd $DEPLOYMENT_PATH &&
            tar -xzf /tmp/mevana-deploy.tar.gz
        "

        # Cleanup
        rm -f mevana-deploy.tar.gz
        log_success "Code deployed to $SERVER_IP"
    else
        log_info "Local deployment - code already in place"
    fi

    # Step 5: Infrastructure Services (Docker)
    step "Infrastructure Services Deployment"

    local deploy_cmd="
        cd $DEPLOYMENT_PATH &&
        pkill -f 'hotpath_arb_engine' || true &&
        sleep 3 &&
        cd infrastructure &&
        cp $PROMETHEUS_CONFIG prometheus.yml &&
        docker compose down 2>/dev/null || true &&
        docker compose up -d &&
        sleep 15
    "

    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        ssh "$SERVER_USER@$SERVER_IP" "$deploy_cmd"
    else
        eval "$deploy_cmd"
    fi

    # Step 6: Infrastructure Verification
    step "Infrastructure Services Verification"

    local verify_cmd="
        echo 'Verifying PostgreSQL...' &&
        docker exec postgres pg_isready -U $POSTGRES_USER &&
        echo 'Verifying Redis...' &&
        docker exec redis redis-cli ping &&
        echo 'Verifying Prometheus...' &&
        curl -s http://localhost:9090/-/healthy > /dev/null &&
        echo 'Verifying Grafana...' &&
        curl -s http://localhost:3000/api/health > /dev/null
    "

    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        if ssh "$SERVER_USER@$SERVER_IP" "$verify_cmd"; then
            log_success "Infrastructure services verified"
        else
            log_error "Infrastructure verification failed"
            exit 1
        fi
    else
        if eval "$verify_cmd"; then
            log_success "Infrastructure services verified"
        else
            log_error "Infrastructure verification failed"
            exit 1
        fi
    fi

    # Step 7: Application Build (Docker)
    step "Application Container Build"

    local build_cmd="
        cd $DEPLOYMENT_PATH &&
        docker build -t mevana/hotpath_arb_engine:latest -f Dockerfile .
    "

    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        if ssh "$SERVER_USER@$SERVER_IP" "$build_cmd"; then
            log_success "Application container build completed"
        else
            log_error "Application container build failed"
            exit 1
        fi
    else
        if eval "$build_cmd"; then
            log_success "Application container build completed"
        else
            log_error "Application container build failed"
            exit 1
        fi
    fi

    # Step 8: Application Deployment (Containerized)
    step "Application Service Start (Docker)"

    local start_cmd="
        cd $DEPLOYMENT_PATH/infrastructure &&
        docker compose up -d hotpath_arb_engine &&
        sleep 10 &&
        docker ps | grep hotpath_arb_engine
    "

    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        if ssh "$SERVER_USER@$SERVER_IP" "$start_cmd"; then
            log_success "Containerized application started successfully"
        else
            log_error "Application container failed to start"
            exit 1
        fi
    else
        if eval "$start_cmd"; then
            log_success "Containerized application started successfully"
        else
            log_error "Application container failed to start"
            exit 1
        fi
    fi

    # Step 9: Application Verification with Data
    step "Application Data Verification"

    sleep 15  # Allow startup time

    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        verify_data_flow "HotPath Engine" "http://$SERVER_IP:$METRICS_PORT/metrics" "arbitrage_opportunities"
        verify_data_flow "Geyser Connection" "http://$SERVER_IP:$METRICS_PORT/metrics" "multi_geyser_stream"
    else
        verify_data_flow "HotPath Engine" "http://localhost:$METRICS_PORT/metrics" "arbitrage_opportunities"
        verify_data_flow "Geyser Connection" "http://localhost:$METRICS_PORT/metrics" "multi_geyser_stream"
    fi

    # Step 10: End-to-End System Verification
    step "End-to-End System Verification"

    local prometheus_url="http://localhost:9090"
    if [ "$DEPLOYMENT_TARGET" = "production" ]; then
        prometheus_url="http://$SERVER_IP:9090"
    fi

    # Verify Prometheus can scrape MEVana metrics
    sleep 10  # Allow Prometheus scrape cycle
    verify_data_flow "Prometheus Scraping" "$prometheus_url/api/v1/query?query=up{job=\"hotpath-engine\"}" "\"value\""

    # Final Report
    if [ ${#FAILED_CHECKS[@]} -eq 0 ]; then
        log_success "🎉 Deployment completed successfully!"
        echo ""
        log_info "📊 Service Endpoints:"
        if [ "$DEPLOYMENT_TARGET" = "production" ]; then
            log_info "   • Metrics: http://$SERVER_IP:$METRICS_PORT/metrics"
            log_info "   • Grafana: http://$SERVER_IP:3000 (admin/<ADMIN_PASSWORD>)"
            log_info "   • Prometheus: http://$SERVER_IP:9090"
        else
            log_info "   • Metrics: http://localhost:$METRICS_PORT/metrics"
            log_info "   • Grafana: http://localhost:3000 (admin/<ADMIN_PASSWORD>)"
            log_info "   • Prometheus: http://localhost:9090"
        fi
    else
        log_error "❌ Deployment completed with issues:"
        for check in "${FAILED_CHECKS[@]}"; do
            log_error "   • $check"
        done
        exit 1
    fi
}

# Error handling
trap 'log_error "Deployment failed on line $LINENO"' ERR

# Run main function
main "$@"