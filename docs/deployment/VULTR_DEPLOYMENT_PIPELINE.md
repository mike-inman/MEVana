# 🚀 MEVana Vultr Deployment Pipeline
Last updated: 2025-09-15

## 🎯 **Tomorrow's Deployment Plan**

### **Phase 1: Vultr Server Setup (30 minutes)**
```bash
# 1. Deploy Vultr High Frequency Compute
Location: New York
Instance: 8 vCPU, 32GB RAM, NVMe SSD
Cost: $192/month
OS: Ubuntu 22.04 LTS

# 2. Initial server setup
ssh root@your-vultr-ip
apt update && apt upgrade -y
apt install -y docker.io docker-compose git curl wget build-essential
```

### **Phase 2: MEVana Stack Deployment (60 minutes)**
```bash
# 1. Clone repository
git clone https://github.com/mike-inman/MEVana.git
cd MEVana

# 2. Run latency test first
./tools/latency_test.sh
# Expected: 8-12ms to Solana (vs local 27ms)

# 3. Install Rust and build
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
cargo build --release

# 4. Start infrastructure
cd infrastructure
docker-compose up -d

# 5. Verify infrastructure
docker-compose ps
# Expected: Kafka, Redis, PostgreSQL, Grafana all running
```

### **Phase 3: Configuration & Testing (45 minutes)**
```bash
# 1. Set environment variables
export ENDPOINT=https://grpc.ny.shyft.to
export X_TOKEN=efff2d23-97ae-4c5f-8b8b-9a6265001189
export KAFKA_BROKER=localhost:29092
export ARB_MIN_PROFIT_BPS=25
export HOTPATH_LOG=true

# 2. Start services
./target/release/whirlpool-live-stream &
./target/release/arb-engine &

# 3. Monitor for data flow
tail -f logs/arb-engine.log
# Expected: Stop seeing "kafka_idle_no_messages_yet"
# Expected: Start seeing EdgeDelta processing

# 4. Test wallet setup (when ready for live trading)
cargo run --bin wallet-setup
```

---

## 🔄 **Automated Deployment Pipeline**

### **Deploy Script** (`deploy.sh`):
```bash
#!/bin/bash
set -e

echo "🚀 MEVana Automated Deployment"
echo "=============================="

# Check if running on Vultr (or similar cloud)
if [ ! -f /etc/cloud/cloud.cfg ]; then
    echo "Warning: Not running on cloud instance"
fi

# System setup
echo "📦 Installing dependencies..."
apt update && apt upgrade -y
apt install -y docker.io docker-compose git curl wget build-essential

# Install Rust
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# Build MEVana
echo "🏗️ Building MEVana..."
cargo build --release

# Start infrastructure
echo "🐳 Starting infrastructure..."
cd infrastructure
docker-compose up -d
sleep 30  # Wait for services to start

# Verify infrastructure
echo "✅ Verifying infrastructure..."
docker-compose ps

echo "🎉 Deployment complete!"
echo "Next steps:"
echo "1. Run latency test: ./tools/latency_test.sh"
echo "2. Start services: ./start_services.sh"
echo "3. Monitor logs: tail -f logs/*.log"
```

### **Service Management Script** (`start_services.sh`):
```bash
#!/bin/bash
set -e

echo "🎯 Starting MEVana Services"
echo "=========================="

# Set environment
export ENDPOINT=https://grpc.ny.shyft.to
export X_TOKEN=efff2d23-97ae-4c5f-8b8b-9a6265001189
export KAFKA_BROKER=localhost:29092
export ARB_MIN_PROFIT_BPS=25
export HOTPATH_LOG=true

# Create logs directory
mkdir -p logs

# Start discovery stream
echo "📡 Starting discovery stream..."
nohup ./target/release/whirlpool-live-stream > logs/discovery.log 2>&1 &
echo $! > pids/discovery.pid

# Wait for discovery to connect
sleep 10

# Start arbitrage engine
echo "🤖 Starting arbitrage engine..."
nohup ./target/release/arb-engine > logs/arb-engine.log 2>&1 &
echo $! > pids/arb-engine.pid

# Monitor initial startup
echo "👀 Monitoring startup..."
tail -f logs/arb-engine.log &
TAIL_PID=$!

# Wait for first EdgeDelta messages
timeout 60 bash -c '
while ! grep -q "EdgeDelta processed" logs/arb-engine.log; do
    if grep -q "kafka_idle_no_messages_yet" logs/arb-engine.log; then
        echo "⏳ Waiting for EdgeDelta messages..."
    fi
    sleep 5
done
'

kill $TAIL_PID 2>/dev/null || true

echo "✅ MEVana services started successfully!"
echo "📊 Monitor at: http://localhost:3000 (Grafana)"
echo "🔍 Check logs: tail -f logs/*.log"
```

