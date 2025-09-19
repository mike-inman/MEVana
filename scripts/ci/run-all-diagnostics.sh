#!/bin/bash

# Master diagnostic runner - executes all Phase 1 verification tests
# Run this script after deployment to comprehensively validate the fixes

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

echo -e "${PURPLE}🚀 MEVana Phase 1 - Complete Diagnostic Suite${NC}"
echo "=============================================="
echo "Running comprehensive validation of Phase 1 networking and service discovery fixes"
echo ""

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Array of diagnostic scripts to run
DIAGNOSTIC_SCRIPTS=(
    "quick-health-check.sh:Quick health overview"
    "verify-prometheus-targets.sh:Prometheus targets verification"
    "validate-grafana-dashboards.sh:Grafana dashboard validation"
    "post-deployment-diagnostics.sh:Complete system validation"
)

failed_tests=0
total_tests=${#DIAGNOSTIC_SCRIPTS[@]}

echo -e "${BLUE}📋 Diagnostic Test Suite:${NC}"
for script_info in "${DIAGNOSTIC_SCRIPTS[@]}"; do
    IFS=':' read -r script_name description <<< "$script_info"
    echo "  • $description ($script_name)"
done

echo ""
echo -e "${BLUE}🏁 Starting diagnostics...${NC}"
echo ""

# Function to run a diagnostic script
run_diagnostic() {
    local script_name=$1
    local description=$2
    local script_path="${SCRIPT_DIR}/${script_name}"

    echo -e "${BLUE}▶️  Running: $description${NC}"
    echo "$(printf '%.0s-' {1..50})"

    if [ ! -f "$script_path" ]; then
        echo -e "${RED}❌ Script not found: $script_path${NC}"
        return 1
    fi

    if [ ! -x "$script_path" ]; then
        echo -e "${YELLOW}⚠️  Making script executable: $script_name${NC}"
        chmod +x "$script_path"
    fi

    # Run the script and capture exit code
    if bash "$script_path"; then
        echo -e "${GREEN}✅ $description - PASSED${NC}"
        echo ""
        return 0
    else
        local exit_code=$?
        echo -e "${RED}❌ $description - FAILED (exit code: $exit_code)${NC}"
        echo ""
        return 1
    fi
}

# Run each diagnostic script
for script_info in "${DIAGNOSTIC_SCRIPTS[@]}"; do
    IFS=':' read -r script_name description <<< "$script_info"

    if ! run_diagnostic "$script_name" "$description"; then
        failed_tests=$((failed_tests + 1))
    fi
done

# Final summary
echo "$(printf '%.0s=' {1..60})"
echo -e "${PURPLE}📊 DIAGNOSTIC SUMMARY${NC}"
echo "$(printf '%.0s=' {1..60})"

passed_tests=$((total_tests - failed_tests))
success_rate=$(( passed_tests * 100 / total_tests ))

echo "Diagnostic suites executed: $total_tests"
echo "Passed: $passed_tests"
echo "Failed: $failed_tests"
echo "Success rate: ${success_rate}%"

if [ $failed_tests -eq 0 ]; then
    echo ""
    echo -e "${GREEN}🎉 ALL DIAGNOSTICS PASSED!${NC}"
    echo -e "${GREEN}Phase 1 deployment is fully successful${NC}"
    echo ""
    echo -e "${BLUE}🎯 Your MEVana deployment is ready:${NC}"
    echo "  • Grafana:    http://localhost:3000 (admin/admin123)"
    echo "  • Prometheus: http://localhost:9090"
    echo "  • Kafka UI:   http://localhost:8080"
    echo ""
    echo -e "${PURPLE}✨ Phase 1 Complete - Ready for Phase 2 enhanced workflows!${NC}"
    exit 0

elif [ $success_rate -ge 75 ]; then
    echo ""
    echo -e "${YELLOW}⚠️  MOSTLY SUCCESSFUL with some warnings${NC}"
    echo "Review failed tests above - system may be functional but needs attention"
    echo ""
    echo -e "${BLUE}You can still proceed to test the deployment manually:${NC}"
    echo "  • Grafana:    http://localhost:3000 (admin/admin123)"
    echo "  • Prometheus: http://localhost:9090"
    exit 1

else
    echo ""
    echo -e "${RED}❌ MULTIPLE CRITICAL ISSUES DETECTED${NC}"
    echo "Phase 1 deployment has significant problems that need resolution"
    echo ""
    echo -e "${RED}Please review all failed diagnostics above before proceeding${NC}"
    exit 2
fi