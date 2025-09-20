#!/bin/bash
#
# GitHub Deployment Notifier
# Creates GitHub issues/comments with deployment reports
# Uses GitHub CLI (gh) - no external services needed
#

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

REPORT_FILE="$1"
STATUS="$2"
SUMMARY="$3"

if [ -z "$REPORT_FILE" ] || [ ! -f "$REPORT_FILE" ]; then
    echo -e "${RED}❌ Report file required: $REPORT_FILE${NC}"
    exit 1
fi

# Check if gh CLI is available
if ! command -v gh >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  GitHub CLI not available - skipping GitHub notification${NC}"
    exit 0
fi

# Check if we're in a git repo with GitHub remote
if ! gh repo view >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Not in a GitHub repository - skipping GitHub notification${NC}"
    exit 0
fi

echo -e "${BLUE}🐙 Creating GitHub deployment notification...${NC}"

# Determine issue title and labels based on status
case "$STATUS" in
    "SUCCESS")
        TITLE="✅ Deployment Report - SUCCESS"
        LABELS="deployment,success,automated"
        ;;
    "BLOCKED")
        TITLE="🚨 Deployment Report - BLOCKED"
        LABELS="deployment,blocked,needs-attention"
        ;;
    "ERROR")
        TITLE="❌ Deployment Report - ERROR"
        LABELS="deployment,error,urgent"
        ;;
    *)
        TITLE="📊 Deployment Report - $STATUS"
        LABELS="deployment,automated"
        ;;
esac

# Create issue body from report
ISSUE_BODY=$(cat "$REPORT_FILE")

# Create GitHub issue with report
echo -e "${BLUE}📝 Creating GitHub issue: $TITLE${NC}"

ISSUE_URL=$(gh issue create \
    --title "$TITLE" \
    --body "$ISSUE_BODY" \
    --label "$LABELS" \
    --assignee "@me" 2>/dev/null || echo "")

if [ -n "$ISSUE_URL" ]; then
    echo -e "${GREEN}✅ GitHub issue created: $ISSUE_URL${NC}"

    # Add comment with quick summary if provided
    if [ -n "$SUMMARY" ]; then
        gh issue comment "$ISSUE_URL" --body "**Quick Summary**: $SUMMARY" >/dev/null 2>&1 || true
    fi

    echo -e "${BLUE}📧 You'll receive GitHub notifications for this issue${NC}"
else
    echo -e "${YELLOW}⚠️  Could not create GitHub issue (may need authentication)${NC}"
fi

# Alternative: Create a gist if issue creation fails
if [ -z "$ISSUE_URL" ]; then
    echo -e "${BLUE}📄 Creating GitHub Gist as fallback...${NC}"

    GIST_URL=$(gh gist create "$REPORT_FILE" \
        --desc "MEVana Deployment Report - $STATUS - $(date)" \
        --public 2>/dev/null || echo "")

    if [ -n "$GIST_URL" ]; then
        echo -e "${GREEN}✅ Deployment report saved as Gist: $GIST_URL${NC}"
    fi
fi