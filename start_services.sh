#!/bin/bash
set -e

echo "🎯 Starting MEVana Services"
echo "=========================="
echo "Timestamp: $(date)"
echo ""

# Source environment variables
if [ -f .env ]; then
    echo "📁 Loading environment from .env"
    source .env
else
    echo "⚙️  Setting default environment variables"
    export ENDPOINT=https://grpc.ny.shyft.to
    export X_TOKEN=efff2d23-97ae-4c5f-8b8b-9a6265001189
    export KAFKA_BROKER=localhost:29092
    export ARB_MIN_PROFIT_BPS=25
    export ARB_HTTP_PORT=8081
    export HOTPATH_LOG=true
    export RUST_LOG=info
    export RUST_BACKTRACE=1
fi

# Create necessary directories
mkdir -p logs pids data

# Function to check if a process is running
is_running() {
    local pid_file=$1
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            return 0
        else
            rm -f "$pid_file"
            return 1
        fi
    fi
    return 1
}

# Function to stop existing services
stop_existing_services() {
    echo "🛑 Checking for existing services..."
    
    # Stop discovery stream
    if is_running pids/discovery.pid; then
        echo "  Stopping existing discovery stream..."
        kill $(cat pids/discovery.pid) 2>/dev/null || true
        rm -f pids/discovery.pid
        sleep 2
    fi
    
    # Stop arbitrage engine
    if is_running pids/arb-engine.pid; then
        echo "  Stopping existing arbitrage engine..."
        kill $(cat pids/arb-engine.pid) 2>/dev/null || true
        rm -f pids/arb-engine.pid
        sleep 2
    fi
    
    # Stop executor service
    if is_running pids/executor.pid; then
        echo "  Stopping existing executor service..."
        kill $(cat pids/executor.pid) 2>/dev/null || true
        rm -f pids/executor.pid
        sleep 2
    fi
    
    # Kill any remaining processes
    pkill -f "whirlpool-live-stream" 2>/dev/null || true
    pkill -f "arb-engine" 2>/dev/null || true
    pkill -f "executor-service" 2>/dev/null || true
    
    sleep 3
    echo "✅ Cleanup completed"
}

# Function to verify infrastructure is running
verify_infrastructure() {
    echo "🔍 Verifying infrastructure services..."
    
    # Check Kafka
    if ! echo "exit" | timeout 10 nc localhost 29092 2>/dev/null; then
        echo "❌ Kafka not responding on localhost:29092"
        echo "   Starting infrastructure services..."
        cd infrastructure
        docker-compose -f docker-compose.prod.yml up -d
        cd ..
        echo "   Waiting 30 seconds for services to start..."
        sleep 30
    else
        echo "✅ Kafka is ready"
    fi
    
    # Check Redis
    if ! timeout 10 redis-cli ping > /dev/null 2>&1; then
        echo "❌ Redis not responding"
        return 1
    else
        echo "✅ Redis is ready"
    fi
    
    # Check if binaries exist
    for binary in "target/release/whirlpool-live-stream" "target/release/arb-engine"; do
        if [ ! -f "$binary" ]; then
            echo "❌ Binary not found: $binary"
            echo "   Please run ./deploy.sh first"
            return 1
        fi
    done
    
    echo "✅ All infrastructure services ready"
    return 0
}

# Stop any existing services
stop_existing_services

# Verify infrastructure
if ! verify_infrastructure; then
    echo "❌ Infrastructure verification failed"
    exit 1
fi

# Start discovery stream (whirlpool-live-stream)
echo ""
echo "📡 Starting discovery stream..."
echo "   Binary: target/release/whirlpool-live-stream"
echo "   Endpoint: $ENDPOINT"
echo "   Kafka: $KAFKA_BROKER"

nohup ./target/release/whirlpool-live-stream > logs/discovery.log 2>&1 &
DISCOVERY_PID=$!
echo $DISCOVERY_PID > pids/discovery.pid

echo "   ✅ Discovery stream started (PID: $DISCOVERY_PID)"

# Wait for discovery stream to connect
echo "⏳ Waiting for discovery stream to establish connection..."
DISCOVERY_READY=false
for i in {1..60}; do
    if grep -q "geyser stream connected" logs/discovery.log 2>/dev/null; then
        echo "   ✅ Discovery stream connected to geyser"
        DISCOVERY_READY=true
        break
    elif grep -q "Connected to" logs/discovery.log 2>/dev/null; then
        echo "   ✅ Discovery stream connected"
        DISCOVERY_READY=true
        break
    fi
    sleep 1
    if [ $((i % 10)) -eq 0 ]; then
        echo "     ... still waiting ($i/60 seconds)"
    fi
done

if [ "$DISCOVERY_READY" = false ]; then
    echo "⚠️  Discovery stream connection timeout after 60 seconds"
    echo "   Check logs/discovery.log for details"
    echo "   Continuing anyway..."
