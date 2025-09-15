# 🎭 Fake Geyser Mode - ISP Throttling Workaround
Last updated: 2025-09-15

## Problem
Your ISP is throttling/caching Geyser blockchain data streams, causing:
- Stale data (thousands of slots behind current blockchain)
- "Burst-then-fade" degradation pattern  
- Zero useful arbitrage opportunities detected
- Empty Grafana dashboards

## Easy Solutions: Bypass ISP Throttling

### Option 1: Database Transaction Replay (RECOMMENDED)
Uses your actual 174,008 recorded transactions with updated timestamps:

```bash
# Start database replay (continuous loop of real data)
./scripts/maintenance/replay_database.sh
```

This gives you:
- ✅ **Real historical transaction patterns** from your database
- ✅ **Jupiter V6, MEVana Engine, and actual DEX activity**
- ✅ **174,008 authentic transactions** continuously replaying
- ✅ **Current timestamps** (fixes 1981 timestamp issue)

### Option 2: Standalone Fake Geyser
Simple synthetic data generation:

```bash
# Start standalone fake Geyser
./scripts/development/start_fake_geyser.sh
```

### Option 3: Environment Variable Mode
Set environment variable for synthetic data mode:

```bash
export FAKE_GEYSER=true    # Enable synthetic mode
unset FAKE_GEYSER         # Switch back to real Geyser
```

## Usage Examples

### Test Grafana Dashboards with Synthetic Data
```bash
# Clean startup with fake data
./scripts/lifecycle/shutdown_system.sh
FAKE_GEYSER=true PAPER_TRADING=true DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" METRICS_PORT=8088 RUST_LOG=info cargo run -p hotpath_arb_engine --bin hotpath_arb_engine
```

### Real Production Mode (when ISP allows)
```bash  
# Clean startup with real Geyser data
./scripts/lifecycle/shutdown_system.sh
PAPER_TRADING=true DATABASE_URL="postgresql://mev:mevpassword@localhost:5433/mevdb" GEYSER_ENDPOINT="https://grpc.va.shyft.to" X_TOKEN="efff2d23-97ae-4c5f-8b8b-9a6265001189" METRICS_PORT=8088 RUST_LOG=info cargo run -p hotpath_arb_engine --bin hotpath_arb_engine
```

## What Fake Geyser Provides

- ✅ **Current blockchain slots** (not stale data)
- ✅ **Realistic arbitrage opportunities** (2-79 BPS profits)
- ✅ **Multiple DEX venues** (Jupiter, Whirlpool, Raydium, etc.)
- ✅ **Live Grafana dashboard data** 
- ✅ **Sub-millisecond latency simulation** (300-1500μs)
- ✅ **No ISP bandwidth conflicts**

## How It Works

1. **Environment Check**: System detects `FAKE_GEYSER=true`
2. **Synthetic Generator**: Creates realistic trading data in real-time
3. **Same Data Path**: Uses identical queues/metrics as real Geyser
4. **Dashboard Compatible**: Your Grafana charts work normally
5. **Easy Switch**: Change one environment variable to toggle modes

## When to Use Each Mode

### Use Fake Geyser When:
- ISP throttles blockchain data 
- Testing Grafana dashboards
- Developing new features
- Demonstrating system capabilities
- Bandwidth is limited

### Use Real Geyser When:  
- ISP allows full bandwidth
- Production trading
- Real profit opportunities
- Live market conditions
- Actual arbitrage execution

## Alternative Solutions

If fake mode isn't suitable:

1. **VPN**: Route traffic through different IP
2. **Mobile Hotspot**: Use different ISP connection  
3. **Cloud Deployment**: Run on VPS/AWS
4. **Alternative APIs**: WebSocket instead of Geyser
5. **ISP Change**: Switch to crypto-friendly provider

The fake Geyser mode gives you full development capability while you resolve the ISP bandwidth issue.
