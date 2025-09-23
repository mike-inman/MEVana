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

# Default port configuration (override via MEVANA_* env vars when needed)
GRAFANA_PORT="${MEVANA_GRAFANA_PORT:-3000}"
PROMETHEUS_PORT="${MEVANA_PROMETHEUS_PORT:-9090}"
KAFKA_UI_PORT="${MEVANA_KAFKA_UI_PORT:-8084}"
ADMINER_PORT="${MEVANA_ADMINER_PORT:-8087}"
HOTPATH_METRICS_PORT="${MEVANA_HOTPATH_METRICS_PORT:-8088}"
HOTPATH_API_PORT="${MEVANA_HOTPATH_API_PORT:-8089}"
NORMALIZER_METRICS_PORT="${MEVANA_NORMALIZER_PORT:-9901}"
DISCOVERY_METRICS_PORT="${MEVANA_DISCOVERY_PORT:-9102}"
EXECUTOR_METRICS_PORT="${MEVANA_EXECUTOR_PORT:-9904}"

# Internal ports scraped by Prometheus (container network)
HOTPATH_INTERNAL_PORT="${MEVANA_HOTPATH_INTERNAL_PORT:-8088}"
NORMALIZER_INTERNAL_PORT="${MEVANA_NORMALIZER_INTERNAL_PORT:-8085}"
DISCOVERY_INTERNAL_PORT="${MEVANA_DISCOVERY_INTERNAL_PORT:-9102}"
EXECUTOR_INTERNAL_PORT="${MEVANA_EXECUTOR_INTERNAL_PORT:-9904}"

CURRENT_TEST_SEVERITY="critical"

total_tests=0
passed_tests=0
failed_tests=0

optional_tests=0
optional_passed=0
optional_failed=0

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

report_pass() {
    local message=$1
    print_status "pass" "$message"
    return 0
}

report_fail() {
    local message=$1
    local severity="${CURRENT_TEST_SEVERITY:-critical}"
    if [ "$severity" = "optional" ]; then
        print_status "warn" "$message (optional check)"
    else
        print_status "fail" "$message"
    fi
    return 1
}

# Function to test HTTP endpoint
test_endpoint() {
    local url=$1
    local description=$2
    local expected_code=${3:-200}
    local attempts=${4:-3}
    local delay=${5:-5}
    local severity="${CURRENT_TEST_SEVERITY:-critical}"

    print_status "test" "Testing $description: $url"

    local response_code="000"
    local attempt=1
    while [ $attempt -le $attempts ]; do
        set +e
        response_code=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 10 --max-time 30 "$url")
        local curl_status=$?
        set -e
        if [ $curl_status -ne 0 ] || [ -z "$response_code" ]; then
            response_code="000"
        fi

        if [ "$response_code" = "$expected_code" ]; then
            print_status "pass" "$description is healthy (HTTP $response_code)"
            return 0
        fi

        if [ $attempt -lt $attempts ]; then
            sleep $delay
        fi
        attempt=$((attempt + 1))
    done

    if [ "$severity" = "optional" ]; then
        print_status "warn" "$description failed (HTTP $response_code, expected $expected_code) — optional check"
    else
        print_status "fail" "$description failed (HTTP $response_code, expected $expected_code)"
    fi
    return 1
}

# Function to test endpoint with content check
test_endpoint_content() {
    local url=$1
    local description=$2
    local expected_content=$3
    local attempts=${4:-3}
    local delay=${5:-5}
    local severity="${CURRENT_TEST_SEVERITY:-critical}"

    print_status "test" "Testing $description content: $url"

    local content=""
    local attempt=1
    local -a content_patterns
    while [ $attempt -le $attempts ]; do
        set +e
        content=$(curl -s --connect-timeout 10 --max-time 30 "$url")
        local curl_status=$?
        set -e
        if [ $curl_status -ne 0 ]; then
            content=""
        fi

        IFS='|' read -r -a content_patterns <<< "$expected_content"
        for pattern in "${content_patterns[@]}"; do
            if [ -n "$pattern" ] && [[ "$content" == *"$pattern"* ]]; then
                print_status "pass" "$description has expected content"
                return 0
            fi
        done

        if [ $attempt -lt $attempts ]; then
            sleep $delay
        fi
        attempt=$((attempt + 1))
    done

    if [ "$severity" = "optional" ]; then
        print_status "warn" "$description missing expected content: '$expected_content' — optional check"
    else
        print_status "fail" "$description missing expected content: '$expected_content'"
    fi
    printf '  Received: %.100s...\n' "$content"
    return 1
}

