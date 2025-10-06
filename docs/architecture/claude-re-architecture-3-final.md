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

#[derive(Clone, Debug)]
pub struct SwapQuote {
    pub expected_output: u64,
    pub price_impact_bps: f64,
    pub fee_amount: u64,
    pub minimum_output: u64,
    pub route: Vec<Pubkey>,
}

#[derive(Clone, Debug)]
pub struct SwapEvent {
    pub signature: Signature,
    pub pool_id: Pubkey,
    pub dex: String,
    pub user: Pubkey,
    pub input_token: Pubkey,
    pub output_token: Pubkey,
    pub input_amount: u64,
    pub output_amount: u64,
    pub fee_amount: u64,
    pub timestamp_ns: u64,
    pub slot: u64,
}
```

### Implementation Examples

**Raydium AMM (Reference: Solana-Arbitrage-Bot1 + solana-defi)**
- Account parsing: `solana-defi/Raydium/Typescript/stream_and_parse_raydium_amm_transactions/`
- Swap math: Constant product AMM formula
- Instruction building: 8-byte discriminator pattern

**Orca Whirlpool (Reference: Solana-Arbitrage-Bot1 + solana-defi)**
- Concentrated liquidity with tick arrays
- sqrt price calculations
- Dynamic fee tiers (0.01%, 0.05%, 0.3%, 1%)

**PumpFun Bonding Curve (Reference: solana-defi)**
- Virtual reserves for pricing
- Migration detection to Raydium
- Bonding curve completion events

---

## 2. POOL STATE MANAGEMENT

**Reference:** `solana-defi/Raydium/Typescript/streaming_newly_added_pool_accounts/`

```rust
// crates/data-layer/src/state/pool_monitor.rs

pub struct PoolMonitor {
    geyser_client: Arc<GeyserClient>,
    dex_protocols: HashMap<String, Box<dyn DexProtocol>>,
    pool_cache: Arc<PoolCache>,
    update_tx: broadcast::Sender<PoolUpdate>,
}

impl PoolMonitor {
    pub async fn subscribe_all_pools(&self) -> Result<()> {
        for (name, protocol) in &self.dex_protocols {
            let program_id = protocol.program_id();
            
            let request = SubscribeRequest {
                accounts: HashMap::from([(
                    program_id.to_string(),
                    AccountsFilterProto {
                        account: vec![],
                        owner: vec![program_id.to_string()],
                        filters: vec![/* pool discriminator filters */],
                    }
                )]),
                commitment: Some(CommitmentLevel::Processed as i32),
                ..Default::default()
            };
            
            let mut stream = self.geyser_client.subscribe(request).await?;
            
            tokio::spawn(async move {
                while let Some(update) = stream.next().await {
                    if let Ok(pool_state) = protocol.parse_pool_account(&update.data) {
                        cache.update(update.pubkey, pool_state.clone()).await;
                        update_tx.send(PoolUpdate { /* ... */ }).ok();
                    }
                }
            });
        }
        Ok(())
    }
}

pub struct PoolCache {
    pools: Arc<DashMap<Pubkey, CachedPool>>,
    ttl_ms: u64,
}

impl PoolCache {
    pub async fn get(&self, pool_id: &Pubkey) -> Option<PoolState> {
        self.pools.get(pool_id).and_then(|cached| {
            if cached.cached_at.elapsed().as_millis() as u64 > self.ttl_ms {
                None
            } else {
                Some(cached.state.clone())
            }
        })
    }
}
```

---

## 3. EVENT DETECTION PIPELINE

**Reference:** `solana-defi/Raydium/Typescript/stream_raydium_amm_transactions_and_detect_buy_sell_events/`

```rust
// crates/data-layer/src/normalization/event_detectors/

pub struct SwapDetector {
    dex_protocols: HashMap<Pubkey, Box<dyn DexProtocol>>,
}

