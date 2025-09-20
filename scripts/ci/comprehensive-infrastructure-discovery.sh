#!/bin/bash
#
# Comprehensive Infrastructure Discovery Script
# Provides detailed analysis of Prometheus, Grafana, Docker network, and MEVana configuration
# Use this to understand server state before deployment
#

set -e

# Colors for beautiful output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}"
cat << "EOF"
╔══════════════════════════════════════════════════════════╗
║               🔍 INFRASTRUCTURE DISCOVERY 🔍             ║
║          Comprehensive Pre-Deployment Analysis          ║
║                Server State Assessment                   ║
╚══════════════════════════════════════════════════════════╝
EOF
echo -e "${NC}"

echo -e "${BLUE}🕐 Discovery started at: $(date)${NC}"
echo -e "${BLUE}📍 Server: $(hostname) ($(hostname -I | awk '{print $1}'))${NC}"
echo -e "${BLUE}📂 Working directory: $(pwd)${NC}"
echo ""

# =============================================================================
# DOCKER INFRASTRUCTURE ANALYSIS
# =============================================================================

echo -e "${CYAN}═══ DOCKER INFRASTRUCTURE ANALYSIS ═══${NC}"

# Check Docker status
echo -e "${BLUE}🐳 Docker System Status:${NC}"
if command -v docker >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Docker installed: $(docker --version)${NC}"

    if docker ps >/dev/null 2>&1; then
        echo -e "${GREEN}✅ Docker daemon running${NC}"

        # Docker system info
        echo -e "${BLUE}   System info:${NC}"
        docker system df --format "table {{.Type}}\t{{.Size}}\t{{.Reclaimable}}" | grep -E "(Images|Containers|Local)" | sed 's/^/     /'

        # Show running containers
        echo -e "${BLUE}   Running containers:${NC}"
        docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | sed 's/^/     /'

    else
        echo -e "${RED}❌ Docker daemon not running${NC}"
    fi
else
    echo -e "${RED}❌ Docker not installed${NC}"
fi

echo ""

# Check Docker networks
echo -e "${BLUE}🌐 Docker Networks:${NC}"
if command -v docker >/dev/null 2>&1 && docker ps >/dev/null 2>&1; then
    echo -e "${BLUE}   All networks:${NC}"
    docker network ls --format "table {{.Name}}\t{{.Driver}}\t{{.Scope}}" | sed 's/^/     /'

    echo -e "${BLUE}   MEVana network details:${NC}"
    if docker network ls | grep -q "mevana_net"; then
        echo -e "${GREEN}     ✅ mevana_net exists${NC}"
        docker network inspect mevana_net --format "{{.Driver}} network, {{len .Containers}} containers connected" | sed 's/^/       /'

        # Show connected containers
        echo -e "${BLUE}     Connected containers:${NC}"
        docker network inspect mevana_net --format '{{range .Containers}}{{.Name}} ({{.IPv4Address}}){{"\n"}}{{end}}' | sed 's/^/       /'
    else
        echo -e "${RED}     ❌ mevana_net not found${NC}"
        echo -e "${YELLOW}     💡 Create with: docker network create mevana_net${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Cannot inspect Docker networks (Docker not available)${NC}"
fi

echo ""

# =============================================================================
# PROMETHEUS ANALYSIS
# =============================================================================

echo -e "${CYAN}═══ PROMETHEUS MONITORING ANALYSIS ═══${NC}"

PROMETHEUS_PORTS=(9090 9091)
PROMETHEUS_URL=""

# Find Prometheus
echo -e "${BLUE}🎯 Prometheus Discovery:${NC}"
for port in "${PROMETHEUS_PORTS[@]}"; do
    if curl -s --connect-timeout 3 "http://localhost:$port/-/healthy" >/dev/null 2>&1; then
        PROMETHEUS_URL="http://localhost:$port"
        echo -e "${GREEN}✅ Prometheus found at port $port${NC}"
        break
    fi
done

