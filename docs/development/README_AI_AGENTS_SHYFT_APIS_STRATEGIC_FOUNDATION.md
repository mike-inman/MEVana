# README FOR AI AGENTS: Shyft APIs Strategic Foundation
Last updated: 2025-09-15
**Document Last Updated: 2024-08-17 18:45 CDT**

## 🎯 **CRITICAL STRATEGIC DIRECTION FOR AI AGENTS**

### **Hybrid Architecture: Geyser PRIMARY, Shyft SUPPORTING**
- **Geyser Streams**: Primary data source for sub-10ms trading latency
- **Shyft APIs**: Validation, debugging, and AI-driven development support
- **NEVER replace Geyser with Shyft APIs for live trading** - this would create speed bottlenecks

### **AI-Driven Development Strategy**
- **Long-term goal**: Self-learning, self-improving trading system run by AI agents
- **Validation services**: Available to AI agents for independent work validation
- **GraphQL, RPC, REST APIs**: Implemented as needed for comprehensive validation
- **Self-improvement**: AI agents can validate and improve parsing, strategies, and performance

## 🔑 **API Credentials (DO NOT EXPOSE IN CODE)**

### **Shyft API Key (for REST/GraphQL APIs)**
```
SHYFT_API_KEY=<SHYFT_API_KEY>
```

### **Shyft X-Token (for Geyser gRPC Stream)**
```
X_TOKEN=[REDACTED - Check .env files]
```

### **Endpoints**
- **RPC**: `https://rpc.shyft.to?api_key=<SHYFT_API_KEY>`
- **WebSocket**: `wss://rpc.shyft.to?api_key=<SHYFT_API_KEY>`
- **GraphQL**: `https://programs.shyft.to/v0/graphql/?api_key=<SHYFT_API_KEY>&network=mainnet-beta`
- **Swagger UI**: `https://api.shyft.to/sol/api/explore/`

## 🏗️ **Architecture Components**

### **1. Core Trading Services (Speed-Critical)**
- `dex-stream-universal`: Geyser stream consumer (sub-10ms latency)
- `dex-backfill`: On-chain data parsing and Redis population
- `arb-test-mode`: Arbitrage detection and PnL calculation
- `depth-builder`: Pool depth and slippage calculations

### **2. Validation Services (Accuracy-Critical)**
- `shyft-validator`: Standalone validation service for AI agents
- **Performance**: ~650-800ms per validation (non-blocking)
- **Use Cases**: Development testing, debugging, production monitoring
- **AI Integration**: Available as library for AI agent validation

### **3. AI Foundation Services**
- `discovery-service`: Unknown program ID detection for AI analysis
- `ui-gateway`: API endpoints for monitoring and management

## 📡 **Shyft API Catalog for AI Agents**

### **DeFi Pool Discovery APIs**
- `get_pools_by_address`: Fetch pools by specific addresses
- `get_pools_by_liquidity_pair`: Find pools for token pairs
- `get_all_pools_for_token`: All pools containing a specific token
- `get_liquidity_details_of_pool`: Detailed liquidity information

### **Wallet & Portfolio APIs**
- `get_all_tokens_balance`: Complete wallet token balances
- `get_token_balance`: Specific token balance
- `get_stake_accounts`: Staking account information

### **Transaction & Data APIs**
- `get_transaction_parsed`: Human-readable transaction data
- `get_transaction_history`: Transaction history with metadata
- **GraphQL APIs**: Advanced querying for complex data relationships

### **Validation & Debugging Tools**
- **Translator Tool**: `https://translator.shyft.to/` - Validate parsing accuracy
- **Swagger UI**: Interactive API testing and exploration

## 🚀 **Implementation Patterns for AI Agents**

### **Validation Pattern**
```rust
use shyft_validator::ShyftValidator;

let validator = ShyftValidator::new(api_key, "mainnet-beta".to_string());
let result = validator.validate_tickarray_parsing(pool_address, our_parsing).await?;

if !result.matches {
    // AI agent can analyze mismatch and improve parsing
    log::warn!("Validation failed: our parsing doesn't match Shyft data");
}
```

### **Non-Blocking Validation**
```rust
// Spawn validation in background to avoid blocking core trading path
tokio::spawn(async move {
    if let Ok(validation) = validator.validate_tickarray_parsing(&pool_address, &our_parsing).await {
        // Handle validation result asynchronously
    }
});
```

### **Batch Validation for Efficiency**
```rust
let validations = vec![
    (pool1, parsing1),
    (pool2, parsing2),
    // ... more pools
];

let results = validator.batch_validate_pools(validations).await?;
```

## ⚠️ **CRITICAL WARNINGS FOR AI AGENTS**

### **DO NOT:**
1. **Replace Geyser streams with Shyft APIs** for live trading data
2. **Block core trading path** with validation calls
3. **Expose API keys** in code or logs
4. **Ignore rate limits** on Shyft APIs
5. **Mix SHYFT_API_KEY and X_TOKEN** - they serve different purposes

### **ALWAYS:**
1. **Use Geyser streams** for sub-10ms trading data
2. **Validate asynchronously** to maintain performance
3. **Implement rate limiting** for Shyft API calls
4. **Test locally first** before Docker deployment
5. **Document validation results** for AI learning

## 🔮 **AI-Driven Future Architecture**

### **Self-Learning Components**
- **Validation Feedback Loop**: AI agents learn from validation mismatches
- **Parsing Improvement**: Automatic IDL updates and parsing refinements
- **Strategy Optimization**: AI-driven arbitrage strategy evolution
- **Performance Tuning**: Continuous latency and accuracy improvements

### **AI Agent Capabilities**
- **Independent Validation**: Validate work without human intervention
- **Cross-Service Learning**: Learn from multiple service interactions
- **Strategy Discovery**: Find new profitable trading opportunities
- **System Optimization**: Continuously improve overall system performance

## 📁 **Key Files for AI Agents**

### **Validation Service**
- `crates/shyft-validator/src/main.rs`: Core validation logic
- `crates/shyft-validator/Cargo.toml`: Dependencies and configuration

### **Core Services**
- `crates/dex-backfill/src/main.rs`: TickArray parsing and Redis population
- `crates/dex-stream-universal/src/main.rs`: Geyser stream processing
- `crates/arb-test-mode/src/main.rs`: Arbitrage detection logic

### **Configuration & Environment**
- `.env` files: API keys and configuration
- `infra/docker/docker-compose.yml`: Service orchestration
- `scripts/test-validation-pipeline.sh`: Validation testing

## 🎯 **Next Steps for AI Agents**

1. **Implement comprehensive validation** using Shyft APIs
2. **Create AI agent validation interfaces** for independent work
3. **Build self-improvement feedback loops** for parsing and strategies
4. **Develop cross-service learning capabilities** for system optimization
5. **Establish AI agent collaboration protocols** for collective improvement

---

**Remember**: This system is designed to become self-learning and self-improving. Every validation, every parsing attempt, every strategy test contributes to the AI agents' knowledge and capabilities. Build with this future in mind! 🚀
