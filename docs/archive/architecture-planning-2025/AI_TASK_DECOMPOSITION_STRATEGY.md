# AI Task Decomposition Strategy

**Created**: 2025-10-02
**Purpose**: Break claude-re-architecture-3-final.md into independently testable, AI-assignable tasks

---

## Decomposition Principles

### 1. **Dependency Ordering**
Tasks must be ordered so dependencies are completed first:
```
DexProtocol trait → Implementations → Pool Monitor → Event Detection → Strategies
```

### 2. **Test-Driven Assignment**
Every task includes:
- **Input specification** (what data/interfaces the AI receives)
- **Output specification** (what the AI must produce)
- **Test criteria** (how to verify success)
- **Mock dependencies** (what to stub out)

### 3. **Interface Contracts**
Define clear interfaces between tasks so AIs can work independently:
```rust
// Task A produces this interface
pub trait DexProtocol { /* ... */ }

// Task B consumes this interface (can mock Task A)
pub struct PoolMonitor {
    dex_protocols: HashMap<String, Box<dyn DexProtocol>>
}
```

### 4. **Isolated Testing**
Each task must be testable without requiring other tasks to be complete:
- Use mocks for dependencies
- Provide sample test data
- Include integration test skeleton for later

---

## Phase 1: Foundation Layer (Weeks 1-2)

### Task 1.1: DexProtocol Trait Definition
**Assigned to**: AI Agent Alpha
**Duration**: 2 days
**Dependencies**: None

**Deliverable**:
```rust
// crates/dex-abstraction/src/traits.rs
pub trait DexProtocol: Send + Sync {
    fn name(&self) -> &str;
    fn program_id(&self) -> Pubkey;
    fn parse_pool_account(&self, data: &[u8]) -> Result<PoolState>;
    fn calculate_swap_output(&self, pool: &PoolState, input_token: Pubkey, input_amount: u64) -> Result<SwapQuote>;
    fn build_swap_instruction(&self, pool: &PoolState, user: &Pubkey, user_input_ata: &Pubkey, user_output_ata: &Pubkey, input_amount: u64, min_output_amount: u64) -> Result<Instruction>;
    fn parse_swap_transaction(&self, tx: &EncodedTransaction) -> Result<Option<SwapEvent>>;
    fn get_pool_accounts(&self, pool_id: &Pubkey) -> Vec<Pubkey>;
}

pub struct PoolState { /* ... */ }
pub struct SwapQuote { /* ... */ }
pub struct SwapEvent { /* ... */ }
```

**Test Criteria**:
- [ ] Trait compiles with all method signatures
- [ ] PoolState struct has all required fields (including optional fields for different DEX types)
- [ ] SwapQuote includes expected_output, price_impact, fee
- [ ] SwapEvent includes timestamp, pool_id, amounts, user

**Mock Data Provided**:
```rust
// tests/mock_data.rs
pub fn sample_pool_state() -> PoolState { /* ... */ }
pub fn sample_raydium_account_data() -> Vec<u8> { /* ... */ }
```

**Reference**:
- `docs/architecture/claude-re-architecture-3-final.md` lines 114-160
- `Solana-Arbitrage-Bot1/arbitrage/programs/tmp/src/swaps/mod.rs`

---

### Task 1.2: RaydiumAMM Implementation
**Assigned to**: AI Agent Beta
**Duration**: 3 days
**Dependencies**: Task 1.1 complete

**Deliverable**:
```rust
// crates/dex-abstraction/src/raydium/amm.rs
pub struct RaydiumAMM {
    program_id: Pubkey,
}

impl DexProtocol for RaydiumAMM {
    // Implement all trait methods
    // Focus on constant product (x * y = k) formula
    // parse_pool_account: deserialize Raydium AMM account layout
    // calculate_swap_output: apply x*y=k math with fees
    // build_swap_instruction: construct Raydium swap instruction
}
```

**Test Criteria**:
- [ ] `parse_pool_account()` correctly deserializes real Raydium account data
- [ ] `calculate_swap_output()` matches expected output within 0.1% (accounting for fees)
- [ ] `build_swap_instruction()` produces valid instruction with correct accounts
- [ ] Unit test: 1000 USDC → SOL swap calculation
- [ ] Unit test: SOL → USDC reverse swap

