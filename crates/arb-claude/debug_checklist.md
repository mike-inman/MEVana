# 🔍 Arbitrage Engine Debugging Checklist

## Step 1: Verify Infrastructure

### Kafka Setup
```bash
# Check if Kafka is running
docker ps | grep kafka

# List topics
kafka-topics --bootstrap-server localhost:29092 --list

# Check edge topic exists
kafka-topics --bootstrap-server localhost:29092 --describe --topic edge.delta.v1

# Monitor incoming edges
kafka-console-consumer --bootstrap-server localhost:29092 \
  --topic edge.delta.v1 \
  --from-beginning \
  --max-messages 5
```

### Arb Engine Health
```bash
# Check if engine is running
ps aux | grep arb-engine

# Check health endpoints
curl http://localhost:8080/ready
curl http://localhost:8080/health
curl http://localhost:8080/metrics | grep edge_total

# Check circuit breaker status
curl http://localhost:8080/metrics | grep circuit_open
```

## Step 2: Verify Edge Data Quality

### Common Edge Issues

1. **Invalid Price Format**
   - Price must be Q64.64 format (16 bytes)
   - Check: `marginal_price_q64.len() == 16`

2. **Invalid Size Format**
   - Size must be u128 (16 bytes)
   - Check: `size_to_next_boundary_in.len() == 16`

3. **Direction Issues**
   - Direction must be 0 or 1
   - Need edges in BOTH directions for arbitrage

4. **Venue ID Mismatch**
   - Different venues needed for arbitrage
   - Check venue_id values differ between legs

### Test with Known Good Data
```bash
# Generate synthetic edges
cargo run --bin gen_synthetic_edges test_edges.jsonl simple

# Replay with debugging
RUST_LOG=arb_engine=debug cargo run --bin arb_replay -- \
  --input test_edges.jsonl \
  --burst \
  --signals-out signals.jsonl
```

## Step 3: Check Configuration

### Profit Thresholds
```bash
# Check current thresholds
curl http://localhost:8080/ai/config

# Lower thresholds for testing (10 bps = 0.1%)
curl -X POST http://localhost:8080/ai/config/update \
  -H "Content-Type: application/json" \
  -d '{"min_profit_bps": 10, "max_slippage_bps": 500}'
```

### Environment Variables
```bash
# Key configuration
export ARB_MIN_PROFIT_BPS=10           # Lower for testing
export ARB_MAX_SLIPPAGE_BPS=500        # Higher tolerance
export ARB_DEPTH_PRICE_IMPACT_BPS=0    # Disable depth adjustment
export CIRCUIT_ERROR_RATIO_THRESHOLD=0.9  # Higher tolerance
export ARB_SIZING_MODE=boundary        # Use full boundary size
```

## Step 4: Monitor Arbitrage Detection

### Real-time Monitoring
```bash
# Watch WebSocket for signals
wscat -c ws://localhost:8080/ws/arbs

# Monitor metrics
watch -n 1 'curl -s http://localhost:8080/metrics | grep -E "arb_signals_total|edge_total|circuit"'

# Check recent signals
curl http://localhost:8080/ai/recent?limit=10
```

### Log Analysis
```bash
# Enable debug logging
RUST_LOG=arb_engine=trace cargo run --bin arb-engine 2>&1 | tee arb.log

# Look for key events
grep "process_edge_update" arb.log
grep "scan_for_arbitrage" arb.log
grep "evaluate_2leg" arb.log
grep "profit_bps" arb.log
```

## Step 5: Common Fixes

### 1. No Edges Coming In
```bash
# Start edge-assembler
cd ../edge-assembler
cargo run --bin edge-assembler

# Or produce test edges
cargo run --bin gen_synthetic_edges edges.jsonl simple
kafka-console-producer --broker-list localhost:29092 --topic edge.delta.v1 < edges.jsonl
```

### 2. Circuit Breaker Tripped
```bash
# Reset circuit breaker
curl -X POST http://localhost:8080/circuit/reset

# Check error ratio
curl http://localhost:8080/ready | jq .
```

### 3. Price Calculation Issues
```rust
// Verify price calculation in test
let price_q64 = (price_float * 18446744073709551616.0) as u128;
let bytes = price_q64.to_le_bytes();
```

### 4. Missing Reverse Edges
- Ensure you have edges with direction=0 AND direction=1
- For each token pair, need edges in both directions
- Different venues for each direction create arbitrage

## Step 6: Performance Tuning

### Optimize Detection
```bash
# Enable diagnostics
cargo build --features diagnostics
cargo run --features diagnostics --bin arb-engine

# Monitor performance metrics
curl http://localhost:8080/metrics | grep -E "latency|cycles_examined|pruning"
```

### Batch Processing
```rust
// Process multiple edges before scanning
let batch_size = 10;
for edge in edges.take(batch_size) {
    detector.process_edge_update(edge)?;
}
```

## Step 7: Integration Testing

### Full System Test
```bash
# 1. Start all components
docker-compose up -d kafka zookeeper
cargo run --bin arb-engine &
cargo run --bin edge-assembler &

# 2. Generate test scenario
cargo run --bin gen_synthetic_edges test.jsonl triangle

# 3. Feed test data
cat test.jsonl | while read line; do
  echo "$line" | kafka-console-producer --broker-list localhost:29092 --topic edge.delta.v1
  sleep 0.01
done

# 4. Check results
curl http://localhost:8080/ai/recent?limit=100 | jq .
```

## Quick Diagnosis Commands

```bash
# One-liner health check
curl -s http://localhost:8080/ready | jq -r 'if .ready then "✅ READY" else "❌ NOT READY: " + .status end'

# Edge flow check
echo "Edges In: $(curl -s http://localhost:8080/metrics | grep -oP 'edge_total \K\d+')"
echo "Errors: $(curl -s http://localhost:8080/metrics | grep -oP 'edge_errors_total \K\d+')"
echo "Signals: $(curl -s http://localhost:8080/metrics | grep -oP 'arb_signals_total \K\d+')"

# Quick config adjustment
curl -X POST http://localhost:8080/ai/config/update \
  -H "Content-Type: application/json" \
  -d '{"min_profit_bps": 1}'  # Ultra-low threshold for testing
```

## Expected Outputs

### Healthy System
- `edge_total` increasing
- `circuit_open` = 0
- `error_ratio` < 0.5
- `arb_signals_total` > 0 after edges processed

### Problem Indicators
- `edge_total` = 0 → No edges coming in
- `circuit_open` = 1 → Too many errors
- `error_ratio` > 0.5 → Bad edge format
- `arb_signals_total` = 0 with edges → Threshold too high or missing pairs