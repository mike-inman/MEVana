#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 MEVana Post-Deployment Diagnostics${NC}"
echo "==========================================="
echo "Verifying Phase 1 fixes: service discovery, networking, and data flow"
echo ""

# Function to print status
print_status() {
    local status=$1
    local message=$2
    case "$status" in
        "pass") echo -e "${GREEN}✅ $message${NC}" ;;
        "warn") echo -e "${YELLOW}⚠️  $message${NC}" ;;
        "fail") echo -e "${RED}❌ $message${NC}" ;;
        "info") echo -e "${CYAN}ℹ️  $message${NC}" ;;
        "test") echo -e "${PURPLE}🧪 $message${NC}" ;;
    esac
}

# Function to test HTTP endpoint
test_endpoint() {
    local url=$1
    local description=$2
    local expected_code=${3:-200}

    print_status "test" "Testing $description: $url"

    local response_code
    response_code=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 10 --max-time 30 "$url" 2>/dev/null || echo "000")

    if [ "$response_code" = "$expected_code" ]; then
        print_status "pass" "$description is healthy (HTTP $response_code)"
        return 0
    else
        print_status "fail" "$description failed (HTTP $response_code, expected $expected_code)"
        return 1
    fi
}

# Function to test endpoint with content check
test_endpoint_content() {
    local url=$1
    local description=$2
    local expected_content=$3

    print_status "test" "Testing $description content: $url"

    local content
    content=$(curl -s --connect-timeout 10 --max-time 30 "$url" 2>/dev/null || echo "")

    if [[ "$content" == *"$expected_content"* ]]; then
        print_status "pass" "$description has expected content"
        return 0
    else
        print_status "fail" "$description missing expected content: '$expected_content'"
        echo "  Received: ${content:0:100}..."
        return 1
    fi
}

# Initialize counters
total_tests=0
passed_tests=0
failed_tests=0

run_test() {
    local test_name=$1
    shift
    total_tests=$((total_tests + 1))

    if "$@"; then
        passed_tests=$((passed_tests + 1))
    else
        failed_tests=$((failed_tests + 1))
    fi
}

# Determine if we're running locally or on server
if curl -s --connect-timeout 2 localhost:3000/api/health >/dev/null 2>&1; then
    SERVER_BASE="http://localhost"
    print_status "info" "Running diagnostics on local/server environment"
else
    print_status "info" "No local services detected - use this script on the deployment server"
    exit 1
fi

echo -e "\n${BLUE}🏗️ Phase 1: Infrastructure Services${NC}"
echo "==================================="

# Test core infrastructure
run_test "Grafana Health" test_endpoint "${SERVER_BASE}:3000/api/health" "Grafana health endpoint"
run_test "Prometheus Health" test_endpoint "${SERVER_BASE}:9090/-/healthy" "Prometheus health endpoint"
run_test "Kafka UI Health" test_endpoint "${SERVER_BASE}:8080" "Kafka UI interface"
run_test "Adminer Health" test_endpoint "${SERVER_BASE}:8082" "Adminer database interface"

echo -e "\n${BLUE}🎯 Phase 2: MEVana Application Services${NC}"
echo "======================================="

# Test MEVana services metrics endpoints
run_test "Hotpath Ready" test_endpoint "${SERVER_BASE}:8089/health/ready" "Hotpath readiness endpoint"
run_test "Hotpath Metrics" test_endpoint_content "${SERVER_BASE}:8088/metrics" "Hotpath metrics endpoint" "mevana_"
run_test "Normalizer Metrics" test_endpoint "${SERVER_BASE}:8085/metrics" "Normalizer metrics endpoint"
run_test "Discovery Metrics" test_endpoint "${SERVER_BASE}:9102/metrics" "Discovery metrics endpoint"

echo -e "\n${BLUE}📊 Phase 3: Prometheus Data Collection${NC}"
echo "======================================"

# Test Prometheus targets
print_status "test" "Checking Prometheus targets status"
if command -v jq >/dev/null 2>&1; then
    targets_json=$(curl -s "${SERVER_BASE}:9090/api/v1/targets" 2>/dev/null || echo '{"data":{"activeTargets":[]}}')

    # Check key targets
    key_targets=("mevana-hotpath:8088" "prometheus-service:9090" "grafana-service:3000" "kafka-ui-service:8080")

    for target in "${key_targets[@]}"; do
        target_health=$(echo "$targets_json" | jq -r ".data.activeTargets[] | select(.labels.instance == \"$target\") | .health" 2>/dev/null || echo "unknown")

        if [ "$target_health" = "up" ]; then
            run_test "Prometheus Target $target" print_status "pass" "Target $target is UP"
        else
            run_test "Prometheus Target $target" print_status "fail" "Target $target is DOWN or missing (status: $target_health)"
        fi
    done

    # Count healthy vs total targets
    total_targets=$(echo "$targets_json" | jq '.data.activeTargets | length' 2>/dev/null || echo "0")
    healthy_targets=$(echo "$targets_json" | jq '.data.activeTargets | map(select(.health == "up")) | length' 2>/dev/null || echo "0")

    print_status "info" "Prometheus targets: $healthy_targets/$total_targets healthy"
else
    print_status "warn" "jq not available - skipping detailed Prometheus target analysis"
fi

echo -e "\n${BLUE}🎨 Phase 4: Grafana Data Source & Dashboards${NC}"
echo "============================================="

