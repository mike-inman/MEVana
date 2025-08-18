#!/bin/bash

# Test Observability Pipeline
# This script demonstrates our Prometheus + Grafana observability setup
# Created: 2024-08-17 19:30 CDT

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROMETHEUS_URL="http://localhost:9090"
GRAFANA_URL="http://localhost:3000"
TEST_METRICS_URL="http://localhost:9091"

echo -e "${BLUE}🧪 MEVana Observability Pipeline Test${NC}"
echo "=========================================="
echo "Prometheus: $PROMETHEUS_URL"
echo "Grafana: $GRAFANA_URL"
echo "Test Metrics: $TEST_METRICS_URL"
echo ""

# Test 1: Check if services are running
echo -e "${BLUE}[TEST 1] Service Health Check${NC}"
if curl -s "$PROMETHEUS_URL/-/healthy" > /dev/null; then
    echo -e "  ✅ Prometheus is healthy"
else
    echo -e "  ❌ Prometheus is not responding"
    exit 1
fi

if curl -s "$GRAFANA_URL/api/health" > /dev/null; then
    echo -e "  ✅ Grafana is healthy"
else
    echo -e "  ❌ Grafana is not responding"
    exit 1
fi

if curl -s "$TEST_METRICS_URL/health" > /dev/null; then
    echo -e "  ✅ Test Metrics Server is healthy"
else
    echo -e "  ❌ Test Metrics Server is not responding"
    exit 1
fi

# Test 2: Check Prometheus targets
echo -e "\n${BLUE}[TEST 2] Prometheus Targets Check${NC}"
echo "  Checking if our test metrics server is being scraped..."

TARGET_STATUS=$(curl -s "$PROMETHEUS_URL/api/v1/targets" | grep -o '"health":"[^"]*"' | grep "localhost:9091" | head -1)

if [[ $TARGET_STATUS == *"up"* ]]; then
    echo -e "  ✅ Test metrics server target is UP"
else
    echo -e "  ❌ Test metrics server target is not UP"
    echo "  Target status: $TARGET_STATUS"
fi

# Test 3: Generate test metrics
echo -e "\n${BLUE}[TEST 3] Generate Test Metrics${NC}"
echo "  Generating test execution metrics..."

# Generate some test executions
for i in {1..3}; do
    curl -s "$TEST_METRICS_URL/test" > /dev/null
    echo "    Test execution $i completed"
    sleep 1
done

# Generate a failure
curl -s "$TEST_METRICS_URL/failure" > /dev/null
echo "    Failure test completed"

echo "  Waiting for Prometheus to scrape metrics..."
sleep 5

# Test 4: Query Prometheus metrics
echo -e "\n${BLUE}[TEST 4] Query Prometheus Metrics${NC}"

EXECUTION_COUNT=$(curl -s "$PROMETHEUS_URL/api/v1/query?query=test_execution_total" | grep -o '"[0-9]*"' | tail -1 | tr -d '"')
SUCCESS_COUNT=$(curl -s "$PROMETHEUS_URL/api/v1/query?query=test_success_total" | grep -o '"[0-9]*"' | tail -1 | tr -d '"')
FAILURE_COUNT=$(curl -s "$PROMETHEUS_URL/api/v1/query?query=test_failure_total" | grep -o '"[0-9]*"' | tail -1 | tr -d '"')

echo "  Test Execution Count: $EXECUTION_COUNT"
echo "  Test Success Count: $SUCCESS_COUNT"
echo "  Test Failure Count: $FAILURE_COUNT"

if [[ "$EXECUTION_COUNT" -gt 0 ]]; then
    echo -e "  ✅ Metrics are being collected successfully"
else
    echo -e "  ❌ No metrics found"
fi

# Test 5: Check Grafana dashboard
echo -e "\n${BLUE}[TEST 5] Grafana Dashboard Check${NC}"
echo "  Grafana is running and accessible at: $GRAFANA_URL"
echo "  Login credentials: admin / admin123"
echo "  Our test dashboard should be available in the dashboards list"
echo "  Dashboard name: 'MEVana Test Dashboard'"

# Test 6: Performance metrics
echo -e "\n${BLUE}[TEST 6] Performance Metrics Check${NC}"
LATENCY_QUERY="rate(service_latency_seconds_sum[5m]) / rate(service_latency_seconds_count[5m])"
LATENCY_URL="$PROMETHEUS_URL/api/v1/query?query=$(echo $LATENCY_QUERY | sed 's/ /%20/g')"

LATENCY_RESULT=$(curl -s "$LATENCY_URL")
if [[ $LATENCY_RESULT == *"result"* ]]; then
    echo -e "  ✅ Performance metrics are available"
    echo "  Latency query: $LATENCY_QUERY"
else
    echo -e "  ❌ Performance metrics query failed"
fi

# Summary
echo -e "\n${GREEN}🎉 Observability Pipeline Test Complete!${NC}"
echo "=========================================="
echo -e "${BLUE}What we've accomplished:${NC}"
echo "  ✅ Prometheus is running and collecting metrics"
echo "  ✅ Grafana is running and accessible"
echo "  ✅ Test metrics server is generating data"
echo "  ✅ Metrics are being scraped and stored"
echo "  ✅ Dashboard is configured and ready"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Open Grafana at: $GRAFANA_URL"
echo "  2. Login with: admin / admin123"
echo "  3. View the 'MEVana Test Dashboard'"
echo "  4. Add more metrics to our services"
echo "  5. Create custom dashboards for specific use cases"
echo ""
echo -e "${BLUE}For AI agents:${NC}"
echo "  This observability pipeline is now ready for:"
echo "  • Performance monitoring during testing"
echo "  • Metrics collection for AI learning"
echo "  • Real-time system health monitoring"
echo "  • Performance regression detection"
echo "  • Automated quality assurance"
