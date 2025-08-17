# README FOR AI AGENTS: SHYFT APIs STRATEGIC FOUNDATION
**Last Updated**: 2024-08-17  
**Critical Reading**: AI agents MUST read this document before making any changes to the MEVana platform  
**Strategic Context**: This document explains the pivot from on-chain parsing to Shyft APIs and why this approach is critical for success

## 🚨 CRITICAL CONTEXT FOR AI AGENTS

### **WHY THIS DOCUMENT EXISTS**
- **Previous Approach Failed**: On-chain TickArray deserialization consistently failed with "Invalid bool representation" errors
- **Strategic Pivot**: We moved to a HYBRID approach combining geyser streams with Shyft API validation
- **Performance Target**: Sub-10ms latency from signal to execution requires FAST geyser streaming
- **AI Agent Warning**: Use HYBRID approach - geyser for speed, Shyft APIs for validation/cache warming

### **WHAT AI AGENTS MUST UNDERSTAND**
1. **Geyser streams are PRIMARY** - for live transaction data and speed
2. **Shyft APIs are SUPPORTING** - for validation, cache warming, and fallback data
3. **Performance optimization** requires both speed (geyser) and reliability (Shyft validation)
4. **This hybrid approach is strategic** - combines best of both worlds

## 🔑 SHYFT API CREDENTIALS & ENDPOINTS

### **Primary API Key**
```
SHYFT_API_KEY=fWrVh_mDygK358FC
```

### **Core Endpoints**
- **RPC**: `https://rpc.shyft.to?api_key=fWrVh_mDygK358FC`
- **WebSocket**: `wss://rpc.shyft.to?api_key=fWrVh_mDygK358FC`
- **GraphQL**: `https://programs.shyft.to/v0/graphql/?api_key=fWrVh_mDygK358FC&network=mainnet-beta`

### **Environment Variables Required**
```bash
export SHYFT_API_KEY=fWrVh_mDygK358FC
export SHYFT_NETWORK=mainnet-beta
export SHYFT_GRPC_ENDPOINT=<your_grpc_endpoint>
export SHYFT_GRPC_TOKEN=<your_grpc_token>
```

## 📊 SHYFT API CATALOG - COMPLETE REFERENCE

### **DeFi Pool Discovery APIs (CRITICAL FOR MEVANA)**

#### 1. `get_pools_by_address`
- **Endpoint**: `https://docs.shyft.to/solana-defi-apis/defi-apis#get-pools-by-address`
- **Purpose**: Direct pool lookup by address
- **Use Case**: Pool-specific operations, validation
- **Strategic Value**: Eliminates need for complex account parsing

#### 2. `get_all_pools_for_token`
- **Endpoint**: `https://docs.shyft.to/solana-defi-apis/defi-apis#get-all-pools-for-a-token`
- **Purpose**: Discover all pools containing a specific token
- **Use Case**: Token pair discovery, arbitrage opportunity identification
- **Strategic Value**: Comprehensive market coverage without parsing

#### 3. `get_pools_by_liquidity_pair`
- **Endpoint**: `https://docs.shyft.to/solana-defi-apis/defi-apis#get-pools-by-liquidity-pair`
- **Purpose**: Find pools for specific token pairs
- **Use Case**: Direct pair trading, arbitrage path finding
- **Strategic Value**: Efficient pair-specific queries

#### 4. `get_liquidity_details_of_a_pool`
- **Endpoint**: `https://docs.shyft.to/solana-defi-apis/defi-apis#get-liquidity-details-of-a-pool`
- **Purpose**: Detailed liquidity information and depth data
- **Use Case**: Slippage calculations, trade size optimization
- **Strategic Value**: Pre-calculated depth data vs. complex TickArray parsing

### **Wallet & Portfolio APIs**

#### 5. `get_all_tokens_balance`
- **Endpoint**: `https://docs.shyft.to/solana-apis/wallet#get-all-tokens-balance`
- **Purpose**: Complete wallet portfolio overview
- **Use Case**: Position tracking, risk management
- **Strategic Value**: Real-time portfolio data

