#!/bin/bash

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Configuration
ARB_HTTP_PORT=${ARB_HTTP_PORT:-8081}
REFRESH_INTERVAL=30

# Function to get service status
get_service_status() {
    local pid_file=$1
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo -e "${GREEN}●${NC}"
        else
            echo -e "${RED}●${NC}"
        fi
    else
        echo -e "${RED}●${NC}"
    fi
}

# Function to get CPU and memory for a PID
get_resource_usage() {
    local pid_file=$1
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            local stats=$(ps -p $pid -o %cpu,%mem --no-headers 2>/dev/null)
            if [ -n "$stats" ]; then
                echo "$stats"
            else
                echo "0.0  0.0"
            fi
        else
            echo "0.0  0.0"
        fi
    else
        echo "0.0  0.0"
    fi
}

# Function to get recent log activity
get_log_activity() {
    local log_file=$1
    local pattern=$2
    local lines=${3:-20}
    
    if [ -f "$log_file" ]; then
        tail -$lines "$log_file" | grep -c "$pattern" 2>/dev/null || echo 0
    else
        echo 0
    fi
}

# Function to get last log timestamp
get_last_activity() {
    local log_file=$1
    if [ -f "$log_file" ]; then
        tail -1 "$log_file" 2>/dev/null | grep -o '[0-9]*-[0-9]*-[0-9]*T[0-9]*:[0-9]*:[0-9]*\.[0-9]*' | head -1 || echo "N/A"
    else
        echo "N/A"
    fi
}

# Function to format bytes to human readable
format_bytes() {
    local bytes=$1
    if [ -z "$bytes" ] || [ "$bytes" -eq 0 ] 2>/dev/null; then
        echo "0B"
    else
        echo $bytes | awk '
        function human(x) {
            if (x<1024) {return x "B"}
            else if (x<1048576) {return sprintf("%.1fKB", x/1024)}
            else if (x<1073741824) {return sprintf("%.1fMB", x/1048576)}
            else if (x<1099511627776) {return sprintf("%.1fGB", x/1073741824)}
            else {return sprintf("%.1fTB", x/1099511627776)}
        }
        {print human($1)}
        '
    fi
}

# Main monitoring loop
echo -e "${CYAN}🚀 MEVana Live Performance Monitor${NC}"
echo -e "${CYAN}===================================${NC}"
echo "Press Ctrl+C to exit"
echo ""

