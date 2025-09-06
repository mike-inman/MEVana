#!/bin/bash

# Quick Arbitrage Engine Diagnosis & Fix Script
set -e

echo "🔍 Arbitrage Engine Quick Diagnosis"
echo "===================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Check if arb-engine is running
echo -e "\n1️⃣ Checking if arb-engine is running..."
if pgrep -f "arb-engine" > /dev/null; then
    echo -e "${GREEN}✅ Arb-engine is running${NC}"
    
    # Check health
    if curl -s http://localhost:8080/ready > /dev/null 2>&1; then
        READY=$(curl -s http://localhost:8080/ready | jq -r '.ready')
        if [ "$READY" = "true" ]; then
            echo -e "${GREEN}✅ Engine is healthy and ready${NC}"
        else
            STATUS=$(curl -s http://localhost:8080/ready | jq -r '.status')
            echo -e "${YELLOW}⚠️  Engine not ready: $STATUS${NC}"
            
            # Check specific issues
            ERROR_RATIO=$(curl -s http://localhost:8080/ready | jq -r '.error_ratio')
            CIRCUIT=$(curl -s http://localhost:8080/ready | jq -r '.circuit_open')
            
            if [ "$CIRCUIT" = "true" ]; then
                echo -e "${RED}❌ Circuit breaker is OPEN - resetting...${NC}"
                curl -X POST http://localhost:8080/circuit/reset
                echo -e "${GREEN}✅ Circuit breaker reset${NC}"
            fi
        fi
    else
        echo -e "${RED}❌ Cannot connect to engine HTTP endpoint${NC}"
    fi
else
    echo -e "${RED}❌ Arb-engine is NOT running${NC}"
    echo "Starting arb-engine..."
    cargo run --bin arb-engine > arb-engine.log 2>&1 &
    sleep 3
fi

# Step 2: Check edge flow
echo -e "\n2️⃣ Checking edge data flow..."
EDGE_COUNT=$(curl -s http://localhost:8080/metrics 2>/dev/null | grep -oP 'arb_edge_messages_total \K\d+' || echo "0")
echo "Total edges received: $EDGE_COUNT"

if [ "$EDGE_COUNT" -eq "0" ]; then
    echo -e "${RED}❌ No edges received - checking Kafka...${NC}"
    
    # Check if Kafka is running
    if ! nc -z localhost 29092 2>/dev/null; then
        echo -e "${RED}❌ Kafka not accessible on localhost:29092${NC}"
        echo "Please start Kafka first:"
        echo "  docker-compose up -d kafka zookeeper"
        exit 1
    fi
    
    echo -e "${YELLOW}📝 Generating test edges...${NC}"
    cargo build --bin gen_synthetic_edges 2>/dev/null
    cargo run -q --bin gen_synthetic_edges /tmp/test_edges.jsonl simple
    
    echo "Publishing test edges to Kafka..."
    cat /tmp/test_edges.jsonl | kafka-console-producer \
        --broker-list localhost:29092 \
        --topic edge.delta.v1 2>/dev/null
    
    sleep 2
    NEW_COUNT=$(curl -s http://localhost:8080/metrics 2>/dev/null | grep -oP 'arb_edge_messages_total \K\d+' || echo "0")
    echo "Edges after publishing: $NEW_COUNT"
else
    echo -e "${GREEN}✅ Edges are flowing (count: $EDGE_COUNT)${NC}"
fi

# Step 3: Check configuration
echo -e "\n3️⃣ Checking configuration..."
CONFIG=$(curl -s http://localhost:8080/ai/config)
MIN_PROFIT=$(echo $CONFIG | jq -r '.min_profit_bps')
MAX_SLIP=$(echo $CONFIG | jq -r '.max_slippage_bps')

echo "Current thresholds:"
echo "  Min profit: $MIN_PROFIT bps"
echo "  Max slippage: $MAX_SLIP bps"

if [ "$MIN_PROFIT" -gt "50" ]; then
    echo -e "${YELLOW}⚠️  Profit threshold might be too high for testing${NC}"
    echo "Lowering to 10 bps (0.1%)..."
    curl -s -X POST http://localhost:8080/ai/config/update \
        -H "Content-Type: application/json" \
        -d '{"min_profit_bps": 10, "max_slippage_bps": 500}' > /dev/null
    echo -e "${GREEN}✅ Thresholds adjusted${NC}"
fi

# Step 4: Test with known good data
echo -e "\n4️⃣ Testing with synthetic arbitrage scenario..."
cargo build --bin test_edge_integration 2>/dev/null
OUTPUT=$(cargo run -q --bin test_edge_integration 2>&1)

if echo "$OUTPUT" | grep -q "ARBITRAGE SIGNAL"; then
    echo -e "${GREEN}✅ Engine can detect arbitrage!${NC}"
    echo "$OUTPUT" | grep -A 5 "ARBITRAGE SIGNAL"
else
    echo -e "${RED}❌ No signals generated in test${NC}"
    echo "Debug output:"
    echo "$OUTPUT"
fi

# Step 5: Check for signals
echo -e "\n5️⃣ Checking for live signals..."
SIGNAL_COUNT=$(curl -s http://localhost:8080/metrics 2>/dev/null | grep -oP 'arb_signals_total \K\d+' || echo "0")
echo "Total signals generated: $SIGNAL_COUNT"

if [ "$SIGNAL_COUNT" -gt "0" ]; then
    echo -e "${GREEN}✅ System is generating signals!${NC}"
    echo "Recent signals:"
    curl -s http://localhost:8080/ai/recent?limit=3 | jq '.[].profit_bps'
else
    echo -e "${YELLOW}⚠️  No signals generated yet${NC}"
    
    # Generate more test data
    echo "Generating high-frequency test scenario..."
    cargo run -q --bin gen_synthetic_edges /tmp/highfreq.jsonl highfreq
    
    echo "Replaying test data..."
    cargo run -q --bin arb_replay -- --input /tmp/highfreq.jsonl --burst --signals-out /tmp/signals.jsonl
    
    if [ -s /tmp/signals.jsonl ]; then
        REPLAY_COUNT=$(wc -l < /tmp/signals.jsonl)
        echo -e "${GREEN}✅ Replay generated $REPLAY_COUNT signals${NC}"
        echo "Sample signal:"
        head -1 /tmp/signals.jsonl | jq '.'
    else
        echo -e "${RED}❌ Even replay didn't generate signals${NC}"
    fi
fi

# Step 6: Summary and recommendations
echo -e "\n📊 Diagnosis Summary"
echo "===================="

ISSUES=0

# Check each component
if [ "$EDGE_COUNT" -eq "0" ]; then
    echo -e "${RED}❌ Issue: No edge data${NC}"
    echo "   Fix: Check edge-assembler is running and publishing to Kafka"
    ISSUES=$((ISSUES + 1))
fi

ERROR_COUNT=$(curl -s http://localhost:8080/metrics 2>/dev/null | grep -oP 'arb_edge_errors_total \K\d+' || echo "0")
if [ "$ERROR_COUNT" -gt "$((EDGE_COUNT / 2))" ] && [ "$EDGE_COUNT" -gt "0" ]; then
    echo -e "${RED}❌ Issue: High error rate ($ERROR_COUNT/$EDGE_COUNT)${NC}"
    echo "   Fix: Check edge data format matches EdgeDelta protobuf"
    ISSUES=$((ISSUES + 1))
fi

if [ "$SIGNAL_COUNT" -eq "0" ] && [ "$EDGE_COUNT" -gt "100" ]; then
    echo -e "${YELLOW}⚠️  Issue: Edges flowing but no signals${NC}"
    echo "   Possible causes:"
    echo "   - Profit threshold too high (current: $MIN_PROFIT bps)"
    echo "   - Missing reverse direction edges"
    echo "   - All edges from same venue"
    echo "   - Insufficient liquidity in edges"
    ISSUES=$((ISSUES + 1))
fi

if [ "$ISSUES" -eq "0" ]; then
    echo -e "${GREEN}✅ System appears to be working correctly!${NC}"
    echo "Monitor signals at: ws://localhost:8080/ws/arbs"
else
    echo -e "\n🔧 Quick fix commands:"
    echo "# Reset everything and test:"
    echo "curl -X POST http://localhost:8080/circuit/reset"
    echo "curl -X POST http://localhost:8080/ai/config/update -H 'Content-Type: application/json' -d '{\"min_profit_bps\": 1}'"
    echo "cargo run --bin gen_synthetic_edges test.jsonl simple"
    echo "cargo run --bin arb_replay -- --input test.jsonl --burst"
fi

echo -e "\n📚 For detailed debugging, check:"
echo "  - Logs: RUST_LOG=arb_engine=trace cargo run --bin arb-engine"
echo "  - Metrics: http://localhost:8080/metrics"
echo "  - WebSocket: ws://localhost:8080/ws/arbs"