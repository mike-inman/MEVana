#!/bin/bash

echo "🛑 Stopping MEVana Services"
echo "==========================="
echo "Timestamp: $(date)"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Function to stop a service gracefully
stop_service() {
    local pid_file=$1
    local service_name=$2
    local timeout=${3:-10}
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo "🔄 Stopping $service_name (PID: $pid)..."
            
            # Try graceful shutdown first
            kill -TERM "$pid" 2>/dev/null
            
            # Wait for graceful shutdown
            local count=0
            while [ $count -lt $timeout ] && kill -0 "$pid" 2>/dev/null; do
                sleep 1
                count=$((count + 1))
                if [ $((count % 3)) -eq 0 ]; then
                    echo "   ... waiting for graceful shutdown ($count/${timeout}s)"
                fi
            done
            
            # Force kill if still running
            if kill -0 "$pid" 2>/dev/null; then
                echo "   ⚠️  Graceful shutdown timeout, forcing termination..."
                kill -KILL "$pid" 2>/dev/null
                sleep 1
            fi
            
            # Verify process is stopped
            if ! kill -0 "$pid" 2>/dev/null; then
                echo -e "   ${GREEN}✅ $service_name stopped successfully${NC}"
                rm -f "$pid_file"
            else
                echo -e "   ${RED}❌ Failed to stop $service_name${NC}"
                return 1
            fi
        else
            echo "🗑️  Removing stale PID file for $service_name"
            rm -f "$pid_file"
        fi
    else
        echo -e "${YELLOW}ℹ️  $service_name was not running (no PID file)${NC}"
    fi
    return 0
}

# Function to kill processes by name (backup cleanup)
cleanup_by_name() {
    local process_name=$1
    local service_name=$2
    
    local pids=$(pgrep -f "$process_name" 2>/dev/null)
    if [ -n "$pids" ]; then
        echo "🧹 Cleaning up remaining $service_name processes..."
        for pid in $pids; do
            echo "   Killing PID: $pid"
            kill -TERM "$pid" 2>/dev/null || true
        done
        sleep 2
        
        # Force kill any remaining
        local remaining_pids=$(pgrep -f "$process_name" 2>/dev/null)
        if [ -n "$remaining_pids" ]; then
            echo "   Force killing remaining processes..."
            for pid in $remaining_pids; do
                kill -KILL "$pid" 2>/dev/null || true
            done
        fi
    fi
}

echo "🎯 Stopping MEVana Services:"
echo "============================="

# Stop services in reverse order (opposite of startup)

# 1. Stop Executor Service (if running)
if [ -f "pids/executor.pid" ]; then
    stop_service "pids/executor.pid" "Executor Service" 15
fi

# 2. Stop Arbitrage Engine
if [ -f "pids/arb-engine.pid" ]; then
    stop_service "pids/arb-engine.pid" "Arbitrage Engine" 15
fi

# 3. Stop Discovery Stream
if [ -f "pids/discovery.pid" ]; then
    stop_service "pids/discovery.pid" "Discovery Stream" 10
fi

echo ""
echo "🧹 Additional Cleanup:"
echo "======================"

# Cleanup any remaining processes by name
cleanup_by_name "arb-engine" "Arbitrage Engine"
cleanup_by_name "whirlpool-live-stream" "Discovery Stream"  
cleanup_by_name "executor-service" "Executor Service"

# Stop any cargo run processes for MEVana
CARGO_PIDS=$(pgrep -f "cargo.*arb-engine\|cargo.*whirlpool\|cargo.*executor" 2>/dev/null)
if [ -n "$CARGO_PIDS" ]; then
    echo "🧹 Stopping cargo processes..."
    for pid in $CARGO_PIDS; do
        echo "   Stopping cargo PID: $pid"
        kill -TERM "$pid" 2>/dev/null || true
    done
    sleep 3
    
    # Force kill cargo processes if still running
    REMAINING_CARGO=$(pgrep -f "cargo.*arb-engine\|cargo.*whirlpool\|cargo.*executor" 2>/dev/null)
    if [ -n "$REMAINING_CARGO" ]; then
        for pid in $REMAINING_CARGO; do
            kill -KILL "$pid" 2>/dev/null || true
        done
    fi
fi

echo ""
echo "🔍 Verification:"
echo "================"

# Verify all processes are stopped
REMAINING_PROCESSES=$(pgrep -f "arb-engine\|whirlpool-live-stream\|executor-service" 2>/dev/null)
if [ -z "$REMAINING_PROCESSES" ]; then
    echo -e "${GREEN}✅ All MEVana services stopped successfully${NC}"
else
    echo -e "${YELLOW}⚠️  Some processes may still be running:${NC}"
    ps aux | grep -E "(arb-engine|whirlpool|executor)" | grep -v grep
fi

# Clean up any remaining PID files
echo ""
echo "🗑️  Cleanup:"
echo "============"

if [ -d "pids" ]; then
    PID_FILES=$(find pids/ -name "*.pid" 2>/dev/null)
    if [ -n "$PID_FILES" ]; then
        echo "Removing PID files..."
        rm -f pids/*.pid
        echo -e "${GREEN}✅ PID files cleaned${NC}"
    else
        echo "No PID files to clean"
    fi
else
    echo "No pids directory found"
fi

# Optional: Stop infrastructure services
echo ""
echo "🐳 Infrastructure Services:"
echo "=========================="
read -p "Do you want to stop infrastructure services (Kafka, Redis, etc.)? [y/N]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Stopping infrastructure services..."
    cd infrastructure 2>/dev/null || cd .
    
    if [ -f "docker-compose.prod.yml" ]; then
        docker-compose -f docker-compose.prod.yml down
    elif [ -f "docker-compose.yml" ]; then
        docker-compose down
    else
        echo "No docker-compose configuration found"
    fi
    cd - > /dev/null
    echo -e "${GREEN}✅ Infrastructure services stopped${NC}"
else
    echo "Infrastructure services left running"
fi

echo ""
echo "📊 Final Status:"
echo "================"

# Check for any remaining MEVana processes
FINAL_CHECK=$(pgrep -f "arb-engine\|whirlpool\|executor" 2>/dev/null | wc -l)
if [ "$FINAL_CHECK" -eq 0 ]; then
    echo -e "${GREEN}✅ Clean shutdown - no MEVana processes running${NC}"
else
    echo -e "${YELLOW}⚠️  Warning: $FINAL_CHECK MEVana processes still running${NC}"
    echo "You may need to manually kill remaining processes:"
    ps aux | grep -E "(arb-engine|whirlpool|executor)" | grep -v grep
fi

# Log file preservation notice
echo ""
echo "📁 Log Files:"
echo "============="
if [ -d "logs" ]; then
    LOG_COUNT=$(ls logs/*.log 2>/dev/null | wc -l)
    if [ "$LOG_COUNT" -gt 0 ]; then
        echo "Log files preserved in logs/ directory:"
        ls -la logs/*.log 2>/dev/null | awk '{printf "   %s %s %s\n", $9, $5, $6" "$7}'
    else
        echo "No log files found"
    fi
else
    echo "No logs directory found"
fi

echo ""
echo "🎯 Stop Services Complete!"
echo "=========================="
echo "All MEVana services have been stopped."
echo ""
echo "To start services again:"
echo "   ./start_services.sh"
echo ""
echo "To check what's still running:"
echo "   ./health_check.sh"
echo ""
echo "Shutdown completed at $(date)"