# Test Grafana datasource connectivity
print_status "test" "Testing Grafana → Prometheus connectivity"
grafana_ds_test=$(curl -s "${SERVER_BASE}:3000/api/datasources/proxy/1/api/v1/query?query=up" 2>/dev/null || echo "")

if [[ "$grafana_ds_test" == *'"status":"success"'* ]]; then
    run_test "Grafana Datasource" print_status "pass" "Grafana can query Prometheus successfully"
else
    run_test "Grafana Datasource" print_status "fail" "Grafana cannot query Prometheus"
fi

# Test dashboard availability
print_status "test" "Checking Grafana dashboards"
dashboards_response=$(curl -s "${SERVER_BASE}:3000/api/search?type=dash-db" 2>/dev/null || echo "[]")

if command -v jq >/dev/null 2>&1; then
    dashboard_count=$(echo "$dashboards_response" | jq 'length' 2>/dev/null || echo "0")

    if [ "$dashboard_count" -gt 0 ]; then
        run_test "Grafana Dashboards" print_status "pass" "$dashboard_count dashboards loaded successfully"

        # Check for key dashboards
        key_dashboards=("Hotpath" "Infrastructure" "Trading")
        for dash_keyword in "${key_dashboards[@]}"; do
            if echo "$dashboards_response" | jq -r '.[].title' | grep -qi "$dash_keyword" 2>/dev/null; then
                print_status "pass" "Found dashboard containing '$dash_keyword'"
            else
                print_status "warn" "No dashboard found containing '$dash_keyword'"
            fi
        done
    else
        run_test "Grafana Dashboards" print_status "fail" "No dashboards loaded"
    fi
else
    run_test "Grafana Dashboards" print_status "warn" "Cannot analyze dashboards without jq"
fi

echo -e "\n${BLUE}🔗 Phase 5: Service Discovery Verification${NC}"
echo "==========================================="

# Test container naming
print_status "test" "Verifying container names match expectations"

expected_containers=(
    "prometheus-service"
    "grafana-service"
    "mevana-hotpath"
    "kafka-ui-service"
    "mevana-normalizer"
    "mevana-discovery"
)

if command -v docker >/dev/null 2>&1; then
    running_containers=$(docker ps --format "{{.Names}}" 2>/dev/null || echo "")

    container_check_passed=0
    for container in "${expected_containers[@]}"; do
        if echo "$running_containers" | grep -q "^${container}$"; then
            print_status "pass" "Container $container is running with correct name"
            container_check_passed=$((container_check_passed + 1))
        else
            print_status "fail" "Container $container not found or incorrect name"
        fi
    done

    run_test "Container Naming" [ $container_check_passed -eq ${#expected_containers[@]} ]
else
    run_test "Container Naming" print_status "warn" "Docker not available for container verification"
fi

echo -e "\n${BLUE}📈 Phase 6: Data Flow Validation${NC}"
echo "================================="

# Test that metrics are flowing from hotpath to Prometheus
print_status "test" "Verifying MEVana metrics are available in Prometheus"

if command -v jq >/dev/null 2>&1; then
    # Query for MEVana-specific metrics
    mevana_metrics=$(curl -s "${SERVER_BASE}:9090/api/v1/label/__name__/values" 2>/dev/null | jq -r '.data[]' 2>/dev/null | grep -c "mevana_" || echo "0")

    if [ "$mevana_metrics" -gt 0 ]; then
        run_test "MEVana Metrics" print_status "pass" "$mevana_metrics MEVana-specific metrics found in Prometheus"
    else
        run_test "MEVana Metrics" print_status "fail" "No MEVana-specific metrics found in Prometheus"
    fi

    # Test specific important metrics
    important_metrics=("up" "prometheus_build_info" "grafana_build_info")
    for metric in "${important_metrics[@]}"; do
        metric_data=$(curl -s "${SERVER_BASE}:9090/api/v1/query?query=${metric}" 2>/dev/null || echo "")
        if [[ "$metric_data" == *'"status":"success"'* ]] && [[ "$metric_data" == *'"result":'* ]]; then
            print_status "pass" "Metric '$metric' is available and queryable"
        else
            print_status "warn" "Metric '$metric' may not be available"
        fi
    done
else
    run_test "MEVana Metrics" print_status "warn" "Cannot verify metrics without jq"
fi

echo -e "\n${BLUE}🏁 Diagnostic Summary${NC}"
echo "======================"

success_rate=$(( passed_tests * 100 / total_tests ))

echo "Tests executed: $total_tests"
echo "Passed: $passed_tests"
echo "Failed: $failed_tests"
echo "Success rate: ${success_rate}%"

if [ $failed_tests -eq 0 ]; then
    print_status "pass" "All diagnostics passed! Phase 1 deployment is successful."
    echo ""
    echo "🎯 Next steps:"
    echo "  • Visit Grafana: ${SERVER_BASE}:3000 (admin/admin123)"
    echo "  • Check Prometheus: ${SERVER_BASE}:9090"
    echo "  • Verify dashboards are loading data"
    exit 0
elif [ $success_rate -ge 80 ]; then
    print_status "warn" "Most diagnostics passed with some warnings. Phase 1 deployment mostly successful."
    echo ""
    echo "⚠️  Review failed tests above and consider fixes"
    exit 1
else
    print_status "fail" "Multiple diagnostics failed. Phase 1 deployment needs attention."
    echo ""
    echo "❌ Critical issues detected - review all failed tests"
    exit 2
fi