**Mock Data Provided**:
```rust
// tests/raydium_test_data.rs
pub fn raydium_sol_usdc_pool_account() -> Vec<u8> { /* real mainnet data */ }
pub fn raydium_sol_usdc_pool_state() -> PoolState {
    PoolState {
        pool_id: pubkey!("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"),
        dex: "raydium_amm".to_string(),
        token_a: pubkey!("So11111111111111111111111111111111111111112"), // SOL
        token_b: pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
        reserve_a: 1_000_000_000_000, // 1000 SOL
        reserve_b: 150_000_000_000, // 150k USDC
        fee_bps: 25, // 0.25%
        // ... other fields
    }
}
```

**Reference**:
- `Solana-Arbitrage-Bot1/arbitrage/programs/tmp/src/swaps/raydium.rs`
- `solana-defi/Raydium/Typescript/stream_and_parse_raydium_amm_transactions/`
- Raydium AMM constant product formula: `output = (input * 997 * reserve_out) / (reserve_in * 1000 + input * 997)`

---

### Task 1.3: OrcaWhirlpool Implementation
**Assigned to**: AI Agent Gamma
**Duration**: 4 days
**Dependencies**: Task 1.1 complete

**Deliverable**:
```rust
// crates/dex-abstraction/src/orca/whirlpool.rs
pub struct OrcaWhirlpool {
    program_id: Pubkey,
}

impl DexProtocol for OrcaWhirlpool {
    // Implement concentrated liquidity (tick-based) math
    // parse_pool_account: deserialize Whirlpool account + tick arrays
    // calculate_swap_output: traverse ticks, apply sqrt_price math
    // build_swap_instruction: construct Whirlpool swap instruction
}
```

**Test Criteria**:
- [ ] `parse_pool_account()` correctly handles tick_current and sqrt_price
- [ ] `calculate_swap_output()` for concentrated liquidity (more complex than AMM)
- [ ] Handles tick array crossings correctly
- [ ] Unit test: Swap within single tick range
- [ ] Unit test: Swap crossing multiple ticks

**Mock Data Provided**:
```rust
// tests/orca_test_data.rs
pub fn orca_sol_usdc_whirlpool_account() -> Vec<u8> { /* ... */ }
pub fn orca_tick_array_accounts() -> Vec<Vec<u8>> { /* ... */ }
```

**Reference**:
- `Solana-Arbitrage-Bot1/arbitrage/programs/tmp/src/swaps/orca.rs`
- `solana-defi/Orca/Typescript/stream_and_parse_whirlpool_transactions/`
- Orca Whirlpool math: sqrt_price, tick arrays, liquidity calculations

**Note**: This is more complex than Raydium due to concentrated liquidity. Provide detailed math reference.

---

### Task 1.4: PumpFun Bonding Curve Implementation
**Assigned to**: AI Agent Delta
**Duration**: 3 days
**Dependencies**: Task 1.1 complete

**Deliverable**:
```rust
// crates/dex-abstraction/src/pumpfun/bonding.rs
pub struct PumpFunBonding {
    program_id: Pubkey,
}

impl DexProtocol for PumpFunBonding {
    // Implement bonding curve (virtual reserves) math
    // parse_pool_account: deserialize bonding curve state
    // calculate_swap_output: virtual reserve pricing
    // Detect migration trigger (bonding_curve_complete)
}
```

**Test Criteria**:
- [ ] `parse_pool_account()` extracts virtual_sol_reserves, virtual_token_reserves, bonding_curve_complete
- [ ] `calculate_swap_output()` uses virtual reserves (not real reserves)
- [ ] Detects when bonding curve is complete (migration imminent)
- [ ] Unit test: Early phase buy (low liquidity)
- [ ] Unit test: Near-migration buy (high liquidity)

**Mock Data Provided**:
```rust
// tests/pumpfun_test_data.rs
pub fn pumpfun_early_phase_account() -> Vec<u8> { /* 10% to Raydium */ }
pub fn pumpfun_near_migration_account() -> Vec<u8> { /* 95% to Raydium */ }
```

**Reference**:
- `solana-defi/PumpFun/Typescript/stream_pump_fun_bonding_curve_progress_accounts/`
- PumpFun bonding curve formula (virtual reserves)

