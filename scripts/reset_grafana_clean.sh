#!/bin/bash

# =============================================================================
# Clean Grafana Reset Script
# =============================================================================
# Completely resets Grafana database and starts fresh with working dashboards

set -e

SERVER_IP="${1:-149.28.56.125}"

echo "🧹 Resetting Grafana to clean state..."
echo "Server: ${SERVER_IP}"
echo

# Step 1: Stop and recreate Grafana container
echo "Step 1: Resetting Grafana container..."
ssh root@${SERVER_IP} << 'RESET_SCRIPT'
    echo "Stopping Grafana..."
    docker stop grafana || echo "Grafana not running"

    echo "Removing Grafana container..."
    docker rm grafana || echo "Grafana container not found"

    echo "Removing Grafana data volume..."
    docker volume rm grafana_data || echo "Grafana volume not found"

    echo "Recreating Grafana with fresh data..."
    docker run -d \
        --name grafana \
        --network mevana_net \
        -p 3000:3000 \
        -e GF_SECURITY_ADMIN_PASSWORD=<ADMIN_PASSWORD> \
        -v grafana_data:/var/lib/grafana \
        --restart unless-stopped \
        grafana/grafana:latest

    echo "Waiting for Grafana to start..."
    sleep 15
RESET_SCRIPT

echo "✅ Grafana has been reset and is starting fresh"

# Step 2: Wait for health check
echo "Step 2: Waiting for Grafana to be ready..."
for i in {1..10}; do
    if curl -s http://${SERVER_IP}:3000/api/health > /dev/null; then
        echo "✅ Grafana is healthy!"
        break
    else
        echo "⏳ Waiting... (attempt $i/10)"
        sleep 5
    fi
done

echo
echo "🎯 Fresh Grafana Setup Instructions:"
echo "1. Open: http://${SERVER_IP}:3000"
echo "2. Login: admin / <ADMIN_PASSWORD>"
echo "3. Add Prometheus data source:"
echo "   - Go to Configuration > Data Sources > Add data source"
echo "   - Select Prometheus"
echo "   - URL: http://prometheus:9090"
echo "   - Click 'Save & Test' (should show green checkmark)"
echo

echo "4. Import the clean dashboard:"
echo "   - Go to Create > Import"
echo "   - Upload or paste: dashboards/infrastructure_health_dashboard.json"
echo

echo "🔍 Test these queries work in Explore:"
echo "   - multi_geyser_stream_messages_total"
echo "   - rate(multi_geyser_stream_messages_total[1m])"
echo

echo "🎉 Grafana is now clean and ready for your pretty charts!"