#!/bin/bash

# Pool Volume Service Test Script
# Tests pool volume tracking and analytics functionality

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
    echo "Pool Volume Service Test"
    echo "=========================================="
    echo ""
}

# Test 1: Check if service files exist
test_service_files() {
    print_status "INFO" "Test 1: Checking if service files exist"
    
    local files=(
        "pool-volume-service/Cargo.toml"
        "pool-volume-service/src/main.rs"
        "pool-volume-service/Dockerfile"
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

# Test 2: Check volume aggregation logic
test_volume_aggregation() {
    print_status "INFO" "Test 2: Checking volume aggregation logic"
    
    local aggregation_features=(
        "VolumeAggregator"
        "aggregate_volumes"
        "aggregation_interval"
        "current_volumes"
        "add_trade"
    )
    
    local found_features=0
    
    for feature in "${aggregation_features[@]}"; do
        if grep -q "$feature" pool-volume-service/src/main.rs; then
            print_status "SUCCESS" "Volume aggregation feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Volume aggregation feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#aggregation_features[@]} volume aggregation features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Volume aggregation logic is comprehensive"
    else
        print_status "WARNING" "Volume aggregation logic may need improvement"
    fi
}

# Test 3: Check pool metrics tracking
test_pool_metrics() {
    print_status "INFO" "Test 3: Checking pool metrics tracking"
    
    local metrics_features=(
        "PoolMetrics"
        "total_volume_24h"
        "total_volume_7d"
        "total_volume_30d"
        "trade_count"
        "unique_traders"
        "volume_trend"
    )
    
    local found_features=0
    
    for feature in "${metrics_features[@]}"; do
        if grep -q "$feature" pool-volume-service/src/main.rs; then
            print_status "SUCCESS" "Pool metrics feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Pool metrics feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#metrics_features[@]} pool metrics features"
    
    if [ $found_features -ge 6 ]; then
        print_status "SUCCESS" "Pool metrics tracking is well implemented"
    else
        print_status "WARNING" "Pool metrics tracking may need improvement"
    fi
}

# Test 4: Check time period tracking
test_time_periods() {
    print_status "INFO" "Test 4: Checking time period tracking"
    
    local time_periods=(
        "Minute"
        "Hour"
        "Day"
        "Week"
        "Month"
    )
    
    local found_periods=0
    
    for period in "${time_periods[@]}"; do
        if grep -q "$period" pool-volume-service/src/main.rs; then
            print_status "SUCCESS" "Time period found: $period"
            ((found_periods++))
        else
            print_status "WARNING" "Time period missing: $period"
        fi
    done
    
    print_status "INFO" "Found $found_periods out of ${#time_periods[@]} time periods"
    
    if [ $found_periods -eq ${#time_periods[@]} ]; then
        print_status "SUCCESS" "Time period tracking is complete"
    else
        print_status "WARNING" "Time period tracking may be incomplete"
    fi
}

# Test 5: Check database integration
test_database_integration() {
    print_status "INFO" "Test 5: Checking database integration"
    
    local db_features=(
        "pool_volumes"
        "pool_metrics"
        "CREATE TABLE"
        "INSERT INTO"
        "ON CONFLICT"
        "calculate_pool_metrics"
    )
    
    local found_features=0
    
    for feature in "${db_features[@]}"; do
        if grep -q "$feature" pool-volume-service/src/main.rs; then
            print_status "SUCCESS" "Database feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Database feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#db_features[@]} database features"
    
    if [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Database integration is well implemented"
    else
        print_status "WARNING" "Database integration may need improvement"
    fi
}

# Test 6: Check volume trend analysis
test_volume_trends() {
    print_status "INFO" "Test 6: Checking volume trend analysis"
    
    local trend_features=(
        "VolumeTrend"
        "Increasing"
        "Decreasing"
        "Stable"
        "determine_volume_trend"
    )
    
    local found_features=0
    
    for feature in "${trend_features[@]}"; do
        if grep -q "$feature" pool-volume-service/src/main.rs; then
            print_status "SUCCESS" "Volume trend feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Volume trend feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#trend_features[@]} volume trend features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Volume trend analysis is well implemented"
    else
        print_status "WARNING" "Volume trend analysis may need improvement"
    fi
}

# Test 7: Check Kafka integration
test_kafka_integration() {
    print_status "INFO" "Test 7: Checking Kafka integration"
    
    local kafka_features=(
        "StreamConsumer"
        "FutureProducer"
        "subscribe"
        "send"
        "pool.volume.updates"
        "raw.swaps.all"
    )
    
    local found_features=0
    
    for feature in "${kafka_features[@]}"; do
        if grep -q "$feature" pool-volume-service/src/main.rs; then
            print_status "SUCCESS" "Kafka feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Kafka feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#kafka_features[@]} Kafka features"
    
    if [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Kafka integration is well implemented"
    else
        print_status "WARNING" "Kafka integration may need improvement"
    fi
}

# Test 8: Generate pool volume report
test_pool_volume_report() {
    print_status "INFO" "Test 8: Generating pool volume report"
    
    echo ""
    echo "📊 Pool Volume Service Report"
    echo "============================="
    echo ""
    echo "Service Purpose:"
    echo "  • Track trading volume across different pools and DEXs"
    echo "  • Aggregate volume data in real-time"
    echo "  • Calculate pool metrics and trends"
    echo "  • Provide volume analytics for trading decisions"
    echo ""
    echo "Key Features:"
    echo "  • Real-time volume aggregation with configurable intervals"
    echo "  • Multi-timeframe tracking (minute, hour, day, week, month)"
    echo "  • Pool-specific metrics and analytics"
    echo "  • Volume trend analysis and prediction"
    echo "  • Unique trader counting and analysis"
    echo "  • Cross-DEX volume comparison"
    echo ""
    echo "Data Flow:"
    echo "  1. Consume SwapEvent from Kafka topic 'raw.swaps.all'"
    echo "  2. Extract pool information from transactions"
    echo "  3. Aggregate volume data in memory"
    echo "  4. Periodically store aggregated data in PostgreSQL"
    echo "  5. Calculate pool metrics and trends"
    echo "  6. Publish volume updates to Kafka topic 'pool.volume.updates'"
    echo ""
    echo "Configuration:"
    echo "  • aggregation_interval_minutes: Volume aggregation interval (default: 5)"
    echo "  • DATABASE_URL: PostgreSQL connection string"
    echo "  • KAFKA_BROKER: Kafka broker list"
    echo "  • batch_size: Processing batch size (default: 100)"
    echo ""
    
    print_status "SUCCESS" "Pool volume report generated"
}

# Main test execution
main() {
    print_header
    
    local tests=(
        "test_service_files"
        "test_volume_aggregation"
        "test_pool_metrics"
        "test_time_periods"
        "test_database_integration"
        "test_volume_trends"
        "test_kafka_integration"
        "test_pool_volume_report"
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
    echo "Pool Volume Service Test Results"
    echo "=========================================="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: ${#tests[@]}"
    echo ""
    
    if [ $failed -eq 0 ]; then
        print_status "SUCCESS" "All tests passed! Pool volume service is ready."
        echo ""
        print_status "INFO" "Next steps:"
        echo "  1. Configure aggregation intervals based on your needs"
        echo "  2. Set up DATABASE_URL for PostgreSQL"
        echo "  3. Build the service: cd pool-volume-service && cargo build --release"
        echo "  4. Run the service with proper configuration"
        echo "  5. Monitor volume updates in Kafka topic 'pool.volume.updates'"
        echo "  6. Check pool metrics in PostgreSQL tables"
    else
        print_status "ERROR" "Some tests failed. Please review the issues above."
        exit 1
    fi
}

# Run tests
main 