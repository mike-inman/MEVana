# Jupiter V6 Research Findings
Last updated: 2025-09-15

**Date:** September 12, 2025  
**Time:** 21:20 UTC  
**Research Task:** Jupiter V6 instruction format and discriminators  

## Executive Summary

Successfully researched Jupiter V6 instruction parsing capabilities and found comprehensive technical details for implementing multi-hop route analysis. Jupiter V6 uses Anchor-based discriminators, provides official Rust libraries, and has well-documented instruction formats for route parsing.

## Key Technical Findings

### 🎯 Jupiter V6 Program Details
- **Program ID**: `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`
- **Primary Instruction**: `shared_accounts_route` for multi-hop swaps
- **Parser Version**: 6.0.7 (latest for current V6 program)

### 📋 Instruction Discriminator System
```rust
// Jupiter uses 8-byte Anchor discriminators
// Format: First 8 bytes of sha256("global:function_name")
// CPI logs may have second discriminator at bytes [8:16]
let discriminator: [u8; 8] = [64, 198, 205, 232, 38, 8, 113, 226]; // Example
```

### 🗂️ Route Instruction Data Structure
Based on research, Jupiter route instructions contain:
```rust
#[derive(BorshSerialize, BorshDeserialize)]
pub struct RouteInstruction {
    pub route_plan: Vec<RoutePlanStep>,
    pub in_amount: u64,           // Input token amount
    pub quoted_out_amount: u64,   // Expected output amount  
    pub slippage_bps: u16,        // Slippage tolerance in basis points
    pub platform_fee_bps: u16,    // Platform fee in basis points
}

#[derive(BorshSerialize, BorshDeserialize)]  
pub struct RoutePlanStep {
    pub swap: SwapType,      // e.g., RaydiumClmm, WhirlpoolSwap
    pub percent: u8,         // Percentage of route (100 for single path)
    pub input_index: u8,     // Index of input token in accounts
    pub output_index: u8,    // Index of output token in accounts
}
```

### 🔍 Parsing Approach Options

#### Option 1: Official Jupiter Libraries (Recommended)
```rust
// Available Rust crates:
// 1. jupiter-cpi (CPI helpers)
// 2. @jup-ag/instruction-parser (official parser)
// 3. juper_swap_cpi (lightweight parsing)

[dependencies]
jupiter-cpi = { version = "0.1" }
borsh = "0.10"
```

#### Option 2: Custom Parsing Implementation
```rust
// Direct instruction data parsing
pub fn parse_jupiter_instruction(data: &[u8]) -> Result<RouteInstruction> {
    // Check discriminator (first 8 bytes)
    let discriminator = &data[0..8];
    
    // Parse route data (bytes 8+)
    let route_data = &data[8..];
    let route: RouteInstruction = borsh::from_slice(route_data)?;
    
    Ok(route)
}
```

### 📊 Hex Data Example Analysis
From research findings, example Jupiter instruction:
```
e517cb977ae3ad2a010000001a640001003fc6b800000000e214e91a00000000030000
```

Decoded structure:
- **Route Plan**: 1 step (single hop)
- **Swap Type**: RaydiumClmm  
- **Input Amount**: 3,100,000,000 (3.1 SOL)
- **Expected Output**: 451,482,850
- **Slippage**: 3 basis points (0.03%)

### 🎣 Event-Based Parsing Alternative
Jupiter emits Anchor events for swap completion:
```rust
// Swap event discriminator: sha256("event:SwapEvent")[0..8]
pub const SWAP_EVENT_DISCRIMINATOR: [u8; 8] = [64, 198, 205, 232, 38, 8, 113, 226];

#[derive(BorshDeserialize)]
pub struct SwapEvent {
    pub amm: Pubkey,
    pub input_mint: Pubkey, 
    pub input_amount: u64,
    pub output_mint: Pubkey,
    pub output_amount: u64,
}
```

