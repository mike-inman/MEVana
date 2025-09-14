#!/bin/bash

# MEVana Instance Conflict Detector
# Checks for multiple instances that could trigger Geyser endpoint monitoring

echo "🔍 MEVana Instance Conflict Check"
echo "=================================="
echo ""

# Check for active processes on MEVana ports
echo "📊 Checking active services on MEVana ports..."
ACTIVE_SERVICES=$(lsof -i :8081,8088,8090,8091,8095 2>/dev/null | grep LISTEN)
ACTIVE_COUNT=$(echo "$ACTIVE_SERVICES" | wc -l)

if [ "$ACTIVE_COUNT" -eq 0 ]; then
    echo "✅ No active MEVana services found"
    echo "   Safe to start new instance"
elif [ "$ACTIVE_COUNT" -eq 1 ]; then
    echo "✅ Single MEVana service running (safe)"
    echo "$ACTIVE_SERVICES"
else
    echo "⚠️  WARNING: Multiple MEVana services detected ($ACTIVE_COUNT)"
    echo "   This could trigger Geyser endpoint abuse detection!"
    echo ""
    echo "🔌 Active services:"
    echo "$ACTIVE_SERVICES"
    echo ""
    echo "🚨 RISK: Multiple instances hitting same endpoints may cause:"
    echo "   - Endpoint throttling/blocking"
    echo "   - Stuck at old blockchain slots"
    echo "   - Service degradation"
fi

echo ""

# Check for cargo run processes
echo "🔧 Checking for cargo run processes..."
CARGO_PROCESSES=$(ps aux | grep "cargo run" | grep -E "(hotpath|arb-engine|system-orchestrator)" | grep -v grep)
CARGO_COUNT=$(echo "$CARGO_PROCESSES" | grep -v '^$' | wc -l)

if [ "$CARGO_COUNT" -eq 0 ]; then
    echo "✅ No active cargo run processes"
elif [ "$CARGO_COUNT" -eq 1 ]; then
    echo "✅ Single cargo run process (normal)"
else
    echo "⚠️  Found $CARGO_COUNT cargo run processes:"
    echo "$CARGO_PROCESSES"
fi

echo ""

# Overall assessment
if [ "$ACTIVE_COUNT" -le 1 ] && [ "$CARGO_COUNT" -le 1 ]; then
    echo "🎯 ASSESSMENT: SAFE - No conflicts detected"
    echo "   Single instance policy maintained"
else
    echo "🚨 ASSESSMENT: CONFLICT RISK - Multiple instances detected"
    echo ""
    echo "💡 RECOMMENDED ACTIONS:"
    echo "   1. Stop all services: ./scripts/lifecycle/shutdown_system.sh"
    echo "   2. Clean restart: ./scripts/lifecycle/start_clean.sh"
    echo "   3. Verify single instance: ./scripts/lifecycle/check_conflicts.sh"
fi

echo ""
echo "=================================="