# Arb Engine Troubleshooting & Performance Guide

## Quick Fixes for Common Issues

### 1. Compilation Errors

**Problem**: Modules are commented out due to compilation errors
```rust
// pub mod engine; // Temporarily disabled due to compilation errors
```

**Solutions**:
```bash
# Update Cargo.toml with proper feature flags
cargo clean
cargo build --features=dev-full

# For production
cargo build --release --features=prod-full

# If still failing, start minimal
cargo build --features=whirlpool,stubs
```

### 2. Edge Processing Bottlenecks  

**Symptoms**:
- High latency between edge updates and signals
- Circuit breaker frequently opens
- Memory usage growing unbounded

**Solutions**:

```bash
# Tune performance parameters
export ARB_MAX_EDGES_PER_PAIR=10          # Reduce memory usage
export ARB_CLEANUP_INTERVAL_MS=15000      # More frequent cleanup
export ARB_MIN_PROFIT_BPS=50              # Higher profit threshold
export ARB_MAX_SLIPPAGE_BPS=100           # More tolerant slippage

# Enable diagnostics
cargo run --features=diagnostics -- --help

# Monitor metrics
curl http://localhost:8080/metrics | grep arb_
curl http://localhost:8080/ready
```

### 3. Memory Issues

**Problem**: Memory usage growing without bounds

**Debug Steps**:
```bash
# Check current stats
curl http://localhost:8080/ai/sizing

# Enable memory diagnostics  
export ARBS_PROFIT_BUCKETS=10
export ARBS_LATENCY_BUCKETS=10
cargo run --features=diagnostics

# Memory profiling (if available)
export PPROF_FREQ=1000
export PPROF_FLAME=profile.svg
cargo run --features=profiling
```

**Fixes**:
```bash
# Reduce cache sizes
export ARB_MAX_PAIRS_TOTAL=5000
export ARB_MAX_EDGES_PER_PAIR=5
export ARB_MAX_EDGE_AGE_MS=30000

# Use production allocator
cargo build --release --features=production
```

### 4. Signal Queue Issues

**Symptoms**:
- "Signal queue full" warnings
- Missing arbitrage opportunities

**Solutions**:
```rust
// In your main.rs, increase queue size
let q = Arc::new(SignalQueue::new(2048)); // Increased from 1024

// Or tune processing
export ARB_MIN_PROFIT_BPS=100  // Higher threshold = fewer signals
```

### 5. Circuit Breaker Problems

**Problem**: Circuit breaker opening too frequently

**Debug**:
```bash
curl http://localhost:8080/ready
# Check error_ratio and circuit_open fields

# Reset circuit breaker
curl -X POST -H "X-AI-Token: your-token" http://localhost:8080/circuit/reset
```

**Tuning**:
```bash
export CIRCUIT_ERROR_RATIO_THRESHOLD=0.30     # Lower threshold  
export CIRCUIT_MIN_MESSAGES=500               # Fewer messages before evaluation
export CIRCUIT_COOLDOWN_MS=15000               # Shorter cooldown
export READINESS_MAX_ERROR_RATIO=0.40          # More tolerant readiness
```

## Performance Optimization

### 1. Hot Path Optimization

**Current Bottlenecks**:
- Edge validation and parsing
- Arbitrage detection algorithms  
- Signal queue contention

**Optimizations Applied**:
```rust
// Pre-computed cached values in EdgeCacheEntry
struct EdgeCacheEntry {
    edge: Arc<EdgeDelta>,
    price: f64,           // Cached price conversion
    fee_factor: f64,      // Cached fee calculation  
    size_boundary: u128,  // Cached size parsing
}

// Fast validation with early returns
if edge.direction > 1 || edge.marginal_price_q64.len() != 16 {
    return false; // Fail fast
}
```

### 2. Memory Management

**Strategy**:
- LRU eviction for edge cache
- Periodic background cleanup
- Bounded data structures everywhere

```bash
# Monitor memory usage
export ARB_ADAPT_STATE_PATH=/tmp/arb_state.json
export ARB_ADAPT_PERSIST_INTERVAL=25

# Check memory metrics  
curl http://localhost:8080/metrics | grep resident_bytes
```

### 3. Algorithm Optimizations

**2-Leg Arbitrage** (optimized):
- O(n²) where n = edges per pair (typically < 20)
- Cached price/fee calculations
- Early pruning based on profit thresholds

**3-Leg Arbitrage** (limited):
- Only runs on small graphs (< 1000 pairs)
- Token adjacency indexing
- Aggressive pruning

```bash
# Control 3-hop search
export ARB_MAX_3HOP_TOKENS=25
export ARB_MAX_3HOP_DEPTH=50
```

## Development Workflow

### 1. Local Development Setup

```bash
# Development build with all features
cargo build --features=dev-full

# Run with comprehensive logging
RUST_LOG=arb_engine=debug,edge_assembler=info \
  cargo run --features=dev-full

# Test edge integration  
cargo run --bin test_edge_integration --features=dev-utils
```

### 2. Testing Strategy