impl SwapDetector {
    pub fn detect(&self, tx: &EncodedConfirmedTransactionWithStatusMeta) -> Vec<SwapEvent> {
        let mut events = Vec::new();
        for instruction in &tx.transaction.message.instructions {
            if let Some(protocol) = self.dex_protocols.get(&instruction.program_id) {
                if let Ok(Some(event)) = protocol.parse_swap_transaction(tx) {
                    events.push(event);
                }
            }
        }
        events
    }
}

pub struct MigrationDetector;

impl MigrationDetector {
    pub fn detect(&self, tx: &EncodedConfirmedTransactionWithStatusMeta) -> Option<MigrationEvent> {
        let has_pumpfun = tx.transaction.message.instructions.iter()
            .any(|ix| ix.program_id == PUMPFUN_PROGRAM_ID);
        let has_raydium = tx.transaction.message.instructions.iter()
            .any(|ix| ix.program_id == RAYDIUM_PROGRAM_ID);
        
        if has_pumpfun && has_raydium {
            Some(MigrationEvent { /* ... */ })
        } else {
            None
        }
    }
}
```

---

## 4. MULTI-DEX ARBITRAGE STRATEGY

**Reference:** `Solana-Arbitrage-Bot1/arbitrage/client/`

```rust
// crates/strategy-layer/src/alpha/multi_dex_arbitrage.rs

pub struct MultiDexArbitrageStrategy {
    dex_protocols: HashMap<String, Box<dyn DexProtocol>>,
    pool_cache: Arc<PoolCache>,
    ai_orchestrator: Arc<AIOrchestrator>,
}

impl MultiDexArbitrageStrategy {
    async fn find_two_hop_arb(&self, pool: &PoolState) -> Vec<ArbitrageOpportunity> {
        let mut opps = Vec::new();
        
        for (other_pool_id, other_pool) in self.pool_cache.pools.iter() {
            if *other_pool_id == pool.pool_id || other_pool.state.dex == pool.dex {
                continue;
            }
            
            let is_same_pair = (other_pool.state.token_a == pool.token_a && other_pool.state.token_b == pool.token_b)
                || (other_pool.state.token_a == pool.token_b && other_pool.state.token_b == pool.token_a);
            
            if !is_same_pair { continue; }
            
            let protocol1 = &self.dex_protocols[&pool.dex];
            let protocol2 = &self.dex_protocols[&other_pool.state.dex];
            
            for input_amount in [100_000_000, 500_000_000, 1_000_000_000] {
                let quote1 = protocol1.calculate_swap_output(pool, pool.token_a, input_amount)?;
                let quote2 = protocol2.calculate_swap_output(&other_pool.state, pool.token_b, quote1.expected_output)?;
                
                if quote2.expected_output > input_amount {
                    opps.push(ArbitrageOpportunity {
                        pools: vec![pool.pool_id, *other_pool_id],
                        dex_names: vec![pool.dex.clone(), other_pool.state.dex.clone()],
                        input_amount,
                        expected_output: quote2.expected_output,
                    });
                }
            }
        }
        opps
    }
}
```

---

## 5. EXECUTION LAYER WITH DEX ROUTING

```rust
// crates/execution-layer/src/routing/dex_router.rs

pub struct DexRouter {
    dex_protocols: HashMap<String, Box<dyn DexProtocol>>,
    jito_client: Arc<JitoClient>,
}

