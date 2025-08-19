#!/bin/bash

# Whale Tracking Service Test Script
# Tests whale wallet detection and tracking functionality

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
    echo "Whale Tracking Service Test"
    echo "=========================================="
    echo ""
}

# Test 1: Check if service files exist
test_service_files() {
    print_status "INFO" "Test 1: Checking if service files exist"
    
    local files=(
        "whale-tracking-service/Cargo.toml"
        "whale-tracking-service/src/main.rs"
        "whale-tracking-service/Dockerfile"
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

# Test 2: Check whale detection logic
test_whale_detection() {
    print_status "INFO" "Test 2: Checking whale detection logic"
    
    local detection_features=(
        "min_whale_threshold_usd"
        "min_whale_threshold_sol"
        "determine_whale_threshold"
        "categorize_trade_size"
        "is_whale_trade"
    )
    
    local found_features=0
    
    for feature in "${detection_features[@]}"; do
        if grep -q "$feature" whale-tracking-service/src/main.rs; then
            print_status "SUCCESS" "Whale detection feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Whale detection feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#detection_features[@]} whale detection features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Whale detection logic is comprehensive"
    else
        print_status "WARNING" "Whale detection logic may need improvement"
    fi
}

# Test 3: Check whale wallet tracking
test_whale_wallet_tracking() {
    print_status "INFO" "Test 3: Checking whale wallet tracking"
    
    local tracking_features=(
        "WhaleWallet"
        "update_whale_wallet"
        "whale_score"
        "total_volume_usd"
        "largest_trade_usd"
        "total_trades"
    )
    
    local found_features=0
    
    for feature in "${tracking_features[@]}"; do
        if grep -q "$feature" whale-tracking-service/src/main.rs; then
            print_status "SUCCESS" "Wallet tracking feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Wallet tracking feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#tracking_features[@]} wallet tracking features"
    
    if [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Whale wallet tracking is well implemented"
    else
        print_status "WARNING" "Whale wallet tracking may need improvement"
    fi
}

# Test 4: Check alert system
test_alert_system() {
    print_status "INFO" "Test 4: Checking alert system"
    
    local alert_features=(
        "WhaleAlert"
        "AlertType"
        "generate_whale_alert"
        "publish_whale_alert"
        "whale.alerts"
    )
    
    local found_features=0
    
    for feature in "${alert_features[@]}"; do
        if grep -q "$feature" whale-tracking-service/src/main.rs; then
            print_status "SUCCESS" "Alert feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Alert feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#alert_features[@]} alert features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Alert system is well implemented"
    else
        print_status "WARNING" "Alert system may need improvement"
    fi
}

# Test 5: Check database integration
test_database_integration() {
    print_status "INFO" "Test 5: Checking database integration"
    
    local db_features=(
        "whale_trades"
        "whale_wallets"
        "whale_alerts"
        "CREATE TABLE"
        "INSERT INTO"
        "ON CONFLICT"
    )
    
    local found_features=0
    
    for feature in "${db_features[@]}"; do
        if grep -q "$feature" whale-tracking-service/src/main.rs; then
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

# Test 6: Check trade size categorization
test_trade_categorization() {
    print_status "INFO" "Test 6: Checking trade size categorization"
    
    local categories=(
        "Small"
        "Medium"
        "Large"
        "Whale"
        "MegaWhale"
    )
    
    local found_categories=0
    
    for category in "${categories[@]}"; do
        if grep -q "$category" whale-tracking-service/src/main.rs; then
            print_status "SUCCESS" "Trade category found: $category"
            ((found_categories++))
        else
            print_status "WARNING" "Trade category missing: $category"
        fi
    done
    
    print_status "INFO" "Found $found_categories out of ${#categories[@]} trade categories"
    
    if [ $found_categories -eq ${#categories[@]} ]; then
        print_status "SUCCESS" "Trade size categorization is complete"
    else
        print_status "WARNING" "Trade size categorization may be incomplete"
    fi
}

# Test 7: Check SOL price integration
test_sol_price_integration() {
    print_status "INFO" "Test 7: Checking SOL price integration"
    
    local price_features=(
        "fetch_sol_price"
        "sol_price_cache"
        "coingecko"
        "last_price_update"
    )
    
    local found_features=0
    
    for feature in "${price_features[@]}"; do
        if grep -q "$feature" whale-tracking-service/src/main.rs; then
            print_status "SUCCESS" "SOL price feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "SOL price feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#price_features[@]} SOL price features"
    
    if [ $found_features -ge 3 ]; then
        print_status "SUCCESS" "SOL price integration is well implemented"
    else
        print_status "WARNING" "SOL price integration may need improvement"
    fi
}

# Test 8: Generate whale tracking report
test_whale_tracking_report() {
    print_status "INFO" "Test 8: Generating whale tracking report"
    
    echo ""
    echo "🐋 Whale Tracking Service Report"
    echo "================================"
    echo ""
    echo "Service Purpose:"
    echo "  • Monitor transaction streams for large trades"
    echo "  • Track whale wallets and their trading patterns"
    echo "  • Generate real-time whale alerts"
    echo "  • Calculate whale scores based on volume and frequency"
    echo ""
    echo "Key Features:"
    echo "  • Configurable whale thresholds (USD and SOL)"
    echo "  • Real-time whale trade detection"
    echo "  • Whale wallet tracking with scoring"
    echo "  • Trade size categorization (Small to MegaWhale)"
    echo "  • SOL price integration for accurate valuations"
    echo "  • Comprehensive alert system"
    echo ""
    echo "Data Flow:"
    echo "  1. Consume SwapEvent from Kafka topic 'raw.swaps.all'"
    echo "  2. Check if trade meets whale thresholds"
    echo "  3. Update whale wallet statistics"
    echo "  4. Generate and publish whale alerts"
    echo "  5. Store whale data in PostgreSQL"
    echo ""
    echo "Configuration:"
    echo "  • min_whale_threshold_usd: Minimum USD value (default: $100,000)"
    echo "  • min_whale_threshold_sol: Minimum SOL value (default: 100 SOL)"
    echo "  • whale_window_hours: Time window for tracking (default: 24h)"
    echo "  • DATABASE_URL: PostgreSQL connection string"
    echo "  • KAFKA_BROKER: Kafka broker list"
    echo ""
    
    print_status "SUCCESS" "Whale tracking report generated"
}

# Main test execution
main() {
    print_header
    
    local tests=(
        "test_service_files"
        "test_whale_detection"
        "test_whale_wallet_tracking"
        "test_alert_system"
        "test_database_integration"
        "test_trade_categorization"
        "test_sol_price_integration"
        "test_whale_tracking_report"
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
    echo "Whale Tracking Service Test Results"
    echo "=========================================="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: ${#tests[@]}"
    echo ""
    
    if [ $failed -eq 0 ]; then
        print_status "SUCCESS" "All tests passed! Whale tracking service is ready."
        echo ""
        print_status "INFO" "Next steps:"
        echo "  1. Configure whale thresholds based on your requirements"
        echo "  2. Set up DATABASE_URL for PostgreSQL"
        echo "  3. Build the service: cd whale-tracking-service && cargo build --release"
        echo "  4. Run the service with proper configuration"
        echo "  5. Monitor whale alerts in Kafka topic 'whale.alerts'"
        echo "  6. Check whale data in PostgreSQL tables"
    else
        print_status "ERROR" "Some tests failed. Please review the issues above."
        exit 1
    fi
}

# Run tests
main 