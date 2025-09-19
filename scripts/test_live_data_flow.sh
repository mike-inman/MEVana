#!/bin/bash

# Live Transaction Data Flow Test Runner
# This script runs comprehensive tests to verify that MEVana is receiving and processing live transaction data

set -e

echo "🧪 MEVana Live Transaction Data Flow Test"
echo "========================================"

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_TIMEOUT=60
METRICS_PORT=${METRICS_PORT:-8088}
METRICS_ENDPOINT="http://localhost:${METRICS_PORT}/metrics"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
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

# Check if required services are running
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check if MEVana hotpath engine is running
    # Check if Prometheus is accessible
    if ! curl -s http://localhost:9090/api/v1/query > /dev/null 2>&1; then
        log_warning "Prometheus not accessible at localhost:9090"
        return 1
    fi

    # Check if metrics endpoint is accessible
    if ! curl -s "$METRICS_ENDPOINT" > /dev/null 2>&1; then
        log_warning "MEVana metrics endpoint not accessible at ${METRICS_ENDPOINT}"
        return 1
    fi

    log_success "Prerequisites check passed"
    return 0
}

# Run quick diagnostic
run_diagnostic() {
    log_info "Running system diagnostic..."

    # Check metrics endpoint
    if curl -s "$METRICS_ENDPOINT" | grep -q "hotpath_transactions_processed_total"; then
        local tx_count=$(curl -s "$METRICS_ENDPOINT" | grep "hotpath_transactions_processed_total" | head -1 | awk '{print $2}' | cut -d. -f1)
        log_info "Transactions processed: $tx_count"
    else
        log_warning "Transaction metrics not found"
    fi

    # Check Geyser connection status
    if curl -s "$METRICS_ENDPOINT" | grep -q "geyser_transactions_processed_total"; then
        log_success "Geyser stream metrics detected"
    else
        log_warning "Geyser stream metrics missing"
    fi

    # Check arbitrage opportunities
    if curl -s "$METRICS_ENDPOINT" | grep -q "hotpath_arbitrage_opportunities_total"; then
        local arb_count=$(curl -s "$METRICS_ENDPOINT" | grep "hotpath_arbitrage_opportunities_total" | head -1 | awk '{print $2}' | cut -d. -f1)
        log_info "Arbitrage opportunities detected: $arb_count"
    else
        log_warning "Arbitrage opportunity metrics not found"
    fi
}

# Run the actual Rust test
run_rust_tests() {
    log_info "Running Rust integration tests..."

    cd "$PROJECT_ROOT"

    # Run the live transaction data test
    if timeout $TEST_TIMEOUT cargo test -p hotpath_arb_engine test_live_transaction_data_flow_integration --release -- --nocapture; then
        log_success "Live transaction data flow test PASSED"
        return 0
    else
        log_error "Live transaction data flow test FAILED"
        return 1
    fi
}

# Run individual test components
run_component_tests() {
    log_info "Running component tests..."

    cd "$PROJECT_ROOT"

    # Test Geyser connectivity
    log_info "Testing Geyser endpoint connectivity..."
    if timeout 10 cargo test -p hotpath_arb_engine test_geyser_endpoint_connectivity --release -- --nocapture; then
        log_success "Geyser connectivity test passed"
    else
        log_warning "Geyser connectivity test failed"
    fi

    # Test metrics endpoint
    log_info "Testing metrics endpoint availability..."
    if timeout 10 cargo test -p hotpath_arb_engine test_metrics_endpoint_availability --release -- --nocapture; then
        log_success "Metrics endpoint test passed"
    else
        log_error "Metrics endpoint test failed"
        return 1
    fi
}

# Live data flow verification using curl
verify_live_data_with_curl() {
    log_info "Verifying live data flow with direct metrics check..."

    # Capture initial metrics
    local initial_tx=$(curl -s "$METRICS_ENDPOINT" | grep "hotpath_transactions_processed_total" | head -1 | awk '{print $2}' | cut -d. -f1 || echo "0")
    local initial_arb=$(curl -s "$METRICS_ENDPOINT" | grep "hotpath_arbitrage_opportunities_total" | head -1 | awk '{print $2}' | cut -d. -f1 || echo "0")

    log_info "Initial metrics: Transactions=$initial_tx, Arbitrage=$initial_arb"

    # Wait for processing
    log_info "Waiting 15 seconds for live data processing..."
    sleep 15

    # Capture final metrics
    local final_tx=$(curl -s "$METRICS_ENDPOINT" | grep "hotpath_transactions_processed_total" | head -1 | awk '{print $2}' | cut -d. -f1 || echo "0")
    local final_arb=$(curl -s "$METRICS_ENDPOINT" | grep "hotpath_arbitrage_opportunities_total" | head -1 | awk '{print $2}' | cut -d. -f1 || echo "0")

    log_info "Final metrics: Transactions=$final_tx, Arbitrage=$final_arb"

    # Calculate differences
    local tx_diff=$((final_tx - initial_tx))
    local arb_diff=$((final_arb - initial_arb))

    if [ $tx_diff -gt 0 ]; then
        log_success "Live transaction processing verified: +$tx_diff transactions in 15s"
    else
        log_error "No live transaction processing detected"
        return 1
    fi

    if [ $arb_diff -gt 0 ]; then
        log_success "Live arbitrage detection verified: +$arb_diff opportunities in 15s"
    else
        log_warning "No new arbitrage opportunities detected in 15s"
    fi

    return 0
}

# Main test execution
main() {
    echo
    log_info "Starting live transaction data flow verification"
    echo

    # Check prerequisites
    if ! check_prerequisites; then
        log_error "Prerequisites check failed - ensure MEVana services are running"
        exit 1
    fi

    # Run diagnostic
    run_diagnostic
    echo

    # Run live data verification with curl (faster method)
    if verify_live_data_with_curl; then
        log_success "Quick live data verification PASSED"
    else
        log_error "Quick live data verification FAILED"
        exit 1
    fi
    echo

    # Run component tests
    if run_component_tests; then
        log_success "Component tests PASSED"
    else
        log_error "Component tests FAILED"
        exit 1
    fi
    echo

    # Run full Rust integration test
    if run_rust_tests; then
        log_success "Full integration test PASSED"
    else
        log_error "Full integration test FAILED"
        exit 1
    fi

    echo
    log_success "🎉 All live transaction data flow tests PASSED!"
    log_success "✅ MEVana is successfully processing live Solana transaction data"
    echo
}

# Handle script arguments
case "${1:-}" in
    "diagnostic")
        log_info "Running diagnostic only..."
        run_diagnostic
        ;;
    "quick")
        log_info "Running quick verification only..."
        check_prerequisites && verify_live_data_with_curl
        ;;
    "component")
        log_info "Running component tests only..."
        run_component_tests
        ;;
    "rust")
        log_info "Running Rust tests only..."
        run_rust_tests
        ;;
    *)
        main
        ;;
esac