---

## Phase 2: Data Layer (Weeks 3-4)

### Task 2.1: Pool Cache (Lockfree)
**Assigned to**: AI Agent Epsilon
**Duration**: 2 days
**Dependencies**: Task 1.1 complete

**Deliverable**:
```rust
// crates/data-layer/src/pool_cache.rs
pub struct PoolCache {
    pools: DashMap<Pubkey, CachedPool>,
    staleness_threshold: u64, // slots
}

pub struct CachedPool {
    state: PoolState,
    last_updated_slot: u64,
}

impl PoolCache {
    pub fn get(&self, pool_id: &Pubkey) -> Option<PoolState>;
    pub fn update(&self, pool_id: Pubkey, state: PoolState);
    pub fn is_stale(&self, pool_id: &Pubkey, current_slot: u64) -> bool;
    pub fn prune_stale(&self, current_slot: u64) -> usize; // returns count pruned
}
```

**Test Criteria**:
- [ ] Concurrent reads/writes (spawn 100 threads)
- [ ] <1ms lookup latency (benchmark)
- [ ] Staleness detection (pools >2 slots old)
- [ ] Memory safety (no race conditions)

**Mock Data Provided**:
```rust
// tests/pool_cache_test.rs
fn concurrent_read_write_test() { /* 100 threads, 10k ops */ }
```

**Reference**:
- Use `dashmap` crate for lockfree concurrent HashMap
- `docs/architecture/claude-re-architecture-3-final.md` lines 234-264

---

### Task 2.2: Pool Monitor (Geyser Subscriptions)
**Assigned to**: AI Agent Zeta
**Duration**: 4 days
**Dependencies**: Task 1.1, 1.2, 1.3, 1.4, 2.1 complete

**Deliverable**:
```rust
// crates/data-layer/src/pool_monitor.rs
pub struct PoolMonitor {
    geyser_client: Arc<GeyserClient>,
    dex_protocols: HashMap<String, Box<dyn DexProtocol>>,
    pool_cache: Arc<PoolCache>,
    update_tx: broadcast::Sender<PoolUpdate>,
}

impl PoolMonitor {
    pub async fn subscribe_all_pools(&self) -> Result<()>;
    pub async fn subscribe_single_pool(&self, pool_id: &Pubkey, dex: &str) -> Result<()>;
    fn handle_account_update(&self, account: AccountUpdate) -> Result<()>;
}
```

**Test Criteria**:
- [ ] Subscribe to Raydium, Orca, PumpFun program accounts
- [ ] Parse account updates using correct DexProtocol implementation
- [ ] Broadcast PoolUpdate events via tokio::sync::broadcast
- [ ] Update PoolCache on each account change
- [ ] Integration test: Mock Geyser stream → parse → cache update

**Mock Data Provided**:
```rust
// tests/mock_geyser.rs
pub struct MockGeyserClient { /* returns predefined account updates */ }
```

**Reference**:
- `docs/architecture/claude-re-architecture-3-final.md` lines 234-264
- Yellowstone Geyser gRPC client examples

---

### Task 2.3: Swap Event Detector
**Assigned to**: AI Agent Eta
**Duration**: 3 days
**Dependencies**: Task 1.1, 1.2, 1.3, 1.4 complete

**Deliverable**:
```rust
// crates/data-layer/src/swap_detector.rs
pub struct SwapDetector {
    dex_protocols: HashMap<String, Box<dyn DexProtocol>>,
}

impl SwapDetector {
    pub fn detect_swaps(&self, tx: &EncodedTransaction) -> Result<Vec<SwapEvent>>;
    fn identify_dex(&self, tx: &EncodedTransaction) -> Option<String>;
}

pub struct SwapEvent {
    pub slot: u64,
    pub signature: String,
    pub pool_id: Pubkey,
    pub dex: String,
    pub user: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub timestamp: i64,
}
```

**Test Criteria**:
- [ ] Detect Raydium swaps from real transaction
- [ ] Detect Orca swaps from real transaction
- [ ] Detect PumpFun buys from real transaction
- [ ] Handle transactions with multiple swaps
- [ ] Unit test: Parse Raydium buy transaction
- [ ] Unit test: Parse Orca sell transaction

