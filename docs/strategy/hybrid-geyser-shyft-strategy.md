# HYBRID GEYSER + SHYFT STRATEGY
Last updated: 2025-09-15
**Created**: 2024-08-17 15:30 CDT  
**Last Updated**: 2024-08-17 15:30 CDT  
**Strategic Context**: Hybrid approach combining speed (geyser) with reliability (Shyft APIs)  
**Critical Success Factor**: Sub-10ms latency from signal to trade execution

## 🎯 STRATEGIC OVERVIEW

### **Why Hybrid Approach?**
- **Speed Requirement**: Sub-10ms latency is only achievable with direct geyser streaming
- **Reliability Need**: Shyft APIs provide validated data for cross-reference and cache warming
- **TickArray Challenge**: Shyft APIs can solve the "Invalid bool representation" errors
- **Competitive Advantage**: Speed + reliability = profitable arbitrage

### **Core Principles**
1. **Geyser Streams**: Primary data source for live trading decisions
2. **Shyft APIs**: Secondary source for validation, cache warming, and fallback
3. **Cross-Validation**: Compare geyser parsing with Shyft data for accuracy
4. **Performance First**: Never compromise speed for reliability

## 🚀 IMPLEMENTATION STRATEGY

### **Phase 1: Foundation (CURRENT - 2024-08-17)**
1. **GraphQL Backfill**: Seed Redis with pool/mint metadata using Shyft APIs
2. **Dex Backfill**: Maintain current pool data using Shyft APIs
3. **Extractor Caches**: Local LRU caches with Redis read-through
4. **Basic Pipeline**: Geyser (FAST) → Extractor → Arb Engine
5. **Validation Layer**: Cross-reference geyser parsing with Shyft API data

### **Phase 2: TickArray Solution (IMMEDIATE PRIORITY)**
1. **Use Shyft API for TickArray Data**: `get_liquidity_details_of_a_pool`
2. **Cross-Validate**: Compare geyser TickArray parsing with Shyft data
3. **Fallback Strategy**: Use Shyft data when geyser parsing fails
4. **Performance Monitoring**: Measure latency impact of validation layer

### **Phase 3: Optimization (NEXT WEEK)**
1. **FAST_MODE Toggles**: Low-latency Kafka configs
2. **Hot Path Bypass**: Co-locate components, bypass Kafka for critical paths
3. **Performance Monitoring**: Latency histograms, PnL dashboards
4. **Kill Switches**: PnL-based and latency-based safety mechanisms

### **Phase 4: Advanced Features (FUTURE)**
1. **AI Integration**: Pattern recognition, strategy optimization
2. **Multi-Venue**: Raydium, Meteora integration
3. **Advanced Analytics**: Predictive modeling, risk assessment
4. **Production Deployment**: Jito integration, private relays

## 🔧 TICKARRAY CHALLENGE SOLUTION

### **Problem Statement**
- **Issue**: On-chain TickArray deserialization fails with "Invalid bool representation" errors
- **Impact**: Depth calculations return `num_init=0` and `sum_gross=0`
- **Root Cause**: IDL structures don't match on-chain data reality

### **Solution: Shyft API Validation**
1. **Primary Data**: Continue using geyser streams for speed
2. **Validation Data**: Use Shyft `get_liquidity_details_of_a_pool` for cross-reference
3. **Fallback**: When geyser parsing fails, use Shyft data
4. **Learning**: Use Shyft data to improve geyser parsing accuracy

### **Implementation Steps**
1. **API Integration**: Add Shyft API calls to validation layer
2. **Cross-Reference**: Compare geyser TickArray data with Shyft pool data
3. **Error Detection**: Identify when geyser parsing produces invalid results
4. **Fallback Logic**: Switch to Shyft data when validation fails
5. **Performance Monitoring**: Ensure validation doesn't impact sub-10ms target

## 📊 DATA FLOW ARCHITECTURE

### **Live Trading Path (FAST)**
```
Geyser Stream → TickArray Parser → Depth Calculator → Arb Engine
     ↓              ↓              ↓              ↓
   <1ms          <2ms           <3ms          <4ms
```

### **Validation Path (RELIABLE)**
```
Shyft API → Pool Data → Cross-Reference → Cache Update
    ↓           ↓            ↓              ↓
  <50ms      <10ms        <5ms           <1ms
```