if [ -z "$PROMETHEUS_URL" ]; then
    echo -e "${RED}❌ Prometheus not accessible on standard ports${NC}"

    # Check if container exists but not accessible
    if docker ps --format "{{.Names}}" | grep -q prometheus; then
        prometheus_container=$(docker ps --format "{{.Names}}" | grep prometheus | head -1)
        echo -e "${YELLOW}⚠️  Prometheus container '$prometheus_container' exists but not accessible${NC}"
        echo -e "${BLUE}   Container details:${NC}"
        docker ps --filter "name=$prometheus_container" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | sed 's/^/     /'
    fi
else
    echo -e "${BLUE}📊 Prometheus Analysis:${NC}"

    # Prometheus version and config
    if command -v jq >/dev/null 2>&1; then
        prom_buildinfo=$(curl -s "$PROMETHEUS_URL/api/v1/query?query=prometheus_build_info" | jq -r '.data.result[0].metric.version // "unknown"' 2>/dev/null)
        echo -e "${GREEN}   Version: $prom_buildinfo${NC}"
    fi

    # Configuration file location
    if docker ps --format "{{.Names}}" | grep -q prometheus; then
        prometheus_container=$(docker ps --format "{{.Names}}" | grep prometheus | head -1)
        echo -e "${BLUE}   Configuration:${NC}"
        docker exec "$prometheus_container" cat /etc/prometheus/prometheus.yml | head -10 | sed 's/^/     /' 2>/dev/null || echo -e "${YELLOW}     Cannot read config${NC}"
    fi

    # Targets analysis
    echo -e "${BLUE}   Target Status:${NC}"
    if command -v jq >/dev/null 2>&1; then
        targets_json=$(curl -s "$PROMETHEUS_URL/api/v1/targets")

        total_targets=$(echo "$targets_json" | jq '.data.activeTargets | length' 2>/dev/null || echo "0")
        healthy_targets=$(echo "$targets_json" | jq '.data.activeTargets | map(select(.health == "up")) | length' 2>/dev/null || echo "0")

        echo -e "${GREEN}     Total targets: $total_targets${NC}"
        echo -e "${GREEN}     Healthy targets: $healthy_targets${NC}"

        if [ "$total_targets" -gt 0 ]; then
            echo -e "${BLUE}     Target details:${NC}"
            echo "$targets_json" | jq -r '.data.activeTargets[] | "       \(.labels.job):\(.labels.instance) - \(.health)"' 2>/dev/null | head -10
        fi

        # Check for MEVana-specific targets
        echo -e "${BLUE}     MEVana targets:${NC}"
        mevana_targets=$(echo "$targets_json" | jq -r '.data.activeTargets[] | select(.labels.job | test("hotpath|arb|normalizer|discovery")) | "       \(.labels.job):\(.labels.instance) - \(.health)"' 2>/dev/null)
        if [ -n "$mevana_targets" ]; then
            echo "$mevana_targets"
        else
            echo -e "${YELLOW}       No MEVana-specific targets found${NC}"
        fi
    else
        echo -e "${YELLOW}     Install jq for detailed target analysis${NC}"
    fi

    # Data retention and storage
    echo -e "${BLUE}   Storage Information:${NC}"
    if command -v jq >/dev/null 2>&1; then
        storage_info=$(curl -s "$PROMETHEUS_URL/api/v1/query?query=prometheus_tsdb_symbol_table_size_bytes" | jq -r '.data.result[0].value[1] // "unknown"' 2>/dev/null)
        echo -e "${GREEN}     Storage size: $storage_info bytes${NC}"
    fi
fi

echo ""

# =============================================================================
# GRAFANA ANALYSIS
# =============================================================================

echo -e "${CYAN}═══ GRAFANA DASHBOARD ANALYSIS ═══${NC}"

GRAFANA_PORTS=(3000 3300)
GRAFANA_URL=""

# Find Grafana
echo -e "${BLUE}📊 Grafana Discovery:${NC}"
for port in "${GRAFANA_PORTS[@]}"; do
    if curl -s --connect-timeout 3 "http://localhost:$port/api/health" >/dev/null 2>&1; then
        GRAFANA_URL="http://localhost:$port"
        echo -e "${GREEN}✅ Grafana found at port $port${NC}"
        break
    fi
done

