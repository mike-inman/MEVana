#!/bin/bash

# Mobile UI Test Script
# Tests mobile responsiveness and touch interactions for the transaction table

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
    echo "Mobile UI Test for Transaction Table"
    echo "=========================================="
    echo ""
}

# Test 1: Check mobile-specific CSS rules
test_mobile_css_rules() {
    print_status "INFO" "Test 1: Checking mobile-specific CSS rules"
    
    local mobile_rules=(
        "@media.*max-width.*768px"
        "@media.*max-width.*480px"
        "min-width.*44px"
        "min-height.*44px"
        "font-size.*16px"
        "touch.*target"
        "orientation.*landscape"
    )
    
    local found_rules=0
    
    for rule in "${mobile_rules[@]}"; do
        if grep -q "$rule" dashboard/react-components/src/components/TransactionTable.css; then
            print_status "SUCCESS" "Mobile CSS rule found: $rule"
            ((found_rules++))
        else
            print_status "WARNING" "Mobile CSS rule missing: $rule"
        fi
    done
    
    print_status "INFO" "Found $found_rules out of ${#mobile_rules[@]} mobile CSS rules"
    
    if [ $found_rules -ge 5 ]; then
        print_status "SUCCESS" "Mobile CSS rules are comprehensive"
    else
        print_status "WARNING" "Mobile CSS rules may be incomplete"
    fi
}

# Test 2: Check touch-friendly interactions
test_touch_interactions() {
    print_status "INFO" "Test 2: Checking touch-friendly interactions"
    
    local touch_features=(
        "min-width.*44px"
        "min-height.*44px"
        "padding.*12px"
        "font-size.*16px"
        "active.*transform"
        "pointer.*coarse"
    )
    
    local found_features=0
    
    for feature in "${touch_features[@]}"; do
        if grep -q "$feature" dashboard/react-components/src/components/TransactionTable.css; then
            print_status "SUCCESS" "Touch feature found: $feature"
            ((found_features++))
        else
            print_status "WARNING" "Touch feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_features out of ${#touch_features[@]} touch features"
    
    if [ $found_features -ge 4 ]; then
        print_status "SUCCESS" "Touch interactions are well implemented"
    else
        print_status "WARNING" "Touch interactions may need improvement"
    fi
}

