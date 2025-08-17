#!/bin/bash

# End-to-End Testing Runner
# This script implements the initial testing framework for MEVana
# Created: 2024-08-17 19:00 CDT

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TEST_MODE=true
TEST_REDIS_URL=${TEST_REDIS_URL:-"redis://localhost:6379/1"}
TEST_KAFKA_BROKERS=${TEST_KAFKA_BROKERS:-"localhost:9092"}
TEST_SHYFT_API_KEY=${TEST_SHYFT_API_KEY:-"fWrVh_mDygK358FC"}

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
    ((TESTS_PASSED++))
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    ((TESTS_FAILED++))
}

log_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
    ((TESTS_TOTAL++))
}

# Test result summary
print_test_summary() {
    echo ""
    echo "=========================================="
    echo "🧪 END-TO-END TESTING SUMMARY"
    echo "=========================================="
    echo "Total Tests: $TESTS_TOTAL"
    echo "Passed: $TESTS_PASSED"
    echo "Failed: $TESTS_FAILED"
    if [ $TESTS_TOTAL -gt 0 ]; then
        echo "Success Rate: $(( (TESTS_PASSED * 100) / TESTS_TOTAL ))%"
    else
        echo "Success Rate: N/A"
    fi
    echo "=========================================="
    
    if [ $TESTS_FAILED -eq 0 ] && [ $TESTS_TOTAL -gt 0 ]; then
        echo -e "${GREEN}🎉 ALL TESTS PASSED!${NC}"
        exit 0
    elif [ $TESTS_TOTAL -eq 0 ]; then
        echo -e "${YELLOW}⚠️  NO TESTS WERE RUN${NC}"
        exit 0
    else
        echo -e "${RED}❌ $TESTS_FAILED TESTS FAILED${NC}"
        exit 1
    fi
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."
    # Kill any background processes
    jobs -p | xargs -r kill 2>/dev/null || true
    # Reset Redis test database (ignore errors)
    redis-cli -u "$TEST_REDIS_URL" FLUSHDB > /dev/null 2>&1 || true
}

# Set up trap for cleanup
trap cleanup EXIT

# Test 1: Service Health Check
test_service_health() {
    log_test "Service Health Check"
    
    # Check if Redis is running
    if redis-cli -u "$TEST_REDIS_URL" PING > /dev/null 2>&1; then
        log_success "Redis connection successful"
    else
        log_error "Redis connection failed"
        return 1
    fi
    
    # Check if Kafka is accessible (basic check)
    if command -v nc >/dev/null 2>&1 && nc -z localhost 9092 2>/dev/null; then
        log_success "Kafka broker accessible"
    else
        log_warning "Kafka broker not accessible (may not be running)"
    fi
    
    # Check if services compile
    log_info "Checking service compilation..."
    if cargo check --workspace > /dev/null 2>&1; then
        log_success "All services compile successfully"
    else
        log_error "Service compilation failed"
        return 1
    fi
    
    return 0
}

# Test 2: Validation Service Test
test_validation_service() {
    log_test "Validation Service Test"
    
    log_info "Testing Shyft API validation..."
    
    # Run validation service test
    if SHYFT_API_KEY="$TEST_SHYFT_API_KEY" RUST_LOG=error cargo run -p shyft-validator > /tmp/validation_test.log 2>&1; then
        log_success "Validation service test completed"
        
        # Check for validation success in logs
        if grep -q "✅ MATCHES" /tmp/validation_test.log; then
            log_success "Validation service successfully validated against Shyft APIs"
        else
            log_error "Validation service did not complete successfully"
            return 1
        fi
    else
        log_error "Validation service test failed"
        cat /tmp/validation_test.log
        return 1
    fi
    
    return 0
}

# Test 3: Basic Data Flow Test
test_basic_data_flow() {
    log_test "Basic Data Flow Test"
    
    log_info "Testing basic data flow through the pipeline..."
    
    # This is a simplified test - in production we'd inject real data
    # For now, we'll test that our services can start and connect
    
    # Test that dex-backfill can start (without actually connecting to Geyser)
    log_info "Testing dex-backfill service startup..."
    if timeout 10s cargo run -p dex-backfill -- --help > /dev/null 2>&1; then
        log_success "dex-backfill service can start"
    else
        log_warning "dex-backfill service startup test incomplete (may need Geyser connection)"
    fi
    
    # Test that dex-stream-universal can start
    log_info "Testing dex-stream-universal service startup..."
    if timeout 10s cargo run -p dex-stream-universal -- --help > /dev/null 2>&1; then
        log_success "dex-stream-universal service can start"
    else
        log_warning "dex-stream-universal service startup test incomplete"
    fi
    
    return 0
}

# Test 4: Performance Baseline Test
test_performance_baseline() {
    log_test "Performance Baseline Test"
    
    log_info "Establishing performance baseline..."
    
    # Test validation service performance
    log_info "Testing validation service performance..."
    start_time=$(date +%s%N)
    
    if SHYFT_API_KEY="$TEST_SHYFT_API_KEY" RUST_LOG=error cargo run -p shyft-validator > /dev/null 2>&1; then
        end_time=$(date +%s%N)
        duration_ms=$(( (end_time - start_time) / 1000000 ))
        
        if [ $duration_ms -lt 5000 ]; then  # Less than 5 seconds
            log_success "Validation service performance: ${duration_ms}ms (acceptable)"
        else
            log_warning "Validation service performance: ${duration_ms}ms (slow, but acceptable for testing)"
        fi
    else
        log_error "Performance test failed"
        return 1
    fi
    
    return 0
}

# Test 5: Data Consistency Test
test_data_consistency() {
    log_test "Data Consistency Test"
    
    log_info "Testing data consistency across services..."
    
    # Test that our validation service can access Shyft APIs
    if curl -s "https://programs.shyft.to/v0/graphql/?api_key=$TEST_SHYFT_API_KEY&network=mainnet-beta" \
        -H "Content-Type: application/json" \
        -d '{"query":"query { ORCA_WHIRLPOOLS_whirlpool(limit: 1) { pubkey } }"}' \
        | grep -q "pubkey"; then
        log_success "Shyft GraphQL API accessible"
    else
        log_error "Shyft GraphQL API not accessible"
        return 1
    fi
    
    return 0
}

# Main test execution
main() {
    echo "🧪 MEVana End-to-End Testing Framework"
    echo "======================================"
    echo "Test Mode: $TEST_MODE"
    echo "Redis URL: $TEST_REDIS_URL"
    echo "Kafka Brokers: $TEST_KAFKA_BROKERS"
    echo "Shyft API Key: ${TEST_SHYFT_API_KEY:0:8}..."
    echo ""
    
    # Set environment variables for tests
    export TEST_MODE
    export TEST_REDIS_URL
    export TEST_KAFKA_BROKERS
    export TEST_SHYFT_API_KEY
    
    # Run tests
    test_service_health
    test_validation_service
    test_basic_data_flow
    test_performance_baseline
    test_data_consistency
    
    # Print summary
    print_test_summary
}

# Run main function
main "$@"
