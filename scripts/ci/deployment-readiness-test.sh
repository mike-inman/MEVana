#!/bin/bash
#
# Deployment Readiness Test Suite
# Tests server alignment with working local configuration
# Returns PASS/FAIL for deployment readiness
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
ALIGNMENT_ISSUES=()

echo -e "${CYAN}"
cat << "EOF"
╔══════════════════════════════════════════════════════════╗
║            🧪 DEPLOYMENT READINESS TESTS 🧪              ║
║        Server vs Local Configuration Alignment          ║
╚══════════════════════════════════════════════════════════╝
EOF
echo -e "${NC}"

echo -e "${BLUE}🕐 Test started: $(date)${NC}"
echo -e "${BLUE}📂 Working directory: $(pwd)${NC}"
echo ""

# Test execution function
run_test() {
    local test_name="$1"
    local test_command="$2"
    local fix_suggestion="$3"

    ((TOTAL_TESTS++))
    echo -en "${BLUE}🧪 TEST: $test_name... ${NC}"

    if eval "$test_command" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ PASS${NC}"
        ((PASSED_TESTS++))
        return 0
    else
        echo -e "${RED}❌ FAIL${NC}"
        ((FAILED_TESTS++))
        ALIGNMENT_ISSUES+=("$test_name: $fix_suggestion")
        return 1
    fi
}

# Critical file comparison test
compare_config_file() {
    local local_file="$1"
    local server_file="$2"
    local description="$3"

    if [ ! -f "$local_file" ]; then
        echo -e "${YELLOW}⚠️  Local file missing: $local_file${NC}"
        return 1
    fi

    if [ ! -f "$server_file" ]; then
        echo -e "${RED}❌ Server file missing: $server_file${NC}"
        return 1
    fi

    # Compare key content (not exact match due to timestamps, etc.)
    case "$local_file" in
        *"mevana-system-status.json")
            # Check for critical dashboard components
            if grep -q "hotpath_multi_venue_transactions" "$local_file" &&
               grep -q "hotpath_multi_venue_transactions" "$server_file"; then
                return 0
            else
                return 1
            fi
            ;;
        *"prometheus.yml")
            # Compare job count
            local_jobs=$(grep -c "job_name:" "$local_file" 2>/dev/null || echo "0")
            server_jobs=$(grep -c "job_name:" "$server_file" 2>/dev/null || echo "0")
            [ "$local_jobs" -eq "$server_jobs" ]
            ;;
        *"docker-compose.yml")
            # Check for correct volume paths
            grep -q "./infrastructure/grafana" "$server_file"
            ;;
        *)
            # Basic file existence is enough for others
            return 0
            ;;
    esac
}

echo -e "${CYAN}═══ DOCKER INFRASTRUCTURE TESTS ═══${NC}"

# Disable exit-on-error for tests (we want to run all tests)
set +e

# Test 1: Docker network exists
run_test "Docker mevana_net network" \
    "docker network ls | grep -q mevana_net" \
    "Create network: docker network create mevana_net"

# Test 2: Docker daemon running
run_test "Docker daemon running" \
    "docker ps >/dev/null 2>&1" \
    "Start Docker daemon"

echo ""
echo -e "${CYAN}═══ MONITORING INFRASTRUCTURE TESTS ═══${NC}"

# Test 3: Prometheus accessible
PROMETHEUS_PORTS=(9090 9091)
PROMETHEUS_FOUND=false
for port in "${PROMETHEUS_PORTS[@]}"; do
    if curl -s --connect-timeout 3 "http://localhost:$port/-/healthy" >/dev/null 2>&1; then
        PROMETHEUS_URL="http://localhost:$port"
        PROMETHEUS_FOUND=true
        break
    fi
done

run_test "Prometheus accessibility" \
    "$PROMETHEUS_FOUND" \
    "Start Prometheus container/service"

# Test 4: Grafana accessible
GRAFANA_PORTS=(3000 3300)
GRAFANA_FOUND=false
for port in "${GRAFANA_PORTS[@]}"; do
    if curl -s --connect-timeout 3 "http://localhost:$port/api/health" >/dev/null 2>&1; then
        GRAFANA_URL="http://localhost:$port"
        GRAFANA_FOUND=true
        break
    fi
done

run_test "Grafana accessibility" \
    "$GRAFANA_FOUND" \
    "Start Grafana container/service"

echo ""
echo -e "${CYAN}═══ CONFIGURATION ALIGNMENT TESTS ═══${NC}"

# Test 5: Critical configuration files exist
CONFIG_FILES=(
    "infrastructure/prometheus.yml"
    "infrastructure/grafana/datasources/prometheus.yml"
    "infrastructure/grafana/dashboards/curated/dashboard.yml"
    "infrastructure/grafana/dashboards/curated/mevana-system-status.json"
    "infrastructure/docker-compose.yml"
)

for config_file in "${CONFIG_FILES[@]}"; do
    run_test "Config file: $config_file" \
        "[ -f \"$config_file\" ]" \
        "Copy from working local: rsync -av local/$config_file server:$(pwd)/$config_file"
