# MEVana Data Flow - Simplified View
**Date:** 2024-12-17  
**Version:** 1.0  
**Purpose:** Printer-friendly reference for development and testing

## Data Flow Overview

```
SOLANA GEYSER STREAM (Origin)
    ↓ (Sub-10ms latency)
    
DEX STREAM SERVICES
├── dex-stream-orca (Whirlpool parser)
├── dex-stream-universal (Multi-protocol)
└── dex-backfill (Historical data)
    ↓
    
NORMALIZER SERVICES
├── normalizer-service (Protocol-agnostic)
├── normalizer-orca (Orca-specific)
└── normalizer-generated (Auto-generated)
    ↓
    
REDIS CACHE (shared-redis)
    ↓
    
VALIDATION SERVICES
├── shyft-validator (API validation)
├── shyft-pool-depth (Pool depth)
└── discovery-service (Unknown programs)
    ↓
    
BUSINESS LOGIC
├── arb-test-mode (Arbitrage detection)
├── price-aggregator (Price calculation)
└── graphql-backfill (Historical analysis)
    ↓
    
UI & MONITORING
├── ui-gateway (API gateway)
├── ui-web (Dashboard)
└── infrastructure (Prometheus + Grafana)
```

## Key Data Paths

### 1. PRIMARY ARBITRAGE PATH
```
Geyser → dex-stream-orca → normalizer-orca → Redis → arb-test-mode
```

### 2. VALIDATION PATH
```
Redis → shyft-validator → Shyft APIs → Validation Results
```

### 3. DISCOVERY PATH
```
dex-stream-universal → discovery-service → AI Analysis Queue
```

### 4. MONITORING PATH
```
All Services → Prometheus → Grafana → Alerts
```

## Service Status (Current)

### ✅ WORKING SERVICES
- dex-stream-orca
- dex-stream-universal  
- normalizer-service
- shyft-validator
- shyft-pool-depth
- arb-test-mode
- price-aggregator
- Most normalizer binaries

### ❌ BROKEN SERVICES
- discovery-service (compilation errors)

## Testing Priority Order

1. **Data Pipeline Flow** - Test from Geyser to Redis
2. **Validation Pipeline** - Test Shyft API validation
3. **Arbitrage Detection** - Test opportunity identification
4. **Performance Testing** - Verify sub-10ms latency
5. **Data Quality** - Ensure accuracy and completeness

## Next Actions

1. Fix discovery-service compilation errors
2. Implement comprehensive testing suite
3. Add real-time market data validation
4. Set up automated data quality monitoring
5. Create performance benchmarking tools

---

**Note**: Focus on testing the working services first, then fix the broken ones. The core arbitrage detection pipeline is functional.