**Mock Data Provided**:
```rust
// tests/swap_detector_test_data.rs
pub fn raydium_swap_transaction() -> EncodedTransaction { /* ... */ }
pub fn orca_swap_transaction() -> EncodedTransaction { /* ... */ }
```

**Reference**:
- `solana-defi/Raydium/Typescript/stream_and_parse_raydium_amm_transactions/helpers/parse_raydium_buy_sell_from_logs.ts`
- `docs/architecture/claude-re-architecture-3-final.md` lines 266-308

---

### Task 2.4: Migration Event Detector
**Assigned to**: AI Agent Theta
**Duration**: 2 days
**Dependencies**: Task 1.4 complete

**Deliverable**:
```rust
// crates/data-layer/src/migration_detector.rs
pub struct MigrationDetector {
    pumpfun_protocol: Box<dyn DexProtocol>,
}

pub struct MigrationEvent {
    pub slot: u64,
    pub pumpfun_pool: Pubkey,
    pub raydium_pool: Pubkey,
    pub token: Pubkey,
    pub initial_liquidity_sol: u64,
    pub initial_liquidity_token: u64,
    pub timestamp: i64,
}

impl MigrationDetector {
    pub fn detect_migration(&self, tx: &EncodedTransaction) -> Result<Option<MigrationEvent>>;
}
```

**Test Criteria**:
- [ ] Detect PumpFun → Raydium migration from transaction
- [ ] Extract initial Raydium pool liquidity
- [ ] Unit test: Parse real migration transaction

**Mock Data Provided**:
```rust
// tests/migration_test_data.rs
pub fn pumpfun_migration_transaction() -> EncodedTransaction { /* ... */ }
```

**Reference**:
- `solana-defi/PumpFun/Typescript/stream_pumpfun_to_raydium_migration_transactions/`

---

## Phase 3: Strategy Layer (Weeks 5-6)

### Task 3.1: Multi-DEX Arbitrage Strategy (Two-Hop)
**Assigned to**: AI Agent Iota
**Duration**: 4 days
**Dependencies**: Task 1.1, 1.2, 1.3, 2.1, 2.2 complete

**Deliverable**:
```rust
// crates/strategy-layer/src/multi_dex_arb.rs
pub struct MultiDexArbitrageStrategy {
    dex_protocols: HashMap<String, Box<dyn DexProtocol>>,
    pool_cache: Arc<PoolCache>,
    min_profit_bps: u64,
}

pub struct ArbitrageOpportunity {
    pub path: Vec<ArbHop>,
    pub expected_profit_bps: u64,
    pub input_amount: u64,
    pub output_amount: u64,
}

impl MultiDexArbitrageStrategy {
    pub async fn find_two_hop_arb(&self, event: &SwapEvent) -> Vec<ArbitrageOpportunity>;
    // Example: SOL → USDC (Raydium) → SOL (Orca)
}
```

**Test Criteria**:
- [ ] Detect profitable two-hop arbitrage (Raydium → Orca)
- [ ] Calculate profitability using DexProtocol::calculate_swap_output()
- [ ] Filter opportunities below min_profit_bps
- [ ] Unit test: SOL/USDC price difference between DEXs
- [ ] Unit test: No false positives (ignore unprofitable paths)

**Mock Data Provided**:
```rust
// tests/arb_test_data.rs
pub fn profitable_two_hop_scenario() -> (PoolState, PoolState) {
    // Raydium: 1 SOL = 150 USDC
    // Orca: 1 SOL = 155 USDC
    // Profit: 3.3% (330 bps)
}
```

**Reference**:
- `docs/architecture/claude-re-architecture-3-final.md` lines 310-359
- `Solana-Arbitrage-Bot1/arbitrage/programs/arbitrage/src/lib.rs` (logic only, not on-chain implementation)

---

### Task 3.2: PumpFun Migration Sniper Strategy
**Assigned to**: AI Agent Kappa
**Duration**: 3 days
**Dependencies**: Task 2.4 complete

