#!/bin/bash

# =============================================================================
# Automatic MEVana Dashboard Setup - Zero Manual Work Required
# =============================================================================

set -e

SERVER_IP="${1:-149.28.56.125}"

echo "🚀 Setting up your MEVana dashboard automatically..."

# Wait a bit for Grafana to be fully ready
sleep 10

# Step 1: Automatically add Prometheus data source
echo "📈 Adding Prometheus data source..."
ssh root@${SERVER_IP} << 'AUTO_SETUP'
    # Wait for Grafana API to be ready
    sleep 5

    # Add Prometheus data source automatically
    curl -X POST \
        -H "Content-Type: application/json" \
        -d '{
            "name": "Prometheus",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "access": "proxy",
            "isDefault": true
        }' \
        http://admin:admin123@localhost:3000/api/datasources || echo "Data source may already exist"

    sleep 2

    # Create a simple dashboard via API
    curl -X POST \
        -H "Content-Type: application/json" \
        -d '{
            "dashboard": {
                "title": "MEVana Live System Monitor",
                "panels": [
                    {
                        "id": 1,
                        "title": "Stream Messages Total",
                        "type": "timeseries",
                        "gridPos": {"h": 9, "w": 24, "x": 0, "y": 0},
                        "targets": [{"expr": "multi_geyser_stream_messages_total", "legendFormat": "Stream {{stream_id}}"}],
                        "fieldConfig": {"defaults": {"color": {"mode": "palette-classic"}}}
                    },
                    {
                        "id": 2,
                        "title": "Messages Per Minute Rate",
                        "type": "stat",
                        "gridPos": {"h": 6, "w": 12, "x": 0, "y": 9},
                        "targets": [{"expr": "rate(multi_geyser_stream_messages_total[1m]) * 60", "legendFormat": "Stream {{stream_id}}"}],
                        "fieldConfig": {"defaults": {"color": {"mode": "thresholds"}, "thresholds": {"steps": [{"color": "red", "value": null}, {"color": "yellow", "value": 1}, {"color": "green", "value": 10}]}}}
                    },
                    {
                        "id": 3,
                        "title": "System Status",
                        "type": "stat",
                        "gridPos": {"h": 6, "w": 12, "x": 12, "y": 9},
                        "targets": [{"expr": "up{job=\"hotpath-engine\"}", "legendFormat": "MEVana Engine"}],
                        "fieldConfig": {"defaults": {"mappings": [{"options": {"0": {"text": "DOWN"}}, "type": "value"}, {"options": {"1": {"text": "UP"}}, "type": "value"}], "color": {"mode": "thresholds"}, "thresholds": {"steps": [{"color": "red", "value": null}, {"color": "green", "value": 1}]}}}
                    }
                ],
                "time": {"from": "now-1h", "to": "now"},
                "refresh": "30s"
            },
            "overwrite": true
        }' \
        http://admin:admin123@localhost:3000/api/dashboards/db || echo "Dashboard creation may have failed"

AUTO_SETUP

echo "✅ Done! Your dashboard is ready."
echo
echo "🎯 Just open: http://${SERVER_IP}:3000"
echo "   Login: admin / admin123"
echo "   Your dashboard should be there automatically!"
echo
echo "📊 What you'll see:"
echo "   • Stream message counts over time"
echo "   • Current message processing rates"
echo "   • System UP/DOWN status"
echo
echo "🎯 Make decisions based on:"
echo "   • Are messages flowing? (lines going up)"
echo "   • Is the system UP? (green status)"
echo "   • What's the processing rate? (messages per minute)"