#!/bin/bash
#
# Export Dashboard from Local Grafana
# Usage: ./export-dashboard.sh "Dashboard Title"
#

set -e

DASHBOARD_TITLE="$1"
LOCAL_GRAFANA="http://localhost:3300"
GRAFANA_USER="admin"
GRAFANA_PASS="<ADMIN_PASSWORD>"
OUTPUT_DIR="infrastructure/grafana/dashboards/curated"

if [ -z "$DASHBOARD_TITLE" ]; then
    echo "❌ Usage: $0 \"Dashboard Title\""
    echo "📊 Available dashboards:"
    curl -s -u "$GRAFANA_USER:$GRAFANA_PASS" \
        "$LOCAL_GRAFANA/api/search?type=dash-db" | \
        jq -r '.[].title' 2>/dev/null || echo "Could not fetch dashboards"
    exit 1
fi

echo "🔍 Searching for dashboard: $DASHBOARD_TITLE"

# Find dashboard UID
DASHBOARD_UID=$(curl -s -u "$GRAFANA_USER:$GRAFANA_PASS" \
    "$LOCAL_GRAFANA/api/search?query=${DASHBOARD_TITLE}" | \
    jq -r '.[0].uid' 2>/dev/null)

if [ "$DASHBOARD_UID" = "null" ] || [ -z "$DASHBOARD_UID" ]; then
    echo "❌ Dashboard '$DASHBOARD_TITLE' not found"
    exit 1
fi

echo "✅ Found dashboard UID: $DASHBOARD_UID"

# Export dashboard
echo "📤 Exporting dashboard..."
DASHBOARD_JSON=$(curl -s -u "$GRAFANA_USER:$GRAFANA_PASS" \
    "$LOCAL_GRAFANA/api/dashboards/uid/$DASHBOARD_UID")

# Clean up the export (remove id, version, etc. for import)
CLEAN_DASHBOARD=$(echo "$DASHBOARD_JSON" | jq '{
    dashboard: (.dashboard | del(.id, .version, .iteration)),
    folderId: .meta.folderId,
    overwrite: true
}')

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Generate filename from title
FILENAME=$(echo "$DASHBOARD_TITLE" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g' | sed 's/--*/-/g' | sed 's/^-\|-$//g')
OUTPUT_FILE="$OUTPUT_DIR/${FILENAME}.json"

# Save dashboard
echo "$CLEAN_DASHBOARD" | jq '.' > "$OUTPUT_FILE"

echo "✅ Dashboard exported to: $OUTPUT_FILE"
echo "📊 Dashboard ready for deployment!"
echo ""
echo "🚀 Next steps:"
echo "   git add $OUTPUT_FILE"
echo "   git commit -m \"Update dashboard: $DASHBOARD_TITLE\""
echo "   git push  # Auto-deploys via GitHub Actions"