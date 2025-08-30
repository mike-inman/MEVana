# Geyser Streamer

Integrated real-time Solana transaction streaming with IDL-based parsing and arbitrage detection.

## Overview

The Geyser Streamer combines the best of existing infrastructure (`dex-stream-universal` and `searcher-dev`) with comprehensive IDL-based parsing and real-time arbitrage detection.

## Features

### 🔄 Real-Time Streaming
- **Geyser Integration**: Direct connection to Yellowstone Geyser
- **Program Monitoring**: Track transactions from all major DEX programs
- **Account Pool Monitoring**: Monitor specific pool accounts for state changes

### 🧩 IDL-Based Parsing
- **Raydium CPMM/CLMM**: Complete pool state and instruction parsing
- **Meteora DLMM/DAMM**: Dynamic liquidity and bin parsing
- **Orca Whirlpool**: Advanced CLMM with tick array support
- **Automatic Updates**: Uses official DEX program layouts

### 🎯 Arbitrage Detection
- **Cross-DEX Opportunities**: Find price differences across venues
- **Real-Time Alerts**: Immediate opportunity detection
- **Risk Assessment**: Confidence scoring and risk analysis
- **Gas Optimization**: Cost-aware profit calculations

### 📊 Event Extraction
- **SwapEvent**: Every trade across all integrated DEXes
- **LiquidityEvent**: LP additions/removals with position details
- **Pool State Updates**: Real-time liquidity and price changes

## Configuration

Copy and modify the configuration template:

```bash
cp geyser-streamer.env .env
```

### Key Configuration Options

```bash
# Geyser connection
ENDPOINT=https://grpc.ny.shyft.to
X_TOKEN=your_auth_token

# DEX programs to monitor
PROGRAM_IDS=675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8,...

# Specific pool accounts (for state monitoring)
ACCOUNT_POOLS=pool_address1,pool_address2,...

# Arbitrage settings
MIN_SPREAD_BPS=5
ENABLE_ARBITRAGE=true
```

## Usage

### Basic Streaming

```bash
# Start with default DEX programs
cargo run --release

# Monitor specific programs
cargo run --release -- --program-ids "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"

# Monitor specific pool accounts
cargo run --release -- --account-pools "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"
```

### Health Monitoring

```bash
# Health status
curl http://localhost:8080/health

# Prometheus metrics
curl http://localhost:8080/metrics
```

## Architecture

### Data Flow

```
Geyser Stream → IDL Parsers → PoolSnapshot/SwapEvent
    ↓
Arbitrage Detector ← Cross-DEX Price Comparison
    ↓
Trading Signals → Risk Assessment → Opportunities
```

### Components

1. **Geyser Client**: Yellowstone Geyser streaming connection
2. **IDL Registry**: Layout definitions for all supported DEXes
3. **Event Extractors**: Transaction parsing for each DEX
4. **Arbitrage Detector**: Cross-DEX opportunity identification
5. **Health Server**: Monitoring and metrics endpoints

## Supported DEXes

| DEX | Program ID | Features |
|-----|------------|----------|
| **Raydium CPMM** | `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` | Pool state, swaps, liquidity |
| **Raydium CLMM** | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqGnT` | CLMM pools, tick arrays |
| **Meteora DLMM** | `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo` | Dynamic bins, fee tiers |
| **Orca Whirlpool** | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | CLMM, position management |
| **OpenBook** | `srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX` | Order book, trades |
| **GooseFX** | `7WduLbRfYhTJktjLw5K2UqKGavXrGtAwxedvUNW4h4Ez` | Aggregator features |

## Output Formats

### Pool State Updates
```json
{
  "dex": "raydium-clmm",
  "pool_id": "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",
  "slot": 363212195,
  "token_a_mint": "So11111111111111111111111111111111111111112",
  "token_b_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "sqrt_price_x64": "4295048016",
  "liquidity": "1000000000",
  "tick_current": 12345,
  "fee_bps_total": 30
}
```

### Arbitrage Opportunities
```json
{
  "id": "arb-58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2-...",
  "timestamp": 363212195,
  "pair": "SOL/USDC",
  "dex_buy": "raydium-clmm",
  "dex_sell": "meteora-dlmm",
  "buy_price": 150.25,
  "sell_price": 150.50,
  "spread_bps": 16.6,
  "estimated_profit_usd": 125.75,
  "gas_estimate_lamports": 30000,
  "confidence_score": 0.85,
  "risk_factors": []
}
```

## Integration

### With Individual Services

The Geyser Streamer can feed data to individual DEX services:

```bash
# Start Geyser streamer
cargo run --release -- --program-ids "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"

# Start individual Raydium service
cd ../raydium-service
cargo run --release -- --pool-ids "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"
```

### With Arbitrage Detector

```rust
use arbitrage_detector::ArbitrageDetector;

// Create detector
let detector = ArbitrageDetector::new(config);

// Process pool snapshots from streamer
for snapshot in streamer.get_pool_snapshots() {
    let opportunities = detector.process_pool_snapshot(snapshot).await?;
    for opp in opportunities {
        // Execute arbitrage
        execute_arbitrage(opp).await?;
    }
}
```

## Performance

### Benchmarks
- **Throughput**: 1000+ transactions/second
- **Latency**: <100ms from Geyser to arbitrage detection
- **Memory**: ~50MB for typical DEX monitoring
- **CPU**: Single core for basic monitoring

### Scaling
- **Horizontal**: Multiple instances with different program sets
- **Vertical**: Increase resources for high-volume DEXes
- **Filtering**: Focus on high-value programs/pools

## Monitoring

### Health Checks
- **Geyser Connection**: Stream health and reconnection
- **Parser Status**: IDL parsing success rates
- **Arbitrage Detection**: Opportunity finding metrics
- **Performance**: Throughput and latency metrics

### Metrics
```prometheus
# Transaction processing
dex_stream_transactions_processed_total{program="raydium"} 15432

# Arbitrage opportunities
dex_stream_arbitrage_opportunities_total 89

# Pool state updates
dex_stream_pool_updates_total{dex="raydium-clmm"} 567
```

## Troubleshooting

### Common Issues

1. **Geyser Connection Failed**
   ```bash
   # Check endpoint and token
   curl -H "x-token: YOUR_TOKEN" https://your-endpoint.com
   ```

2. **No Transactions**
   ```bash
   # Verify program IDs are correct
   # Check if DEX programs are active
   ```

3. **High Latency**
   ```bash
   # Reduce program monitoring scope
   # Use account-specific monitoring instead
   ```

### Debug Mode

```bash
RUST_LOG=debug cargo run --release
```

## Contributing

### Adding New DEX Support

1. Add IDL layouts to `idl-registry/`
2. Implement parser in appropriate module
3. Add program ID to default configuration
4. Update documentation

### Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test -- --ignored
```

## License

See project root for license information.