if [ -z "$GRAFANA_URL" ]; then
    echo -e "${RED}❌ Grafana not accessible on standard ports${NC}"

    # Check if container exists
    if docker ps --format "{{.Names}}" | grep -q grafana; then
        grafana_container=$(docker ps --format "{{.Names}}" | grep grafana | head -1)
        echo -e "${YELLOW}⚠️  Grafana container '$grafana_container' exists but not accessible${NC}"
        echo -e "${BLUE}   Container details:${NC}"
        docker ps --filter "name=$grafana_container" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | sed 's/^/     /'
    fi
else
    echo -e "${BLUE}📈 Grafana Analysis:${NC}"

    # Health check
    health_status=$(curl -s "$GRAFANA_URL/api/health" | grep -o '"database":"ok"' | wc -l)
    if [ "$health_status" -gt 0 ]; then
        echo -e "${GREEN}   Health: Database OK${NC}"
    else
        echo -e "${YELLOW}   Health: Check needed${NC}"
    fi

    # Datasources
    echo -e "${BLUE}   Datasources:${NC}"
    if command -v jq >/dev/null 2>&1; then
        datasources=$(curl -s "$GRAFANA_URL/api/datasources" 2>/dev/null || echo "[]")
        ds_count=$(echo "$datasources" | jq 'length' 2>/dev/null || echo "0")
        echo -e "${GREEN}     Total datasources: $ds_count${NC}"

        if [ "$ds_count" -gt 0 ]; then
            echo "$datasources" | jq -r '.[] | "       \(.name) (\(.type)) - \(.url)"' 2>/dev/null | head -5

            # Check for Prometheus datasource
            prom_ds=$(echo "$datasources" | jq -r '.[] | select(.type == "prometheus") | .url' 2>/dev/null)
            if [ -n "$prom_ds" ] && [ "$prom_ds" != "null" ]; then
                echo -e "${GREEN}     ✅ Prometheus datasource configured: $prom_ds${NC}"
            else
                echo -e "${RED}     ❌ No Prometheus datasource found${NC}"
            fi
        fi
    fi

    # Dashboards
    echo -e "${BLUE}   Dashboards:${NC}"
    if command -v jq >/dev/null 2>&1; then
        dashboards=$(curl -s "$GRAFANA_URL/api/search?type=dash-db" 2>/dev/null || echo "[]")
        dash_count=$(echo "$dashboards" | jq 'length' 2>/dev/null || echo "0")
        echo -e "${GREEN}     Total dashboards: $dash_count${NC}"

        if [ "$dash_count" -gt 0 ]; then
            echo -e "${BLUE}     Dashboard list:${NC}"
            echo "$dashboards" | jq -r '.[] | "       • \(.title) (UID: \(.uid))"' 2>/dev/null | head -10

            # Check for MEVana-specific dashboards
            mevana_dashboards=$(echo "$dashboards" | jq -r '.[] | select(.title | test("(?i)mevana|hotpath|arb")) | "       🎯 \(.title) (UID: \(.uid))"' 2>/dev/null)
            if [ -n "$mevana_dashboards" ]; then
                echo -e "${BLUE}     MEVana dashboards:${NC}"
                echo "$mevana_dashboards"
            else
                echo -e "${YELLOW}     No MEVana-specific dashboards found${NC}"
            fi
        fi
    fi

    # Volume mounts (if container accessible)
    if docker ps --format "{{.Names}}" | grep -q grafana; then
        grafana_container=$(docker ps --format "{{.Names}}" | grep grafana | head -1)
        echo -e "${BLUE}   Volume Configuration:${NC}"

        # Check if dashboard directory is mounted
        dashboard_path=$(docker inspect "$grafana_container" | jq -r '.[0].Mounts[] | select(.Destination == "/etc/grafana/provisioning/dashboards") | .Source' 2>/dev/null || echo "not mounted")
        if [ "$dashboard_path" != "not mounted" ]; then
            echo -e "${GREEN}     ✅ Dashboard provisioning: $dashboard_path${NC}"

            # Check if path contains our working dashboard
            if [ -f "$dashboard_path/mevana-system-status.json" ]; then
                echo -e "${GREEN}       ✅ Working dashboard file exists${NC}"
            else
                echo -e "${YELLOW}       ⚠️  Working dashboard file missing${NC}"
            fi
        else
            echo -e "${RED}     ❌ Dashboard provisioning not mounted${NC}"
        fi

        datasource_path=$(docker inspect "$grafana_container" | jq -r '.[0].Mounts[] | select(.Destination == "/etc/grafana/provisioning/datasources") | .Source' 2>/dev/null || echo "not mounted")
        if [ "$datasource_path" != "not mounted" ]; then
            echo -e "${GREEN}     ✅ Datasource provisioning: $datasource_path${NC}"
        else
            echo -e "${RED}     ❌ Datasource provisioning not mounted${NC}"
        fi
    fi
