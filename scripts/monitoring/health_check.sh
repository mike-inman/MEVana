#!/bin/bash

echo "🏥 MEVana Health Check"
echo "====================="
echo "Timestamp: $(date)"
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to check if a process is running
is_running() {
    local pid_file=$1
    local name=$2
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo -e "${GREEN}✅ $name: Running (PID: $pid)${NC}"
            return 0
        else
            echo -e "${RED}❌ $name: PID file exists but process dead${NC}"
            rm -f "$pid_file"
            return 1
        fi
    else
        echo -e "${RED}❌ $name: Not running (no PID file)${NC}"
        return 1
    fi
}

# Function to check HTTP endpoint
check_http() {
    local url=$1
    local name=$2
    if curl -s --max-time 5 "$url" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ $name: Responding${NC}"
        return 0
    else
        echo -e "${RED}❌ $name: Not responding${NC}"
        return 1
    fi
}

# Function to check network connection
check_network() {
    local host=$1
    local port=$2
    local name=$3
    if timeout 5 bash -c "echo > /dev/tcp/$host/$port" 2>/dev/null; then
        echo -e "${GREEN}✅ $name: Connected${NC}"
        return 0
    else
        echo -e "${RED}❌ $name: Connection failed${NC}"
        return 1
    fi
}

echo "🔍 Service Status:"
echo "=================="

# Check MEVana services
SERVICES_OK=0
TOTAL_SERVICES=0

# Discovery Stream
TOTAL_SERVICES=$((TOTAL_SERVICES + 1))
if is_running "pids/discovery.pid" "Discovery Stream"; then
    SERVICES_OK=$((SERVICES_OK + 1))
fi

# Arbitrage Engine
TOTAL_SERVICES=$((TOTAL_SERVICES + 1))
if is_running "pids/arb-engine.pid" "Arbitrage Engine"; then
    SERVICES_OK=$((SERVICES_OK + 1))
    
    # Check HTTP API
    ARB_HTTP_PORT=${ARB_HTTP_PORT:-8081}
    check_http "http://localhost:$ARB_HTTP_PORT/ai/config" "Arbitrage HTTP API"
fi

# Executor Service (if running)
if [ -f "pids/executor.pid" ]; then
    TOTAL_SERVICES=$((TOTAL_SERVICES + 1))
    if is_running "pids/executor.pid" "Executor Service"; then
        SERVICES_OK=$((SERVICES_OK + 1))
    fi
fi

echo ""
echo "🐳 Infrastructure Status:"
echo "========================"

# Check Docker services
if command -v docker &> /dev/null; then
    echo "Docker Services:"
    cd infrastructure 2>/dev/null || cd .
    if [ -f "docker-compose.prod.yml" ]; then
        docker-compose -f docker-compose.prod.yml ps 2>/dev/null || docker-compose ps 2>/dev/null || echo "No docker-compose found"
    else
        docker-compose ps 2>/dev/null || echo "No docker-compose found"
    fi
    cd - > /dev/null
else
    echo -e "${RED}❌ Docker not available${NC}"
fi

echo ""
echo "Network Connectivity:"

# Check Kafka
check_network "localhost" "29092" "Kafka"

# Check Redis  
check_network "localhost" "6379" "Redis"

# Check PostgreSQL
check_network "localhost" "5432" "PostgreSQL"

# Check external dependencies
echo ""
echo "External Dependencies:"
check_network "api.mainnet-beta.solana.com" "443" "Solana Mainnet"
check_network "rpc.shyft.to" "443" "Shyft.to RPC"

echo ""
echo "📊 Performance Metrics:"
echo "======================"

# System resources
echo "System Resources:"
echo "  CPU Load: $(uptime | awk -F'load average:' '{ print $2 }' | xargs)"
echo "  Memory Usage: $(free -h | grep Mem | awk '{printf "%.1f%% (%s used / %s total)", $3/$2*100, $3, $2}')"
echo "  Disk Usage: $(df -h / | tail -1 | awk '{printf "%s used / %s total (%s)", $3, $2, $5}')"

# Network latency check
echo ""
echo "Network Performance:"
if command -v ping &> /dev/null; then
    SOLANA_LATENCY=$(ping -c 1 api.mainnet-beta.solana.com 2>/dev/null | grep 'time=' | awk -F'time=' '{print $2}' | awk '{print $1}' || echo "N/A")
    echo "  Solana RPC Latency: ${SOLANA_LATENCY}"
    
    if [[ "$SOLANA_LATENCY" != "N/A" ]]; then
        LATENCY_NUM=$(echo $SOLANA_LATENCY | grep -o '[0-9.]*')
        if (( $(echo "$LATENCY_NUM < 10" | bc -l 2>/dev/null || echo 0) )); then
            echo -e "    ${GREEN}✅ Excellent (<10ms)${NC}"
        elif (( $(echo "$LATENCY_NUM < 20" | bc -l 2>/dev/null || echo 0) )); then
            echo -e "    ${YELLOW}⚠️  Good (<20ms)${NC}"
        else
            echo -e "    ${RED}❌ Poor (>20ms)${NC}"
        fi
    fi
fi

echo ""
echo "📈 Recent Activity:"
echo "=================="