# Test runner helper that tracks critical vs optional checks
run_test() {
    local severity="critical"
    if [ "$1" = "optional" ]; then
        severity="optional"
        shift
    fi

    local test_name=$1
    shift

    local prev_severity="${CURRENT_TEST_SEVERITY:-critical}"
    CURRENT_TEST_SEVERITY="$severity"

    if [ "$severity" = "optional" ]; then
        optional_tests=$((optional_tests + 1))
        set +e
        "$@"
        local status=$?
        set -e
        CURRENT_TEST_SEVERITY="$prev_severity"
        if [ $status -eq 0 ]; then
            optional_passed=$((optional_passed + 1))
        else
            optional_failed=$((optional_failed + 1))
        fi
    else
        total_tests=$((total_tests + 1))
        set +e
        "$@"
        local status=$?
        set -e
        CURRENT_TEST_SEVERITY="$prev_severity"
        if [ $status -eq 0 ]; then
            passed_tests=$((passed_tests + 1))
        else
            failed_tests=$((failed_tests + 1))
        fi
    fi

    return 0
}

check_prometheus_target() {
    local target=$1
    local health=$2

    if [ "$health" = "up" ]; then
        report_pass "Target $target is UP"
    else
        report_fail "Target $target is DOWN or missing (status: ${health:-unknown})"
    fi
}

escape_regex() {
    printf '%s' "$1" | sed 's/[.[\()*^$?+{}|\\]/\\&/g'
}

# Determine if we're running locally or on server
if curl -s --connect-timeout 2 "http://localhost:${GRAFANA_PORT}/api/health" >/dev/null 2>&1; then
    SERVER_BASE="http://localhost"
    print_status "info" "Running diagnostics on local/server environment"
else
    print_status "info" "No local services detected - use this script on the deployment server"
    exit 1
fi

echo -e "\n${BLUE}🏗️ Phase 1: Infrastructure Services${NC}"
echo "==================================="

# Test core infrastructure
run_test "Grafana Health" test_endpoint "${SERVER_BASE}:${GRAFANA_PORT}/api/health" "Grafana health endpoint" 200 5 5
run_test "Prometheus Health" test_endpoint "${SERVER_BASE}:${PROMETHEUS_PORT}/-/healthy" "Prometheus health endpoint" 200 5 5
run_test optional "Kafka UI Health" test_endpoint "${SERVER_BASE}:${KAFKA_UI_PORT}" "Kafka UI interface" 200 3 5
run_test optional "Adminer Health" test_endpoint "${SERVER_BASE}:${ADMINER_PORT}" "Adminer database interface" 200 3 5

echo -e "\n${BLUE}🎯 Phase 2: MEVana Application Services${NC}"
echo "======================================="

# Test MEVana services metrics endpoints
run_test "Hotpath Ready" test_endpoint "${SERVER_BASE}:${HOTPATH_API_PORT}/health/ready" "Hotpath readiness endpoint" 200 12 10
run_test "Hotpath Metrics" test_endpoint_content "${SERVER_BASE}:${HOTPATH_METRICS_PORT}/metrics" "Hotpath metrics endpoint" "mevana_|hotpath_transactions_processed_total" 5 10
run_test "Normalizer Metrics" test_endpoint "${SERVER_BASE}:${NORMALIZER_METRICS_PORT}/metrics" "Normalizer metrics endpoint" 200 5 10
run_test "Discovery Metrics" test_endpoint "${SERVER_BASE}:${DISCOVERY_METRICS_PORT}/metrics" "Discovery metrics endpoint" 200 5 10

echo -e "\n${BLUE}📊 Phase 3: Prometheus Data Collection${NC}"
echo "======================================"

# Test Prometheus targets
print_status "test" "Checking Prometheus targets status"
if command -v jq >/dev/null 2>&1; then
    targets_json=$(curl -s "${SERVER_BASE}:${PROMETHEUS_PORT}/api/v1/targets" 2>/dev/null || echo '{"data":{"activeTargets":[]}}')

    # Check key targets based on current scrape configuration
    key_targets=(
        "hotpath-arb-engine:${HOTPATH_INTERNAL_PORT}"
        "normalizer-service:${NORMALIZER_INTERNAL_PORT}"
        "discovery-service:${DISCOVERY_INTERNAL_PORT}"
        "executor-service:${EXECUTOR_INTERNAL_PORT}"
    )

    for target in "${key_targets[@]}"; do
        target_health=$(echo "$targets_json" | jq -r ".data.activeTargets[] | select(.labels.instance == \"$target\") | .health" 2>/dev/null)
        if [ -z "$target_health" ] || [ "$target_health" = "null" ]; then
            target_health="missing"
        fi
        run_test "Prometheus Target $target" check_prometheus_target "$target" "$target_health"
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
grafana_ds_test=$(curl -s "${SERVER_BASE}:${GRAFANA_PORT}/api/datasources/proxy/1/api/v1/query?query=up" 2>/dev/null || echo "")

if [[ "$grafana_ds_test" == *'"status":"success"'* ]]; then
    run_test "Grafana Datasource" report_pass "Grafana can query Prometheus successfully"
else
    run_test "Grafana Datasource" report_fail "Grafana cannot query Prometheus"
fi

