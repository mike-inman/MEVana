#!/bin/bash

# MEVana Production Grafana Dashboard Setup
# Automatically configures Grafana with Prometheus data source and live trading dashboard

set -e

GRAFANA_URL="http://149.28.56.125:3000"
GRAFANA_USER="admin"
GRAFANA_PASS="${GRAFANA_PASS:?Set GRAFANA_PASS in the environment before running this script}"
PROMETHEUS_URL="http://prometheus:9090"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }

# Function to wait for Grafana to be ready
wait_for_grafana() {
    log_info "⏳ Waiting for Grafana to be ready..."

    local retries=30
    for i in $(seq 1 $retries); do
        if curl -s --connect-timeout 5 "${GRAFANA_URL}/api/health" > /dev/null 2>&1; then
            log_success "✅ Grafana is ready!"
            return 0
        else
            log_info "Waiting for Grafana... ($i/$retries)"
            sleep 5
        fi
    done

    log_error "❌ Grafana failed to become ready after $((retries * 5)) seconds"
    return 1
}

# Function to configure Prometheus data source
setup_prometheus_datasource() {
    log_info "🔗 Setting up Prometheus data source..."

    local datasource_payload=$(cat <<EOF
{
  "name": "Prometheus",
  "type": "prometheus",
  "url": "$PROMETHEUS_URL",
  "access": "proxy",
  "isDefault": true,
  "basicAuth": false,
  "jsonData": {
    "scrapeInterval": "5s",
    "queryTimeout": "60s",
    "timeInterval": "5s"
  }
}
EOF
    )

    local response=$(curl -s -w "%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -u "${GRAFANA_USER}:${GRAFANA_PASS}" \
        -d "$datasource_payload" \
        "${GRAFANA_URL}/api/datasources")

    local http_code="${response: -3}"

    if [[ "$http_code" == "200" || "$http_code" == "409" ]]; then
        log_success "✅ Prometheus data source configured"
        return 0
    else
        log_error "❌ Failed to setup data source. HTTP Code: $http_code"
        return 1
    fi
}

# Function to import live trading dashboard
import_dashboard() {
    log_info "📊 Importing MEVana Live Trading Dashboard..."

    local dashboard_json="/Users/michaelinman/Repos/MEVana/dashboards/live_trading_dashboard.json"

    if [[ ! -f "$dashboard_json" ]]; then
        log_error "❌ Dashboard file not found: $dashboard_json"
        return 1
    fi

    # Wrap dashboard JSON in import payload
    local dashboard_content=$(cat "$dashboard_json")
    local import_payload=$(cat <<EOF
{
  "dashboard": $dashboard_content,
  "overwrite": true,
  "inputs": [
    {
      "name": "DS_PROMETHEUS",
      "type": "datasource",
      "pluginId": "prometheus",
      "value": "Prometheus"
    }
  ]
}
EOF
    )

    local response=$(curl -s -w "%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -u "${GRAFANA_USER}:${GRAFANA_PASS}" \
        -d "$import_payload" \
        "${GRAFANA_URL}/api/dashboards/import")

    local http_code="${response: -3}"

    if [[ "$http_code" == "200" ]]; then
        log_success "✅ Live Trading Dashboard imported successfully"
        return 0
    else
        log_error "❌ Failed to import dashboard. HTTP Code: $http_code"
        echo "Response: ${response%???}"
        return 1
    fi
}

# Function to verify dashboard is working
verify_dashboard() {
    log_info "🔍 Verifying dashboard functionality..."

    # Check if Prometheus data source can query MEVana metrics
    local test_query="up{job=\"hotpath-engine\"}"
    local query_url="${GRAFANA_URL}/api/datasources/proxy/1/api/v1/query"

    local response=$(curl -s -w "%{http_code}" \
        -X GET \
        -H "Content-Type: application/json" \
        -u "${GRAFANA_USER}:${GRAFANA_PASS}" \
        -G -d "query=$test_query" \
        "$query_url")

    local http_code="${response: -3}"

    if [[ "$http_code" == "200" ]]; then
        local response_body="${response%???}"
        if echo "$response_body" | grep -q '"status":"success"'; then
            log_success "✅ Dashboard can query MEVana metrics"
            return 0
        else
            log_warning "⚠️ Dashboard configured but MEVana service may not be running yet"
            return 0
        fi
    else
        log_error "❌ Failed to verify dashboard. HTTP Code: $http_code"
        return 1
    fi
}

# Function to show dashboard access info
show_access_info() {
    log_success "🎉 Grafana Setup Complete!"
    echo ""
    echo -e "${GREEN}📊 PRODUCTION DASHBOARD ACCESS:${NC}"
    echo -e "   URL: ${BLUE}http://149.28.56.125:3000${NC}"
    echo -e "   Username: ${YELLOW}admin${NC}"
    echo -e "   Password: ${YELLOW}(provided through GRAFANA_PASS)${NC}"
    echo ""
    echo -e "${GREEN}🚀 LIVE TRADING METRICS:${NC}"
    echo -e "   • ${BLUE}🚨 Geyser Stream Status${NC} - CRITICAL: Must be green"
    echo -e "   • ${BLUE}💰 Arbitrage Opportunities${NC} - Real-time opportunity flow"
    echo -e "   • ${BLUE}🎯 Execution Success Rate${NC} - Trading performance"
    echo -e "   • ${BLUE}⚡ System Latency${NC} - Sub-millisecond detection"
    echo -e "   • ${BLUE}📈 Revenue Tracking${NC} - Live P&L monitoring"
    echo ""
    echo -e "${GREEN}📋 FOR LIVE TRADING DECISIONS:${NC}"
    echo -e "   1. ${YELLOW}Verify Geyser streams are HEALTHY (green)${NC}"
    echo -e "   2. ${YELLOW}Check success rate >15% before going live${NC}"
    echo -e "   3. ${YELLOW}Monitor opportunity flow >500/hour${NC}"
    echo -e "   4. ${YELLOW}Ensure latency <1000μs consistently${NC}"
}

# Main execution
main() {
    log_info "🚀 MEVana Production Grafana Setup Starting..."

    # Wait for Grafana to be ready
    if ! wait_for_grafana; then
        log_error "❌ Cannot proceed without Grafana being ready"
        exit 1
    fi

    # Setup Prometheus data source
    if ! setup_prometheus_datasource; then
        log_error "❌ Failed to setup Prometheus data source"
        exit 1
    fi

    # Import live trading dashboard
    if ! import_dashboard; then
        log_error "❌ Failed to import dashboard"
        exit 1
    fi

    # Verify dashboard functionality
    verify_dashboard

    # Show access information
    show_access_info

    log_success "🎯 Production dashboard ready for live trading decisions!"
}

# Run with error handling
trap 'log_error "Setup failed on line $LINENO"' ERR
main "$@"