#### 6. `get_token_balance`
- **Endpoint**: `https://docs.shyft.to/solana-apis/wallet#get-token-balance`
- **Purpose**: Specific token balance in wallet
- **Use Case**: Individual position checks
- **Strategic Value**: Precise balance monitoring

#### 7. `get_stake_accounts`
- **Purpose**: Staking position information
- **Use Case**: Staking management, yield optimization
- **Strategic Value**: Complete financial position data

### **Transaction & Data APIs**

#### 8. `get_transaction_parsed`
- **Reference**: `https://blogs.shyft.to/how-to-get-decoded-solana-transactions-d73d57ef5b66`
- **Purpose**: Human-readable transaction data
- **Use Case**: Transaction analysis, debugging, audit trails
- **Strategic Value**: Eliminates need for complex transaction parsing

#### 9. `get_transaction_history`
- **Purpose**: Historical transaction data
- **Use Case**: Pattern analysis, performance tracking
- **Strategic Value**: Historical context without parsing overhead

### **Advanced Querying & Tools**

#### 10. **GraphQL APIs**
- **Endpoint**: `https://programs.shyft.to/v0/graphql/?api_key=fWrVh_mDygK358FC&network=mainnet-beta`
- **Purpose**: Advanced querying with filters and relationships
- **Use Case**: Complex data queries, real-time subscriptions
- **Strategic Value**: Flexible data access patterns

#### 11. **GraphQL Filtering**
- **Reference**: `https://docs.shyft.to/solana-indexers/graphql-apis/applying-filters`
- **Purpose**: Targeted data queries
- **Use Case**: Specific data extraction, performance optimization
- **Strategic Value**: Efficient data filtering

#### 12. **Swagger UI**
- **Endpoint**: `https://api.shyft.to/sol/api/explore/`
- **Purpose**: Interactive API documentation and testing
- **Use Case**: API exploration, endpoint testing, development
- **Strategic Value**: Self-documenting API interface

#### 13. **Translator Tool**
- **Endpoint**: `https://translator.shyft.to/`
- **Purpose**: Validate parsing of raw blockchain data
- **Use Case**: Debug parsing issues, validate data structures
- **Strategic Value**: Quality assurance for any remaining parsing needs

## 🎯 STRATEGIC IMPLEMENTATION GUIDANCE

### **Phase 1: Foundation (CURRENT)**
1. **GraphQL Backfill**: Seed Redis with pool/mint metadata using Shyft APIs
2. **Dex Backfill**: Maintain current pool data using Shyft APIs
3. **Extractor Caches**: Local LRU caches with Redis read-through
4. **Basic Pipeline**: Geyser (FAST) → Extractor → Arb Engine
5. **Validation Layer**: Cross-reference geyser parsing with Shyft API data

### **Phase 2: Optimization**
1. **FAST_MODE Toggles**: Low-latency Kafka configs
2. **Hot Path Bypass**: Co-locate components, bypass Kafka for critical paths
3. **Performance Monitoring**: Latency histograms, PnL dashboards
4. **Kill Switches**: PnL-based and latency-based safety mechanisms

### **Phase 3: Advanced Features**
1. **AI Integration**: Pattern recognition, strategy optimization
2. **Multi-Venue**: Raydium, Meteora integration
3. **Advanced Analytics**: Predictive modeling, risk assessment
4. **Production Deployment**: Jito integration, private relays

## 🚫 WHAT NOT TO DO (AI AGENT WARNINGS)

### **❌ AVOID THESE APPROACHES**
1. **Complete API Replacement**: Don't abandon geyser streams - they're needed for speed
2. **Complex IDL Deserialization**: Data structures don't match on-chain reality
3. **Raw Account Data Parsing**: Unreliable and performance-intensive
4. **Ignoring Shyft APIs**: Use them for validation and cache warming

