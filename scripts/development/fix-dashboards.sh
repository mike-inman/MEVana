#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 MEVana Dashboard Repair Tool${NC}"
echo "=================================="

# Function to print status
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "pass" ]; then
        echo -e "${GREEN}✅ $message${NC}"
    elif [ "$status" = "warn" ]; then
        echo -e "${YELLOW}⚠️  $message${NC}"
    else
        echo -e "${RED}❌ $message${NC}"
    fi
}

# Check if jq is available
if ! command -v jq >/dev/null 2>&1; then
    print_status "fail" "jq is required for dashboard fixing"
    exit 1
fi

# Dashboard directory
DASHBOARD_DIR="infrastructure/grafana/dashboards"

if [ ! -d "$DASHBOARD_DIR" ]; then
    print_status "fail" "Dashboard directory not found: $DASHBOARD_DIR"
    exit 1
fi

echo -e "\n${BLUE}📊 Scanning dashboard files...${NC}"

fixed_count=0
error_count=0

# Function to generate title from filename
generate_title() {
    local filename=$1
    # Remove .json extension, replace dashes/underscores with spaces, capitalize words
    echo "$filename" | sed 's/\.json$//' | tr '-' ' ' | tr '_' ' ' | sed 's/\b\w/\u&/g'
}

# Process each JSON file in the dashboard directory
for dashboard_file in "$DASHBOARD_DIR"/*.json; do
    if [ -f "$dashboard_file" ]; then
        dashboard_name=$(basename "$dashboard_file")

        echo -e "\n${BLUE}Processing: $dashboard_name${NC}"

        # Check if file is valid JSON
        if ! jq empty "$dashboard_file" >/dev/null 2>&1; then
            print_status "fail" "Invalid JSON syntax in $dashboard_name"
            error_count=$((error_count + 1))
            continue
        fi

        # Check if title exists and is not empty/null
        title=$(jq -r '.title // empty' "$dashboard_file" 2>/dev/null)

        if [ -z "$title" ] || [ "$title" = "null" ]; then
            # Generate title from filename
            new_title=$(generate_title "$dashboard_name")

            print_status "warn" "Dashboard missing/empty title, setting to: '$new_title'"

            # Create backup
            cp "$dashboard_file" "${dashboard_file}.backup"

            # Fix the title
            if jq --arg title "$new_title" '.title = $title' "$dashboard_file" > "${dashboard_file}.tmp"; then
                mv "${dashboard_file}.tmp" "$dashboard_file"
                print_status "pass" "Fixed title in $dashboard_name"
                fixed_count=$((fixed_count + 1))
            else
                print_status "fail" "Failed to fix title in $dashboard_name"
                rm -f "${dashboard_file}.tmp"
                error_count=$((error_count + 1))
            fi
        else
            print_status "pass" "Dashboard $dashboard_name already has title: '$title'"
        fi

        # Additional validations and fixes

        # Check for required dashboard fields
        if ! jq -e '.panels' "$dashboard_file" >/dev/null 2>&1; then
            print_status "warn" "Dashboard $dashboard_name missing panels array"
        fi

        # Check for dashboard ID (should be null for provisioned dashboards)
        dashboard_id=$(jq -r '.id // empty' "$dashboard_file" 2>/dev/null)
        if [ -n "$dashboard_id" ] && [ "$dashboard_id" != "null" ]; then
            print_status "warn" "Dashboard $dashboard_name has ID ($dashboard_id) - provisioned dashboards should have null ID"

            # Fix the ID
            cp "$dashboard_file" "${dashboard_file}.backup.id" 2>/dev/null || true
            if jq '.id = null' "$dashboard_file" > "${dashboard_file}.tmp"; then
                mv "${dashboard_file}.tmp" "$dashboard_file"
                print_status "pass" "Fixed ID in $dashboard_name"
            else
                rm -f "${dashboard_file}.tmp"
                print_status "fail" "Failed to fix ID in $dashboard_name"
            fi
        fi
    fi
done

# Summary
echo -e "\n${BLUE}📋 Dashboard Repair Summary${NC}"
echo "============================"
echo "Dashboards fixed: $fixed_count"
echo "Errors encountered: $error_count"

if [ $error_count -gt 0 ]; then
    print_status "warn" "Some dashboards had errors - check the output above"
    echo -e "\n${YELLOW}Backup files created with .backup extension${NC}"
else
    print_status "pass" "All dashboard issues resolved"
fi

echo -e "\n${GREEN}🎉 Dashboard repair completed!${NC}"

# Run validation to confirm fixes
if [ -f "scripts/ci/validate-deployment.sh" ]; then
    echo -e "\n${BLUE}🔍 Running validation to confirm fixes...${NC}"
    bash scripts/ci/validate-deployment.sh
fi