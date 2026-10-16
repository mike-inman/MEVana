#!/bin/bash

# Grafana dashboard validation script
# Validates Phase 1 dashboard fixes and data connectivity

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}📊 Grafana Dashboard Validation${NC}"
echo "================================="

GRAFANA_URL="http://localhost:3000"
GRAFANA_API="${GRAFANA_URL}/api"

# Test Grafana availability
echo -e "${BLUE}Testing Grafana availability...${NC}"
if ! curl -s --connect-timeout 5 "${GRAFANA_URL}/api/health" >/dev/null 2>&1; then
    echo -e "${RED}❌ Grafana is not accessible at $GRAFANA_URL${NC}"
    echo "Ensure Grafana is running and accessible"
    exit 1
fi

echo -e "${GREEN}✅ Grafana is accessible${NC}"

# Check if jq is available
if ! command -v jq >/dev/null 2>&1; then
    echo -e "${RED}❌ jq is required for this script${NC}"
    exit 1
fi

# Test anonymous access to API (basic endpoint)
echo -e "\n${BLUE}Testing Grafana API access...${NC}"

# Test datasources
echo -e "\n${BLUE}📡 Datasource Validation:${NC}"

# Test if the Prometheus datasource is configured correctly
datasources=$(curl -s "${GRAFANA_API}/datasources" 2>/dev/null || echo "[]")

prometheus_ds=$(echo "$datasources" | jq -r '.[] | select(.type == "prometheus") | .url' 2>/dev/null)

if [ -n "$prometheus_ds" ] && [ "$prometheus_ds" != "null" ]; then
    echo -e "${GREEN}✅ Prometheus datasource configured${NC}"
    echo "   URL: $prometheus_ds"

    # Check if the datasource URL matches our Phase 1 fix
    if [[ "$prometheus_ds" == *"prometheus-service:9090"* ]]; then
        echo -e "${GREEN}✅ Datasource uses correct service name (prometheus-service)${NC}"
    else
        echo -e "${YELLOW}⚠️  Datasource URL may not match Phase 1 fixes${NC}"
        echo "   Expected: prometheus-service:9090, Got: $prometheus_ds"
    fi
else
    echo -e "${RED}❌ No Prometheus datasource found${NC}"
fi

# Test datasource connectivity
echo -e "\n${BLUE}🔗 Testing Datasource Connectivity:${NC}"

# Test a simple query through Grafana's proxy
query_test=$(curl -s "${GRAFANA_API}/datasources/proxy/1/api/v1/query?query=up" 2>/dev/null || echo "{}")

if echo "$query_test" | jq -e '.status == "success"' >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Grafana can successfully query Prometheus${NC}"

    # Count how many "up" metrics we can see
    up_count=$(echo "$query_test" | jq '.data.result | length' 2>/dev/null || echo "0")
    echo "   Found $up_count 'up' metrics from monitored services"
else
    echo -e "${RED}❌ Grafana cannot query Prometheus${NC}"
    echo "   This indicates a datasource connectivity issue"
fi

# Test dashboard loading
echo -e "\n${BLUE}📊 Dashboard Validation:${NC}"

dashboards=$(curl -s "${GRAFANA_API}/search?type=dash-db" 2>/dev/null || echo "[]")
dashboard_count=$(echo "$dashboards" | jq 'length' 2>/dev/null || echo "0")

echo "Total dashboards found: $dashboard_count"

if [ "$dashboard_count" -gt 0 ]; then
    echo -e "${GREEN}✅ Dashboards are loaded${NC}"

    # Check for specific dashboard types that should exist
    expected_dashboard_keywords=("hotpath" "infrastructure" "arbitrage" "trading" "geyser")

    echo -e "\n${BLUE}Checking for key dashboard types:${NC}"
    for keyword in "${expected_dashboard_keywords[@]}"; do
        found_dash=$(echo "$dashboards" | jq -r ".[].title" | grep -i "$keyword" | head -1 || echo "")
        if [ -n "$found_dash" ]; then
            echo -e "${GREEN}✅ Found '$keyword' dashboard: $found_dash${NC}"
        else
            echo -e "${YELLOW}⚠️  No '$keyword' dashboard found${NC}"
        fi
    done

    # List all dashboards
    echo -e "\n${BLUE}All available dashboards:${NC}"
    echo "$dashboards" | jq -r '.[].title' | while read -r title; do
        echo "  • $title"
    done