### **✅ ALWAYS USE THESE APPROACHES**
1. **Hybrid Approach**: Geyser streams for speed, Shyft APIs for validation
2. **Redis Caching**: With TTL-based LRU for performance
3. **FAST_MODE**: For low-latency configurations
4. **Iterative Development**: Small changes, test, commit, repeat
5. **Cross-Validation**: Compare geyser parsing with Shyft API data

## 🔧 IMPLEMENTATION PATTERNS

### **Redis Data Structure**
```redis
# Pool Data
HSET dex:pool:{pool_id} mint_a, mint_b, decimals_a, decimals_b, fee_bps, sqrt_price_x64, tick, updated_at, pair, pair_symbol

# Mint Metadata
HSET dex:mint:{mint} decimals, symbol, name, updated_at

# Pair Mappings
SET dex:pair:latest_pool:{mintA/mintB} -> {POOL_ID}
SET dex:pair_symbol:latest_pool:{SYM_A/SYM_B} -> {POOL_ID}
```

### **FAST_MODE Configuration**
```rust
if fast_mode {
    config
        .set("linger.ms", "0")
        .set("acks", "1")
        .set("compression.type", "lz4")
        .set("enable.idempotence", "false")
        .set("message.timeout.ms", "100");
}
```

### **Cache Implementation Pattern**
```rust
// Local LRU with Redis read-through
static MINT_DECIMALS_CACHE: Lazy<StdMutex<lru::LruCache<String, TtlEntry<u8>>>> = Lazy::new(|| {
    StdMutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap()))
});
```

## 📚 RELATED DOCUMENTATION

### **Must-Read Documents**
1. `docs/roadmap.md` - Current project status and phases
2. `docs/strategy/hybrid-shyft-architecture.md` - Architecture overview
3. `docs/strategy/fast-mode.md` - Performance optimization details
4. `docs/strategy/arb-test-mode.md` - Arbitrage testing framework

### **Code References**
1. `crates/shyft-pool-depth/` - Shyft API integration service
2. `crates/graphql-backfill/` - Pool metadata seeding
3. `crates/dex-backfill/` - Continuous data maintenance
4. `crates/normalizer-service/` - Data processing with local caches

## 🎯 SUCCESS METRICS

### **Performance Targets**
- **Latency**: Sub-10ms from signal to execution
- **Reliability**: 99.9% uptime for data feeds
- **Throughput**: Handle 1000+ transactions per second
- **Accuracy**: Zero parsing errors from Shyft APIs

### **Quality Indicators**
- **Data Freshness**: Pool data updated within 1 second
- **Cache Hit Rate**: >90% for mint decimals and pool snapshots
- **Error Rate**: <0.1% for API calls
- **Latency P99**: <25ms for critical paths

## 🚀 NEXT STEPS FOR AI AGENTS

### **Immediate Actions Required**
1. **Read this document completely** before making any changes
2. **Understand the strategic pivot** from on-chain to Shyft APIs
3. **Implement using Shyft APIs** for all data needs
4. **Avoid on-chain parsing** - it will fail

### **Development Workflow**
1. **Small Changes**: Make incremental improvements
2. **Test Thoroughly**: Verify with logs and metrics
3. **Document Changes**: Update this document with new discoveries
4. **Commit Frequently**: Use professional Git practices

### **When in Doubt**
1. **Check this document** for guidance
2. **Use Shyft APIs** instead of complex parsing
3. **Implement caching** for performance
4. **Test with real data** before deploying

---

**Remember**: This document represents a strategic decision that was made after extensive testing and failure analysis. The HYBRID approach combining geyser streams with Shyft API validation is the foundation for a reliable, high-performance arbitrage platform. AI agents must respect this architectural decision and build upon it rather than attempting to replace it.

---

**Document Last Updated**: 2024-08-17 15:30 CDT  
**Strategic Revision**: Corrected from complete API replacement to hybrid approach  
**Next Update**: When TickArray challenge is solved using Shyft API validation