impl DexRouter {
    pub async fn build_arbitrage_transaction(
        &self,
        signal: &Signal,
        wallet: &Keypair,
    ) -> Result<Transaction> {
        let (path, dex_route, input_token, input_amount, min_output) = match &signal.opportunity {
            Opportunity::Arbitrage { path, dex_route, input_token, input_amount, min_output, .. } => {
                (path, dex_route, input_token, input_amount, min_output)
            }
            _ => return Err(Error::InvalidOpportunity),
        };
        
        let mut instructions = Vec::new();
        let mut current_token = *input_token;
        let mut current_amount = *input_amount;
        
        for (i, (pool_id, dex_name)) in path.iter().zip(dex_route.iter()).enumerate() {
            let protocol = &self.dex_protocols[dex_name];
            let pool = self.pool_cache.get(pool_id).await?;
            
            let output_token = if current_token == pool.token_a { pool.token_b } else { pool.token_a };
            let user_input_ata = get_associated_token_address(&wallet.pubkey(), &current_token);
            let user_output_ata = get_associated_token_address(&wallet.pubkey(), &output_token);
            
            let swap_ix = protocol.build_swap_instruction(
                &pool,
                &wallet.pubkey(),
                &user_input_ata,
                &user_output_ata,
                current_amount,
                if i == path.len() - 1 { *min_output } else { 0 },
            )?;
            
            instructions.push(swap_ix);
            current_token = output_token;
        }
        
        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&instructions, Some(&wallet.pubkey()));
        tx.sign(&[wallet], recent_blockhash);
        