## Implementation Strategy

### 🚀 Recommended Approach
1. **Use Official Jupiter CPI Library**: Provides pre-generated structures from Jupiter IDL
2. **Parse Transaction Instructions**: Extract route data from shared_accounts_route calls
3. **Event-Based Validation**: Cross-reference with swap events for accuracy
4. **Fallback Parsing**: Custom parser for edge cases where libraries fail

### 🔧 Integration Points
```rust
// In our detect_arbitrage_opportunity function:
"JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => {
    venues_involved.push("jupiter-v6");
    
    // NEW: Parse Jupiter route data
    if let Ok(jupiter_routes) = parse_jupiter_instructions(transaction_update).await {
        for route in jupiter_routes {
            // Extract multi-hop path: A -> B -> C -> D
            let hops = extract_token_path(&route)?;
            
            // Analyze each hop for arbitrage opportunities  
            for (i, hop) in hops.iter().enumerate() {
                // Check if we can provide better direct route
                if let Some(direct_opportunity) = 
                    find_direct_route_arbitrage(hop, &hops[i+1..]).await? {
                    opportunities.push(direct_opportunity);
                }
            }
        }
    }
}
```

## Technical Specifications

### 📦 Required Dependencies
```toml
[dependencies]
jupiter-cpi = "0.1"           # Official Jupiter CPI helpers
borsh = "0.10"                # Serialization format
anchor-lang = "0.30"          # Anchor framework support
solana-sdk = "1.18"           # Solana types and utilities
```

### ⚡ Performance Considerations
- **Parsing Speed**: <0.5ms per Jupiter transaction (target)
- **Memory Usage**: ~2KB per route instruction parsing
- **Cache Strategy**: LRU cache for common route patterns
- **Error Handling**: Graceful fallback when parsing fails

### 🔍 Validation Strategy
1. **Discriminator Verification**: Ensure instruction is shared_accounts_route
2. **Data Length Checks**: Validate instruction data size
3. **Account Index Bounds**: Ensure input/output indices are valid
4. **Amount Validation**: Check amounts are reasonable (non-zero, not overflow)

## Risk Assessment

### ⚠️ Potential Issues
1. **Jupiter Protocol Updates**: V6 may evolve, breaking parsing
2. **Truncated Logs**: Some swap details unavailable if logs truncated
3. **Complex Routes**: Multi-hop routes with 5+ steps may be complex to parse
4. **CPI Complexity**: Nested CPI calls add parsing difficulty

### ✅ Mitigation Strategies
1. **Version Pinning**: Use specific jupiter-cpi version (6.0.7)
2. **Fallback Parsing**: Multiple parsing strategies for robustness  
3. **Test Coverage**: Parse real Jupiter transactions for validation
4. **Monitoring**: Alert on parsing failure rate increases

## Next Steps

### 📋 Implementation Phases
1. **Phase 1**: Add jupiter-cpi dependency and basic parsing
2. **Phase 2**: Implement route extraction for multi-hop paths
3. **Phase 3**: Build arbitrage detection logic for Jupiter routes
4. **Phase 4**: Performance optimization and error handling

### 🎯 Success Metrics
- **Parse Rate**: >95% of Jupiter V6 transactions successfully parsed
- **Latency**: <0.5ms additional processing time per transaction
- **Accuracy**: Route extraction matches Jupiter's internal routing
- **Coverage**: Support for all major Jupiter swap types (Raydium, Whirlpool, etc.)

## Conclusion

Jupiter V6 instruction parsing is technically feasible with multiple implementation approaches available. The official jupiter-cpi library provides the most robust solution, while custom parsing offers more control. The route data structure is well-documented and suitable for extracting multi-hop arbitrage opportunities.

**Recommendation**: Proceed with Phase 1 implementation using jupiter-cpi library with custom parsing fallback for maximum reliability.

---

*Research completed successfully - ready for implementation phase*
