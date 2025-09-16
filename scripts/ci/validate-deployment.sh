#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 MEVana Deployment Configuration Validation${NC}"
echo "=================================================="

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

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
echo -e "${BLUE}📋 Checking Prerequisites...${NC}"

if command_exists docker; then
    print_status "pass" "Docker is installed"
else
    print_status "fail" "Docker is not installed"
    exit 1
fi

if command_exists jq; then
    print_status "pass" "jq is installed"
else
    print_status "warn" "jq is not installed - JSON validation will be limited"
fi

# Validate Docker Compose syntax
echo -e "\n${BLUE}🐳 Validating Docker Compose Configuration...${NC}"

if [ -f "infrastructure/docker-compose.server.yml" ]; then
    print_status "pass" "docker-compose.server.yml exists"

    # Check syntax
    if docker compose -f infrastructure/docker-compose.server.yml config > /dev/null 2>&1; then
        print_status "pass" "Docker Compose syntax is valid"
    else
        print_status "fail" "Docker Compose syntax error"
        echo "Run: docker compose -f infrastructure/docker-compose.server.yml config"
        exit 1
    fi

    # Check for container names
    if grep -q "container_name:" infrastructure/docker-compose.server.yml; then
        print_status "pass" "Container names are specified"
    else
        print_status "warn" "No container names found - may cause service discovery issues"
    fi

    # Check for external network
    if grep -q "mevana_net" infrastructure/docker-compose.server.yml; then
        print_status "pass" "Uses mevana_net network"
    else
        print_status "fail" "Missing mevana_net network configuration"
    fi
else
    print_status "fail" "docker-compose.server.yml not found"
    exit 1
fi

# Validate Prometheus configuration
echo -e "\n${BLUE}📊 Validating Prometheus Configuration...${NC}"

if [ -f "infrastructure/prometheus.core.yml" ]; then
    print_status "pass" "prometheus.core.yml exists"

    # Check syntax using promtool if available
    if command_exists docker; then
        if docker run --rm -v "${PWD}/infrastructure/prometheus.core.yml:/tmp/prometheus.yml" --entrypoint="/bin/promtool" prom/prometheus:latest check config /tmp/prometheus.yml > /dev/null 2>&1; then
            print_status "pass" "Prometheus configuration syntax is valid"
        else
            print_status "fail" "Prometheus configuration syntax error"
            echo "Run: docker run --rm -v \${PWD}/infrastructure/prometheus.core.yml:/tmp/prometheus.yml --entrypoint=\"/bin/promtool\" prom/prometheus:latest check config /tmp/prometheus.yml"
            exit 1
        fi
    fi

    # Check for required targets
    required_targets=("mevana-hotpath:8088" "prometheus-service:9090" "grafana-service:3000")
    for target in "${required_targets[@]}"; do
        if grep -q "$target" infrastructure/prometheus.core.yml; then
            print_status "pass" "Target $target is configured"
        else
            print_status "fail" "Missing target: $target"
        fi
    done
else
    print_status "fail" "prometheus.core.yml not found"
    exit 1
fi

# Validate Grafana configuration
echo -e "\n${BLUE}📈 Validating Grafana Configuration...${NC}"

# Check datasource configuration
if [ -f "infrastructure/grafana/datasources/prometheus.yml" ]; then
    print_status "pass" "Grafana datasource configuration exists"

    if grep -q "prometheus-service:9090" infrastructure/grafana/datasources/prometheus.yml; then
        print_status "pass" "Datasource points to correct Prometheus service"
    else
        print_status "warn" "Datasource may not point to correct Prometheus service"
    fi
else
    print_status "fail" "Grafana datasource configuration not found"
fi

# Validate dashboard files
echo -e "\n${BLUE}📊 Validating Grafana Dashboards...${NC}"

dashboard_count=0
valid_dashboards=0
invalid_dashboards=0

if [ -d "infrastructure/grafana/dashboards" ]; then
    for dashboard in infrastructure/grafana/dashboards/*.json; do
        if [ -f "$dashboard" ]; then
            dashboard_count=$((dashboard_count + 1))
            dashboard_name=$(basename "$dashboard")

            # Check JSON syntax
            if command_exists jq && jq empty "$dashboard" >/dev/null 2>&1; then
                # Check required fields
                if jq -e '.title' "$dashboard" > /dev/null 2>&1; then
                    title=$(jq -r '.title' "$dashboard")
                    if [ "$title" != "null" ] && [ -n "$title" ]; then
                        print_status "pass" "Dashboard $dashboard_name has valid title: '$title'"
                        valid_dashboards=$((valid_dashboards + 1))
                    else
                        print_status "fail" "Dashboard $dashboard_name has empty or null title"
                        invalid_dashboards=$((invalid_dashboards + 1))
                    fi
                else
                    print_status "fail" "Dashboard $dashboard_name missing title field"
                    invalid_dashboards=$((invalid_dashboards + 1))
                fi
            else
                print_status "fail" "Dashboard $dashboard_name has invalid JSON syntax"
                invalid_dashboards=$((invalid_dashboards + 1))
            fi
        fi
    done
else
    print_status "warn" "No dashboard directory found"
fi

# Check environment files
echo -e "\n${BLUE}🔐 Validating Environment Configuration...${NC}"

if [ -f ".env" ] || [ -f "infrastructure/.env" ]; then
    print_status "pass" "Environment files exist"

    # Check for required variables (without showing values)
    required_vars=("ENDPOINT" "X_TOKEN" "MEVANA_TAG")
    for var in "${required_vars[@]}"; do
        if grep -q "^${var}=" .env 2>/dev/null || grep -q "^${var}=" infrastructure/.env 2>/dev/null; then
            print_status "pass" "Environment variable $var is set"
        else
            print_status "warn" "Environment variable $var may not be set"
        fi
    done
else
    print_status "warn" "No environment files found"
fi

# Summary
echo -e "\n${BLUE}📋 Validation Summary${NC}"
echo "=========================="
echo "Dashboards: $valid_dashboards valid, $invalid_dashboards invalid (total: $dashboard_count)"

if [ $invalid_dashboards -gt 0 ]; then
    print_status "fail" "Configuration validation failed - fix dashboard issues before deployment"
    exit 1
else
    print_status "pass" "All validations passed - configuration is ready for deployment"
fi

echo -e "\n${GREEN}🎉 Configuration validation completed successfully!${NC}"