fi

echo ""

# =============================================================================
# MEVANA SERVICES ANALYSIS
# =============================================================================

echo -e "${CYAN}═══ MEVANA SERVICES ANALYSIS ═══${NC}"

echo -e "${BLUE}🚀 MEVana Service Discovery:${NC}"

# Standard MEVana ports
MEVANA_SERVICES=(
    "hotpath-arb-engine:8088:/metrics"
    "hotpath-arb-engine:8089:/metrics"
    "normalizer-service:8085:/metrics"
    "normalizer-service:9901:/metrics"
    "discovery-service:9102:/metrics"
    "executor-service:9904:/metrics"
    "arb-engine:8085:/metrics"
    "monitoring-service:8091:/metrics"
)

for service_info in "${MEVANA_SERVICES[@]}"; do
    IFS=':' read -r service_name port endpoint <<< "$service_info"

    echo -en "${BLUE}   Checking $service_name:$port... ${NC}"

    if curl -s --connect-timeout 3 "http://localhost:$port$endpoint" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ RESPONDING${NC}"

        # Get some metrics if it's a metrics endpoint
        if [ "$endpoint" = "/metrics" ]; then
            metric_count=$(curl -s "http://localhost:$port$endpoint" | grep -c "^[a-zA-Z]" 2>/dev/null || echo "0")
            echo -e "${GREEN}     📊 $metric_count metrics available${NC}"

            # Check for specific MEVana metrics
            if curl -s "http://localhost:$port$endpoint" | grep -q "hotpath_"; then
                echo -e "${GREEN}     🎯 Contains MEVana-specific metrics${NC}"
            fi
        fi
    else
        echo -e "${RED}❌ NOT RESPONDING${NC}"

        # Check if process is running
        if ps aux | grep -v grep | grep -q "$service_name"; then
            echo -e "${YELLOW}     ⚠️  Process running but port not accessible${NC}"
        fi
    fi
done

echo ""

# =============================================================================
# CONFIGURATION ANALYSIS
# =============================================================================

echo -e "${CYAN}═══ CONFIGURATION ANALYSIS ═══${NC}"

echo -e "${BLUE}📋 Configuration Files:${NC}"

# Check for key configuration files
CONFIG_FILES=(
    "infrastructure/prometheus.yml"
    "infrastructure/grafana/datasources/prometheus.yml"
    "infrastructure/grafana/dashboards/curated/dashboard.yml"
    "infrastructure/grafana/dashboards/curated/mevana-system-status.json"
    "infrastructure/docker-compose.yml"
    "infrastructure/docker-compose.server.yml"
    ".env"
    "infrastructure/.env"
)

for config_file in "${CONFIG_FILES[@]}"; do
    echo -en "${BLUE}   $config_file... ${NC}"
    if [ -f "$config_file" ]; then
        echo -e "${GREEN}✅ EXISTS${NC}"

        # Additional analysis for key files
        case "$config_file" in
            "infrastructure/prometheus.yml")
                target_count=$(grep -c "job_name:" "$config_file" 2>/dev/null || echo "0")
                echo -e "${GREEN}     📊 $target_count scrape jobs configured${NC}"
                ;;
            "infrastructure/grafana/dashboards/curated/mevana-system-status.json")
                if grep -q "hotpath_multi_venue_transactions" "$config_file" 2>/dev/null; then
                    echo -e "${GREEN}     🎯 Contains arbitrage detection metrics${NC}"
                fi
                panel_count=$(grep -c '"id":' "$config_file" 2>/dev/null || echo "0")
                echo -e "${GREEN}     📊 $panel_count panels configured${NC}"
                ;;
            "infrastructure/docker-compose.yml")
                if grep -q "./infrastructure/grafana" "$config_file" 2>/dev/null; then
                    echo -e "${GREEN}     ✅ Correct volume paths${NC}"
                else
                    echo -e "${RED}     ❌ May have incorrect volume paths${NC}"
                fi
                ;;
        esac
    else
        echo -e "${RED}❌ MISSING${NC}"
    fi
