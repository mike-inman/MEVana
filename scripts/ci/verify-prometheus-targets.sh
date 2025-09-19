#!/bin/bash

# Prometheus targets verification script
# Specifically validates the Phase 1 service discovery fixes

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🎯 Prometheus Targets Verification${NC}"
echo "==================================="

PROMETHEUS_URL="http://localhost:9090"

# Check if Prometheus is available
if ! curl -s --connect-timeout 5 "${PROMETHEUS_URL}/-/healthy" >/dev/null 2>&1; then
    echo -e "${RED}❌ Prometheus is not accessible at $PROMETHEUS_URL${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Prometheus is accessible${NC}"

# Check if jq is available
if ! command -v jq >/dev/null 2>&1; then
    echo -e "${RED}❌ jq is required for this script${NC}"
    exit 1
fi

# Fetch targets
echo -e "\n${BLUE}Fetching targets data...${NC}"
TARGETS_JSON=$(curl -s "${PROMETHEUS_URL}/api/v1/targets")

if [ $? -ne 0 ]; then
    echo -e "${RED}❌ Failed to fetch targets from Prometheus${NC}"
    exit 1
fi

# Parse targets
echo -e "\n${BLUE}📊 Target Status Analysis:${NC}"

# Expected targets from our Phase 1 fixes
EXPECTED_TARGETS=(
    "prometheus-service:9090:prometheus"
    "grafana-service:3000:grafana"
    "kafka-ui-service:8080:kafka-ui"
    "mevana-hotpath:8088:hotpath-arb-engine"
    "mevana-normalizer:8085:normalizer-service"
    "mevana-discovery:9102:discovery-service"
)

echo ""
printf "%-30s %-15s %-15s %s\n" "TARGET" "STATUS" "JOB" "LAST ERROR"
echo "$(printf '%.0s-' {1..80})"

all_healthy=true

for expected in "${EXPECTED_TARGETS[@]}"; do
    IFS=':' read -r target_host target_port job_name <<< "$expected"
    target_instance="${target_host}:${target_port}"

    # Find this target in the JSON
    target_info=$(echo "$TARGETS_JSON" | jq -r "
        .data.activeTargets[] |
        select(.labels.instance == \"$target_instance\" and .labels.job == \"$job_name\") |
        \"\(.health)|\(.lastError // \"none\")\"
    " 2>/dev/null)

    if [ -n "$target_info" ]; then
        IFS='|' read -r health last_error <<< "$target_info"

        if [ "$health" = "up" ]; then
            printf "%-30s ${GREEN}%-15s${NC} %-15s %s\n" "$target_instance" "UP" "$job_name" "✅ healthy"
        else
            printf "%-30s ${RED}%-15s${NC} %-15s %s\n" "$target_instance" "DOWN" "$job_name" "$last_error"
            all_healthy=false
        fi
    else
        printf "%-30s ${RED}%-15s${NC} %-15s %s\n" "$target_instance" "MISSING" "$job_name" "❌ not configured"
        all_healthy=false
    fi
done

# Check for unexpected targets
echo -e "\n${BLUE}🔍 All Configured Targets:${NC}"
echo "$TARGETS_JSON" | jq -r '.data.activeTargets[] | "\(.labels.instance) (\(.labels.job)) - \(.health)"' | while read -r line; do
    if [[ "$line" == *"up"* ]]; then
        echo -e "${GREEN}✅${NC} $line"
    else
        echo -e "${RED}❌${NC} $line"
    fi
done

# Summary
echo -e "\n${BLUE}📋 Summary:${NC}"

total_targets=$(echo "$TARGETS_JSON" | jq '.data.activeTargets | length')
healthy_targets=$(echo "$TARGETS_JSON" | jq '.data.activeTargets | map(select(.health == "up")) | length')

echo "Total targets configured: $total_targets"
echo "Healthy targets: $healthy_targets"

if [ "$healthy_targets" -eq "$total_targets" ] && [ "$total_targets" -gt 0 ]; then
    echo -e "${GREEN}🎉 All targets are healthy!${NC}"
elif [ "$healthy_targets" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Some targets are unhealthy${NC}"
else
    echo -e "${RED}❌ No healthy targets found${NC}"
fi

# Phase 1 specific validation
echo -e "\n${BLUE}🔧 Phase 1 Validation:${NC}"

# Check if the key Phase 1 fix (mevana-hotpath) is working
hotpath_health=$(echo "$TARGETS_JSON" | jq -r '.data.activeTargets[] | select(.labels.instance == "mevana-hotpath:8088") | .health' 2>/dev/null)

if [ "$hotpath_health" = "up" ]; then
    echo -e "${GREEN}✅ Critical fix verified: mevana-hotpath:8088 is UP${NC}"
    echo "   This confirms Phase 1 service discovery fix is working!"
else
    echo -e "${RED}❌ Critical issue: mevana-hotpath:8088 is not UP${NC}"
    echo "   This suggests Phase 1 service discovery fix needs attention"
    all_healthy=false
fi

if $all_healthy; then
    echo -e "\n${GREEN}🎯 Phase 1 networking fixes are working correctly!${NC}"
    exit 0
else
    echo -e "\n${YELLOW}⚠️  Some targets need attention - check the details above${NC}"
    exit 1
fi