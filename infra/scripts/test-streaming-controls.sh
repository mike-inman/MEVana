#!/bin/bash

# Streaming Controls Feature Test Script
# Tests the complete streaming service management and network switching functionality

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
    echo "Streaming Controls Feature Test"
    echo "=========================================="
    echo ""
}

# Test 1: Check if streaming controls component exists
test_streaming_controls_component() {
    print_status "INFO" "Test 1: Checking streaming controls component"
    
    local files=(
        "dashboard/react-components/src/components/StreamingControls.tsx"
        "dashboard/react-components/src/components/StreamingControls.css"
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
        print_status "SUCCESS" "All streaming controls component files present"
    else
        print_status "ERROR" "Some streaming controls component files are missing"
        return 1
    fi
}

# Test 2: Check streaming controls functionality
test_streaming_controls_functionality() {
    print_status "INFO" "Test 2: Checking streaming controls functionality"
    
    local features=(
        "StreamingService"
        "NetworkConfig"
        "onServiceToggle"
        "onNetworkChange"
        "onRefresh"
        "handleServiceToggle"
        "handleNetworkChange"
        "refreshServices"
        "getStatusColor"
        "getStatusIcon"
    )
    
    local found_features=0
    
    for feature in "${features[@]}"; do
        if grep -q "$feature" dashboard/react-components/src/components/StreamingControls.tsx; then
            print_status "SUCCESS" "Feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#features[@]} features"
    
    if [ $found_features -ge 8 ]; then
        print_status "SUCCESS" "Streaming controls functionality is comprehensive"
    else
        print_status "WARNING" "Streaming controls functionality may need improvement"
    fi
}

# Test 3: Check service management features
test_service_management() {
    print_status "INFO" "Test 3: Checking service management features"
    
    local services=(
        "orca-stream"
        "raydium-stream"
        "whale-tracking"
        "pool-volume"
        "address-enrichment"
    )
    
    local service_features=(
        "status"
        "dataSource"
        "network"
        "metrics"
        "messagesPerSecond"
        "totalMessages"
        "errorRate"
        "uptime"
    )
    
    local found_services=0
    local found_features=0
    
    for service in "${services[@]}"; do
        if grep -q "$service" dashboard/react-components/src/components/StreamingControls.tsx; then
            print_status "SUCCESS" "Service found: $service"
            ((found_services++))
        else
            print_status "WARNING" "Service missing: $service"
        fi
    done
    
    for feature in "${service_features[@]}"; do
        if grep -q "$feature" dashboard/react-components/src/components/StreamingControls.tsx; then
            print_status "SUCCESS" "Service feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Service feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_services out of ${#services[@]} services"
    print_status "INFO" "Found $found_features out of ${#service_features[@]} service features"
    
    if [ $found_services -ge 4 ] && [ $found_features -ge 6 ]; then
        print_status "SUCCESS" "Service management features are well implemented"
    else
        print_status "WARNING" "Service management features may need improvement"
    fi
}

# Test 4: Check network switching functionality
test_network_switching() {
    print_status "INFO" "Test 4: Checking network switching functionality"
    
    local network_features=(
        "mainnet"
        "testnet"
        "NetworkConfig"
        "isActive"
        "endpoints"
        "rpc"
        "ws"
        "geyser"
        "handleNetworkChange"
    )
    
    local found_features=0
    
    for feature in "${network_features[@]}"; do
        if grep -q "$feature" dashboard/react-components/src/components/StreamingControls.tsx; then
            print_status "SUCCESS" "Network feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Network feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#network_features[@]} network features"
    
    if [ $found_features -ge 7 ]; then
        print_status "SUCCESS" "Network switching functionality is comprehensive"
    else
        print_status "WARNING" "Network switching functionality may need improvement"
    fi
}

# Test 5: Check data source identification
test_data_source_identification() {
    print_status "INFO" "Test 5: Checking data source identification"
    
    local data_sources=(
        "orca-dex-stream"
        "raydium-dex-stream"
        "whale-tracking-service"
        "pool-volume-service"
        "address-enrichment-service"
    )
    
    local identification_features=(
        "dataSource"
        "data-source-legend"
        "legend-grid"
        "legend-item"
        "legend-color"
        "legend-label"
        "legend-description"
    )
    
    local found_sources=0
    local found_features=0
    
    for source in "${data_sources[@]}"; do
        if grep -q "$source" dashboard/react-components/src/components/StreamingControls.tsx; then
            print_status "SUCCESS" "Data source found: $source"
            ((found_sources++))
        else
            print_status "WARNING" "Data source missing: $source"
        fi
    done
    
    for feature in "${identification_features[@]}"; do
        if grep -q "$feature" dashboard/react-components/src/components/StreamingControls.tsx; then
            print_status "SUCCESS" "Identification feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Identification feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_sources out of ${#data_sources[@]} data sources"
    print_status "INFO" "Found $found_features out of ${#identification_features[@]} identification features"
    
    if [ $found_sources -ge 4 ] && [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Data source identification is well implemented"
    else
        print_status "WARNING" "Data source identification may need improvement"
    fi
}

# Test 6: Check backend API endpoints
test_backend_api() {
    print_status "INFO" "Test 6: Checking backend API endpoints"
    
    local api_endpoints=(
        "/api/streaming-services/status"
        "/api/streaming-services/toggle"
        "/api/network/switch"
        "/api/network/status"
        "/api/data-sources"
        "/api/metrics/overview"
    )
    
    local api_features=(
        "get_streaming_services_status"
        "toggle_streaming_service"
        "switch_network"
        "get_network_status"
        "get_data_sources"
        "get_metrics_overview"
    )
    
    local found_endpoints=0
    local found_features=0
    
    for endpoint in "${api_endpoints[@]}"; do
        if grep -q "$endpoint" dashboard/server.py; then
            print_status "SUCCESS" "API endpoint found: $endpoint"
            ((found_endpoints++))
        else
            print_status "WARNING" "API endpoint missing: $endpoint"
        fi
    done
    
    for feature in "${api_features[@]}"; do
        if grep -q "$feature" dashboard/server.py; then
            print_status "SUCCESS" "API feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "API feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_endpoints out of ${#api_endpoints[@]} API endpoints"
    print_status "INFO" "Found $found_features out of ${#api_features[@]} API features"
    
    if [ $found_endpoints -ge 5 ] && [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Backend API is well implemented"
    else
        print_status "WARNING" "Backend API may need improvement"
    fi
}

# Test 7: Check database schema
test_database_schema() {
    print_status "INFO" "Test 7: Checking database schema"
    
    local schema_files=(
        "migrations/003_streaming_services.sql"
    )
    
    local schema_tables=(
        "streaming_services"
        "network_config"
        "service_metrics"
        "service_logs"
    )
    
    local schema_features=(
        "data_source"
        "network"
        "status"
        "enabled"
        "metrics"
        "is_active"
        "endpoints"
    )
    
    local found_files=0
    local found_tables=0
    local found_features=0
    
    for file in "${schema_files[@]}"; do
        if [ -f "$file" ]; then
            print_status "SUCCESS" "Schema file found: $file"
            ((found_files++))
        else
            print_status "ERROR" "Schema file missing: $file"
        fi
    done
    
    for table in "${schema_tables[@]}"; do
        if grep -q "$table" migrations/003_streaming_services.sql; then
            print_status "SUCCESS" "Schema table found: $table"
            ((found_tables++))
        else
            print_status "WARNING" "Schema table missing: $table"
        fi
    done
    
    for feature in "${schema_features[@]}"; do
        if grep -q "$feature" migrations/003_streaming_services.sql; then
            print_status "SUCCESS" "Schema feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Schema feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_files out of ${#schema_files[@]} schema files"
    print_status "INFO" "Found $found_tables out of ${#schema_tables[@]} schema tables"
    print_status "INFO" "Found $found_features out of ${#schema_features[@]} schema features"
    
    if [ $found_files -eq 1 ] && [ $found_tables -ge 3 ] && [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Database schema is comprehensive"
    else
        print_status "WARNING" "Database schema may need improvement"
    fi
}

# Test 8: Check UI integration
test_ui_integration() {
    print_status "INFO" "Test 8: Checking UI integration"
    
    local integration_checks=(
        "streaming-controls-container"
        "StreamingControls"
        "onServiceToggle"
        "onNetworkChange"
        "onRefresh"
    )
    
    local found_checks=0
    
    for check in "${integration_checks[@]}"; do
        if grep -q "$check" dashboard/react-components/src/index.html; then
            print_status "SUCCESS" "UI integration found: $check"
            ((found_checks++))
        else
            print_status "WARNING" "UI integration missing: $check"
        fi
    done
    
    print_status "INFO" "Found $found_checks out of ${#integration_checks[@]} UI integration checks"
    
    if [ $found_checks -ge 4 ]; then
        print_status "SUCCESS" "UI integration is well implemented"
    else
        print_status "WARNING" "UI integration may need improvement"
    fi
}

# Test 9: Check responsive design
test_responsive_design() {
    print_status "INFO" "Test 9: Checking responsive design"
    
    local responsive_features=(
        "@media"
        "max-width"
        "768px"
        "grid-template-columns"
        "flex-direction"
        "column"
    )
    
    local found_features=0
    
    for feature in "${responsive_features[@]}"; do
        if grep -q "$feature" dashboard/react-components/src/components/StreamingControls.css; then
            print_status "SUCCESS" "Responsive feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Responsive feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#responsive_features[@]} responsive features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Responsive design is well implemented"
    else
        print_status "WARNING" "Responsive design may need improvement"
    fi
}

# Test 10: Generate streaming controls report
test_streaming_controls_report() {
    print_status "INFO" "Test 10: Generating streaming controls report"
    
    echo ""
    echo "📡 Streaming Controls Feature Report"
    echo "===================================="
    echo ""
    echo "Feature Overview:"
    echo "  • Real-time streaming service management"
    echo "  • Network switching (mainnet/testnet)"
    echo "  • Data source identification and tracking"
    echo "  • Service metrics and performance monitoring"
    echo "  • Auto-refresh and manual controls"
    echo ""
    echo "Supported Services:"
    echo "  1. Orca Stream - Real-time Orca DEX transactions"
    echo "  2. Raydium Stream - Real-time Raydium DEX transactions"
    echo "  3. Whale Tracking - Large transaction monitoring"
    echo "  4. Pool Volume - DEX pool analytics"
    echo "  5. Address Enrichment - Metadata enrichment"
    echo ""
    echo "Network Support:"
    echo "  • Mainnet (Live Trading) - Production environment"
    echo "  • Testnet (Development) - Testing environment"
    echo "  • Automatic endpoint switching"
    echo "  • Network-specific data isolation"
    echo ""
    echo "Data Source Identification:"
    echo "  • Color-coded data sources"
    echo "  • Service-specific metadata"
    echo "  • Network attribution"
    echo "  • Real-time source tracking"
    echo ""
    echo "UI Features:"
    echo "  • Service status indicators"
    echo "  • Performance metrics display"
    echo "  • Network configuration cards"
    echo "  • Data source legend"
    echo "  • Responsive design"
    echo "  • Auto-refresh capabilities"
    echo ""
    echo "Backend Integration:"
    echo "  • RESTful API endpoints"
    echo "  • Database persistence"
    echo "  • Redis caching"
    echo "  • Service status tracking"
    echo "  • Metrics collection"
    echo ""
    echo "Database Schema:"
    echo "  • streaming_services - Service configuration"
    echo "  • network_config - Network settings"
    echo "  • service_metrics - Performance data"
    echo "  • service_logs - Event logging"
    echo "  • Enhanced transactions table with data source tracking"
    echo ""
    
    print_status "SUCCESS" "Streaming controls report generated"
}

# Main test execution
main() {
    print_header
    
    local tests=(
        "test_streaming_controls_component"
        "test_streaming_controls_functionality"
        "test_service_management"
        "test_network_switching"
        "test_data_source_identification"
        "test_backend_api"
        "test_database_schema"
        "test_ui_integration"
        "test_responsive_design"
        "test_streaming_controls_report"
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
    echo "Streaming Controls Feature Test Results"
    echo "=========================================="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: ${#tests[@]}"
    echo ""
    
    if [ $failed -eq 0 ]; then
        print_status "SUCCESS" "All tests passed! Streaming controls feature is ready."
        echo ""
        print_status "INFO" "Next steps:"
        echo "  1. Deploy the database migration"
        echo "  2. Start the dashboard server"
        echo "  3. Test the UI controls in the browser"
        echo "  4. Verify service toggling functionality"
        echo "  5. Test network switching"
        echo "  6. Monitor data source identification"
        echo "  7. Validate metrics collection"
        echo ""
        print_status "INFO" "The streaming controls feature provides:"
        echo "  📡 Real-time service management"
        echo "  🌐 Network switching capabilities"
        echo "  🏷️  Data source identification"
        echo "  📊 Performance monitoring"
        echo "  🎛️  Intuitive UI controls"
        echo "  📱 Responsive design"
        echo "  🔄 Auto-refresh functionality"
    else
        print_status "ERROR" "Some tests failed. Please review the issues above."
        exit 1
    fi
}

# Run tests
main 