done

# Check environment variables
echo -e "${BLUE}   Environment variables:${NC}"
if [ -f ".env" ] || [ -f "infrastructure/.env" ]; then
    echo -e "${GREEN}     ✅ Environment files found${NC}"

    # Check for critical variables (redacted)
    for env_file in ".env" "infrastructure/.env"; do
        if [ -f "$env_file" ]; then
            echo -e "${BLUE}     $env_file:${NC}"
            if grep -q "ENDPOINT=" "$env_file" 2>/dev/null; then
                endpoint_val=$(grep "ENDPOINT=" "$env_file" | cut -d'=' -f2)
                echo -e "${GREEN}       ENDPOINT: ${endpoint_val:0:20}...${NC}"
            fi
            if grep -q "X_TOKEN=" "$env_file" 2>/dev/null; then
                echo -e "${GREEN}       X_TOKEN: [REDACTED]${NC}"
            fi
        fi
    done
else
    echo -e "${YELLOW}     ⚠️  No environment files found${NC}"
fi

echo ""

# =============================================================================
# SUMMARY AND RECOMMENDATIONS
# =============================================================================

echo -e "${CYAN}═══ DISCOVERY SUMMARY ═══${NC}"

# Count issues
issues=0
recommendations=()

# Docker issues
if ! docker ps >/dev/null 2>&1; then
    ((issues++))
    recommendations+=("🐳 Fix Docker: Ensure Docker daemon is running")
fi

if ! docker network ls | grep -q "mevana_net"; then
    ((issues++))
    recommendations+=("🌐 Create network: docker network create mevana_net")
fi

# Monitoring issues
if [ -z "$PROMETHEUS_URL" ]; then
    ((issues++))
    recommendations+=("🎯 Start Prometheus: Check prometheus container/service")
fi

if [ -z "$GRAFANA_URL" ]; then
    ((issues++))
    recommendations+=("📊 Start Grafana: Check grafana container/service")
fi

# Configuration issues
if [ ! -f "infrastructure/grafana/dashboards/curated/mevana-system-status.json" ]; then
    ((issues++))
    recommendations+=("📋 Copy working dashboard: Sync from local development")
fi

if [ ! -f "infrastructure/prometheus.yml" ]; then
    ((issues++))
    recommendations+=("⚙️  Copy Prometheus config: Sync from local development")
fi

# Results
echo -e "${BLUE}📊 Discovery Results:${NC}"
echo -e "${GREEN}✅ Services discovered: $(curl -s --connect-timeout 1 http://localhost:8088/metrics >/dev/null 2>&1 && echo "MEVana services running" || echo "MEVana services need startup")${NC}"
echo -e "${GREEN}✅ Monitoring stack: $([ -n "$PROMETHEUS_URL" ] && [ -n "$GRAFANA_URL" ] && echo "Operational" || echo "Needs attention")${NC}"
echo -e "${GREEN}✅ Docker infrastructure: $(docker ps >/dev/null 2>&1 && echo "Running" || echo "Needs startup")${NC}"

if [ $issues -eq 0 ]; then
    echo -e "${GREEN}🎉 INFRASTRUCTURE READY FOR DEPLOYMENT!${NC}"
    echo -e "${BLUE}💡 Next step: Deploy with confidence using existing workflows${NC}"
else
    echo -e "${YELLOW}⚠️  $issues issues found that should be addressed${NC}"
    echo -e "${BLUE}🔧 Recommendations:${NC}"
    for rec in "${recommendations[@]}"; do
        echo -e "${YELLOW}   $rec${NC}"
    done
fi

echo ""
echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}🕐 Discovery completed at: $(date)${NC}"
echo -e "${BLUE}📝 Save this output for deployment planning${NC}"
echo -e "${CYAN}════════════════════════════════════════════════════════════${NC}"