```bash
# Unit tests
cargo test --features=dev-full

# Integration tests with real data
cargo run --bin arb_replay --features=dev-utils -- \
  --input sample_edges.jsonl --max 1000 --burst

# Generate synthetic test data
cargo run --bin gen_synthetic_edges --features=dev-utils -- \
  test_edges.jsonl 5000
```

### 3. Performance Testing

```bash
# Benchmark with criterion
cargo bench --features=full

# Profile hot paths
cargo build --release --features=profiling
PPROF_FREQ=10000 ./target/release/arb-engine

# Memory profiling
valgrind --tool=massif ./target/release/arb-engine
```

## Production Deployment

### 1. Build Configuration

```bash
# Production build
cargo build --release --features=prod-full
strip target/release/arb-engine  # Remove debug symbols

# Docker multi-stage build
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features=prod-full

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/arb-engine /usr/local/bin/
CMD ["arb-engine"]
```

### 2. Environment Configuration

```bash
# Core settings
export KAFKA_BROKER=kafka1:9092,kafka2:9092,kafka3:9092
export EDGE_DELTA_TOPIC=edge.delta.v1
export WS_BIND=0.0.0.0:8080

# Performance tuning  
export ARB_MIN_PROFIT_BPS=75
export ARB_MAX_SLIPPAGE_BPS=50
export ARB_MAX_PAIRS_TOTAL=10000
export ARB_CLEANUP_INTERVAL_MS=30000

# Health checks
export READINESS_MAX_STALENESS_MS=5000
export READINESS_MAX_ERROR_RATIO=0.20

# Monitoring
export SERVICE_NAME=arb-engine
export BUILD_VERSION=v0.2.0
export GIT_SHA=$(git rev-parse --short HEAD)
```

### 3. Monitoring & Alerting

**Key Metrics**:
```bash
# Health status
curl http://localhost:8080/ready

# Performance metrics  
arb_signal_profit_bps              # Profit distribution
arb_signal_latency_ms              # Edge-to-signal latency
arb_edge_ingest_latency_ms         # Kafka processing latency  
arb_edge_error_ratio               # Error rate
arb_edge_staleness_ms              # Data freshness

# Structural metrics (diagnostics feature)
arb_edge_pairs_total               # Total pairs tracked
arb_tokens_indexed_total           # Tokens in adjacency index
arb_max_edge_factor                # Pricing efficiency
arb_process_resident_bytes         # Memory usage
```

**Alerting Rules**:
```yaml
# Prometheus alerting rules
groups:
  - name: arb-engine
    rules:
      - alert: ArbEngineDown
        expr: up{job="arb-engine"} == 0
        for: 30s

      - alert: HighErrorRate  
        expr: arb_edge_error_ratio > 0.25
        for: 2m

      - alert: StaleData
        expr: arb_edge_staleness_ms > 10000  # 10 seconds
        for: 1m

      - alert: LowProfitability
        expr: rate(arb_signals_total[5m]) < 0.1  # <6 signals/minute
        for: 5m
```

### 4. Troubleshooting Production Issues

**High Memory Usage**:
```bash
# Check memory metrics
curl http://localhost:8080/metrics | grep resident_bytes

# Trigger cleanup
curl -X POST http://localhost:8080/ai/config/reload

# Reduce cache sizes
kubectl set env deployment/arb-engine ARB_MAX_PAIRS_TOTAL=5000
```

**Circuit Breaker Issues**:
```bash
# Check status
kubectl exec -it arb-engine-pod -- \
  curl http://localhost:8080/ready

# Reset if needed
kubectl exec -it arb-engine-pod -- \
  curl -X POST -H "X-AI-Token: $AI_TOKEN" \
  http://localhost:8080/circuit/reset
```

**Performance Degradation**:
```bash
# Check recent signals
curl http://localhost:8080/ai/recent?limit=10

# Analyze profit distribution
curl http://localhost:8080/metrics | grep arb_signal_profit_bps

# Check structural metrics
curl http://localhost:8080/metrics | grep arb_edge_pairs_total
```

## Common Error Messages & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| "Signal queue full" | Too many signals generated | Increase queue size or profit threshold |
| "Circuit breaker open" | High error rate | Check edge data quality, reset circuit |
| "Pool metadata missing" | WhirlpoolMeta not populated | Enable whirlpool feature, check normalizer |
| "Feature not enabled" | Missing Cargo feature | Build with `--features=whirlpool` |
| "Edge decode errors" | Invalid protobuf data | Check edge-assembler output format |
| "No recent blockhash" | Missing blockhash cache | Implement proper blockhash fetching |

## Performance Benchmarks

**Target Performance** (release build):
- Edge processing: < 1ms per edge
- 2-leg detection: < 5ms per pair update  
- Memory usage: < 500MB for 10K pairs
- Signal latency: < 10ms edge-to-emission
- Throughput: > 1000 edges/second

**Scaling Limits**:
- Max pairs: 50,000 (with 16GB RAM)
- Max tokens: 10,000 in adjacency index
- Max edges per pair: 50 (before performance degradation)

If you're still encountering issues, check the logs with:
```bash
RUST_LOG=arb_engine=debug cargo run --features=dev-full 2>&1 | tee debug.log
```

And examine the specific error patterns in the debug output.