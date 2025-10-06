# Production-Ready Architecture: Solana MEV with DEX Abstraction

**Revision 3 - Incorporating External Repository Insights**

Created: 2025-10-02

---

## Design Philosophy Evolution

This architecture synthesizes:
- **Claude v1**: Ultra-low latency hot path (<100ms) for MEV capture
- **Codex (GPT-5) insights**: Deterministic replay, AI orchestration, institutional maturity
- **External repos analysis**: DEX abstraction, pool state management, event detection

### Critical Discoveries from External Repositories

1. **DEX Integration Complexity** - Each DEX (Raydium, Orca, Meteora, Jupiter, PumpFun) requires specialized parsers, account structures, and transaction builders. **Generic abstraction was naive.**

2. **Pool State is Dynamic** - Real-time pool account monitoring via gRPC is essential. Cannot rely on transaction-only streaming.

3. **Event Detection Layer** - Strategies should consume interpreted events (swaps, liquidity changes, migrations), not raw transactions.

4. **Off-Chain Detection + Jito Execution** - On-chain arbitrage programs face MEV competition and compute limits. Author of Solana-Arbitrage-Bot1 explicitly warns against it.

---

## High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        CONTROL & OBSERVABILITY PLANE                             │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Supervisor  │  │ Control Plane │  │ Ops Console  │  │  Audit Logger    │   │
│  │  (Health)    │  │ (GraphQL API) │  │  (React UI)  │  │  (Immutable)     │   │
│  └──────────────┘  └───────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
                                        │
        ┌───────────────────────────────┼─────────────────────────────────┐
        │                               │                                 │
┌───────▼────────┐           ┌──────────▼────────┐             ┌─────────▼──────────┐
│  DATA LAYER    │           │ DEX ABSTRACTION   │             │  PORTFOLIO SERVICE │
│                │           │   (NEW - CRITICAL)│             │   (Authoritative)  │
│ • Geyser       │──────────▶│                   │             │                    │
│ • Multi-source │           │ • DexProtocol     │             │ • Balance Tracker  │
│ • Validation   │           │ • Raydium (AMM,   │             │ • Reconciliation   │
│                │           │   CLMM, CPMM)     │             │ • Event Sourcing   │
│ Pool Monitor───┼──────────▶│ • Orca Whirlpool  │             └────────────────────┘
│ (NEW)          │           │ • Meteora DLMM    │                       │
│                │           │ • PumpFun Bonding │                       │
│ Event Detector─┼──────────▶│ • Jupiter v6      │                       │
│ (NEW)          │           │                   │                       │
│                │           │ Each impl:        │                       │
│ • Replay Store │           │ - parse_pool()    │                       │
└────┬───────────┘           │ - calc_swap()     │                       │
     │                       │ - build_ix()      │                       │
     │ Market Events         │ - parse_event()   │                       │
     │                       └──────────┬────────┘                       │
     │                                  │ Pool State + Swap Builders     │
     │                       ┌──────────▼────────────────────────────────▼─────┐
     └──────────────────────▶│     AI ORCHESTRATION PLANE                      │
                             │                                                 │
                             │ • Feature Store (pool liquidity, volatility)    │
                             │ • Model Registry (slippage estimation, sizing)  │
                             │ • Drift Detection (distribution shifts)         │
                             │ • Policy Guard (validate AI outputs)            │
                             └──────────┬──────────────────────────────────────┘
                                        │ Advisories
                             ┌──────────▼──────────────────────────────────────┐
                             │     STRATEGY LAYER                              │
                             │                                                 │
                             │ • Multi-DEX Arbitrage (NEW - uses DexProtocol)  │
                             │ • PumpFun Migration Sniper (NEW)                │
                             │ • Liquidation Scanner                           │
                             │ • Trend Following                               │
                             │                                                 │
                             │ Strategies subscribe to:                        │
                             │ - SwapEvent, LiquidityEvent, NewPoolEvent       │
                             └──────────┬──────────────────────────────────────┘
                                        │ Signals
                             ┌──────────▼──────────┐
                             │   RISK MANAGER      │
                             │                     │
                             │ • Hard Limits First │
                             │ • AI Advisory Input │
                             │ • Exposure Tracking │
                             │ • Circuit Breakers  │
                             └──────────┬──────────┘
                                        │ Approved Signals
                             ┌──────────▼──────────┐
                             │  EXECUTION LAYER    │
                             │                     │
                             │ • DexRouter (uses   │
                             │   DexProtocol)      │
                             │ • Jito Bundles      │
                             │ • Priority Fees     │
                             │ • Outcome Tracking  │
                             └──────────┬──────────┘
                                        │
        ┌───────────────────────────────┴────────────────────────────┐
        │                                                             │