**Deliverable**:
```rust
// crates/strategy-layer/src/pumpfun_sniper.rs
pub struct PumpFunSniperStrategy {
    min_initial_liquidity_sol: u64,
    max_entry_delay_ms: u64,
}

impl Strategy for PumpFunSniperStrategy {
    async fn process_event(&self, event: &MarketEvent) -> Vec<Signal> {
        if let MarketEvent::Migration(m) = event {
            // Check liquidity threshold
            // Emit buy signal if criteria met
        }
    }
}
```

**Test Criteria**:
- [ ] Detect migration events
- [ ] Filter by minimum liquidity (e.g., >10 SOL)
- [ ] Generate buy signal within timing window
- [ ] Unit test: High liquidity migration → buy signal
- [ ] Unit test: Low liquidity migration → no signal

**Reference**:
- `solana-defi/PumpFun/Typescript/stream_pumpfun_to_raydium_migration_transactions/`

---

## Phase 4: Execution Layer (Weeks 7-8)

### Task 4.1: DEX Router (Transaction Builder)
**Assigned to**: AI Agent Lambda
**Duration**: 4 days
**Dependencies**: Task 1.1, 1.2, 1.3, 1.4 complete

**Deliverable**:
```rust
// crates/execution-layer/src/dex_router.rs
pub struct DexRouter {
    dex_protocols: HashMap<String, Box<dyn DexProtocol>>,
    wallet: Keypair,
}

impl DexRouter {
    pub async fn build_swap_transaction(&self, opportunity: &ArbitrageOpportunity) -> Result<Transaction>;
    // Iterate over path, call DexProtocol::build_swap_instruction() for each hop
    // Combine into single transaction
}
```

**Test Criteria**:
- [ ] Build single-hop swap transaction (Raydium)
- [ ] Build two-hop swap transaction (Raydium → Orca)
- [ ] Transaction serialization valid
- [ ] Unit test: Verify instruction accounts match DEX requirements

**Mock Data Provided**:
```rust
// tests/dex_router_test.rs
pub fn mock_wallet() -> Keypair { /* ... */ }
```

**Reference**:
- `docs/architecture/claude-re-architecture-3-final.md` lines 361-429
- `Solana-Arbitrage-Bot1/arbitrage/client/src/lib.rs`

---

### Task 4.2: Jito Bundle Client
**Assigned to**: AI Agent Mu
**Duration**: 3 days
**Dependencies**: Task 4.1 complete

**Deliverable**:
```rust
// crates/execution-layer/src/jito_client.rs
pub struct JitoClient {
    block_engine_url: String,
    tip_account: Pubkey,
}

impl JitoClient {
    pub async fn submit_bundle(&self, txs: Vec<Transaction>, tip_lamports: u64) -> Result<BundleResult>;
    pub async fn get_bundle_status(&self, bundle_id: &str) -> Result<BundleStatus>;
}
```

**Test Criteria**:
- [ ] Submit bundle to Jito block engine (testnet)
- [ ] Include tip transaction
- [ ] Poll bundle status until confirmed/rejected
- [ ] Integration test: Submit mock bundle to Jito devnet

**Reference**:
- Jito bundle API documentation
- Jito tip accounts

---

## Phase 5: AI Orchestration (Week 8)

### Task 5.1: Feature Store
**Assigned to**: AI Agent Nu
**Duration**: 3 days
**Dependencies**: Task 2.1, 2.2 complete

**Deliverable**:
```rust
// crates/ai-orchestration/src/feature_store.rs
pub struct FeatureStore {
    pool_cache: Arc<PoolCache>,
}

pub struct PoolFeatures {
    pub liquidity_depth: f64,      // Reserve size
    pub volatility_1h: f64,         // Price std dev
    pub spread_bps: f64,            // Bid-ask spread
    pub volume_24h: f64,            // Trading volume
}

impl FeatureStore {
    pub fn extract_features(&self, pool_id: &Pubkey) -> Result<PoolFeatures>;
}
```

**Test Criteria**:
- [ ] Calculate liquidity depth from reserves
- [ ] Calculate volatility from historical prices
- [ ] Unit test: Feature extraction for pool

**Reference**:
- `docs/architecture/codex-re-architecture-3-ai-integration.md`

---

### Task 5.2: Slippage Estimation Model (ONNX)
**Assigned to**: AI Agent Xi
**Duration**: 4 days
**Dependencies**: Task 5.1 complete

