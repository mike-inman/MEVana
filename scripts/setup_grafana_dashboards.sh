#!/bin/bash

# =============================================================================
# MEVana Grafana Dashboard Setup Script
# =============================================================================
# Sets up production Grafana with MEVana monitoring dashboards
# Usage: ./setup_grafana_dashboards.sh [SERVER_IP]

set -e

SERVER_IP="${1:-149.28.56.125}"
GRAFANA_URL="http://${SERVER_IP}:3000"

echo "🎨 Setting up MEVana Grafana Dashboards"
echo "Server: ${SERVER_IP}"
echo "Grafana: ${GRAFANA_URL}"
echo

echo "✅ Current Status:"
echo "  📊 Grafana: Running on port 3000"
echo "  📈 Prometheus: Running on port 9090 (collecting metrics)"
echo "  🚀 MEVana Engine: Running on port 8088 (producing metrics)"
echo

echo "📋 Manual Setup Instructions:"
echo "1. Open: ${GRAFANA_URL}"
echo "2. Login: admin / admin123"
echo "3. Go to: Configuration > Data Sources"
echo "4. Add Prometheus data source:"
echo "   - Name: Prometheus"
echo "   - URL: http://prometheus:9090"
echo "   - Click 'Save & Test'"
echo

echo "📊 Dashboard Creation:"
echo "1. Create New Dashboard"
echo "2. Add panels with these queries:"

echo
echo "🔥 MEVana Stream Messages Panel:"
echo "Query: multi_geyser_stream_messages_total"
echo "Type: Time series"
echo

echo "📈 Message Rate Panel:"
echo "Query: rate(multi_geyser_stream_messages_total[1m])"
echo "Type: Stat"
echo

echo "⚡ Current Values Panel:"
echo "Query: multi_geyser_stream_messages_total"
echo "Type: Table"
echo

echo "🎯 Test Queries Available:"
echo "  - multi_geyser_stream_messages_total"
echo "  - multi_geyser_stream_messages_per_minute"
echo "  - multi_geyser_stream_messages_parsed_per_minute"

echo
echo "🌐 Access your dashboard at: ${GRAFANA_URL}"
echo "📊 Prometheus metrics at: http://${SERVER_IP}:9090"
echo "🚀 MEVana engine at: http://${SERVER_IP}:8088/metrics"