# Test dashboard availability
print_status "test" "Checking Grafana dashboards"
dashboards_response=$(curl -s "${SERVER_BASE}:${GRAFANA_PORT}/api/search?type=dash-db" 2>/dev/null || echo "[]")

if command -v jq >/dev/null 2>&1; then
    if echo "$dashboards_response" | jq -e '.' >/dev/null 2>&1; then
        response_type=$(echo "$dashboards_response" | jq -r 'type' 2>/dev/null || echo "unknown")
        if [ "$response_type" = "array" ]; then
            dashboard_count=$(echo "$dashboards_response" | jq 'length' 2>/dev/null || echo "0")

            if [ "$dashboard_count" -gt 0 ]; then
                run_test "Grafana Dashboards" report_pass "$dashboard_count dashboards loaded successfully"

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
                run_test "Grafana Dashboards" report_fail "No dashboards loaded"
            fi
        else
            run_test "Grafana Dashboards" report_fail "Grafana API returned unexpected JSON type '$response_type' (expected array)"
        fi
    else
        run_test "Grafana Dashboards" report_fail "Grafana API returned invalid JSON (check authentication)"
    fi
else
    print_status "warn" "Cannot analyze dashboards without jq"
fi

echo -e "\n${BLUE}🔗 Phase 5: Service Discovery Verification${NC}"
echo "==========================================="

# Test container naming
print_status "test" "Verifying container names match expectations"

expected_services=(
    "prometheus"
    "grafana"
    "hotpath-arb-engine"
    "kafka-ui"
    "normalizer-service"
    "discovery-service"
)

project_name="${COMPOSE_PROJECT_NAME:-mevana}"
escaped_project=$(escape_regex "$project_name")

if command -v docker >/dev/null 2>&1; then
    running_containers=$(docker ps --format "{{.Names}}" 2>/dev/null || echo "")

    container_check_passed=0
    for service in "${expected_services[@]}"; do
        escaped_service=$(escape_regex "$service")
        pattern="^(${escaped_project}-${escaped_service}|${escaped_service})(-[0-9]+)?$"
        match=$(echo "$running_containers" | grep -E -m1 "$pattern" || true)

        if [ -n "$match" ]; then
            print_status "pass" "Container $match is running (service ${service})"
            container_check_passed=$((container_check_passed + 1))
        else
            print_status "fail" "Container for service ${service} not found (expected ${project_name}-${service}-*)"
        fi
    done

    run_test "Container Naming" [ $container_check_passed -eq ${#expected_services[@]} ]
else
    print_status "warn" "Docker not available for container verification"
    run_test "Container Naming" report_fail "Container verification skipped"
fi

echo -e "\n${BLUE}📈 Phase 6: Data Flow Validation${NC}"
echo "================================="

# Test that metrics are flowing from hotpath to Prometheus
print_status "test" "Verifying MEVana metrics are available in Prometheus"

if command -v jq >/dev/null 2>&1; then
    # Query for MEVana-specific metrics
    mevana_metrics=$(curl -s "${SERVER_BASE}:${PROMETHEUS_PORT}/api/v1/label/__name__/values" 2>/dev/null | jq -r '.data[]' 2>/dev/null | grep -c "mevana_" || echo "0")

    if [ "$mevana_metrics" -gt 0 ]; then
        run_test "MEVana Metrics" report_pass "$mevana_metrics MEVana-specific metrics found in Prometheus"
    else
        run_test "MEVana Metrics" report_fail "No MEVana-specific metrics found in Prometheus"
    fi

    # Test specific important metrics
    important_metrics=("up" "prometheus_build_info" "grafana_build_info")
    for metric in "${important_metrics[@]}"; do
        metric_data=$(curl -s "${SERVER_BASE}:${PROMETHEUS_PORT}/api/v1/query?query=${metric}" 2>/dev/null || echo "")
        if [[ "$metric_data" == *'"status":"success"'* ]] && [[ "$metric_data" == *'"result":'* ]]; then
            print_status "pass" "Metric '$metric' is available and queryable"
        else
            print_status "warn" "Metric '$metric' may not be available"
        fi
    done
else
    print_status "warn" "Cannot verify metrics without jq"
fi

echo -e "\n${BLUE}🏁 Diagnostic Summary${NC}"
echo "======================"

if [ $total_tests -gt 0 ]; then
    success_rate=$(( passed_tests * 100 / total_tests ))
else
    success_rate=100
fi

echo "Tests executed (critical): $total_tests"
echo "Passed: $passed_tests"
echo "Failed: $failed_tests"
echo "Success rate: ${success_rate}%"

if [ $optional_tests -gt 0 ]; then
    echo "Optional checks run: $optional_tests"
    echo "Optional checks passed: $optional_passed"
    echo "Optional checks with warnings: $optional_failed"
    if [ $optional_failed -gt 0 ]; then
        print_status "warn" "Optional checks reported issues. Review output above if observability tooling is required."
    fi
fi

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
