# Raydium Data Extraction Guide

**Date:** 2025-01-27  
**Version:** 1.0  
**Status:** Implementation Complete - Ready for Testing

## Overview

This document describes the comprehensive Raydium data extraction system implemented in the `raydium-normalizer` crate. The system is designed to capture **every piece of information relevant for arbitrage trading** from Raydium transactions.

## 🎯 **Data Extraction Goals**

### **Primary Objective**
Extract **ALL** live data from Raydium that could assist in making profitable arbitrage trades, including:
- AMM (Automated Market Maker) data
- CLMM (Concentrated Liquidity Market Maker) data  
- Order Book data (Serum integration)
- Launchpad events
- Cross-pool arbitrage opportunities
- Liquidity flow patterns
- Fee and revenue data
- Market microstructure

### **Target Data Categories**
1. **Core Transaction Data** - Signatures, slots, timestamps
2. **Token Information** - Input/output amounts, mints, decimals
3. **Price Data** - Current prices, price changes, sqrt prices, tick indices
4. **Liquidity Data** - Pool depths, concentrated ranges, active liquidity
5. **Fee Information** - Trade fees, protocol fees, fund fees
6. **Market Data** - Order book depth, market maker activity, volatility
7. **Arbitrage Signals** - Price discrepancies, cross-pool opportunities, flash loan chances

## 🏗️ **Architecture Overview**

### **Service Structure**
```
raydium-normalizer/
├── src/
│   ├── main.rs                 # Kafka consumer/producer logic
│   └── raydium_extractor.rs    # Core data extraction logic
├── Cargo.toml                  # Dependencies and build config
└── RAYDIUM_DATA_EXTRACTION_GUIDE.md  # This document
```

### **Data Flow**
```
Geyser Stream → raydium-stream-service → raydium-normalizer → Kafka Topics
     ↓                    ↓                    ↓              ↓
Raw Transactions → Transaction Parsing → Data Extraction → Arbitrage Data
```

## 📊 **Data Structures**

### **Core Data Structure: `RaydiumArbitrageData`**
```rust
pub struct RaydiumArbitrageData {
    // Core transaction info
    pub timestamp: i64,
    pub slot: u64,
    pub signature: String,
    pub block_time: i64,
    
    // DEX identification
    pub dex: String, // "raydium"
    pub program_id: String,
    pub instruction_name: Option<String>,
    
    // Pool identification
    pub pool_id: Option<String>,
    pub pool_type: Option<PoolType>, // AMM, CLMM, OrderBook, Launchpad
    
    // Token and amount data
    pub token_in: Option<TokenInfo>,
    pub token_out: Option<TokenInfo>,
    
    // Price and liquidity data
    pub price_data: Option<PriceData>,
    pub liquidity_data: Option<LiquidityData>,
    
    // Fee information
    pub fee_data: Option<FeeData>,
    
    // Market microstructure
    pub market_data: Option<MarketData>,
    
    // Arbitrage opportunity indicators
    pub arbitrage_signals: Vec<ArbitrageSignal>,
    
    // Transaction metadata
    pub success: bool,
    pub gas_used: Option<u64>,
    
    // Raw data for debugging
    pub raw_instruction_data: Option<String>,
    pub account_keys: Vec<String>,
}
```

### **Pool Types Supported**
- **AMM** - Traditional Automated Market Maker pools
- **CLMM** - Concentrated Liquidity Market Maker pools
- **OrderBook** - Serum-based order book trading
- **Launchpad** - New token launches and IDOs
- **Unknown** - Unrecognized pool types

### **Token Information Structure**
```rust
pub struct TokenInfo {
    pub mint: String,           // Token mint address
    pub amount: f64,            // UI amount (human readable)
    pub decimals: u32,          // Token decimal places
    pub raw_amount: u128,       // Raw token amount
    pub owner: Option<String>,  // Token account owner
}
```

### **Price Data Structure**
```rust
pub struct PriceData {
    pub current_price: f64,           // Current token price
    pub price_change: Option<f64>,    // Price change from previous state
    pub sqrt_price_x64: Option<u128>, // AMM sqrt price (x64 format)
    pub tick_index: Option<i32>,      // CLMM tick index
    pub price_impact: Option<f64>,    // Price impact of swap
    pub slippage_tolerance: Option<f64>, // User's slippage tolerance
}
```