### **Cache Warming Path (BACKGROUND)**
```
Shyft API → Pool Metadata → Redis Cache → Pre-populated Data
    ↓           ↓            ↓              ↓
  <100ms     <20ms        <5ms           <1ms
```

## 🎯 SUCCESS METRICS

### **Performance Targets**
- **Geyser Path**: <5ms total latency
- **Validation Path**: <100ms (non-blocking)
- **Cache Hit Rate**: >95% for pool data
- **Error Rate**: <0.1% for TickArray parsing

### **Quality Indicators**
- **Data Freshness**: Pool data updated within 1 second
- **Validation Success**: >99% geyser parsing accuracy
- **Fallback Usage**: <5% of requests need Shyft fallback
- **Overall Latency**: Sub-10ms from signal to execution

## 🚫 WHAT NOT TO DO

### **❌ AVOID THESE APPROACHES**
1. **Complete API Replacement**: Don't abandon geyser streams - they're needed for speed
2. **Complex IDL Deserialization**: Data structures don't match on-chain reality
3. **Raw Account Data Parsing**: Unreliable and performance-intensive
4. **Ignoring Shyft APIs**: Use them for validation and cache warming
5. **Blocking Validation**: Don't let validation impact live trading speed

### **✅ ALWAYS USE THESE APPROACHES**
1. **Hybrid Approach**: Geyser streams for speed, Shyft APIs for validation
2. **Redis Caching**: With TTL-based LRU for performance
3. **FAST_MODE**: For low-latency configurations
4. **Iterative Development**: Small changes, test, commit, repeat
5. **Cross-Validation**: Compare geyser parsing with Shyft API data
6. **Non-blocking Validation**: Validation should not impact live trading

## 🔧 IMPLEMENTATION PATTERNS

### **TickArray Validation Pattern**
```rust
async fn validate_tickarray_data(
    geyser_data: &TickArrayData,
    pool_id: &str,
    shyft_client: &ShyftClient,
) -> Result<ValidationResult> {
    // Get Shyft pool data for validation
    let shyft_pool = shyft_client.get_pool_details(pool_id).await?;
    
    // Cross-reference geyser data with Shyft data
    let validation = cross_reference_tickarray(geyser_data, &shyft_pool)?;
    
    if validation.is_valid {
        Ok(ValidationResult::Valid(geyser_data.clone()))
    } else {
        // Use Shyft data as fallback
        Ok(ValidationResult::Fallback(shyft_pool.to_tickarray_data()))
    }
}
```

### **Non-blocking Validation Pattern**
```rust
// Spawn validation in background, don't block live trading
tokio::spawn(async move {
    if let Err(e) = validate_tickarray_data(&geyser_data, pool_id, &shyft_client).await {
        warn!("Validation failed: {}", e);
    }
});

// Continue with live trading immediately
process_tickarray_data(geyser_data);
```

## 📚 RELATED DOCUMENTATION

### **Must-Read Documents**
1. `docs/README_AI_AGENTS_SHYFT_APIS_STRATEGIC_FOUNDATION.md` - Complete API reference
2. `docs/roadmap.md` - Current project status and phases
3. `docs/strategy/fast-mode.md` - Performance optimization details
4. `docs/strategy/arb-test-mode.md` - Arbitrage testing framework

### **Code References**
1. `crates/shyft-pool-depth/` - Shyft API integration service
2. `crates/graphql-backfill/` - Pool metadata seeding
3. `crates/dex-backfill/` - Continuous data maintenance
4. `crates/normalizer-service/` - Data processing with local caches

## 🚀 IMMEDIATE NEXT STEPS

### **This Week (2024-08-17 to 2024-08-24)**
1. **Implement TickArray Validation**: Use Shyft APIs to validate geyser parsing
2. **Add Cross-Reference Layer**: Compare geyser data with Shyft data
3. **Implement Fallback Logic**: Switch to Shyft data when validation fails
4. **Performance Testing**: Ensure validation doesn't impact sub-10ms target

### **Next Week (2024-08-25 to 2024-09-01)**
1. **Optimize Validation Path**: Reduce validation latency
2. **Enhance Cache Warming**: Use Shyft APIs to pre-populate more data
3. **Advanced Monitoring**: Add validation success metrics
4. **Production Readiness**: Test complete pipeline end-to-end

---

**Document Status**: Active Strategy Document  
**Next Review**: 2024-08-24 (after TickArray validation implementation)  
**Success Criteria**: TickArray parsing working with <5ms latency using hybrid approach