fi

# Wait additional time for first EdgeDeltas
echo "⏳ Waiting for first EdgeDelta messages..."
sleep 15

# Start arbitrage engine
echo ""
echo "🤖 Starting arbitrage engine..."
echo "   Binary: target/release/arb-engine" 
echo "   HTTP Port: $ARB_HTTP_PORT"
echo "   Min Profit: $ARB_MIN_PROFIT_BPS bps"
echo "   Hotpath Logging: $HOTPATH_LOG"

nohup ./target/release/arb-engine > logs/arb-engine.log 2>&1 &
ARB_PID=$!
echo $ARB_PID > pids/arb-engine.pid

echo "   ✅ Arbitrage engine started (PID: $ARB_PID)"

# Monitor initial startup
echo ""
echo "👀 Monitoring initial startup..."
echo "   (Ctrl+C to stop monitoring, services will continue running)"

# Wait for first EdgeDelta processing
EDGEDELTA_READY=false
echo "⏳ Waiting for EdgeDelta processing to begin..."

# Monitor logs for 2 minutes
timeout 120 bash -c '
    tail -f logs/arb-engine.log | while read line; do
        echo "   [ARB] $line"
        
        # Check for EdgeDelta processing
        if echo "$line" | grep -q "EdgeDelta.*processed\|opportunity.*detected"; then
            echo ""
            echo "🎉 EdgeDelta processing started!"
            echo "   First arbitrage opportunity detection active"
            exit 0
        fi
        
        # Check for error conditions
        if echo "$line" | grep -q "kafka.*error\|connection.*failed\|panic"; then
            echo ""
            echo "❌ Critical error detected in arbitrage engine"
            exit 1
        fi
    done
' 2>/dev/null || true

# Final status check
echo ""
echo "📊 Service Status Check:"
echo "======================="

# Check if processes are still running
if is_running pids/discovery.pid; then
    echo "✅ Discovery Stream: Running (PID: $(cat pids/discovery.pid))"
else
    echo "❌ Discovery Stream: Not running"
fi

if is_running pids/arb-engine.pid; then
    echo "✅ Arbitrage Engine: Running (PID: $(cat pids/arb-engine.pid))"
    
    # Check HTTP endpoint
    if curl -s http://localhost:$ARB_HTTP_PORT/ai/config > /dev/null 2>&1; then
        echo "   ✅ HTTP API responding on port $ARB_HTTP_PORT"
    else
        echo "   ⚠️  HTTP API not yet responding (may take a few more seconds)"
    fi
else
    echo "❌ Arbitrage Engine: Not running"
fi

# Check recent activity
echo ""
echo "📈 Recent Activity Summary:"
echo "=========================="

# Discovery stream activity
if [ -f logs/discovery.log ]; then
    DISCOVERY_LINES=$(tail -20 logs/discovery.log | grep -c "EdgeDelta\|processed\|published" 2>/dev/null || echo 0)
    echo "📡 Discovery Stream: $DISCOVERY_LINES recent EdgeDelta messages"
fi

# Arbitrage engine activity  
if [ -f logs/arb-engine.log ]; then
    ARB_PROCESSED=$(tail -50 logs/arb-engine.log | grep -c "EdgeDelta.*processed" 2>/dev/null || echo 0)
    ARB_OPPORTUNITIES=$(tail -50 logs/arb-engine.log | grep -c "opportunity" 2>/dev/null || echo 0)
    echo "🤖 Arbitrage Engine: $ARB_PROCESSED EdgeDeltas processed, $ARB_OPPORTUNITIES opportunities found"
    
    # Check if we're still seeing "kafka_idle" messages
    if tail -10 logs/arb-engine.log | grep -q "kafka_idle_no_messages_yet" 2>/dev/null; then
        echo "⚠️  Note: Still seeing 'kafka_idle' - may need a few more minutes for data flow"
    fi
fi

echo ""
echo "✅ MEVana Services Started Successfully!"
echo "======================================"
echo ""
echo "📊 Monitoring URLs:"
echo "   - Arbitrage Config: http://localhost:$ARB_HTTP_PORT/ai/config"
echo "   - Arbitrage Sizing: http://localhost:$ARB_HTTP_PORT/ai/sizing"
echo "   - Metrics: http://localhost:$ARB_HTTP_PORT/metrics"
echo "   - Grafana Dashboard: http://$(curl -s ipinfo.io/ip 2>/dev/null || echo localhost):3000"
echo ""
echo "📋 Management Commands:"
echo "   - Monitor performance: ./monitor.sh"
echo "   - Check health: ./health_check.sh"
echo "   - Stop services: ./stop_services.sh"
echo "   - View logs: tail -f logs/*.log"
echo ""
echo "🎯 MEVana is now ready for arbitrage opportunity detection!"
echo ""
echo "💡 Pro tip: Run './monitor.sh' in another terminal to watch live performance"