#!/bin/bash

# Quick health check for immediate deployment verification
# This script provides a fast overview of service status

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}⚡ MEVana Quick Health Check${NC}"
echo "============================"

# Function to test endpoint quickly
quick_test() {
    local url=$1
    local name=$2
    local timeout=${3:-5}

    printf "%-25s" "$name:"

    if curl -s --connect-timeout $timeout --max-time $timeout "$url" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ UP${NC}"
        return 0
    else
        echo -e "${RED}❌ DOWN${NC}"
        return 1
    fi
}

# Test infrastructure services
echo -e "\n${BLUE}Infrastructure Services:${NC}"
quick_test "http://localhost:3000/api/health" "Grafana"
quick_test "http://localhost:9090/-/healthy" "Prometheus"
quick_test "http://localhost:8080" "Kafka UI"

# Test MEVana services
echo -e "\n${BLUE}MEVana Services:${NC}"
quick_test "http://localhost:8088/metrics" "Hotpath Metrics"
quick_test "http://localhost:8085/metrics" "Normalizer Metrics" 3
quick_test "http://localhost:9102/metrics" "Discovery Metrics" 3

# Test Prometheus targets quickly
echo -e "\n${BLUE}Prometheus Targets:${NC}"
if command -v jq >/dev/null 2>&1; then
    targets=$(curl -s --connect-timeout 3 http://localhost:9090/api/v1/targets 2>/dev/null || echo '{}')
    up_count=$(echo "$targets" | jq '.data.activeTargets | map(select(.health == "up")) | length' 2>/dev/null || echo "0")
    total_count=$(echo "$targets" | jq '.data.activeTargets | length' 2>/dev/null || echo "0")

    if [ "$up_count" -gt 0 ]; then
        echo -e "Healthy targets: ${GREEN}${up_count}/${total_count}${NC}"
    else
        echo -e "Healthy targets: ${RED}${up_count}/${total_count}${NC}"
    fi
else
    echo "jq not available - install for detailed target info"
fi

echo -e "\n${BLUE}Quick URLs:${NC}"
echo "• Grafana:    http://localhost:3000 (admin/<ADMIN_PASSWORD>)"
echo "• Prometheus: http://localhost:9090"
echo "• Kafka UI:   http://localhost:8080"

echo -e "\n${BLUE}For detailed diagnostics, run:${NC}"
echo "./scripts/ci/post-deployment-diagnostics.sh"