---

## 📊 **Monitoring & Pipeline Management**

### **Health Check Script** (`health_check.sh`):
```bash
#!/bin/bash

echo "🏥 MEVana Health Check"
echo "====================="

# Check services
echo "🔍 Service Status:"
ps aux | grep -E "(whirlpool-live-stream|arb-engine)" | grep -v grep

# Check infrastructure
echo "🐳 Infrastructure Status:"
docker-compose ps

# Check recent activity
echo "📊 Recent Activity:"
echo "Discovery stream:"
tail -5 logs/discovery.log | grep -E "(connected|EdgeDelta)" || echo "No recent activity"

echo "Arbitrage engine:"
tail -5 logs/arb-engine.log | grep -E "(opportunity|trade)" || echo "No recent trades"

# Check system resources
echo "💻 System Resources:"
df -h / | tail -1
free -h | grep Mem
```

### **Performance Monitor** (`monitor.sh`):
```bash
#!/bin/bash

echo "📈 MEVana Performance Monitor"
echo "============================"

# Continuous monitoring loop
while true; do
    clear
    echo "$(date): MEVana Live Status"
    echo "================================"
    
    # Service uptime
    echo "🟢 Services Running:"
    pgrep -f whirlpool-live-stream > /dev/null && echo "✅ Discovery Stream" || echo "❌ Discovery Stream"
    pgrep -f arb-engine > /dev/null && echo "✅ Arb Engine" || echo "❌ Arb Engine"
    
    # Recent EdgeDeltas
    RECENT_DELTAS=$(tail -100 logs/arb-engine.log | grep -c "EdgeDelta" || echo 0)
    echo "📡 EdgeDeltas (last 100 lines): $RECENT_DELTAS"
    
    # Recent opportunities
    RECENT_OPPS=$(tail -100 logs/arb-engine.log | grep -c "opportunity" || echo 0)
    echo "🎯 Opportunities (last 100 lines): $RECENT_OPPS"
    
    # Latency check
    PING_TIME=$(ping -c 1 api.mainnet-beta.solana.com | grep -o 'time=[0-9.]*' | cut -d= -f2 || echo "unknown")
    echo "⚡ Solana Latency: ${PING_TIME}ms"
    
    sleep 30
done
```

---

## 🔧 **Weekend Development Ideas Setup**

### **Idea 1: Jupiter Integration** (Ready for Weekend)
```bash
# Prepare Jupiter aggregator integration
mkdir -p integrations/jupiter
touch integrations/jupiter/route_optimizer.rs
touch integrations/jupiter/price_comparison.rs

# Research endpoints
curl "https://quote-api.jup.ag/v6/quote?inputMint=So11111111111111111111111111111111111111112&outputMint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v&amount=1000000" > integrations/jupiter/sample_response.json
```

### **Idea 2: Pump.fun Integration** (Framework)
```bash
# Prepare new token discovery
mkdir -p integrations/pumpfun
touch integrations/pumpfun/token_scanner.rs
touch integrations/pumpfun/launch_detector.rs
```

### **Idea 3: Advanced Analytics** (Monitoring++)
```bash
# Enhanced performance tracking
mkdir -p analytics
touch analytics/profit_tracker.rs
touch analytics/market_analyzer.rs
touch analytics/competition_monitor.rs
```

---

## 🎯 **Success Metrics for Tomorrow**

### **Deployment Success:**
- ✅ Server deployed and accessible
- ✅ All infrastructure services running  
- ✅ MEVana services start without errors
- ✅ Latency < 15ms to Solana (target: 8-12ms)
- ✅ EdgeDelta messages flowing (no more "kafka_idle")

### **Trading Readiness:**
- ✅ First EdgeDelta processed within 5 minutes
- ✅ First arbitrage opportunity detected
- ✅ Wallet setup tools working
- ✅ Test mode execution ready
- ✅ Monitoring dashboards accessible

### **Weekend Prep:**
- ✅ Stable 4+ hour operation
- ✅ Performance baseline established
- ✅ Integration framework ready
- ✅ Development environment configured

---

## 📞 **Emergency Procedures**

### **If Services Crash:**
```bash
# Restart everything
./stop_services.sh
./start_services.sh
./health_check.sh
```

### **If Data Flow Stops:**
```bash
# Check Kafka
docker-compose restart kafka
sleep 30
./start_services.sh
```

### **If Performance is Poor:**
```bash
# Run latency test
./tools/latency_test.sh

# If >20ms, consider different Vultr location
# If <10ms but still poor, check system resources
```

The pipeline is designed for **quick deployment** tomorrow and **easy iteration** over the weekend! 🚀