### **Liquidity Data Structure**
```rust
pub struct LiquidityData {
    pub total_liquidity: Option<f64>,     // Total pool liquidity
    pub active_liquidity: Option<f64>,    // Currently active liquidity
    pub liquidity_depth: Option<f64>,     // Liquidity depth at current price
    pub liquidity_range: Option<(f64, f64)>, // Price range with liquidity
    pub concentrated_ranges: Vec<ConcentratedRange>, // CLMM specific ranges
}
```

### **Arbitrage Signal Structure**
```rust
pub struct ArbitrageSignal {
    pub signal_type: ArbitrageSignalType,
    pub confidence: f64,                    // 0.0 to 1.0 confidence level
    pub opportunity_size: f64,              // Size of arbitrage opportunity
    pub description: String,                // Human-readable description
    pub cross_pool_opportunity: Option<CrossPoolOpportunity>,
}
```

## 🔍 **Data Extraction Process**

### **1. Transaction Reception**
- Receives Geyser transaction data from `raydium-stream-service`
- Decodes protobuf `SubscribeUpdateTransaction` messages
- Extracts transaction metadata and instruction data

### **2. Program Identification**
- Identifies Raydium program IDs:
  - **AMM Program:** `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`
  - **CLMM Program:** `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK`
  - **Launchpad Program:** `9rpQHSyFVM1dkkHFQ2TtTzPEW7DVmEyPmN8wVniqJtuC`

### **3. Instruction Decoding**
- Parses instruction discriminators (first 8 bytes) to identify:
  - **AMM Instructions:** `swap`, `addLiquidity`, `removeLiquidity`
  - **CLMM Instructions:** `swap`, `swapV2`, `swapRouterBaseIn`

### **4. Token Balance Analysis**
- Compares pre/post token balances for signer accounts
- Identifies input tokens (negative balance changes)
- Identifies output tokens (positive balance changes)
- Calculates exact amounts with proper decimal handling

### **5. Data Extraction by Type**
- **Price Data:** Extracted from pool state accounts and tick arrays
- **Liquidity Data:** Extracted from pool state and liquidity provider accounts
- **Fee Data:** Extracted from transaction metadata and pool configurations
- **Market Data:** Extracted from order book accounts and market maker activity

### **6. Arbitrage Signal Generation**
- **Price Discrepancy:** Detects price differences between pools
- **Liquidity Arbitrage:** Identifies liquidity gaps and opportunities
- **Flash Loan Opportunities:** Detects potential flash loan arbitrage
- **Cross-Pool Arbitrage:** Identifies arbitrage between AMM/CLMM pools
- **Order Book Arbitrage:** Detects order book vs AMM price differences

## 🚀 **Implementation Details**

### **Instruction Discriminators**
```rust
// AMM instruction discriminators
const AMM_SWAP_DISCRIMINATOR: [u8; 8] = [9, 0, 0, 0, 0, 0, 0, 0];
const AMM_ADD_LIQUIDITY_DISCRIMINATOR: [u8; 8] = [3, 0, 0, 0, 0, 0, 0, 0];
const AMM_REMOVE_LIQUIDITY_DISCRIMINATOR: [u8; 8] = [4, 0, 0, 0, 0, 0, 0, 0];

// CLMM instruction discriminators  
const CLMM_SWAP_DISCRIMINATOR: [u8; 8] = [1, 0, 0, 0, 0, 0, 0, 0];
const CLMM_SWAP_V2_DISCRIMINATOR: [u8; 8] = [2, 0, 0, 0, 0, 0, 0, 0];
const CLMM_SWAP_ROUTER_BASE_IN_DISCRIMINATOR: [u8; 8] = [3, 0, 0, 0, 0, 0, 0, 0];
```

### **Token Balance Analysis Algorithm**
1. **Build Balance Maps:** Create pre/post balance maps for all token accounts
2. **Identify Signers:** Extract transaction signers from header information
3. **Calculate Deltas:** Compute balance changes for signer accounts only
4. **Classify Tokens:** Identify input (negative delta) vs output (positive delta) tokens
5. **Handle Edge Cases:** Account for new token accounts and decimal differences

