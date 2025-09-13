#!/bin/bash

# MEVana Quick Health Check
# Run this anytime to verify the system is working

echo "🔍 MEVana Quick Health Check"
echo "=========================="
echo "$(date)"
echo ""

# 1. Check critical processes
echo "📊 PROCESS STATUS:"
HOTPATH_PID=$(ps aux | grep hotpath_arb_engine | grep -v grep | awk '{print $2}' | head -1)
ORCHESTRATOR_PID=$(ps aux | grep system-orchestrator | grep -v grep | awk '{print $2}' | head -1)

if [ ! -z "$HOTPATH_PID" ]; then
    echo "✅ hotpath-arb-engine: RUNNING (PID: $HOTPATH_PID)"
else
    echo "❌ hotpath-arb-engine: NOT RUNNING"
fi

if [ ! -z "$ORCHESTRATOR_PID" ]; then
    echo "✅ system-orchestrator: RUNNING (PID: $ORCHESTRATOR_PID)"
else
    echo "❌ system-orchestrator: NOT RUNNING"
fi
echo ""

# 2. Check system orchestrator status
echo "🌐 SYSTEM STATUS:"
SYSTEM_STATUS=$(curl -s http://localhost:9999/status 2>/dev/null)
if [ $? -eq 0 ]; then
    echo "✅ System orchestrator API: RESPONDING"
    echo "   Status: $SYSTEM_STATUS"
else
    echo "❌ System orchestrator API: NOT RESPONDING"
fi
echo ""

# 3. Check hotpath-arb-engine API
echo "⚡ HOTPATH ENGINE STATUS:"
HOTPATH_STATUS=$(curl -s http://localhost:8088/performance 2>/dev/null | head -3)
if [ $? -eq 0 ]; then
    echo "✅ hotpath-arb-engine API: RESPONDING"
else
    echo "❌ hotpath-arb-engine API: NOT RESPONDING"
fi
echo ""

# 4. Check Docker infrastructure
echo "🐳 INFRASTRUCTURE STATUS:"
DOCKER_SERVICES=$(docker compose -f infrastructure/docker-compose.yml ps --format "table {{.Service}}\t{{.Status}}" 2>/dev/null)
if [ $? -eq 0 ]; then
    echo "✅ Docker infrastructure: RUNNING"
    echo "$DOCKER_SERVICES" | grep -E "(postgres|kafka|redis)" | head -3
else
    echo "❌ Docker infrastructure: CHECK NEEDED"
fi
echo ""

# 5. Check database connectivity and recent opportunities
echo "💾 DATABASE STATUS:"
DB_CHECK=$(docker exec postgres psql -U mev -d mevdb -c "SELECT 1" -t -q 2>/dev/null)
if [ $? -eq 0 ]; then
    echo "✅ Database: CONNECTED"
    
    # Check recent opportunities
    RECENT_OPPS=$(docker exec postgres psql -U mev -d mevdb -c "SELECT COUNT(*) FROM arbitrage_opportunities WHERE timestamp > NOW() - INTERVAL '10 minutes';" -t -q 2>/dev/null | xargs)
    TOTAL_OPPS=$(docker exec postgres psql -U mev -d mevdb -c "SELECT COUNT(*) FROM arbitrage_opportunities;" -t -q 2>/dev/null | xargs)
    
    echo "   Recent opportunities (10 min): $RECENT_OPPS"
    echo "   Total opportunities: $TOTAL_OPPS"
    
    if [ "$RECENT_OPPS" -gt "0" ]; then
        echo "✅ Data flow: ACTIVE"
    else
        echo "⚠️  Data flow: NO RECENT DATA"
    fi
else
    echo "❌ Database: CONNECTION FAILED"
fi
echo ""

# 6. Check filter settings
echo "🎛️  FILTER SETTINGS:"
FILTER_SETTINGS=$(docker exec postgres psql -U mev -d mevdb -c "SELECT parameter_name, new_value FROM current_smart_filter_config WHERE parameter_name IN ('min_profit_bps', 'min_net_profit_sol') ORDER BY parameter_name;" -t 2>/dev/null)
if [ $? -eq 0 ]; then
    echo "✅ Filter configuration loaded:"
    echo "$FILTER_SETTINGS" | sed 's/^/   /'
else
    echo "❌ Filter configuration: CHECK NEEDED"
fi
echo ""

# 7. Check service crashes
echo "⚠️  SERVICE STATUS:"
CRASHED_SERVICES=$(ps aux | grep -E "(normalizer-service|edge-assembler|arb-engine|executor-service|token-registry-service|monitoring-service)" | grep -v grep | wc -l | xargs)
echo "   Active optional services: $CRASHED_SERVICES/6"
if [ "$CRASHED_SERVICES" -lt "3" ]; then
    echo "   ℹ️  Some services not running (may be normal - under development)"
fi

# 8. Overall assessment
echo ""
echo "📋 QUICK ASSESSMENT:"
if [ ! -z "$HOTPATH_PID" ] && [ "$RECENT_OPPS" -gt "0" ]; then
    echo "✅ SYSTEM HEALTHY: Data is flowing normally"
elif [ ! -z "$HOTPATH_PID" ] && [ "$RECENT_OPPS" -eq "0" ]; then
    echo "⚠️  SYSTEM RUNNING: No recent opportunities (could be normal market conditions)"
else
    echo "❌ SYSTEM ISSUE: Core services not running properly"
fi

echo ""
echo "🔄 To run again: ./quick_health_check.sh"
echo "📊 For continuous monitoring: watch -n 30 ./quick_health_check.sh"