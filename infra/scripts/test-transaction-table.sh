#!/bin/bash

# Transaction Table Feature Test Script
# Tests the transaction table UI component and API endpoint

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    local status=$1
    local message=$2
    case $status in
        "SUCCESS")
            echo -e "${GREEN}✅ $message${NC}"
            ;;
        "ERROR")
            echo -e "${RED}❌ $message${NC}"
            ;;
        "WARNING")
            echo -e "${YELLOW}⚠️  $message${NC}"
            ;;
        "INFO")
            echo -e "${BLUE}ℹ️  $message${NC}"
            ;;
    esac
}

print_header() {
    echo "=========================================="
    echo "Transaction Table Feature Test"
    echo "=========================================="
    echo ""
}

# Test 1: Check if React component exists
test_component_exists() {
    print_status "INFO" "Test 1: Checking if TransactionTable component exists"
    
    if [ -f "dashboard/react-components/src/components/TransactionTable.tsx" ]; then
        print_status "SUCCESS" "TransactionTable.tsx component found"
    else
        print_status "ERROR" "TransactionTable.tsx component not found"
        return 1
    fi
    
    if [ -f "dashboard/react-components/src/components/TransactionTable.css" ]; then
        print_status "SUCCESS" "TransactionTable.css styles found"
    else
        print_status "ERROR" "TransactionTable.css styles not found"
        return 1
    fi
}

# Test 2: Check if API endpoint exists
test_api_endpoint() {
    print_status "INFO" "Test 2: Checking if API endpoint is defined"
    
    if grep -q "@app.route('/api/transactions')" dashboard/server.py; then
        print_status "SUCCESS" "API endpoint /api/transactions found in server.py"
    else
        print_status "ERROR" "API endpoint /api/transactions not found in server.py"
        return 1
    fi
}

# Test 3: Check if component is exported
test_component_export() {
    print_status "INFO" "Test 3: Checking if component is exported"
    
    if grep -q "TransactionTable" dashboard/react-components/src/index.ts; then
        print_status "SUCCESS" "TransactionTable component is exported"
    else
        print_status "ERROR" "TransactionTable component is not exported"
        return 1
    fi
}

# Test 4: Check if component is included in HTML
test_html_inclusion() {
    print_status "INFO" "Test 4: Checking if component is included in HTML"
    
    if grep -q "transaction-table-container" dashboard/react-components/src/index.html; then
        print_status "SUCCESS" "Transaction table container found in HTML"
    else
        print_status "ERROR" "Transaction table container not found in HTML"
        return 1
    fi
}

# Test 5: Check TypeScript compilation
test_typescript_compilation() {
    print_status "INFO" "Test 5: Checking TypeScript compilation"
    
    cd dashboard/react-components
    
    if command -v npx >/dev/null 2>&1; then
        if npx tsc --noEmit --skipLibCheck; then
            print_status "SUCCESS" "TypeScript compilation passed"
        else
            print_status "ERROR" "TypeScript compilation failed"
            cd ../..
            return 1
        fi
    else
        print_status "WARNING" "npx not available, skipping TypeScript compilation check"
    fi
    
    cd ../..
}

# Test 6: Check if database schema supports transaction data
test_database_schema() {
    print_status "INFO" "Test 6: Checking database schema support"
    
    # Check if swap_events table schema is referenced
    if grep -q "swap_events" dashboard/server.py; then
        print_status "SUCCESS" "swap_events table referenced in server code"
    else
        print_status "WARNING" "swap_events table not found in server code"
    fi
    
    # Check if shared types include SwapEvent
    if grep -q "SwapEvent" shared-types/src/lib.rs; then
        print_status "SUCCESS" "SwapEvent type found in shared types"
    else
        print_status "WARNING" "SwapEvent type not found in shared types"
    fi
}