while true; do
    clear
    
    # Header with timestamp
    echo -e "${CYAN}📈 MEVana Live Performance Monitor${NC}"
    echo -e "${CYAN}===================================${NC}"
    echo "$(date): Live Status Update every ${REFRESH_INTERVAL}s"
    echo ""
    
    # Service Status Section
    echo -e "${YELLOW}🔧 SERVICE STATUS${NC}"
    echo "=================="
    
    # Discovery Stream
    DISCOVERY_STATUS=$(get_service_status "pids/discovery.pid")
    DISCOVERY_RESOURCES=$(get_resource_usage "pids/discovery.pid")
    echo -e "$DISCOVERY_STATUS Discovery Stream     CPU: ${DISCOVERY_RESOURCES%% *}%  MEM: ${DISCOVERY_RESOURCES##* }%"
    
    # Arbitrage Engine
    ARB_STATUS=$(get_service_status "pids/arb-engine.pid")
    ARB_RESOURCES=$(get_resource_usage "pids/arb-engine.pid")
    echo -e "$ARB_STATUS Arbitrage Engine   CPU: ${ARB_RESOURCES%% *}%  MEM: ${ARB_RESOURCES##* }%"
    
    # HTTP API Status
    if curl -s --max-time 2 "http://localhost:$ARB_HTTP_PORT/ai/config" > /dev/null 2>&1; then
        echo -e "${GREEN}● HTTP API${NC}           Port: $ARB_HTTP_PORT (responding)"
    else
        echo -e "${RED}● HTTP API${NC}           Port: $ARB_HTTP_PORT (not responding)"
    fi
    
    echo ""
    
    # Performance Metrics Section
    echo -e "${YELLOW}⚡ PERFORMANCE METRICS${NC}"
    echo "======================"
    
    # System Resources
    LOAD_AVG=$(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | tr -d ',')
    MEM_USAGE=$(free | grep Mem | awk '{printf "%.1f", $3/$2*100}')
    DISK_USAGE=$(df -h / | tail -1 | awk '{print $5}' | tr -d '%')
    
    echo -e "System Load:     ${LOAD_AVG}"
    echo -e "Memory Usage:    ${MEM_USAGE}%"
    echo -e "Disk Usage:      ${DISK_USAGE}%"
    
    # Network Latency (quick check)
    if command -v ping &> /dev/null; then
        PING_RESULT=$(timeout 3 ping -c 1 api.mainnet-beta.solana.com 2>/dev/null | grep 'time=' | awk -F'time=' '{print $2}' | awk '{print $1}' || echo "timeout")
        if [ "$PING_RESULT" != "timeout" ]; then
            echo -e "Solana Latency:  ${PING_RESULT}"
        else
            echo -e "Solana Latency:  ${RED}timeout${NC}"
        fi
    fi
    
    echo ""
    
    # Trading Activity Section
    echo -e "${YELLOW}📊 TRADING ACTIVITY (Last 30 seconds)${NC}"
    echo "======================================"
    
    # Discovery Stream Stats
    if [ -f "logs/discovery.log" ]; then
        EDGE_DELTAS=$(get_log_activity "logs/discovery.log" "EdgeDelta\|published" 10)
        DISCOVERY_ERRORS=$(get_log_activity "logs/discovery.log" "error\|Error\|ERROR" 10)
        LAST_DISCOVERY=$(get_last_activity "logs/discovery.log")
        
        echo -e "📡 Discovery Stream:"
        if [ "$EDGE_DELTAS" -gt 0 ]; then
            echo -e "   EdgeDeltas:      ${GREEN}$EDGE_DELTAS messages${NC}"
        else
            echo -e "   EdgeDeltas:      ${YELLOW}0 messages${NC}"
        fi
        
        if [ "$DISCOVERY_ERRORS" -gt 0 ]; then
            echo -e "   Errors:          ${RED}$DISCOVERY_ERRORS${NC}"
        else
            echo -e "   Errors:          ${GREEN}0${NC}"
        fi
        
        echo -e "   Last Activity:   $LAST_DISCOVERY"
    else
        echo -e "📡 Discovery Stream: ${RED}No logs${NC}"
    fi
    
    echo ""
    
    # Arbitrage Engine Stats
    if [ -f "logs/arb-engine.log" ]; then
        ARB_PROCESSED=$(get_log_activity "logs/arb-engine.log" "EdgeDelta.*processed" 20)
        ARB_OPPORTUNITIES=$(get_log_activity "logs/arb-engine.log" "opportunity.*detected\|ArbitrageOpportunity" 20)
        ARB_IDLE=$(get_log_activity "logs/arb-engine.log" "kafka_idle_no_messages_yet" 5)
        ARB_ERRORS=$(get_log_activity "logs/arb-engine.log" "error\|Error\|ERROR" 10)
        LAST_ARB=$(get_last_activity "logs/arb-engine.log")
        
        echo -e "🤖 Arbitrage Engine:"
        if [ "$ARB_PROCESSED" -gt 0 ]; then
            echo -e "   EdgeDeltas:      ${GREEN}$ARB_PROCESSED processed${NC}"
        else
            echo -e "   EdgeDeltas:      ${YELLOW}0 processed${NC}"
        fi
        
        if [ "$ARB_OPPORTUNITIES" -gt 0 ]; then
            echo -e "   Opportunities:   ${GREEN}$ARB_OPPORTUNITIES found${NC}"
        else
            echo -e "   Opportunities:   ${BLUE}0 found${NC}"
        fi
        
        if [ "$ARB_IDLE" -gt 0 ]; then
            echo -e "   Status:          ${YELLOW}Kafka idle ($ARB_IDLE msgs)${NC}"
        else
            echo -e "   Status:          ${GREEN}Active${NC}"
        fi
        
        if [ "$ARB_ERRORS" -gt 0 ]; then
            echo -e "   Errors:          ${RED}$ARB_ERRORS${NC}"
        else
            echo -e "   Errors:          ${GREEN}0${NC}"
        fi
        
        echo -e "   Last Activity:   $LAST_ARB"
    else
        echo -e "🤖 Arbitrage Engine: ${RED}No logs${NC}"
    fi
    
    echo ""
    
    # Infrastructure Status
    echo -e "${YELLOW}🐳 INFRASTRUCTURE${NC}"
    echo "================="
    
    # Kafka
    if echo "exit" | timeout 2 nc localhost 29092 2>/dev/null; then
        echo -e "${GREEN}●${NC} Kafka           localhost:29092"
    else
        echo -e "${RED}●${NC} Kafka           localhost:29092 (disconnected)"
    fi
    
    # Redis
    if timeout 2 redis-cli ping > /dev/null 2>&1; then
        # Get Redis memory usage
        REDIS_MEM=$(timeout 2 redis-cli info memory 2>/dev/null | grep used_memory_human | cut -d: -f2 | tr -d '\r' || echo "N/A")
        echo -e "${GREEN}●${NC} Redis           localhost:6379 (mem: $REDIS_MEM)"
    else
        echo -e "${RED}●${NC} Redis           localhost:6379 (disconnected)"
    fi
    
    # PostgreSQL
    if timeout 2 bash -c "echo > /dev/tcp/localhost/5432" 2>/dev/null; then
        echo -e "${GREEN}●${NC} PostgreSQL      localhost:5432"
    else
        echo -e "${RED}●${NC} PostgreSQL      localhost:5432 (disconnected)"
    fi
    
    echo ""
    
    # Log File Sizes
    echo -e "${YELLOW}📁 LOG FILES${NC}"
    echo "============"
    if [ -d "logs" ]; then
        for log in logs/*.log; do
            if [ -f "$log" ]; then
                SIZE=$(stat -c%s "$log" 2>/dev/null || stat -f%z "$log" 2>/dev/null || echo 0)
                FORMATTED_SIZE=$(format_bytes $SIZE)
                BASENAME=$(basename "$log")
                printf "%-20s %8s\n" "$BASENAME" "$FORMATTED_SIZE"
            fi
        done
    else
        echo "No logs directory"
    fi
    
    echo ""
    
    # Quick Actions
    echo -e "${YELLOW}🎮 QUICK ACTIONS${NC}"
    echo "================="
    echo "h - Health Check    r - Restart Services    s - Stop Services"
    echo "l - View Logs       c - Check Config        q - Quit Monitor"
    
    echo ""
    echo -e "${MAGENTA}💡 Tip: For detailed analysis run './health_check.sh'${NC}"
    echo -e "${BLUE}🔄 Next update in ${REFRESH_INTERVAL} seconds... (Ctrl+C to exit)${NC}"
    
    # Wait with timeout, allowing for user input
    if read -t $REFRESH_INTERVAL -n 1 action 2>/dev/null; then
        case $action in
            h|H)
                echo ""
                echo "Running health check..."
                ./health_check.sh
                echo ""
                echo "Press any key to return to monitor..."
                read -n 1
                ;;
            r|R)
                echo ""
                echo "Restarting services..."
                ./start_services.sh
                echo ""
                echo "Press any key to return to monitor..."
                read -n 1
                ;;
            s|S)
                echo ""
                echo "Stopping services..."
                ./stop_services.sh
                echo ""
                echo "Press any key to return to monitor..."
                read -n 1
                ;;
            l|L)
                echo ""
                echo "Recent arbitrage engine logs:"
                tail -20 logs/arb-engine.log 2>/dev/null || echo "No logs available"
                echo ""
                echo "Press any key to return to monitor..."
                read -n 1
                ;;
            c|C)
                echo ""
                echo "Current configuration:"
                curl -s "http://localhost:$ARB_HTTP_PORT/ai/config" 2>/dev/null | head -20 || echo "Config endpoint not available"
                echo ""
                echo "Press any key to return to monitor..."
                read -n 1
                ;;
            q|Q)
                echo ""
                echo "Exiting monitor..."
                exit 0
                ;;
        esac
    fi
done