done

# Test 6: Docker Compose volume paths (critical fix)
run_test "Docker Compose volume paths" \
    "grep -q './infrastructure/grafana' infrastructure/docker-compose.yml" \
    "Fix volume paths: Change ./grafana/ to ./infrastructure/grafana/"

# Test 7: Working dashboard contains arbitrage metrics
run_test "Dashboard arbitrage metrics" \
    "grep -q 'hotpath_multi_venue_transactions' infrastructure/grafana/dashboards/curated/mevana-system-status.json" \
    "Copy working dashboard from local development"

echo ""
echo -e "${CYAN}═══ MEVANA SERVICES TESTS ═══${NC}"

# Test 8: Check if key MEVana ports are available or running
MEVANA_PORTS=(8088 8089 9901 9102 9904)
for port in "${MEVANA_PORTS[@]}"; do
    # Port is either free or responding (both are OK)
    if ! lsof -i:$port >/dev/null 2>&1; then
        # Port is free - good
        run_test "Port $port available" "true" "Port is free"
    else
        # Port is used - check if it's responding with metrics
        run_test "Port $port service responding" \
            "curl -s --connect-timeout 2 http://localhost:$port/metrics >/dev/null 2>&1" \
            "Check service on port $port - may need restart"
    fi
done

echo ""
echo -e "${CYAN}═══ MONITORING DATA FLOW TESTS ═══${NC}"

# Test 9: Prometheus can query targets (if accessible)
if [ "$PROMETHEUS_FOUND" = true ] && command -v jq >/dev/null 2>&1; then
    run_test "Prometheus targets responding" \
        "curl -s \"$PROMETHEUS_URL/api/v1/targets\" | jq -e '.data.activeTargets | length > 0' >/dev/null" \
        "Check Prometheus target configuration"

    run_test "Prometheus has MEVana targets" \
        "curl -s \"$PROMETHEUS_URL/api/v1/targets\" | jq -r '.data.activeTargets[].labels.job' | grep -E 'hotpath|arb|normalizer' >/dev/null" \
        "Configure MEVana service targets in prometheus.yml"
fi

# Test 10: Grafana datasource configuration (if accessible)
if [ "$GRAFANA_FOUND" = true ] && command -v jq >/dev/null 2>&1; then
    run_test "Grafana Prometheus datasource" \
        "curl -s \"$GRAFANA_URL/api/datasources\" | jq -e '.[] | select(.type == \"prometheus\")' >/dev/null" \
        "Configure Prometheus datasource in Grafana"

    run_test "Grafana dashboards loaded" \
        "curl -s \"$GRAFANA_URL/api/search?type=dash-db\" | jq -e 'length > 0' >/dev/null" \
        "Load dashboards into Grafana"
fi

echo ""
echo -e "${CYAN}═══ DEPLOYMENT READINESS SUMMARY ═══${NC}"

echo -e "${BLUE}📊 Test Results:${NC}"
# Re-enable exit-on-error for final logic
set -e

echo -e "${GREEN}✅ Passed: $PASSED_TESTS/$TOTAL_TESTS tests${NC}"
echo -e "${RED}❌ Failed: $FAILED_TESTS/$TOTAL_TESTS tests${NC}"

# Calculate readiness percentage
READINESS_PERCENTAGE=$((PASSED_TESTS * 100 / TOTAL_TESTS))
echo -e "${BLUE}📈 Deployment Readiness: $READINESS_PERCENTAGE%${NC}"

# Determine deployment readiness
if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}🎉 DEPLOYMENT READY! All tests passed!${NC}"
    echo -e "${GREEN}✅ Proceed to Phase 3: Deployment${NC}"
    exit 0
elif [ $READINESS_PERCENTAGE -ge 80 ]; then
    echo -e "${YELLOW}⚠️  DEPLOYMENT CAUTION: Most tests passed but some issues remain${NC}"
    echo -e "${YELLOW}💡 Consider fixing critical issues before deployment${NC}"
    exit 2
else
    echo -e "${RED}🚨 DEPLOYMENT NOT READY: Multiple critical issues found${NC}"
    echo -e "${RED}❌ Must complete Phase 2: Alignment before deployment${NC}"
    exit 1
fi

echo ""
echo -e "${CYAN}═══ ALIGNMENT ISSUES TO FIX ═══${NC}"
if [ ${#ALIGNMENT_ISSUES[@]} -gt 0 ]; then
    echo -e "${BLUE}🔧 Required fixes:${NC}"
    for issue in "${ALIGNMENT_ISSUES[@]}"; do
        echo -e "${YELLOW}   • $issue${NC}"
    done

    echo ""
    echo -e "${BLUE}🔄 After fixing issues, re-run this test:${NC}"
    echo -e "${CYAN}   ./scripts/ci/deployment-readiness-test.sh${NC}"
else
    echo -e "${GREEN}🎯 No alignment issues found!${NC}"
fi

echo ""
echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}🕐 Test completed: $(date)${NC}"
echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"