# Discovery stream activity
if [ -f "logs/discovery.log" ]; then
    DISCOVERY_RECENT=$(tail -50 logs/discovery.log | grep -c "EdgeDelta\|processed\|published" 2>/dev/null || echo 0)
    DISCOVERY_ERRORS=$(tail -50 logs/discovery.log | grep -c "error\|Error\|ERROR" 2>/dev/null || echo 0)
    
    echo "Discovery Stream (last 50 log lines):"
    if [ "$DISCOVERY_RECENT" -gt 0 ]; then
        echo -e "  ${GREEN}✅ $DISCOVERY_RECENT EdgeDelta messages${NC}"
    else
        echo -e "  ${YELLOW}⚠️  No recent EdgeDelta messages${NC}"
    fi
    
    if [ "$DISCOVERY_ERRORS" -gt 0 ]; then
        echo -e "  ${RED}❌ $DISCOVERY_ERRORS error messages${NC}"
    fi
    
    # Check last activity timestamp
    LAST_ACTIVITY=$(tail -1 logs/discovery.log 2>/dev/null | head -1 || echo "No logs")
    echo "  Last activity: $LAST_ACTIVITY"
else
    echo -e "${YELLOW}⚠️  No discovery stream logs found${NC}"
fi

echo ""

# Arbitrage engine activity
if [ -f "logs/arb-engine.log" ]; then
    ARB_PROCESSED=$(tail -50 logs/arb-engine.log | grep -c "EdgeDelta.*processed" 2>/dev/null || echo 0)
    ARB_OPPORTUNITIES=$(tail -50 logs/arb-engine.log | grep -c "opportunity" 2>/dev/null || echo 0)
    ARB_IDLE=$(tail -10 logs/arb-engine.log | grep -c "kafka_idle_no_messages_yet" 2>/dev/null || echo 0)
    ARB_ERRORS=$(tail -50 logs/arb-engine.log | grep -c "error\|Error\|ERROR" 2>/dev/null || echo 0)
    
    echo "Arbitrage Engine (last 50 log lines):"
    if [ "$ARB_PROCESSED" -gt 0 ]; then
        echo -e "  ${GREEN}✅ $ARB_PROCESSED EdgeDeltas processed${NC}"
    else
        echo -e "  ${YELLOW}⚠️  No EdgeDeltas processed recently${NC}"
    fi
    
    if [ "$ARB_OPPORTUNITIES" -gt 0 ]; then
        echo -e "  ${GREEN}✅ $ARB_OPPORTUNITIES arbitrage opportunities found${NC}"
    else
        echo -e "  ${BLUE}ℹ️  No recent arbitrage opportunities${NC}"
    fi
    
    if [ "$ARB_IDLE" -gt 0 ]; then
        echo -e "  ${YELLOW}⚠️  Kafka idle messages detected (data flow issue)${NC}"
    fi
    
    if [ "$ARB_ERRORS" -gt 0 ]; then
        echo -e "  ${RED}❌ $ARB_ERRORS error messages${NC}"
    fi
    
    # Check last activity timestamp
    LAST_ARB_ACTIVITY=$(tail -1 logs/arb-engine.log 2>/dev/null | head -1 || echo "No logs")
    echo "  Last activity: $LAST_ARB_ACTIVITY"
else
    echo -e "${YELLOW}⚠️  No arbitrage engine logs found${NC}"
fi

echo ""
echo "💾 Log File Sizes:"
echo "=================="
if [ -d "logs" ]; then
    ls -lh logs/ 2>/dev/null | grep -v '^total' | awk '{printf "  %s: %s (modified %s %s)\n", $9, $5, $6, $7}'
else
    echo "  No logs directory found"
fi

echo ""
echo "🔧 Quick Diagnostics:"
echo "===================="

# Overall health score
HEALTH_SCORE=$((SERVICES_OK * 100 / TOTAL_SERVICES))
echo "Service Health Score: $SERVICES_OK/$TOTAL_SERVICES services running ($HEALTH_SCORE%)"

# Recommendations
echo ""
echo "💡 Recommendations:"
echo "=================="

if [ "$SERVICES_OK" -eq "$TOTAL_SERVICES" ]; then
    echo -e "${GREEN}✅ All services healthy - no action needed${NC}"
else
    echo -e "${YELLOW}⚠️  Some services need attention:${NC}"
    echo "   - Run './start_services.sh' to restart failed services"
    echo "   - Check logs in logs/ directory for error details"
fi

# Check for data flow issues
if [ -f "logs/arb-engine.log" ] && tail -10 logs/arb-engine.log | grep -q "kafka_idle_no_messages_yet" 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Data flow issue detected:${NC}"
    echo "   - Discovery stream may not be publishing EdgeDeltas"
    echo "   - Check logs/discovery.log for connection issues"
    echo "   - Consider restarting services: ./start_services.sh"
fi

# Performance warnings
if command -v ping &> /dev/null && [[ "$SOLANA_LATENCY" != "N/A" ]]; then
    LATENCY_NUM=$(echo $SOLANA_LATENCY | grep -o '[0-9.]*')
    if (( $(echo "$LATENCY_NUM > 20" | bc -l 2>/dev/null || echo 0) )); then
        echo -e "${RED}❌ High latency detected (${LATENCY_NUM}ms):${NC}"
        echo "   - Consider deploying to a datacenter closer to Solana infrastructure"
        echo "   - Run './scripts/testing/latency_test.sh' for detailed network analysis"
    fi
fi

echo ""
echo "📋 Next Steps:"
echo "=============="
echo "   - Monitor live: ./monitor.sh"
echo "   - View logs: tail -f logs/*.log"  
echo "   - Restart services: ./start_services.sh"
echo "   - Stop all: ./stop_services.sh"
echo ""
echo "Health check completed at $(date)"