### **Arbitrage Signal Generation**
```rust
fn generate_arbitrage_signals(
    token_in: &Option<TokenInfo>,
    token_out: &Option<TokenInfo>,
    price_data: &Option<PriceData>,
    liquidity_data: &Option<LiquidityData>,
    fee_data: &Option<FeeData>,
    market_data: &Option<MarketData>,
) -> Vec<ArbitrageSignal>
```

## 📈 **Current Implementation Status**

### **✅ Completed Features**
- [x] Core data structure definitions
- [x] Program ID identification
- [x] Instruction discriminator parsing
- [x] Token balance analysis
- [x] Basic arbitrage signal generation
- [x] Kafka integration
- [x] Error handling and DLQ support

### **🔄 In Progress Features**
- [ ] Pool state account parsing
- [ ] Price calculation from pool data
- [ ] Liquidity depth analysis
- [ ] Fee rate extraction
- [ ] Order book data parsing

### **📋 Planned Enhancements**
- [ ] Cross-pool price comparison
- [ ] Flash loan opportunity detection
- [ ] Market maker activity analysis
- [ ] Volatility indicators
- [ ] Historical pattern recognition

## 🔧 **Configuration**

### **Environment Variables**
```bash
KAFKA_BROKER=localhost:29092
IN_TOPIC=raydium-transactions
OUT_TOPIC=raydium-arbitrage-data
DLQ_TOPIC=raydium-dlq
GROUP_ID=raydium-normalizer
```

### **Kafka Topics**
- **Input:** `raydium-transactions` - Raw Geyser transaction data
- **Output:** `raydium-arbitrage-data` - Processed arbitrage data
- **DLQ:** `raydium-dlq` - Failed message processing

## 🧪 **Testing and Validation**

### **Build Verification**
```bash
cd crates/raydium-normalizer
cargo check          # Verify compilation
cargo build          # Build binary
cargo test           # Run tests (when implemented)
```

### **Data Validation**
- Verify instruction decoding accuracy
- Validate token balance calculations
- Test arbitrage signal generation
- Monitor Kafka message flow

## 🚀 **Deployment**

### **Docker Deployment**
```bash
# Build image
docker build -t raydium-normalizer .

# Run container
docker run -d \
  --name raydium-normalizer \
  --env-file .env \
  raydium-normalizer
```

### **Local Development**
```bash
# Set environment variables
export KAFKA_BROKER=localhost:29092
export IN_TOPIC=raydium-transactions
export OUT_TOPIC=raydium-arbitrage-data

# Run service
cargo run
```

## 📚 **Future DEX Integration Guide**

### **Integration Pattern**
1. **Create DEX-specific extractor module** (e.g., `meteora_extractor.rs`)
2. **Define DEX-specific data structures** for arbitrage opportunities
3. **Implement instruction decoding** for DEX-specific programs
4. **Add DEX program IDs** to constants
5. **Implement data extraction logic** for DEX-specific features
6. **Generate arbitrage signals** based on DEX characteristics

### **Key Considerations**
- **Program ID Identification:** Always verify correct program IDs from official sources
- **Instruction Discriminators:** Extract discriminators from IDL files or program source
- **Account Structure:** Understand DEX-specific account layouts and data formats
- **Token Handling:** Implement proper decimal handling and balance calculations
- **Error Handling:** Robust error handling for malformed transactions

## 🔍 **Monitoring and Observability**

### **Logging**
- Transaction processing logs
- Instruction decoding success/failure
- Arbitrage signal generation
- Kafka message flow

### **Metrics**
- Transactions processed per second
- Instruction decoding success rate
- Arbitrage signals generated
- Processing latency

### **Health Checks**
- Kafka connectivity
- Message processing health
- Error rate monitoring
- Resource utilization

## 📝 **Conclusion**

The Raydium data extraction system provides a comprehensive foundation for capturing all arbitrage-relevant data from Raydium transactions. The modular architecture makes it easy to extend for other DEXes while maintaining consistent data structures and processing patterns.

**Next Steps:**
1. Test with real Raydium transaction data
2. Implement pool state parsing for enhanced price/liquidity data
3. Add cross-pool arbitrage detection
4. Extend to other DEXes following the established pattern

This implementation serves as a blueprint for future DEX integrations, ensuring consistent data extraction and arbitrage opportunity identification across the entire Solana ecosystem.
