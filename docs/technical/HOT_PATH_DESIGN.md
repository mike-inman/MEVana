# HOT PATH ARBITRAGE ARCHITECTURE
Last updated: 2025-09-15

## DESIGN COMMITMENT
**Date**: 2025-09-06  
**Author**: Claude Code Assistant  
**Status**: COMMITTED - This is our north star design principle

## CORE PRINCIPLE
**SPEED BEATS EVERYTHING** - We prioritize nanosecond-level latency over architectural purity. Every microsecond matters in MEV capture.

## HOT PATH DESIGN

### Primary Data Flow (ZERO SERIALIZATION)
```
Geyser Stream → Multi-Geyser-Streams → Direct Memory → Arb-Engine → Execution
     ^                                      ^              ^
   Raw gRPC                         In-process         Direct function
  (unavoidable)                      channels             calls
```

### Design Rules
1. **NEVER serialize hot data** - All critical arbitrage data stays in Rust memory
2. **NEVER use Kafka for hot path** - Kafka is for persistence, metrics, and cold storage only  
3. **Direct function calls** - No message passing for time-critical operations
4. **Shared memory structures** - Use Arc<RwLock<T>> or lockfree structures for shared state
5. **Selective persistence** - Only write to disk/network AFTER opportunity analysis

### Data Lifecycle

#### HOT PATH (In-Memory Only)
- **Fresh DEX transactions** (< 100ms old)
- **Active arbitrage opportunities** (being analyzed)  
- **Live pool states** (current tick/liquidity data)
- **Pending execution decisions** (sub-second processing)

#### WARM PATH (Background Persistence)  
- **Analyzed opportunities** → PostgreSQL (for backtesting)
- **Historical pool states** → Redis (for pattern analysis)
- **Metrics/monitoring** → Prometheus/Kafka (for dashboards)

#### COLD PATH (Async Storage)
- **Executed trades** → PostgreSQL (for accounting)
- **Failed opportunities** → Kafka (for analysis) 
- **Debug/audit data** → Disk logs (for troubleshooting)

## IMPLEMENTATION STRATEGY

### Phase 1: Multi-Geyser Integration
- Merge arb-engine logic directly into multi-geyser-streams binary
- Use direct function calls between stream processing and arbitrage detection
- Eliminate ALL serialization overhead in critical path

### Phase 2: Smart State Management  
- Implement lockfree data structures for pool states
- Use memory-mapped files for large reference data (token metadata, etc.)
- Background threads handle non-critical persistence

### Phase 3: Execution Integration
- Direct wallet/RPC calls from hot path
- Async logging of results
- Circuit breakers for failed execution paths

## PERFORMANCE TARGETS
- **Geyser to Decision**: < 1ms P99
- **Decision to Execution**: < 5ms P99  
- **Total Opportunity Latency**: < 10ms P99

## MONITORING STRATEGY
All monitoring data flows OUT of the hot path via background channels:
- Real-time metrics → Prometheus (async)
- Trade results → PostgreSQL (async)
- Debug data → Structured logs (async)

## COMMITMENT
This architecture prioritizes speed over everything else. We will NOT compromise hot path performance for:
- Easier debugging
- Better observability  
- Architectural "cleanliness"
- Framework convenience

The hot path wins trades. Everything else is secondary.