**Deliverable**:
```rust
// crates/ai-orchestration/src/model_registry.rs
pub struct ModelRegistry {
    slippage_model: Session, // ONNX runtime
}

impl ModelRegistry {
    pub fn predict_slippage(&self, features: &PoolFeatures, amount: u64) -> Result<f64>;
}
```

**Test Criteria**:
- [ ] Load ONNX model from file
- [ ] Run inference on sample features
- [ ] Output slippage estimate (0.0 - 1.0)
- [ ] Unit test: Predict slippage for high/low liquidity pools

**Reference**:
- `docs/architecture/codex-re-architecture-3-ai-integration.md`
- Use `ort` crate for ONNX runtime

---

## Task Assignment Templates

### Template for AI Agent

```markdown
# Task Assignment: {TASK_ID} - {TASK_NAME}

## Context
You are implementing part of a Solana MEV trading system. Your task is independent and testable without requiring other components to be complete.

## Objective
{OBJECTIVE_DESCRIPTION}

## Input Specifications
{WHAT_DATA_YOU_RECEIVE}

## Output Specifications
{WHAT_YOU_MUST_PRODUCE}

## Dependencies
{LIST_OF_COMPLETED_TASKS_YOU_CAN_USE}

## Mock Dependencies
{LIST_OF_THINGS_TO_MOCK}

## Test Criteria
- [ ] {TEST_1}
- [ ] {TEST_2}
- [ ] {TEST_3}

## Reference Code
- External: {EXTERNAL_REPO_PATH}
- Architecture: {ARCHITECTURE_DOC_LINES}

## Mock Data Provided
```rust
{MOCK_DATA_CODE}
```

## Deliverable Files
- `{FILE_PATH_1}`
- `{FILE_PATH_2}`
- `tests/{TEST_FILE}`

## Success Criteria
- All tests pass
- Code compiles with no warnings
- Documented with rustdoc comments
- Benchmarks included (if performance-critical)
```

---

## Parallel Execution Plan

### Week 1 (Parallel Tasks)
- Task 1.1 (DexProtocol trait) - **MUST COMPLETE FIRST**
- Then in parallel:
  - Task 1.2 (Raydium)
  - Task 1.3 (Orca)
  - Task 1.4 (PumpFun)
  - Task 2.1 (Pool Cache)

### Week 2 (Parallel Tasks)
- Task 2.2 (Pool Monitor) - requires 1.2, 1.3, 1.4, 2.1
- Task 2.3 (Swap Detector) - requires 1.2, 1.3, 1.4
- Task 2.4 (Migration Detector) - requires 1.4

### Week 3 (Parallel Tasks)
- Task 3.1 (Multi-DEX Arbitrage) - requires 1.2, 1.3, 2.1, 2.2
- Task 3.2 (PumpFun Sniper) - requires 2.4

### Week 4 (Parallel Tasks)
- Task 4.1 (DEX Router) - requires 1.2, 1.3, 1.4
- Task 5.1 (Feature Store) - requires 2.1, 2.2

### Week 5 (Sequential Tasks)
- Task 4.2 (Jito Client) - requires 4.1
- Task 5.2 (Slippage Model) - requires 5.1

---

## Integration Testing Strategy

After all tasks complete, run integration tests:

```rust
// tests/integration/end_to_end_test.rs

#[tokio::test]
async fn test_full_arbitrage_flow() {
    // 1. Mock Geyser stream with pool updates
    // 2. Pool Monitor updates cache
    // 3. Swap Detector emits SwapEvent
    // 4. Multi-DEX Arbitrage detects opportunity
    // 5. DEX Router builds transaction
    // 6. Jito Client submits bundle
    // 7. Verify execution outcome
}
```

---

## Key Success Factors

1. **Clear Interfaces**: Every task has well-defined input/output
2. **Mock Data**: AI agents can test without dependencies
3. **Parallel Execution**: Most tasks can run simultaneously
4. **Test-Driven**: Success criteria defined upfront
5. **External References**: AI agents can reference real-world code
6. **Integration Last**: Combine all pieces after individual testing

---

**Next Steps**:
1. Review this decomposition strategy
2. Create individual task assignment documents for each AI agent
3. Prepare mock data fixtures
4. Set up CI/CD for automated testing as tasks complete