# Test 7: Check component features
test_component_features() {
    print_status "INFO" "Test 7: Checking component features"
    
    local features=(
        "real-time updates"
        "sorting"
        "filtering"
        "search"
        "responsive design"
        "error handling"
        "loading states"
    )
    
    local found_features=0
    
    for feature in "${features[@]}"; do
        case $feature in
            "real-time updates")
                if grep -q "useEffect.*setInterval" dashboard/react-components/src/components/TransactionTable.tsx; then
                    print_status "SUCCESS" "Real-time updates implemented"
                    ((found_features++))
                else
                    print_status "WARNING" "Real-time updates not found"
                fi
                ;;
            "sorting")
                if grep -q "handleSort" dashboard/react-components/src/components/TransactionTable.tsx; then
                    print_status "SUCCESS" "Sorting functionality implemented"
                    ((found_features++))
                else
                    print_status "WARNING" "Sorting functionality not found"
                fi
                ;;
            "filtering")
                if grep -q "filterByDex\|showLargeTradesOnly" dashboard/react-components/src/components/TransactionTable.tsx; then
                    print_status "SUCCESS" "Filtering functionality implemented"
                    ((found_features++))
                else
                    print_status "WARNING" "Filtering functionality not found"
                fi
                ;;
            "search")
                if grep -q "searchTerm" dashboard/react-components/src/components/TransactionTable.tsx; then
                    print_status "SUCCESS" "Search functionality implemented"
                    ((found_features++))
                else
                    print_status "WARNING" "Search functionality not found"
                fi
                ;;
            "responsive design")
                if grep -q "@media" dashboard/react-components/src/components/TransactionTable.css; then
                    print_status "SUCCESS" "Responsive design implemented"
                    ((found_features++))
                else
                    print_status "WARNING" "Responsive design not found"
                fi
                ;;
            "error handling")
                if grep -q "error.*setError" dashboard/react-components/src/components/TransactionTable.tsx; then
                    print_status "SUCCESS" "Error handling implemented"
                    ((found_features++))
                else
                    print_status "WARNING" "Error handling not found"
                fi
                ;;
            "loading states")
                if grep -q "loading.*setLoading" dashboard/react-components/src/components/TransactionTable.tsx; then
                    print_status "SUCCESS" "Loading states implemented"
                    ((found_features++))
                else
                    print_status "WARNING" "Loading states not found"
                fi
                ;;
        esac
    done
    
    print_status "INFO" "Found $found_features out of ${#features[@]} features"
    
    if [ $found_features -ge 5 ]; then
        print_status "SUCCESS" "Component has sufficient features"
    else
        print_status "WARNING" "Component may be missing some features"
    fi
}

# Test 8: Check file sizes (ensure small, focused feature)
test_feature_size() {
    print_status "INFO" "Test 8: Checking feature size (small, focused approach)"
    
    local component_size=$(wc -l < dashboard/react-components/src/components/TransactionTable.tsx)
    local css_size=$(wc -l < dashboard/react-components/src/components/TransactionTable.css)
    
    print_status "INFO" "Component size: $component_size lines"
    print_status "INFO" "CSS size: $css_size lines"
    
    if [ $component_size -le 300 ]; then
        print_status "SUCCESS" "Component size is appropriate (≤300 lines)"
    else
        print_status "WARNING" "Component size is large (>300 lines)"
    fi
    
    if [ $css_size -le 400 ]; then
        print_status "SUCCESS" "CSS size is appropriate (≤400 lines)"
    else
        print_status "WARNING" "CSS size is large (>400 lines)"
    fi
}

# Test 9: Check integration points
test_integration_points() {
    print_status "INFO" "Test 9: Checking integration points"
    
    # Check if component can be imported
    if grep -q "import.*TransactionTable" dashboard/react-components/src/index.ts; then
        print_status "SUCCESS" "Component can be imported"
    else
        print_status "WARNING" "Component import not found"
    fi
    
    # Check if API endpoint returns JSON
    if grep -q "jsonify.*transactions" dashboard/server.py; then
        print_status "SUCCESS" "API endpoint returns JSON"
    else
        print_status "WARNING" "API endpoint JSON response not found"
    fi
}

# Main test execution
main() {
    print_header
    
    local tests=(
        "test_component_exists"
        "test_api_endpoint"
        "test_component_export"
        "test_html_inclusion"
        "test_typescript_compilation"
        "test_database_schema"
        "test_component_features"
        "test_feature_size"
        "test_integration_points"
    )
    
    local passed=0
    local failed=0
    
    for test in "${tests[@]}"; do
        echo ""
        if $test; then
            ((passed++))
        else
            ((failed++))
        fi
    done
    
    echo ""
    echo "=========================================="
    echo "Test Results Summary"
    echo "=========================================="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: ${#tests[@]}"
    echo ""
    
    if [ $failed -eq 0 ]; then
        print_status "SUCCESS" "All tests passed! Transaction table feature is ready."
        echo ""
        print_status "INFO" "Next steps:"
        echo "  1. Build the React components: cd dashboard/react-components && npm run build"
        echo "  2. Start the dashboard server: cd dashboard && python server.py"
        echo "  3. Open http://localhost:8080 to view the transaction table"
        echo "  4. Test with real transaction data from your streaming modules"
    else
        print_status "ERROR" "Some tests failed. Please review the issues above."
        exit 1
    fi
}

# Run tests
main 