        Ok(tx)
    }
    
    pub async fn submit_via_jito(&self, tx: Transaction) -> Result<Signature> {
        let bundle_id = self.jito_client.send_bundle(vec![tx]).await?;
        tokio::time::timeout(
            Duration::from_secs(30),
            self.jito_client.wait_for_bundle_confirmation(bundle_id)
        ).await??
    }
}
```

---


---

## 6. COMPLETE FILE STRUCTURE

```
MEVana/
├── Cargo.toml (workspace)
├── crates/
│   ├── dex-abstraction/              ← NEW - Critical Addition
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── traits.rs             (DexProtocol, PoolState, SwapQuote)
│   │   │   ├── raydium/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── amm.rs            (Constant product x*y=k)
│   │   │   │   ├── clmm.rs           (Concentrated liquidity)
│   │   │   │   └── cpmm.rs           (Stable swap)
│   │   │   ├── orca/
│   │   │   │   ├── mod.rs
│   │   │   │   └── whirlpool.rs      (Concentrated liquidity, tick arrays)
│   │   │   ├── meteora/
│   │   │   │   ├── mod.rs
│   │   │   │   └── dlmm.rs           (Dynamic liquidity market maker)
│   │   │   ├── pumpfun/
│   │   │   │   ├── mod.rs
│   │   │   │   └── bonding.rs        (Bonding curve, migration detection)
│   │   │   └── jupiter/
│   │   │       ├── mod.rs
│   │   │       └── aggregator.rs     (Jupiter v6 routing)
│   │   └── tests/
│   │       └── integration.rs
│   │
│   ├── data-layer/                    ← ENHANCED
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── geyser_client.rs
│   │   │   ├── pool_monitor.rs       ← NEW (real-time Geyser subscriptions)
│   │   │   ├── pool_cache.rs         ← NEW (lockfree concurrent cache)
│   │   │   ├── event_detector.rs     ← NEW (high-level event parsing)
│   │   │   ├── swap_detector.rs      ← NEW (uses DexProtocol)
│   │   │   ├── liquidity_detector.rs ← NEW
│   │   │   ├── migration_detector.rs ← NEW (PumpFun → Raydium)
│   │   │   ├── market_state.rs
│   │   │   ├── validation.rs
│   │   │   └── replay_store.rs       (immutable event log)
│   │   └── tests/
│   │
│   ├── strategy-layer/                ← ENHANCED
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── traits.rs             (Strategy, Signal)
│   │   │   ├── multi_dex_arb.rs      ← NEW (uses DexProtocol abstraction)
│   │   │   ├── pumpfun_sniper.rs     ← NEW (migration + early liquidity)
│   │   │   ├── liquidation.rs
│   │   │   ├── trend_following.rs
│   │   │   └── portfolio_optimizer.rs
│   │   └── tests/
│   │
│   ├── execution-layer/               ← ENHANCED
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── dex_router.rs         ← NEW (uses DexProtocol for tx building)
│   │   │   ├── jito_client.rs        (bundle submission)
│   │   │   ├── transaction_builder.rs
│   │   │   ├── fee_estimator.rs
│   │   │   ├── outcome_tracker.rs
│   │   │   └── retry_logic.rs
│   │   └── tests/
│   │
│   ├── ai-orchestration/              ← NEW (from Codex insights)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── orchestrator.rs
│   │   │   ├── feature_store.rs      (liquidity depth, volatility, spread)
│   │   │   ├── model_registry.rs     (slippage estimation, position sizing)
│   │   │   ├── drift_detector.rs     (distribution shift monitoring)
│   │   │   ├── policy_guard.rs       (validate AI outputs)
│   │   │   └── feedback_loop.rs      (execution outcome → model update)
│   │   └── models/
│   │       ├── slippage_predictor.onnx
│   │       └── position_sizer.onnx
│   │
│   ├── portfolio-service/             ← NEW (authoritative balance tracking)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── portfolio.rs          (event-sourced state)
│   │   │   ├── reconciliation.rs     (on-chain vs internal)
│   │   │   ├── exposure_tracker.rs
│   │   │   └── wallet_inventory.rs   (hot/warm/cold tiers)
│   │   └── tests/
│   │
│   ├── risk-manager/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── risk_engine.rs        (hard limits first, AI advisory)
│   │   │   ├── circuit_breaker.rs
│   │   │   ├── position_limits.rs
│   │   │   └── drawdown_monitor.rs
│   │   └── tests/
│   │
│   ├── control-plane/                 ← NEW (from Codex insights)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── supervisor.rs         (health checks, auto-restart)
│   │   │   ├── graphql_api.rs        (query layer)
│   │   │   └── audit_logger.rs       (immutable decision log)
│   │   └── tests/
│   │
│   ├── event-bus/                     ← NEW (async, non-blocking)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── kafka_producer.rs
│   │   │   ├── nats_producer.rs
│   │   │   └── topics.rs             (market.events, execution.fills, ai.decisions)
│   │   └── tests/
│   │
│   ├── ops-console/                   ← NEW
│   │   ├── Cargo.toml
│   │   ├── frontend/                 (React/Next.js)
│   │   │   ├── pages/
│   │   │   │   ├── dashboard.tsx
│   │   │   │   ├── strategies.tsx
│   │   │   │   └── risk.tsx
│   │   │   └── components/
│   │   └── backend/
│   │       └── src/
│   │           └── graphql_server.rs
│   │
│   └── hotpath_arb_engine/            ← EXISTING (to be refactored)
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs               (orchestration, now uses dex-abstraction)
│       │   ├── property_store.rs
│       │   └── websocket_broadcaster.rs
│       └── tests/
│
├── infrastructure/
│   ├── docker-compose.yml
│   ├── kafka/
│   ├── postgres/
│   └── grafana/
│
├── docs/
│   └── architecture/
│       ├── claude-re-architecture-1.md
│       ├── claude-re-architecture-2.md
│       ├── claude-re-architecture-3.md (corrupted - deprecated)
│       ├── claude-re-architecture-3-final.md (this document)
│       ├── codex-re-architecture-1.md
│       ├── codex-re-architecture-2.md
│       ├── codex-re-architecture-3-ai-integration.md
│       └── external-repos-assessment.md
│
└── tests/
    ├── integration/
    │   ├── replay_harness.rs         ← NEW (deterministic testing)
    │   └── multi_dex_arb_test.rs     ← NEW
    └── benchmarks/
        └── latency_test.rs