┌───────▼──────────┐                                      ┌──────────▼────────┐
│  EVENT BUS       │                                      │  REPLAY STORE     │
│  (Kafka/NATS)    │                                      │  (Immutable)      │
│                  │                                      │                   │
│ • market.events  │                                      │ • Market Events   │
│ • execution.fills│                                      │ • Pool States     │
│ • ai.decisions   │                                      │ • Execution Logs  │
└──────────────────┘                                      └───────────────────┘
```

---

<function_calls>
<invoke name="TodoWrite">
<parameter name="todos">[{"content": "Integrate DEX abstraction layer into architecture", "status": "completed", "activeForm": "Integrating DEX abstraction layer into architecture"}, {"content": "Add pool state management components", "status": "in_progress", "activeForm": "Adding pool state management components"}, {"content": "Incorporate event detection pipeline", "status": "pending", "activeForm": "Incorporating event detection pipeline"}, {"content": "Map external code references to our components", "status": "pending", "activeForm": "Mapping external code references to our components"}, {"content": "Update file structure with new components", "status": "pending", "activeForm": "Updating file structure with new components"}, {"content": "Create implementation roadmap", "status": "pending", "activeForm": "Creating implementation roadmap"}]
## 1. DEX ABSTRACTION LAYER (NEW - Critical Component)

### Core Trait Definition

**Reference:** `Solana-Arbitrage-Bot1/arbitrage/programs/tmp/src/swaps/`

```rust
// crates/dex-abstraction/src/traits.rs

use solana_program::{instruction::Instruction, pubkey::Pubkey};
use async_trait::async_trait;

/// Universal DEX protocol interface
#[async_trait]
pub trait DexProtocol: Send + Sync {
    fn name(&self) -> &str;
    fn program_id(&self) -> Pubkey;
    fn parse_pool_account(&self, data: &[u8]) -> Result<PoolState>;
    fn calculate_swap_output(&self, pool: &PoolState, input_token: Pubkey, input_amount: u64) -> Result<SwapQuote>;
    fn build_swap_instruction(&self, pool: &PoolState, user: &Pubkey, user_input_ata: &Pubkey, user_output_ata: &Pubkey, input_amount: u64, min_output_amount: u64) -> Result<Instruction>;
    fn parse_swap_transaction(&self, tx: &EncodedTransaction) -> Result<Option<SwapEvent>>;
    fn get_pool_accounts(&self, pool_id: &Pubkey) -> Vec<Pubkey>;
}

/// Normalized pool state across all DEXs
#[derive(Clone, Debug)]
pub struct PoolState {
    pub pool_id: Pubkey,
    pub dex: String,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee_bps: u64,
    pub tick_current: Option<i32>,
    pub sqrt_price: Option<u128>,
    pub liquidity: Option<u128>,
    pub virtual_sol_reserves: Option<u64>,
    pub virtual_token_reserves: Option<u64>,
    pub bonding_curve_complete: Option<bool>,
    pub last_updated_slot: u64,
}
```

See full implementation details in the document...