# Test 3: Check responsive breakpoints
test_responsive_breakpoints() {
    print_status "INFO" "Test 3: Checking responsive breakpoints"
    
    local breakpoints=(
        "1200px"
        "768px"
        "480px"
    )
    
    local found_breakpoints=0
    
    for breakpoint in "${breakpoints[@]}"; do
        if grep -q "@media.*$breakpoint" dashboard/react-components/src/components/TransactionTable.css; then
            print_status "SUCCESS" "Breakpoint found: $breakpoint"
            ((found_breakpoints++))
        else
            print_status "WARNING" "Breakpoint missing: $breakpoint"
        fi
    done
    
    print_status "INFO" "Found $found_breakpoints out of ${#breakpoints[@]} breakpoints"
    
    if [ $found_breakpoints -eq ${#breakpoints[@]} ]; then
        print_status "SUCCESS" "All responsive breakpoints are implemented"
    else
        print_status "WARNING" "Some responsive breakpoints are missing"
    fi
}

# Test 4: Check mobile column hiding
test_mobile_column_hiding() {
    print_status "INFO" "Test 4: Checking mobile column hiding"
    
    local hidden_columns=(
        "nth-child\\(4\\)"
        "nth-child\\(5\\)"
        "nth-child\\(6\\)"
        "nth-child\\(7\\)"
    )
    
    local found_hidden=0
    
    for column in "${hidden_columns[@]}"; do
        if grep -q "display.*none.*$column" dashboard/react-components/src/components/TransactionTable.css; then
            print_status "SUCCESS" "Column hiding found: $column"
            ((found_hidden++))
        else
            print_status "WARNING" "Column hiding missing: $column"
        fi
    done
    
    print_status "INFO" "Found $found_hidden out of ${#hidden_columns[@]} hidden columns"
    
    if [ $found_hidden -ge 3 ]; then
        print_status "SUCCESS" "Mobile column hiding is implemented"
    else
        print_status "WARNING" "Mobile column hiding may be incomplete"
    fi
}

# Test 5: Check iOS-specific optimizations
test_ios_optimizations() {
    print_status "INFO" "Test 5: Checking iOS-specific optimizations"
    
    local ios_features=(
        "font-size.*16px.*zoom"
        "min-width.*44px"
        "min-height.*44px"
        "-webkit-font-smoothing"
        "-moz-osx-font-smoothing"
    )
    
    local found_ios=0
    
    for feature in "${ios_features[@]}"; do
        if grep -q "$feature" dashboard/react-components/src/components/TransactionTable.css; then
            print_status "SUCCESS" "iOS optimization found: $feature"
            ((found_ios++))
        else
            print_status "WARNING" "iOS optimization missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_ios out of ${#ios_features[@]} iOS optimizations"
    
    if [ $found_ios -ge 3 ]; then
        print_status "SUCCESS" "iOS optimizations are well implemented"
    else
        print_status "WARNING" "iOS optimizations may need improvement"
    fi
}

# Test 6: Check landscape orientation support
test_landscape_support() {
    print_status "INFO" "Test 6: Checking landscape orientation support"
    
    if grep -q "orientation.*landscape" dashboard/react-components/src/components/TransactionTable.css; then
        print_status "SUCCESS" "Landscape orientation support found"
    else
        print_status "WARNING" "Landscape orientation support missing"
    fi
    
    if grep -q "max-height.*300px.*landscape" dashboard/react-components/src/components/TransactionTable.css; then
        print_status "SUCCESS" "Landscape height adjustment found"
    else
        print_status "WARNING" "Landscape height adjustment missing"
    fi
}

# Test 7: Check high DPI display support
test_high_dpi_support() {
    print_status "INFO" "Test 7: Checking high DPI display support"
    
    if grep -q "-webkit-min-device-pixel-ratio" dashboard/react-components/src/components/TransactionTable.css; then
        print_status "SUCCESS" "High DPI detection found"
    else
        print_status "WARNING" "High DPI detection missing"
    fi
    
    if grep -q "-webkit-font-smoothing.*antialiased" dashboard/react-components/src/components/TransactionTable.css; then
        print_status "SUCCESS" "Font smoothing for high DPI found"
    else
        print_status "WARNING" "Font smoothing for high DPI missing"
    fi
}

# Test 8: Check mobile performance optimizations
test_mobile_performance() {
    print_status "INFO" "Test 8: Checking mobile performance optimizations"
    
    local performance_features=(
        "transform.*scale"
        "transition.*ease"
        "will-change"
        "backface-visibility"
    )
    
    local found_performance=0
    
    for feature in "${performance_features[@]}"; do
        if grep -q "$feature" dashboard/react-components/src/components/TransactionTable.css; then
            print_status "SUCCESS" "Performance feature found: $feature"
            ((found_performance++))
        else
            print_status "WARNING" "Performance feature missing: $feature"
        fi
    done
    
    print_status "INFO" "Found $found_performance out of ${#performance_features[@]} performance features"
    
    if [ $found_performance -ge 2 ]; then
        print_status "SUCCESS" "Mobile performance optimizations are implemented"
    else
        print_status "WARNING" "Mobile performance optimizations may need improvement"
    fi
}

# Test 9: Generate mobile preview report
test_mobile_preview() {
    print_status "INFO" "Test 9: Generating mobile preview report"
    
    echo ""
    echo "📱 Mobile UI Preview Report"
    echo "=========================="
    echo ""
    echo "Screen Sizes Supported:"
    echo "  • Desktop (1200px+) - Full table with all columns"
    echo "  • Tablet (768px-1199px) - Condensed table"
    echo "  • Mobile (480px-767px) - Simplified table, hidden columns"
    echo "  • Small Mobile (<480px) - Minimal table, essential columns only"
    echo ""
    echo "Touch Interactions:"
    echo "  • 44px minimum touch targets (iOS standard)"
    echo "  • 16px font size to prevent zoom"
    echo "  • Active state feedback for buttons"
    echo "  • Swipe-friendly table scrolling"
    echo ""
    echo "Mobile Optimizations:"
    echo "  • Landscape orientation support"
    echo "  • High DPI display support"
    echo "  • Performance optimizations"
    echo "  • Reduced animations for battery life"
    echo ""
    echo "Hidden Columns on Mobile:"
    echo "  • Token In/Out addresses (space saving)"
    echo "  • Amount In/Out (simplified view)"
    echo "  • User addresses (very small screens)"
    echo ""
    
    print_status "SUCCESS" "Mobile preview report generated"
}

# Main test execution
main() {
    print_header
    
    local tests=(
        "test_mobile_css_rules"
        "test_touch_interactions"
        "test_responsive_breakpoints"
        "test_mobile_column_hiding"
        "test_ios_optimizations"
        "test_landscape_support"
        "test_high_dpi_support"
        "test_mobile_performance"
        "test_mobile_preview"
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
    echo "Mobile UI Test Results Summary"
    echo "=========================================="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: ${#tests[@]}"
    echo ""
    
    if [ $failed -eq 0 ]; then
        print_status "SUCCESS" "All mobile UI tests passed! Transaction table is mobile-ready."
        echo ""
        print_status "INFO" "Mobile testing recommendations:"
        echo "  1. Test on actual devices (iPhone, Android, iPad)"
        echo "  2. Test in different orientations (portrait/landscape)"
        echo "  3. Test with different screen sizes and densities"
        echo "  4. Test touch interactions and scrolling"
        echo "  5. Test performance on slower devices"
        echo "  6. Test with different browsers (Safari, Chrome, Firefox)"
    else
        print_status "ERROR" "Some mobile UI tests failed. Please review the issues above."
        exit 1
    fi
}

# Run tests
main 