```

**NEW Components Summary:**
- `dex-abstraction/` - Universal DEX interface (5 implementations)
- Enhanced `data-layer/` with pool monitor and event detection
- Enhanced `strategy-layer/` with multi-DEX arbitrage
- Enhanced `execution-layer/` with DEX router
- `ai-orchestration/` - Feature store, model registry, policy guard
- `portfolio-service/` - Event-sourced authoritative balance tracking
- `control-plane/` - Supervisor, GraphQL API, audit logger
- `event-bus/` - Async event streaming (Kafka/NATS)
- `ops-console/` - React UI for monitoring and control

---

## 7. EXTERNAL CODE REFERENCE MAP

This table maps our architecture components to specific external code we can reference during implementation.

| Our Component | External Repository | File Path | What to Reference |
|---------------|---------------------|-----------|-------------------|
| **DexProtocol trait** | Solana-Arbitrage-Bot1 | `arbitrage/programs/tmp/src/swaps/mod.rs` | Trait structure, method signatures |
| **RaydiumAMM implementation** | Solana-Arbitrage-Bot1 | `arbitrage/programs/tmp/src/swaps/raydium.rs` | CPI patterns, constant product math |
| **RaydiumAMM implementation** | solana-defi | `Raydium/Typescript/stream_and_parse_raydium_amm_transactions/` | Transaction parsing, account layout |
| **OrcaWhirlpool implementation** | Solana-Arbitrage-Bot1 | `arbitrage/programs/tmp/src/swaps/orca.rs` | Tick array handling, sqrt_price conversions |
| **OrcaWhirlpool implementation** | solana-defi | `Orca/Typescript/stream_and_parse_whirlpool_transactions/` | Event parsing, position tracking |
| **MeteoraLM implementation** | Solana-Arbitrage-Bot1 | `arbitrage/programs/tmp/src/swaps/meteora.rs` | DLMM bin structure, dynamic fees |
| **PumpFun implementation** | solana-defi | `PumpFun/Typescript/stream_pump_fun_bonding_curve_progress_accounts/` | Bonding curve math, virtual reserves |
| **Migration detection** | solana-defi | `PumpFun/Typescript/stream_pumpfun_to_raydium_migration_transactions/` | Migration event parsing, timing |
| **Jupiter integration** | Solana-Arbitrage-Bot1 | `arbitrage/programs/tmp/src/swaps/jupiter.rs` | Jupiter v6 CPI, account meta derivation |
| **SwapDetector** | solana-defi | `Raydium/Typescript/stream_and_parse_raydium_amm_transactions/helpers/parse_raydium_buy_sell_from_logs.ts` | Log parsing patterns |
| **PoolMonitor (Geyser)** | solana-defi | `Orca/Typescript/stream_and_parse_whirlpool_transactions/` | gRPC subscription patterns |
| **Multi-DEX arbitrage** | Solana-Arbitrage-Bot1 | `arbitrage/programs/arbitrage/src/lib.rs` | Two-hop arbitrage logic (avoid on-chain!) |
| **Transaction builder** | Solana-Arbitrage-Bot1 | `arbitrage/client/src/lib.rs` | Off-chain transaction construction |
| **Jito submission** | Solana-Arbitrage-Bot1 | README warnings | Use Jito bundles, not on-chain programs |
| **PumpFun sniper strategy** | solana-defi | `PumpFun/Typescript/stream_pumpfun_to_raydium_migration_transactions/` | Migration timing, liquidity analysis |
| **AI model patterns** | codex | `codex-re-architecture-3-ai-integration.md` | Feature store, drift detection design |
| **Event sourcing** | codex | `codex-re-architecture-1.md` | Immutable event log, replay harness |
| **Risk gateway** | codex | `codex-re-architecture-2.md` | Circuit breaker patterns |

**Key Insight from External Repos:**
- **DO NOT** implement on-chain arbitrage programs (Solana-Arbitrage-Bot1 author explicitly warns: "highly competitive MEV environment")
- **DO** use off-chain detection + Jito bundle submission
- **DO** subscribe to pool accounts via Geyser for real-time state
- **DO** use specialized parsers per DEX (generic approach is naive)

---

## 8. IMPLEMENTATION ROADMAP (8 Weeks)

### **Phase 1: DEX Abstraction Foundation (Weeks 1-2)**

**Week 1: Core Trait + Raydium**
- [ ] Define `DexProtocol` trait and `PoolState` struct
- [ ] Implement `RaydiumAMM` (constant product only)
- [ ] Reference: `Solana-Arbitrage-Bot1/arbitrage/programs/tmp/src/swaps/raydium.rs`
- [ ] Write unit tests for swap calculations
- [ ] Deliverable: Working Raydium swap quote calculation

**Week 2: Orca + PumpFun**
- [ ] Implement `OrcaWhirlpool` (concentrated liquidity)
- [ ] Implement `PumpFunBonding` (bonding curve + migration detection)
- [ ] Reference: solana-defi Orca and PumpFun parsers
- [ ] Integration tests: Parse real transactions from mainnet
- [ ] Deliverable: 3 DEX implementations passing tests

### **Phase 2: Pool State Management (Weeks 3-4)**

**Week 3: Pool Monitor**
- [ ] Implement `PoolMonitor` with Geyser account subscriptions
- [ ] Subscribe to Raydium, Orca, PumpFun program accounts
- [ ] Parse account data using `DexProtocol::parse_pool_account()`
- [ ] Broadcast updates via `tokio::sync::broadcast`
- [ ] Deliverable: Real-time pool state streaming

**Week 4: Pool Cache + Validation**
- [ ] Implement lockfree `PoolCache` (DashMap)
- [ ] Add staleness detection (warn if >2 slots old)
- [ ] Add data validation layer (sanity checks on reserves)
- [ ] Performance test: <1ms lookup latency
- [ ] Deliverable: Production-ready pool state infrastructure

### **Phase 3: Event Detection + Strategy Layer (Weeks 5-6)**

**Week 5: Event Detection**
- [ ] Implement `SwapDetector` using `DexProtocol::parse_swap_transaction()`
- [ ] Implement `MigrationDetector` for PumpFun → Raydium
- [ ] Define high-level events: `SwapEvent`, `LiquidityEvent`, `MigrationEvent`
- [ ] Reference: solana-defi transaction parsing helpers
- [ ] Deliverable: Clean event stream from raw transactions

**Week 6: Multi-DEX Arbitrage Strategy**
- [ ] Implement `MultiDexArbitrageStrategy`
- [ ] Two-hop arbitrage: SOL → USDC (DEX A) → SOL (DEX B)
- [ ] Triangle arbitrage: SOL → USDC → RAY → SOL
- [ ] Use `DexProtocol::calculate_swap_output()` for profitability
- [ ] Deliverable: Working arbitrage detection (paper trading)

### **Phase 4: Execution + AI Orchestration (Weeks 7-8)**

**Week 7: Execution Layer**
- [ ] Implement `DexRouter` using `DexProtocol::build_swap_instruction()`
- [ ] Integrate Jito client for bundle submission
- [ ] Add priority fee estimation (dynamic based on congestion)
- [ ] Implement outcome tracker (compare expected vs actual profit)
- [ ] Deliverable: End-to-end arbitrage execution (testnet)

**Week 8: AI Orchestration (MVP)**
- [ ] Implement `FeatureStore` (pool liquidity, volatility, spread)
- [ ] Train simple slippage estimation model (linear regression → ONNX)
- [ ] Implement `PolicyGuard` (validate AI outputs against hard limits)
- [ ] Wire AI advisory into risk manager
- [ ] Deliverable: AI-enhanced position sizing in production

### **Post-Launch: Continuous Improvement**
- Add Meteora DLMM and Jupiter v6 aggregator
- Implement PumpFun migration sniper strategy
- Add portfolio service with event sourcing
- Build ops console (React UI)
- Implement deterministic replay harness

---

## 9. KEY DESIGN WINS

### **What Changed from v1 → v3:**

1. **DEX Abstraction Layer (NEW)**
   - **Problem**: v1 assumed generic transaction builder could handle all DEXs
   - **Discovery**: Each DEX has unique instruction formats, account structures, AMM math
   - **Solution**: `DexProtocol` trait with specialized implementations
   - **Impact**: Strategies can now work across any DEX without knowing implementation details

2. **Pool State Management (NEW)**
   - **Problem**: v1 relied on transaction-only streaming (stale data)
   - **Discovery**: Real-time pool account monitoring is essential for arbitrage
   - **Solution**: `PoolMonitor` with Geyser account subscriptions
   - **Impact**: <1ms pool state lookups, always current data

3. **Event Detection Pipeline (NEW)**
   - **Problem**: v1 had strategies parsing raw transactions (too complex)
   - **Discovery**: Need high-level event interpretation layer
   - **Solution**: `SwapDetector`, `LiquidityDetector`, `MigrationDetector`
   - **Impact**: Strategies consume clean `SwapEvent` objects instead of raw bytes

4. **AI Orchestration Plane (from Codex)**
   - **Problem**: v1 had no adaptive intelligence
   - **Discovery**: Codex showed brilliant feature store + drift detection design
   - **Solution**: Feature store, model registry, policy guard, feedback loops
   - **Impact**: System learns from execution outcomes, adapts to market shifts

5. **Portfolio Service (from Codex)**
   - **Problem**: v1 had no authoritative balance tracking
   - **Discovery**: Need event-sourced portfolio with reconciliation
   - **Solution**: Portfolio service subscribing to fills from event bus
   - **Impact**: Always know exact positions, can replay to any point in time

6. **Off-Chain Detection + Jito (from External Repos)**
   - **Problem**: Initial thought was on-chain arbitrage programs
   - **Discovery**: Solana-Arbitrage-Bot1 author warns against it (MEV competition, compute limits)
   - **Solution**: Off-chain detection, Jito bundle submission
   - **Impact**: MEV protection, no compute budget constraints

### **What Stayed the Same (Core Principles):**

1. **Ultra-Low Latency** - Still targeting <100ms end-to-end
2. **Risk-First Design** - Hard limits before AI advisory
3. **Modularity** - Clean trait boundaries, crate isolation
4. **Observability** - Prometheus metrics, structured logging
5. **Deterministic Replay** - Immutable event log for debugging

---

## 10. CONCLUSION

This architecture represents a **production-ready synthesis** of three distinct perspectives:

1. **Claude v1**: Ultra-low latency hot path, modular design
2. **Codex (GPT-5)**: Deterministic replay, AI orchestration, institutional maturity
3. **External Repos**: DEX abstraction, pool state management, MEV protection

### **Critical Innovations:**

- **DexProtocol trait** - Universal interface for Raydium, Orca, Meteora, PumpFun, Jupiter
- **Pool state monitor** - Real-time Geyser account subscriptions for <1ms lookups
- **Event detection pipeline** - High-level events (swaps, liquidity changes, migrations)
- **AI orchestration** - Feature store, model registry, drift detection, policy guard
- **Off-chain + Jito** - Avoid on-chain arbitrage programs (MEV vulnerability)

### **Implementation Priority:**

1. **Phase 1-2 (Weeks 1-4)**: DEX abstraction + pool management - **foundation for everything**
2. **Phase 3 (Weeks 5-6)**: Event detection + arbitrage strategy - **value generation**
3. **Phase 4 (Weeks 7-8)**: Execution + AI orchestration - **production readiness**

### **Success Metrics:**

- **Latency**: <100ms from Geyser update to Jito submission
- **Data Quality**: Pool states <2 slots stale
- **Profitability**: 5-25 bps minimum after fees
- **Execution**: >95% fill rate on signals
- **AI Impact**: 10%+ improvement in position sizing vs static rules

### **External Code Leverage:**

- Reference `Solana-Arbitrage-Bot1` for CPI patterns, instruction building
- Reference `solana-defi` parsers for transaction/account parsing
- **DO NOT** copy on-chain arbitrage logic (use off-chain detection instead)

This architecture is **ready for implementation** with clear 8-week roadmap and specific external code references for each component.

---

**Document Status**: Complete ✅  
**Created**: 2025-10-02  
**Version**: 3 (Final)  
**Next Step**: Begin Phase 1 implementation (DEX abstraction layer)