else
    echo -e "${RED}❌ No dashboards found${NC}"
    echo "   This suggests dashboard provisioning failed"
fi

# Test specific dashboard access (if we can get UID)
echo -e "\n${BLUE}🧪 Testing Dashboard Data Loading:${NC}"

# Try to find a dashboard with MEVana metrics
hotpath_dashboard=$(echo "$dashboards" | jq -r '.[] | select(.title | test("(?i)hotpath|arb")) | .uid' | head -1 2>/dev/null || echo "")

if [ -n "$hotpath_dashboard" ] && [ "$hotpath_dashboard" != "null" ]; then
    echo "Testing dashboard: $hotpath_dashboard"

    # Test a basic dashboard query (this might not work without auth, but we can try)
    dashboard_test=$(curl -s "${GRAFANA_API}/dashboards/uid/${hotpath_dashboard}" 2>/dev/null || echo "{}")

    if echo "$dashboard_test" | jq -e '.dashboard.title' >/dev/null 2>&1; then
        dashboard_title=$(echo "$dashboard_test" | jq -r '.dashboard.title')
        echo -e "${GREEN}✅ Dashboard accessible: $dashboard_title${NC}"

        # Check if dashboard has panels with MEVana queries
        panel_count=$(echo "$dashboard_test" | jq '.dashboard.panels | length' 2>/dev/null || echo "0")
        echo "   Dashboard has $panel_count panels"
    else
        echo -e "${YELLOW}⚠️  Dashboard access limited (may require authentication)${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  No testable dashboard found${NC}"
fi

# Test if Grafana can see MEVana-specific metrics
echo -e "\n${BLUE}🎯 MEVana Metrics Availability Test:${NC}"

# Query for MEVana-specific metrics through Grafana
mevana_metrics_test=$(curl -s "${GRAFANA_API}/datasources/proxy/1/api/v1/label/__name__/values" 2>/dev/null || echo "{}")

if echo "$mevana_metrics_test" | jq -e '.status == "success"' >/dev/null 2>&1; then
    mevana_count=$(echo "$mevana_metrics_test" | jq -r '.data[]' | grep -c "mevana_" 2>/dev/null || echo "0")

    if [ "$mevana_count" -gt 0 ]; then
        echo -e "${GREEN}✅ Found $mevana_count MEVana-specific metrics${NC}"
        echo "   This confirms application metrics are flowing to Grafana"
    else
        echo -e "${YELLOW}⚠️  No MEVana-specific metrics found${NC}"
        echo "   Application metrics may not be flowing yet"
    fi
else
    echo -e "${RED}❌ Cannot query metrics through Grafana${NC}"
fi

# Summary
echo -e "\n${BLUE}📋 Validation Summary:${NC}"

issues=0

# Check key validations
if [ "$dashboard_count" -eq 0 ]; then
    echo -e "${RED}❌ Critical: No dashboards loaded${NC}"
    issues=$((issues + 1))
fi

if [ -z "$prometheus_ds" ] || [ "$prometheus_ds" = "null" ]; then
    echo -e "${RED}❌ Critical: No Prometheus datasource${NC}"
    issues=$((issues + 1))
fi

if ! echo "$query_test" | jq -e '.status == "success"' >/dev/null 2>&1; then
    echo -e "${RED}❌ Critical: Cannot query Prometheus through Grafana${NC}"
    issues=$((issues + 1))
fi

if [ $issues -eq 0 ]; then
    echo -e "${GREEN}🎉 All Grafana validations passed!${NC}"
    echo -e "${GREEN}Phase 1 dashboard fixes are working correctly${NC}"
    echo ""
    echo "🎯 Next steps:"
    echo "  • Visit Grafana: $GRAFANA_URL (admin/<ADMIN_PASSWORD>)"
    echo "  • Check that dashboards display data"
    echo "  • Verify MEVana metrics are visible in dashboard panels"
    exit 0
else
    echo -e "${RED}❌ $issues critical issues found${NC}"
    echo "Review the issues above before proceeding"
    exit 1
fi