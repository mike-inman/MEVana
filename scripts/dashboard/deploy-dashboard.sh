#!/bin/bash
#
# Hot-Deploy Dashboard to Server Grafana (Zero Downtime)
# Usage: ./deploy-dashboard.sh path/to/dashboard.json [server-ip]
#

set -e

DASHBOARD_FILE="$1"
SERVER_IP="${2:-149.28.56.125}"
SERVER_GRAFANA="http://$SERVER_IP:3000"
GRAFANA_USER="admin"
GRAFANA_PASS="<ADMIN_PASSWORD>"

if [ ! -f "$DASHBOARD_FILE" ]; then
    echo "❌ Dashboard file not found: $DASHBOARD_FILE"
    exit 1
fi

echo "🚀 Hot-deploying dashboard to server..."
echo "📊 File: $DASHBOARD_FILE"
echo "🌐 Server: $SERVER_GRAFANA"

# Validate JSON
if ! jq . "$DASHBOARD_FILE" >/dev/null 2>&1; then
    echo "❌ Invalid JSON in dashboard file"
    exit 1
fi

echo "✅ Dashboard JSON is valid"

# Get dashboard title for logging
DASHBOARD_TITLE=$(jq -r '.title // .dashboard.title // "Unknown"' "$DASHBOARD_FILE")
echo "📈 Dashboard: $DASHBOARD_TITLE"

# Prepare dashboard for API (wrap in proper format if needed)
TEMP_DASHBOARD=$(mktemp)
if jq -e '.dashboard' "$DASHBOARD_FILE" >/dev/null 2>&1; then
    # Already in correct format
    cp "$DASHBOARD_FILE" "$TEMP_DASHBOARD"
else
    # Wrap raw dashboard in API format
    jq '{dashboard: ., overwrite: true}' "$DASHBOARD_FILE" > "$TEMP_DASHBOARD"
fi

# Deploy via Grafana API
echo "🔄 Deploying dashboard..."
RESPONSE=$(curl -s -w "%{http_code}" -u "$GRAFANA_USER:$GRAFANA_PASS" \
    -X POST "$SERVER_GRAFANA/api/dashboards/db" \
    -H "Content-Type: application/json" \
    -d @"$TEMP_DASHBOARD")

# Cleanup
rm -f "$TEMP_DASHBOARD"

HTTP_CODE="${RESPONSE: -3}"
RESPONSE_BODY="${RESPONSE%???}"

if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ Dashboard deployed successfully!"
    DASHBOARD_URL=$(echo "$RESPONSE_BODY" | jq -r '.url // ""')
    if [ -n "$DASHBOARD_URL" ]; then
        echo "🔗 Dashboard URL: $SERVER_GRAFANA$DASHBOARD_URL"
    fi
else
    echo "❌ Deployment failed (HTTP $HTTP_CODE)"
    echo "Response: $RESPONSE_BODY"
    exit 1
fi

echo ""
echo "🎉 Hot-deployment complete! Dashboard is live with zero downtime."
echo "💡 No container restarts needed - dashboard appeared instantly!"