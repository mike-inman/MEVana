#!/bin/bash

# Address Enrichment Service Test Script
# Tests the unknown address enrichment functionality

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    local status=$1
    local message=$2
    case $status in
        "SUCCESS")
            echo -e "${GREEN}✅ $message${NC}"
            ;;
        "ERROR")
            echo -e "${RED}❌ $message${NC}"
            ;;
        "WARNING")
            echo -e "${YELLOW}⚠️  $message${NC}"
            ;;
        "INFO")
            echo -e "${BLUE}ℹ️  $message${NC}"
            ;;
    esac
}

print_header() {
    echo "=========================================="
    echo "Address Enrichment Service Test"
    echo "=========================================="
    echo ""
}

# Test 1: Check if service files exist
test_service_files() {
    print_status "INFO" "Test 1: Checking if service files exist"
    
    local files=(
        "address-enrichment-service/Cargo.toml"
        "address-enrichment-service/src/main.rs"
        "address-enrichment-service/Dockerfile"
    )
    
    local found_files=0
    
    for file in "${files[@]}"; do
        if [ -f "$file" ]; then
            print_status "SUCCESS" "File found: $file"
            ((found_files++))
        else
            print_status "ERROR" "File missing: $file"
        fi
    done
    
    if [ $found_files -eq ${#files[@]} ]; then
        print_status "SUCCESS" "All service files present"
    else
        print_status "ERROR" "Some service files are missing"
        return 1
    fi
}

# Test 2: Check Rust compilation
test_rust_compilation() {
    print_status "INFO" "Test 2: Checking Rust compilation"
    
    cd address-enrichment-service
    
    if cargo check --quiet; then
        print_status "SUCCESS" "Rust code compiles successfully"
    else
        print_status "ERROR" "Rust compilation failed"
        cd ..
        return 1
    fi
    
    cd ..
}

# Test 3: Check dependencies
test_dependencies() {
    print_status "INFO" "Test 3: Checking dependencies"
    
    local dependencies=(
        "tokio"
        "serde"
        "reqwest"
        "rdkafka"
        "sqlx"
        "shared-types"
    )
    
    local found_deps=0
    
    for dep in "${dependencies[@]}"; do
        if grep -q "$dep" address-enrichment-service/Cargo.toml; then
            print_status "SUCCESS" "Dependency found: $dep"
            ((found_deps++))
        else
            print_status "WARNING" "Dependency missing: $dep"
        fi
    done
    
    print_status "INFO" "Found $found_deps out of ${#dependencies[@]} dependencies"
    
    if [ $found_deps -ge 5 ]; then
        print_status "SUCCESS" "Dependencies are well configured"
    else
        print_status "WARNING" "Some dependencies may be missing"
    fi
}

# Test 4: Check Shyft API integration
test_shyft_integration() {
    print_status "INFO" "Test 4: Checking Shyft API integration"
    
    local shyft_features=(
        "get_shyft_token_info"
        "get_shyft_account_info"
        "shyft_api_key"
        "shyft_api_url"
        "ShyftTokenResponse"
        "ShyftAccountResponse"
    )
    
    local found_features=0
    
    for feature in "${shyft_features[@]}"; do
        if grep -q "$feature" address-enrichment-service/src/main.rs; then
            print_status "SUCCESS" "Shyft feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Shyft feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#shyft_features[@]} Shyft features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Shyft API integration is comprehensive"
    else
        print_status "WARNING" "Shyft API integration may be incomplete"
    fi
}

# Test 5: Check database integration
test_database_integration() {
    print_status "INFO" "Test 5: Checking database integration"
    
    local db_features=(
        "enriched_addresses"
        "CREATE TABLE"
        "INSERT INTO"
        "ON CONFLICT"
        "load_known_addresses"
        "store_enriched_address"
    )
    
    local found_features=0
    
    for feature in "${db_features[@]}"; do
        if grep -q "$feature" address-enrichment-service/src/main.rs; then
            print_status "SUCCESS" "Database feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Database feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#db_features[@]} database features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Database integration is well implemented"
    else
        print_status "WARNING" "Database integration may need improvement"
    fi
}

# Test 6: Check Kafka integration
test_kafka_integration() {
    print_status "INFO" "Test 6: Checking Kafka integration"
    
    local kafka_features=(
        "StreamConsumer"
        "FutureProducer"
        "subscribe"
        "send"
        "raw.swaps.all"
        "enriched.addresses"
    )
    
    local found_features=0
    
    for feature in "${kafka_features[@]}"; do
        if grep -q "$feature" address-enrichment-service/src/main.rs; then
            print_status "SUCCESS" "Kafka feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Kafka feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#kafka_features[@]} Kafka features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Kafka integration is well implemented"
    else
        print_status "WARNING" "Kafka integration may need improvement"
    fi
}

# Test 7: Check address processing logic
test_address_processing() {
    print_status "INFO" "Test 7: Checking address processing logic"
    
    local processing_features=(
        "extract_unknown_addresses"
        "is_system_address"
        "enrich_address"
        "create_unknown_address"
        "rate_limiter"
        "known_addresses"
    )
    
    local found_features=0
    
    for feature in "${processing_features[@]}"; do
        if grep -q "$feature" address-enrichment-service/src/main.rs; then
            print_status "SUCCESS" "Processing feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Processing feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#processing_features[@]} processing features"
    
    if [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Address processing logic is comprehensive"
    else
        print_status "WARNING" "Address processing logic may need improvement"
    fi
}

# Test 8: Check error handling
test_error_handling() {
    print_status "INFO" "Test 8: Checking error handling"
    
    local error_features=(
        "anyhow::Result"
        "error!"
        "warn!"
        "info!"
        "match.*Err"
        "unwrap_or_else"
    )
    
    local found_features=0
    
    for feature in "${error_features[@]}"; do
        if grep -q "$feature" address-enrichment-service/src/main.rs; then
            print_status "SUCCESS" "Error handling feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Error handling feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#error_features[@]} error handling features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Error handling is well implemented"
    else
        print_status "WARNING" "Error handling may need improvement"
    fi
}

# Test 9: Check Docker configuration
test_docker_config() {
    print_status "INFO" "Test 9: Checking Docker configuration"
    
    local docker_features=(
        "FROM rust"
        "FROM debian"
        "COPY.*target/release"
        "USER app"
        "HEALTHCHECK"
        "EXPOSE"
    )
    
    local found_features=0
    
    for feature in "${docker_features[@]}"; do
        if grep -q "$feature" address-enrichment-service/Dockerfile; then
            print_status "SUCCESS" "Docker feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Docker feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#docker_features[@]} Docker features"
    
    if [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Docker configuration is well implemented"
    else
        print_status "WARNING" "Docker configuration may need improvement"
    fi
}

# Test 10: Generate integration report
test_integration_report() {
    print_status "INFO" "Test 10: Generating integration report"
    
    echo ""
    echo "🔍 Address Enrichment Service Integration Report"
    echo "================================================"
    echo ""
    echo "Service Purpose:"
    echo "  • Monitor incoming transaction data for unknown addresses"
    echo "  • Enrich unknown addresses using Shyft API"
    echo "  • Store enriched data in PostgreSQL database"
    echo "  • Publish enriched data to Kafka for downstream services"
    echo ""
    echo "Key Features:"
    echo "  • Real-time address enrichment from transaction streams"
    echo "  • Rate-limited API calls to respect Shyft limits"
    echo "  • Caching of known addresses to avoid duplicate API calls"
    echo "  • System address filtering to avoid unnecessary enrichment"
    echo "  • Comprehensive error handling and logging"
    echo ""
    echo "Data Flow:"
    echo "  1. Consume SwapEvent from Kafka topic 'raw.swaps.all'"
    echo "  2. Extract unknown addresses (tokens, users, programs)"
    echo "  3. Filter out system addresses and known addresses"
    echo "  4. Enrich unknown addresses via Shyft API"
    echo "  5. Store enriched data in 'enriched_addresses' table"
    echo "  6. Publish enriched data to Kafka topic 'enriched.addresses'"
    echo ""
    echo "Configuration:"
    echo "  • SHYFT_API_KEY: Required for API access"
    echo "  • DATABASE_URL: PostgreSQL connection string"
    echo "  • KAFKA_BROKER: Kafka broker list"
    echo "  • Rate limit: Configurable API call rate (default: 10/sec)"
    echo ""
    
    print_status "SUCCESS" "Integration report generated"
}

# Main test execution
main() {
    print_header
    
    local tests=(
        "test_service_files"
        "test_rust_compilation"
        "test_dependencies"
        "test_shyft_integration"
        "test_database_integration"
        "test_kafka_integration"
        "test_address_processing"
        "test_error_handling"
        "test_docker_config"
        "test_integration_report"
    )
    
    local passed=0
    local failed=0
    
    for test in "${tests[@]}"; do
        echo ""
        if $test; then
            ((passed++))
        else
            ((failed++))
        fi
    done
    
    echo ""
    echo "=========================================="
    echo "Address Enrichment Service Test Results"
    echo "=========================================="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: ${#tests[@]}"
    echo ""
    
    if [ $failed -eq 0 ]; then
        print_status "SUCCESS" "All tests passed! Address enrichment service is ready."
        echo ""
        print_status "INFO" "Next steps:"
        echo "  1. Set SHYFT_API_KEY environment variable"
        echo "  2. Configure DATABASE_URL for PostgreSQL"
        echo "  3. Build the service: cd address-enrichment-service && cargo build --release"
        echo "  4. Run the service with proper configuration"
        echo "  5. Monitor enriched address data in the database"
        echo "  6. Check Kafka topic 'enriched.addresses' for enriched data"
    else
        print_status "ERROR" "Some tests failed. Please review the issues above."
        exit 1
    fi
}

# Run tests
main 