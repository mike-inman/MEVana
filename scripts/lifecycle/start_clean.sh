#!/bin/bash

# MEVana Clean Startup Script
# Prevents port/database conflicts and Geyser endpoint monitoring issues

echo "🧹 COMPREHENSIVE CLEANUP: Killing all background services..."

# Kill all cargo run processes (most common cause of conflicts)
echo "  📦 Killing all cargo run processes..."
ps aux | grep "cargo run" | grep -v grep | awk '{print $2}' | xargs kill -9 2>/dev/null || true

# Kill specific MEVana services by name
echo "  🎯 Killing specific MEVana services..."
pkill -f "hotpath_arb_engine" 2>/dev/null || true
pkill -f "system-orchestrator" 2>/dev/null || true
pkill -f "arb-engine" 2>/dev/null || true
pkill -f "normalizer-service" 2>/dev/null || true
pkill -f "edge-assembler" 2>/dev/null || true
pkill -f "executor-service" 2>/dev/null || true
pkill -f "monitoring-service" 2>/dev/null || true
pkill -f "wallet-vault" 2>/dev/null || true
pkill -f "discovery-service" 2>/dev/null || true
pkill -f "dex-stream" 2>/dev/null || true

# Kill any processes using our common ports
echo "  🔌 Killing processes on MEVana ports..."
lsof -ti:8081,8088,8090,8091,8095 2>/dev/null | xargs kill -9 2>/dev/null || true

# Wait for cleanup
echo "  ⏳ Waiting for cleanup to complete..."
sleep 2

# Verify cleanup
REMAINING=$(ps aux | grep -E "(cargo run|hotpath_arb_engine)" | grep -v grep | wc -l)
if [ "$REMAINING" -gt 0 ]; then
    echo "  ⚠️  Warning: $REMAINING processes still running"
    ps aux | grep -E "(cargo run|hotpath_arb_engine)" | grep -v grep
else
    echo "  ✅ All background processes cleaned up"
fi

echo ""
echo "🔍 Checking database status..."
docker exec postgres psql -U mev -d mevdb -c "SELECT COUNT(*) as opportunities, MAX(timestamp) as latest FROM arbitrage_opportunities;" 2>/dev/null || echo "❌ Database not accessible"

echo ""
echo "🔒 FINAL SAFETY CHECK: Ensuring no other MEVana instances are running..."

# Check for any running MEVana processes that might compete for endpoints
RUNNING_INSTANCES=$(lsof -i :8081,8088,8090,8091,8095 2>/dev/null | grep LISTEN | wc -l)
if [ "$RUNNING_INSTANCES" -gt 0 ]; then
    echo "⚠️  WARNING: Found $RUNNING_INSTANCES active MEVana service(s) on known ports!"
    echo "   This could cause Geyser endpoint conflicts and trigger monitoring."
    echo ""
    echo "🔌 Active services:"
    lsof -i :8081,8088,8090,8091,8095 2>/dev/null | grep LISTEN
    echo ""
    echo "❌ ABORTING: Multiple instances would trigger endpoint abuse detection"
    echo "   Stop all running MEVana services first, then retry this script"
    echo ""
    echo "To force cleanup and restart:"
    echo "   ./scripts/lifecycle/shutdown_system.sh"
    echo "   ./scripts/lifecycle/start_clean.sh"
    exit 1
fi

echo "✅ Port check passed - no conflicting instances found"
echo ""
echo "🚀 Starting SINGLE clean hotpath_arb_engine instance..."
echo "   📍 Using multi-endpoint distribution (NY/Virginia/Frankfurt)"
echo "   🔒 PAPER_TRADING=true (prevents endpoint abuse)"
echo "   📊 Metrics on port 8081"
echo "   🛡️  Protected against multiple instance conflicts"
echo ""

# Start single clean instance with multi-endpoint configuration
PAPER_TRADING=true \
DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" \
GEYSER_ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="efff2d23-97ae-4c5f-8b8b-9a6265001189" \
METRICS_PORT=8081 \
RUST_LOG=warn \
cargo run -p hotpath_arb_engine --bin hotpath_arb_engine