# Complete Execution Plan: Multi-Team Development with GitHub Integration

**Created**: 2025-10-02
**Purpose**: Complete plan for coordinating multiple agentic teams with GitHub workflows, CI/CD, and orchestration

---

## Table of Contents

1. [GitHub Repository Structure](#1-github-repository-structure)
2. [Orchestration Team](#2-orchestration-team)
3. [Branch Strategy](#3-branch-strategy)
4. [GitHub Actions CI/CD](#4-github-actions-cicd)
5. [Individual Task Packages](#5-individual-task-packages)
6. [Orchestration Dashboard](#6-orchestration-dashboard)
7. [Week-by-Week Execution Timeline](#7-week-by-week-execution-timeline)
8. [Team Coordination Protocols](#8-team-coordination-protocols)

---

## 1. GitHub Repository Structure

```
MEVana/
├── .github/
│   ├── workflows/
│   │   ├── task-1.1-trait.yml           # CI for Task 1.1
│   │   ├── task-1.2-raydium.yml         # CI for Task 1.2
│   │   ├── task-1.3-orca.yml            # CI for Task 1.3
│   │   ├── task-1.4-pumpfun.yml         # CI for Task 1.4
│   │   ├── task-2.1-pool-cache.yml      # CI for Task 2.1
│   │   ├── task-2.2-pool-monitor.yml    # CI for Task 2.2
│   │   ├── task-2.3-swap-detector.yml   # CI for Task 2.3
│   │   ├── task-2.4-migration-detector.yml
│   │   ├── task-3.1-multi-dex-arb.yml   # CI for Task 3.1
│   │   ├── task-3.2-pumpfun-sniper.yml  # CI for Task 3.2
│   │   ├── task-4.1-dex-router.yml      # CI for Task 4.1
│   │   ├── task-4.2-jito-client.yml     # CI for Task 4.2
│   │   ├── task-5.1-feature-store.yml   # CI for Task 5.1
│   │   ├── task-5.2-slippage-model.yml  # CI for Task 5.2
│   │   ├── integration-phase-1.yml      # Integration after Week 1
│   │   ├── integration-phase-2.yml      # Integration after Week 2
│   │   ├── integration-phase-3.yml      # Integration after Week 3
│   │   ├── integration-phase-4.yml      # Integration after Week 4
│   │   └── orchestration-dashboard.yml  # Update dashboard on push
│   │
│   └── CODEOWNERS                       # Auto-assign PRs to orchestration team
│
├── task-assignments/                    # ← NEW: Task packages for each team
│   ├── TASK_1.1_DexProtocol_Trait/
│   │   ├── ASSIGNMENT.md                # Complete task specification
│   │   ├── mock_data/                   # Sample test data
│   │   ├── interfaces/                  # Trait definitions
│   │   ├── references/                  # External code links
│   │   └── success_checklist.md         # Completion criteria
│   │
│   ├── TASK_1.2_Raydium_Implementation/
│   │   ├── ASSIGNMENT.md
│   │   ├── mock_data/
│   │   │   ├── raydium_sol_usdc_pool.json
│   │   │   ├── raydium_swap_transaction.json
│   │   │   └── expected_calculations.json
│   │   ├── interfaces/
│   │   │   └── dex_protocol_trait.rs    # From Task 1.1
│   │   ├── references/
│   │   │   ├── external_raydium_code.md
│   │   │   └── amm_formula.md
│   │   └── success_checklist.md
│   │
│   ├── TASK_1.3_Orca_Implementation/
│   ├── TASK_1.4_PumpFun_Implementation/
│   ├── TASK_2.1_Pool_Cache/
│   ├── TASK_2.2_Pool_Monitor/
│   ├── TASK_2.3_Swap_Detector/
│   ├── TASK_2.4_Migration_Detector/
│   ├── TASK_3.1_Multi_Dex_Arbitrage/
│   ├── TASK_3.2_PumpFun_Sniper/
│   ├── TASK_4.1_Dex_Router/
│   ├── TASK_4.2_Jito_Client/
│   ├── TASK_5.1_Feature_Store/
│   └── TASK_5.2_Slippage_Model/
│
├── orchestration/                       # ← NEW: Orchestration team workspace
│   ├── dashboard/                       # Live progress tracking
│   │   ├── index.html                   # Web dashboard
│   │   ├── task_status.json             # Machine-readable status
│   │   └── update_dashboard.sh          # Auto-update script
│   │
│   ├── integration/                     # Integration checkpoints
│   │   ├── phase-1-foundation.md        # Week 1 integration plan
│   │   ├── phase-2-data-layer.md        # Week 2 integration plan
│   │   ├── phase-3-strategy.md          # Week 3 integration plan
│   │   └── phase-4-execution.md         # Week 4 integration plan
│   │
│   ├── coordination/
│   │   ├── daily_standup_template.md    # Daily status updates
│   │   ├── blocker_resolution.md        # How to handle blockers
│   │   └── team_contacts.md             # Contact info for each team
│   │
│   └── metrics/
│       ├── velocity_tracker.csv         # Tasks completed per week
│       ├── quality_metrics.csv          # Coverage, test pass rate, etc.
│       └── cost_tracker.csv             # Agent hours, cost per task
│
├── crates/
│   ├── dex-abstraction/                 # Populated by Teams 1.1-1.4
│   ├── data-layer/                      # Populated by Teams 2.1-2.4
│   ├── strategy-layer/                  # Populated by Teams 3.1-3.2
│   ├── execution-layer/                 # Populated by Teams 4.1-4.2
│   └── ai-orchestration/                # Populated by Teams 5.1-5.2
│
└── docs/
    └── architecture/
        ├── claude-re-architecture-3-final.md
        ├── AI_TASK_DECOMPOSITION_STRATEGY.md
        ├── AGENTIC_TEAM_COMPREHENSIVE_PLAN.md
        └── COMPLETE_EXECUTION_PLAN.md (this document)
```

---

## 2. Orchestration Team

### 2.1 Orchestration Team Composition

**Role: Orchestration Lead** (1 person, could be AI + human oversight)
- Tracks all task progress
- Manages dependencies (Task X can't start until Task Y complete)
- Resolves blockers
- Coordinates integration checkpoints
- Makes go/no-go decisions for phases

**Role: Integration Engineers** (2 agents + 1 human)
- Combine completed tasks at integration checkpoints
- Resolve interface mismatches
- Write integration glue code
- Run end-to-end tests
- Deploy to staging server

**Role: DevOps Engineer** (1 agent + 1 human)
- Maintain GitHub Actions workflows
- Manage CI/CD infrastructure
- Set up staging/production servers
- Monitor build failures
- Optimize build times

**Role: Quality Overseer** (1 agent + 1 human)
- Review QA reports from all tasks
- Ensure quality standards consistent across teams
- Track quality metrics (coverage, bugs, vulnerabilities)
- Escalate quality issues

---

### 2.2 Orchestration Team Responsibilities

**Daily**:
- [ ] Check task status dashboard (which tasks completed today?)
- [ ] Review GitHub Actions results (any failures?)
- [ ] Respond to blocker reports from teams
- [ ] Update stakeholders on progress

**Weekly**:
- [ ] Run integration checkpoint (combine completed tasks)
- [ ] Review velocity metrics (on track for timeline?)
- [ ] Adjust task assignments if needed
- [ ] Plan next week's tasks (which teams start next?)

**At Integration Checkpoints**:
- [ ] Pull all completed task branches
- [ ] Run integration tests
- [ ] Resolve interface conflicts
- [ ] Deploy to staging server
- [ ] Run end-to-end performance tests
- [ ] Make go/no-go decision for next phase

---

## 3. Branch Strategy

### 3.1 Branch Naming Convention

```
main                                    # Production-ready code
├── develop                             # Integration branch
│   ├── phase-1-foundation              # Week 1 integration branch
│   │   ├── task-1.1-dex-trait         # Task 1.1 work
│   │   ├── task-1.2-raydium           # Task 1.2 work
│   │   ├── task-1.3-orca              # Task 1.3 work
│   │   ├── task-1.4-pumpfun           # Task 1.4 work
│   │   └── task-2.1-pool-cache        # Task 2.1 work
│   │
│   ├── phase-2-data-layer              # Week 2 integration branch
│   │   ├── task-2.2-pool-monitor      # Task 2.2 work
│   │   ├── task-2.3-swap-detector     # Task 2.3 work
│   │   └── task-2.4-migration-detector # Task 2.4 work
│   │
│   ├── phase-3-strategy                # Week 3 integration branch
│   │   ├── task-3.1-multi-dex-arb     # Task 3.1 work
│   │   └── task-3.2-pumpfun-sniper    # Task 3.2 work
│   │
│   └── phase-4-execution               # Week 4 integration branch
│       ├── task-4.1-dex-router        # Task 4.1 work
│       ├── task-4.2-jito-client       # Task 4.2 work
│       ├── task-5.1-feature-store     # Task 5.1 work
│       └── task-5.2-slippage-model    # Task 5.2 work
```

### 3.2 Branch Workflow

**For Each Task Team**:

1. **Start**: Create branch from phase integration branch
   ```bash
   git checkout phase-1-foundation
   git checkout -b task-1.2-raydium
   ```

2. **Work**: Commit frequently to task branch
   ```bash
   git add .
   git commit -m "feat(raydium): implement parse_pool_account"
   git push origin task-1.2-raydium
   ```

3. **Complete**: Open PR to phase branch
   ```bash
   # PR: task-1.2-raydium → phase-1-foundation
   # Auto-triggers GitHub Actions for Task 1.2
   ```

4. **Review**: Orchestration team reviews PR
   - Check CI passing (all tests, clippy, coverage)
   - Review code quality
   - Verify success checklist complete

5. **Merge**: Orchestration team merges PR
   ```bash
   # PR merged: task-1.2-raydium → phase-1-foundation
   ```

**For Integration Checkpoints**:

1. **All tasks in phase complete**: Orchestration team creates integration PR
   ```bash
   # PR: phase-1-foundation → develop
   # Triggers integration-phase-1.yml workflow
   ```

2. **Integration tests run**: End-to-end tests across all tasks in phase

3. **If pass**: Merge to develop, deploy to staging

4. **If fail**: Create integration fix tasks, assign to Integration Engineers

---

## 4. GitHub Actions CI/CD

### 4.1 Per-Task Workflow Template

**File**: `.github/workflows/task-1.2-raydium.yml`

```yaml
name: Task 1.2 - Raydium Implementation

on:
  push:
    branches:
      - task-1.2-raydium
  pull_request:
    branches:
      - phase-1-foundation

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Build dex-abstraction crate
        run: cargo build --package dex-abstraction --verbose

  test:
    name: Test
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run tests
        run: cargo test --package dex-abstraction --verbose

      - name: Generate coverage report
        uses: actions-rs/tarpaulin@v0.1
        with:
          args: '--package dex-abstraction --out Xml --output-dir coverage/'

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/cobertura.xml
          flags: task-1.2-raydium

      - name: Check coverage threshold
        run: |
          COVERAGE=$(grep -oP 'line-rate="\K[0-9.]+' coverage/cobertura.xml | head -1)
          COVERAGE_PCT=$(echo "$COVERAGE * 100" | bc)
          echo "Coverage: $COVERAGE_PCT%"
          if (( $(echo "$COVERAGE_PCT < 85" | bc -l) )); then
            echo "Coverage $COVERAGE_PCT% is below 85% threshold"
            exit 1
          fi

  lint:
    name: Lint
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy

      - name: Check formatting
        run: cargo fmt --package dex-abstraction --check

      - name: Run clippy
        run: cargo clippy --package dex-abstraction -- -D warnings

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/checkout@v3

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Run security audit
        run: cargo audit

  benchmark:
    name: Benchmark
    runs-on: ubuntu-latest
    needs: test
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run benchmarks
        run: cargo bench --package dex-abstraction -- --save-baseline task-1.2

      - name: Compare with baseline
        run: |
          # Compare current benchmarks with previous baseline
          # Fail if >10% regression

  deploy-staging:
    name: Deploy to Staging
    runs-on: ubuntu-latest
    needs: [test, lint, audit]
    if: github.event_name == 'pull_request' && github.base_ref == 'phase-1-foundation'
    steps:
      - uses: actions/checkout@v3

      - name: Build release binary
        run: cargo build --release --package dex-abstraction

      - name: Deploy to staging server
        uses: easingthemes/ssh-deploy@v2.1.5
        env:
          SSH_PRIVATE_KEY: ${{ secrets.STAGING_SSH_KEY }}
          REMOTE_HOST: ${{ secrets.STAGING_HOST }}
          REMOTE_USER: ${{ secrets.STAGING_USER }}
          SOURCE: "target/release/"
          TARGET: "/opt/mevana/staging/task-1.2/"

      - name: Run integration smoke test
        run: |
          ssh ${{ secrets.STAGING_USER }}@${{ secrets.STAGING_HOST }} \
            "cd /opt/mevana/staging/task-1.2 && ./run_smoke_tests.sh"

  update-dashboard:
    name: Update Orchestration Dashboard
    runs-on: ubuntu-latest
    needs: [test, lint, audit]
    if: always()
    steps:
      - uses: actions/checkout@v3

      - name: Update task status
        run: |
          # Update orchestration/dashboard/task_status.json
          # with test results, coverage, timestamp
          python3 orchestration/dashboard/update_status.py \
            --task "1.2" \
            --status "tests_passed" \
            --coverage "$COVERAGE_PCT" \
            --commit "${{ github.sha }}"

      - name: Commit dashboard update
        run: |
          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add orchestration/dashboard/task_status.json
          git commit -m "chore: update dashboard for task 1.2"
          git push
```

### 4.2 Integration Workflow Template

**File**: `.github/workflows/integration-phase-1.yml`

```yaml
name: Integration Phase 1 - Foundation Layer

on:
  pull_request:
    branches:
      - develop
    paths:
      - 'crates/dex-abstraction/**'

env:
  RUST_BACKTRACE: 1
  DATABASE_URL: "postgresql://test:test@localhost:5432/testdb"

jobs:
  integration-test:
    name: Integration Tests
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: test
          POSTGRES_DB: testdb
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build all foundation crates
        run: |
          cargo build --package dex-abstraction
          cargo build --package data-layer

      - name: Run integration tests
        run: cargo test --test integration_phase_1 --verbose

      - name: Test cross-crate interfaces
        run: |
          # Verify RaydiumAMM, OrcaWhirlpool, PumpFunBonding all implement DexProtocol
          # Verify PoolCache can store all DEX pool types
          cargo test --test cross_dex_compatibility

  performance-test:
    name: Performance Tests
    runs-on: ubuntu-latest
    needs: integration-test
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run end-to-end latency test
        run: |
          # Mock Geyser stream → Pool parse → Cache update
          # Target: <10ms end-to-end
          cargo test --test performance_phase_1 -- --nocapture

      - name: Verify latency targets
        run: |
          # Parse test output, fail if >10ms

  deploy-staging:
    name: Deploy Phase 1 to Staging
    runs-on: ubuntu-latest
    needs: [integration-test, performance-test]
    steps:
      - uses: actions/checkout@v3

      - name: Build release
        run: cargo build --release --workspace

      - name: Deploy to staging
        uses: easingthemes/ssh-deploy@v2.1.5
        env:
          SSH_PRIVATE_KEY: ${{ secrets.STAGING_SSH_KEY }}
          REMOTE_HOST: ${{ secrets.STAGING_HOST }}
          REMOTE_USER: ${{ secrets.STAGING_USER }}
          SOURCE: "target/release/"
          TARGET: "/opt/mevana/staging/phase-1/"

      - name: Restart staging services
        run: |
          ssh ${{ secrets.STAGING_USER }}@${{ secrets.STAGING_HOST }} \
            "sudo systemctl restart mevana-phase-1"

      - name: Run smoke tests on staging
        run: |
          # Wait 30s for service startup
          sleep 30
          # Hit health endpoint
          curl -f http://${{ secrets.STAGING_HOST }}:8088/health || exit 1
```

---

## 5. Individual Task Packages

### 5.1 Task Package Structure

Each task gets a complete package in `task-assignments/{TASK_NAME}/`

**Example: Task 1.2 - Raydium Implementation**

```
task-assignments/TASK_1.2_Raydium_Implementation/
├── ASSIGNMENT.md                        # ← Complete specification (see below)
├── TEAM_ROLES.md                        # ← Who does what
├── SUCCESS_CHECKLIST.md                 # ← Completion criteria
├── HANDOFF_TEMPLATE.md                  # ← Handoff between agents
│
├── mock_data/
│   ├── raydium_sol_usdc_pool.json      # Real mainnet pool account
│   ├── raydium_swap_tx.json            # Real swap transaction
│   ├── expected_swap_output.json       # Ground truth calculations
│   └── edge_cases.json                 # Zero amount, max values, etc.
│
├── interfaces/
│   ├── dex_protocol_trait.rs           # Trait from Task 1.1
│   ├── pool_state_struct.rs            # PoolState definition
│   └── swap_quote_struct.rs            # SwapQuote definition
│
├── references/
│   ├── external_code_links.md          # Solana-Arbitrage-Bot1 references
│   ├── raydium_amm_formula.md          # Constant product math
│   ├── raydium_account_layout.md       # Account deserialization
│   └── raydium_instruction_format.md   # Instruction building
│
├── tests/
│   ├── test_strategy.md                # What tests to write
│   ├── test_data_generator.rs          # Generate test fixtures
│   └── test_templates.rs               # Example test structure
│
└── ci/
    ├── .github_workflow_snippet.yml    # CI configuration for this task
    └── coverage_config.toml            # Coverage requirements
```

---

### 5.2 ASSIGNMENT.md Template

**File**: `task-assignments/TASK_1.2_Raydium_Implementation/ASSIGNMENT.md`

```markdown
# Task 1.2: Raydium AMM Implementation

## Metadata
- **Task ID**: 1.2
- **Phase**: 1 (Foundation Layer)
- **Duration Estimate**: 7.5 person-days
- **Team Size**: 5 agents (Coder, Test Writer, Test Runner, QA, Docs)
- **Dependencies**: Task 1.1 (DexProtocol trait) MUST be complete
- **Branch**: `task-1.2-raydium`
- **PR Target**: `phase-1-foundation`

---

## Objective

Implement the `DexProtocol` trait for Raydium AMM (constant product formula).

**What You're Building**:
A Rust implementation that can:
1. Parse Raydium AMM v4 pool account data
2. Calculate swap output using constant product (x * y = k) formula
3. Build Raydium swap instructions
4. Parse swap transactions to extract swap events

---

## Context

Raydium is a Solana AMM DEX using the constant product market maker formula.

**Key Concepts**:
- **Constant Product**: `reserve_a * reserve_b = k` (must remain constant after swap)
- **Fee**: 0.25% (25 bps) taken from input amount
- **Account Layout**: Raydium AMM v4 has specific account structure
- **Program ID**: `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`

---

## Input Specifications

### 1. Trait Definition (from Task 1.1)

```rust
// You MUST implement this trait
pub trait DexProtocol: Send + Sync {
    fn name(&self) -> &str;
    fn program_id(&self) -> Pubkey;
    fn parse_pool_account(&self, data: &[u8]) -> Result<PoolState>;
    fn calculate_swap_output(&self, pool: &PoolState, input_token: Pubkey, input_amount: u64) -> Result<SwapQuote>;
    fn build_swap_instruction(&self, pool: &PoolState, user: &Pubkey, user_input_ata: &Pubkey, user_output_ata: &Pubkey, input_amount: u64, min_output_amount: u64) -> Result<Instruction>;
    fn parse_swap_transaction(&self, tx: &EncodedTransaction) -> Result<Option<SwapEvent>>;
    fn get_pool_accounts(&self, pool_id: &Pubkey) -> Vec<Pubkey>;
}
```

### 2. Mock Data Provided

**File**: `mock_data/raydium_sol_usdc_pool.json`
```json
{
  "pool_id": "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",
  "token_a": "So11111111111111111111111111111111111111112",
  "token_b": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "reserve_a": 1000000000000,
  "reserve_b": 150000000000,
  "fee_bps": 25,
  "account_data_hex": "..."
}
```

### 3. External Code References

**Solana-Arbitrage-Bot1**:
- File: `arbitrage/programs/tmp/src/swaps/raydium.rs`
- What to reference: CPI patterns, account order, instruction building

**solana-defi**:
- File: `Raydium/Typescript/stream_and_parse_raydium_amm_transactions/`
- What to reference: Transaction parsing, account layout

---

## Output Specifications

### 1. Implementation Files

**File**: `crates/dex-abstraction/src/raydium/amm.rs`

```rust
pub struct RaydiumAMM {
    program_id: Pubkey,
}

impl RaydiumAMM {
    pub fn new() -> Self {
        Self {
            program_id: pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"),
        }
    }
}

impl DexProtocol for RaydiumAMM {
    // TODO: Implement all trait methods
}
```

### 2. Test Files

**File**: `crates/dex-abstraction/tests/raydium_tests.rs`

Minimum test coverage:
- [ ] Parse pool account (real mainnet data)
- [ ] Calculate swap output (1000 USDC → SOL)
- [ ] Calculate reverse swap (1 SOL → USDC)
- [ ] Edge case: Zero input amount (should error)
- [ ] Edge case: Insufficient liquidity (should error)
- [ ] Build swap instruction (verify account order)
- [ ] Parse swap transaction (extract amounts)

### 3. Documentation

**File**: `crates/dex-abstraction/src/raydium/README.md`

Contents:
- Overview of Raydium AMM
- Constant product formula explanation
- Usage example (get swap quote)
- Troubleshooting common issues

---

## Success Criteria

### Implementation Agent
- [ ] Code compiles without errors
- [ ] All trait methods implemented
- [ ] Basic unit tests pass
- [ ] Committed to branch `task-1.2-raydium`

### Test Writer Agent
- [ ] >85% code coverage
- [ ] All edge cases tested
- [ ] Property-based tests (swap output always ≤ reserve)
- [ ] Benchmarks included (calculation latency <1ms)

### Test Runner Agent
- [ ] All tests pass (100% success rate)
- [ ] Coverage report generated (>85%)
- [ ] Benchmarks within target (<1ms)

### QA/Linter Agent
- [ ] Zero clippy warnings
- [ ] Zero rustfmt violations
- [ ] Constant product formula verified correct
- [ ] No integer overflow vulnerabilities

### Documentation Agent
- [ ] README.md complete
- [ ] Every public function has rustdoc comment
- [ ] Example compiles and runs
- [ ] Formula explained with diagram

---

## Timeline

**Day 1-3**: Implementation Agent
**Day 4-5**: Test Writer Agent
**Day 6**: Test Runner + QA Agent (parallel)
**Day 7**: Documentation Agent

**Total**: 7.5 days

---

## Handoff Instructions

When you complete your part, fill out `HANDOFF_TEMPLATE.md` and notify next agent.

**Implementation → Test Writer**:
- Provide: Source code, basic tests, known edge cases
- Notify: Post in #task-1.2-raydium Slack channel

**Test Writer → Test Runner**:
- Provide: Complete test suite, coverage config
- Notify: Create PR to `phase-1-foundation`

**Test Runner → QA**:
- Provide: Test results, coverage report, benchmark results
- Notify: Comment on PR with test results

**QA → Documentation**:
- Provide: Approved code, QA report
- Notify: Add "qa-approved" label to PR

**Documentation → Orchestration**:
- Provide: Complete package (code + tests + docs)
- Notify: Request PR review from @orchestration-team

---

## Blockers

If you encounter a blocker:

1. **Try to resolve** (spend max 2 hours)
2. **Document blocker** in `BLOCKERS.md` in this directory
3. **Notify orchestration team** in #orchestration Slack channel
4. **Wait for response** (SLA: 4 hours)

Common blockers:
- Trait definition unclear → Contact Task 1.1 team
- Mock data insufficient → Request more data from orchestration
- External reference code unavailable → Notify orchestration

---

## References

- Architecture: `docs/architecture/claude-re-architecture-3-final.md` lines 114-160
- Task decomposition: `docs/architecture/AI_TASK_DECOMPOSITION_STRATEGY.md`
- Team plan: `docs/architecture/AGENTIC_TEAM_COMPREHENSIVE_PLAN.md`
- Raydium formula: `task-assignments/TASK_1.2_Raydium_Implementation/references/raydium_amm_formula.md`

---

## Contact

- **Orchestration Lead**: @orchestration-lead (Slack)
- **Integration Engineer**: @integration-team (Slack)
- **Blocker Escalation**: #orchestration (Slack channel)
```

---

### 5.3 TEAM_ROLES.md Template

**File**: `task-assignments/TASK_1.2_Raydium_Implementation/TEAM_ROLES.md`

```markdown
# Team Roles: Task 1.2 - Raydium Implementation

## Team Members

### Implementation Agent
**Agent ID**: Agent-Coder-007
**Duration**: Days 1-3 (3 days)

**Responsibilities**:
- Implement `RaydiumAMM` struct
- Implement all `DexProtocol` trait methods
- Write basic unit tests
- Document code with rustdoc comments

**Inputs**:
- `ASSIGNMENT.md`
- `interfaces/dex_protocol_trait.rs`
- `mock_data/raydium_sol_usdc_pool.json`
- `references/raydium_amm_formula.md`

**Outputs**:
- `crates/dex-abstraction/src/raydium/amm.rs`
- `crates/dex-abstraction/src/raydium/mod.rs`
- Basic unit tests in same file

**Handoff To**: Test Writer Agent
**Handoff Criteria**:
- [ ] Code compiles
- [ ] Basic tests pass
- [ ] Branch pushed to GitHub

---

### Test Writer Agent
**Agent ID**: Agent-Test-042
**Duration**: Days 4-5 (2 days)

**Responsibilities**:
- Write comprehensive test suite
- Write property-based tests
- Write benchmarks
- Generate test coverage report

**Inputs**:
- Implementation code from Agent-Coder-007
- `tests/test_strategy.md`
- `mock_data/` (all files)

**Outputs**:
- `crates/dex-abstraction/tests/raydium_tests.rs`
- `crates/dex-abstraction/benches/raydium_bench.rs`

**Handoff To**: Test Runner Agent
**Handoff Criteria**:
- [ ] Test suite complete
- [ ] >85% coverage expected
- [ ] All edge cases documented

---

### Test Runner Agent
**Agent ID**: Agent-Runner-CI
**Duration**: Day 6 (0.5 days)

**Responsibilities**:
- Run test suite
- Generate coverage report
- Run benchmarks
- Report results

**Inputs**:
- Test suite from Agent-Test-042
- CI configuration from `ci/.github_workflow_snippet.yml`

**Outputs**:
- Test report (pass/fail counts)
- Coverage report (HTML + lcov)
- Benchmark results

**Handoff To**: QA/Linter Agent (parallel) and Report to Implementation Agent if failures
**Handoff Criteria**:
- [ ] All tests pass
- [ ] Coverage >85%
- [ ] Benchmarks within target

---

### QA/Linter Agent
**Agent ID**: Agent-QA-Rust
**Duration**: Day 6 (0.5 days, parallel with Test Runner)

**Responsibilities**:
- Run clippy
- Run rustfmt
- Run cargo audit
- Manual code review

**Inputs**:
- Implementation code
- Test results from Test Runner

**Outputs**:
- QA report
- List of issues (if any)
- Approval decision

**Handoff To**: Documentation Agent (if approved) or back to Implementation Agent (if issues)
**Handoff Criteria**:
- [ ] Zero clippy warnings
- [ ] Zero rustfmt violations
- [ ] No security vulnerabilities
- [ ] Code review passed

---

### Documentation Agent
**Agent ID**: Agent-Docs-Writer
**Duration**: Day 7 (1 day)

**Responsibilities**:
- Write README.md
- Enhance rustdoc comments
- Create usage examples
- Write troubleshooting guide

**Inputs**:
- Approved implementation code
- Test examples for usage
- Architecture docs

**Outputs**:
- `crates/dex-abstraction/src/raydium/README.md`
- Enhanced rustdoc comments in code
- `crates/dex-abstraction/examples/raydium_example.rs`

**Handoff To**: Orchestration Team (task complete)
**Handoff Criteria**:
- [ ] README complete
- [ ] All public APIs documented
- [ ] Examples compile and run

---

## Communication Channels

- **Team Slack Channel**: #task-1.2-raydium
- **GitHub Branch**: `task-1.2-raydium`
- **GitHub PR**: (created by Test Writer)
- **Daily Standup**: 9:00 AM UTC in #task-1.2-raydium

## Escalation Path

1. **Try to resolve** within team (2 hours max)
2. **Post in team channel** (#task-1.2-raydium)
3. **Escalate to orchestration** (#orchestration)
4. **Emergency**: DM @orchestration-lead
```

---

### 5.4 SUCCESS_CHECKLIST.md Template

**File**: `task-assignments/TASK_1.2_Raydium_Implementation/SUCCESS_CHECKLIST.md`

```markdown
# Success Checklist: Task 1.2 - Raydium Implementation

## Implementation Complete ✅

- [ ] `RaydiumAMM` struct defined
- [ ] `DexProtocol::name()` returns "raydium_amm"
- [ ] `DexProtocol::program_id()` returns correct Raydium program ID
- [ ] `DexProtocol::parse_pool_account()` deserializes account data correctly
- [ ] `DexProtocol::calculate_swap_output()` implements constant product formula
- [ ] `DexProtocol::build_swap_instruction()` creates valid instruction
- [ ] `DexProtocol::parse_swap_transaction()` extracts swap event
- [ ] `DexProtocol::get_pool_accounts()` returns all required accounts
- [ ] Code compiles without errors
- [ ] Basic unit tests pass

## Tests Complete ✅

- [ ] Test: Parse real mainnet pool account
- [ ] Test: Calculate swap 1000 USDC → SOL (verify output matches expected)
- [ ] Test: Calculate reverse swap 1 SOL → USDC
- [ ] Test: Edge case - zero input amount (should return error)
- [ ] Test: Edge case - input exceeds reserve (should return error)
- [ ] Test: Build swap instruction (verify accounts in correct order)
- [ ] Test: Parse swap transaction from mainnet
- [ ] Property test: swap output always ≤ reserve
- [ ] Property test: constant product maintained (with fees)
- [ ] Benchmark: calculate_swap_output latency <1ms
- [ ] Coverage >85%

## Quality Assurance Complete ✅

- [ ] `cargo fmt --check` passes (zero violations)
- [ ] `cargo clippy -- -D warnings` passes (zero warnings)
- [ ] `cargo audit` passes (zero vulnerabilities)
- [ ] No `.unwrap()` calls in production code (use `?` operator)
- [ ] No integer overflow vulnerabilities (use checked arithmetic)
- [ ] Error messages are descriptive
- [ ] Constant product formula verified correct

## Documentation Complete ✅

- [ ] README.md exists in `src/raydium/`
- [ ] README explains constant product formula
- [ ] README includes usage example
- [ ] Every public function has rustdoc comment
- [ ] Every struct has rustdoc comment
- [ ] Example code in `examples/raydium_example.rs` compiles
- [ ] Example code runs successfully
- [ ] Troubleshooting section in README

## Integration Ready ✅

- [ ] Committed to branch `task-1.2-raydium`
- [ ] Pushed to GitHub
- [ ] PR created to `phase-1-foundation`
- [ ] CI passing (all checks green)
- [ ] Coverage report uploaded to Codecov
- [ ] Handoff document complete
- [ ] Orchestration team notified

## Sign-Off

**Implementation Agent**: _________________ Date: _______
**Test Writer Agent**: _________________ Date: _______
**Test Runner Agent**: _________________ Date: _______
**QA/Linter Agent**: _________________ Date: _______
**Documentation Agent**: _________________ Date: _______

**Orchestration Lead Approval**: _________________ Date: _______
```

---

## 6. Orchestration Dashboard

### 6.1 Dashboard Structure

**File**: `orchestration/dashboard/index.html`

```html
<!DOCTYPE html>
<html>
<head>
    <title>MEVana Development Dashboard</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .phase { border: 2px solid #333; margin: 20px 0; padding: 15px; }
        .task { border-left: 4px solid #666; margin: 10px 0; padding: 10px; }
        .task.complete { border-color: green; background: #e8f5e9; }
        .task.in-progress { border-color: orange; background: #fff3e0; }
        .task.blocked { border-color: red; background: #ffebee; }
        .task.not-started { border-color: gray; background: #f5f5f5; }
        .metrics { display: flex; justify-content: space-around; margin: 20px 0; }
        .metric { text-align: center; padding: 20px; border: 1px solid #ddd; }
    </style>
</head>
<body>
    <h1>MEVana Development Dashboard</h1>
    <p>Last Updated: <span id="last-updated"></span></p>

    <div class="metrics">
        <div class="metric">
            <h3>Tasks Complete</h3>
            <p id="tasks-complete">0 / 14</p>
        </div>
        <div class="metric">
            <h3>Average Coverage</h3>
            <p id="avg-coverage">0%</p>
        </div>
        <div class="metric">
            <h3>Velocity</h3>
            <p id="velocity">0 tasks/week</p>
        </div>
        <div class="metric">
            <h3>Blockers</h3>
            <p id="blockers">0</p>
        </div>
    </div>

    <div id="phases"></div>

    <script>
        // Load task_status.json and render dashboard
        fetch('task_status.json')
            .then(response => response.json())
            .then(data => renderDashboard(data));

        function renderDashboard(data) {
            document.getElementById('last-updated').innerText = data.last_updated;
            document.getElementById('tasks-complete').innerText =
                `${data.tasks_complete} / ${data.total_tasks}`;
            document.getElementById('avg-coverage').innerText =
                `${data.average_coverage}%`;
            document.getElementById('velocity').innerText =
                `${data.velocity} tasks/week`;
            document.getElementById('blockers').innerText = data.blockers;

            const phasesDiv = document.getElementById('phases');
            data.phases.forEach(phase => {
                const phaseDiv = document.createElement('div');
                phaseDiv.className = 'phase';
                phaseDiv.innerHTML = `<h2>${phase.name}</h2>`;

                phase.tasks.forEach(task => {
                    const taskDiv = document.createElement('div');
                    taskDiv.className = `task ${task.status}`;
                    taskDiv.innerHTML = `
                        <h3>Task ${task.id}: ${task.name}</h3>
                        <p>Status: ${task.status}</p>
                        <p>Progress: ${task.progress}%</p>
                        <p>Coverage: ${task.coverage}%</p>
                        <p>Assigned: ${task.agent}</p>
                        ${task.blocker ? `<p style="color: red;">Blocker: ${task.blocker}</p>` : ''}
                    `;
                    phaseDiv.appendChild(taskDiv);
                });

                phasesDiv.appendChild(phaseDiv);
            });
        }
    </script>
</body>
</html>
```

**File**: `orchestration/dashboard/task_status.json`

```json
{
  "last_updated": "2025-10-02T14:30:00Z",
  "tasks_complete": 3,
  "total_tasks": 14,
  "average_coverage": 87.2,
  "velocity": 2.5,
  "blockers": 1,
  "phases": [
    {
      "name": "Phase 1: Foundation Layer (Week 1)",
      "tasks": [
        {
          "id": "1.1",
          "name": "DexProtocol Trait",
          "status": "complete",
          "progress": 100,
          "coverage": 95.0,
          "agent": "Agent-Coder-001",
          "completed_date": "2025-09-28"
        },
        {
          "id": "1.2",
          "name": "Raydium Implementation",
          "status": "in-progress",
          "progress": 70,
          "coverage": 89.2,
          "agent": "Agent-Coder-007",
          "estimated_completion": "2025-10-03"
        },
        {
          "id": "1.3",
          "name": "Orca Implementation",
          "status": "in-progress",
          "progress": 40,
          "coverage": 0,
          "agent": "Agent-Coder-012",
          "blocker": "Waiting for whirlpool account layout documentation"
        },
        {
          "id": "1.4",
          "name": "PumpFun Implementation",
          "status": "not-started",
          "progress": 0,
          "coverage": 0,
          "agent": "Not assigned",
          "blocked_by": "Task 1.1"
        }
      ]
    }
  ]
}
```

---

## 7. Week-by-Week Execution Timeline

### Week 1: Foundation Layer (Phase 1)

**Monday (Day 1)**:
- [ ] Orchestration: Create task packages for Tasks 1.1-1.4, 2.1
- [ ] Orchestration: Assign teams to each task
- [ ] Task 1.1 Team: Start DexProtocol trait definition
- [ ] All other teams: Wait for Task 1.1 completion

**Tuesday (Day 2)**:
- [ ] Task 1.1: Continue implementation
- [ ] Orchestration: Daily standup (check blockers)

**Wednesday (Day 3)**:
- [ ] Task 1.1: Complete, PR created to `phase-1-foundation`
- [ ] Orchestration: Review Task 1.1 PR, merge if approved
- [ ] **KICKOFF**: Tasks 1.2, 1.3, 1.4, 2.1 all start (parallel)

**Thursday-Friday (Days 4-5)**:
- [ ] Tasks 1.2, 1.3, 1.4, 2.1: Implementation continues
- [ ] Orchestration: Monitor progress, resolve blockers

**Weekend**:
- [ ] Tasks continue (or pause depending on team availability)

**Monday (Day 8)**:
- [ ] Task 1.2: Test writing complete
- [ ] Task 2.1: Complete (small task, 4 days)
- [ ] Tasks 1.3, 1.4: Ongoing

**Tuesday-Wednesday (Days 9-10)**:
- [ ] Task 1.2: QA complete, docs complete, **TASK COMPLETE**
- [ ] Task 1.3, 1.4: Test writing, QA

**Thursday (Day 11)**:
- [ ] Task 1.3: **COMPLETE**
- [ ] Task 1.4: **COMPLETE**
- [ ] Orchestration: All Phase 1 tasks complete

**Friday (Day 12) - INTEGRATION CHECKPOINT 1**:
- [ ] Integration Team: Create PR `phase-1-foundation` → `develop`
- [ ] Integration Team: Run integration tests (cross-DEX compatibility)
- [ ] Integration Team: Deploy to staging server
- [ ] Integration Team: Run end-to-end performance tests
- [ ] **GO/NO-GO DECISION**: Proceed to Phase 2?

---

### Week 2: Data Layer (Phase 2)

**Monday (Day 15)**:
- [ ] Orchestration: Create task packages for Tasks 2.2, 2.3, 2.4
- [ ] Orchestration: Assign teams
- [ ] **KICKOFF**: Tasks 2.2, 2.3, 2.4 all start (parallel)

**Tuesday-Friday (Days 16-19)**:
- [ ] Task 2.2 (Pool Monitor): Implementation (complex, 13 person-days)
- [ ] Task 2.3 (Swap Detector): Implementation
- [ ] Task 2.4 (Migration Detector): Implementation

**Monday (Day 22)**:
- [ ] Task 2.4: **COMPLETE** (small task, 5 days)
- [ ] Task 2.3: Test writing complete

**Tuesday-Thursday (Days 23-25)**:
- [ ] Task 2.3: **COMPLETE**
- [ ] Task 2.2: Test writing, QA

**Friday (Day 26) - INTEGRATION CHECKPOINT 2**:
- [ ] Task 2.2: **COMPLETE**
- [ ] Integration Team: Run integration tests (Geyser → Pool Monitor → Cache)
- [ ] Integration Team: Deploy to staging
- [ ] **GO/NO-GO DECISION**: Proceed to Phase 3?

---

### Week 3: Strategy Layer (Phase 3)

**Monday (Day 29)**:
- [ ] Orchestration: Create task packages for Tasks 3.1, 3.2
- [ ] **KICKOFF**: Tasks 3.1, 3.2 start (parallel)

**Tuesday-Friday (Days 30-33)**:
- [ ] Task 3.1 (Multi-DEX Arb): Implementation (complex, 17 person-days)
- [ ] Task 3.2 (PumpFun Sniper): Implementation

**Monday (Day 36)**:
- [ ] Task 3.2: **COMPLETE**

**Tuesday-Thursday (Days 37-39)**:
- [ ] Task 3.1: Test writing, integration testing with real pool data

**Friday (Day 40) - INTEGRATION CHECKPOINT 3**:
- [ ] Task 3.1: **COMPLETE**
- [ ] Integration Team: Test arbitrage detection with mainnet replay
- [ ] Integration Team: Verify profitability calculations
- [ ] **GO/NO-GO DECISION**: Proceed to Phase 4?

---

### Week 4: Execution Layer (Phase 4)

**Monday (Day 43)**:
- [ ] Orchestration: Create task packages for Tasks 4.1, 4.2, 5.1, 5.2
- [ ] **KICKOFF**: All Phase 4 tasks start (parallel)

**Tuesday-Friday (Days 44-47)**:
- [ ] Task 4.1 (DEX Router): Implementation
- [ ] Task 4.2 (Jito Client): Implementation
- [ ] Task 5.1 (Feature Store): Implementation
- [ ] Task 5.2 (Slippage Model): Implementation

**Monday-Wednesday (Days 50-52)**:
- [ ] All tasks: Testing, QA, documentation

**Thursday (Day 53) - FINAL INTEGRATION**:
- [ ] All Phase 4 tasks: **COMPLETE**
- [ ] Integration Team: End-to-end test (Geyser → Arbitrage → Execution → Jito)
- [ ] Integration Team: Performance validation (<100ms latency)

**Friday (Day 54) - PRODUCTION DEPLOY**:
- [ ] Integration Team: Merge to `main`
- [ ] Integration Team: Deploy to production
- [ ] Orchestration: PROJECT COMPLETE 🎉

---

## 8. Team Coordination Protocols

### 8.1 Daily Standup (Async)

**Time**: 9:00 AM UTC
**Location**: Slack channel (each task has own channel)

**Template**:
```
**Daily Standup - Task 1.2 - Agent-Coder-007**

✅ Yesterday:
- Implemented parse_pool_account
- Started calculate_swap_output

🔄 Today:
- Complete calculate_swap_output
- Start build_swap_instruction

🚫 Blockers:
- None

📊 Progress: 60% complete
```

### 8.2 Blocker Escalation

**Process**:
1. **Identify blocker** (can't proceed without resolution)
2. **Document** in `task-assignments/{TASK}/BLOCKERS.md`
3. **Post** in team Slack channel
4. **Escalate** to orchestration if not resolved in 2 hours
5. **Orchestration responds** within 4 hours (SLA)

**Blocker Document Template**:
```markdown
# Blocker Report

## Task: 1.2
## Agent: Agent-Coder-007
## Date: 2025-10-02 14:30 UTC

## Description
Cannot find Raydium AMM v4 account layout documentation.

## Impact
Cannot implement parse_pool_account without knowing byte offsets.

## Tried
- Searched Solana-Arbitrage-Bot1 repo
- Searched Raydium docs
- Asked in team channel

## Resolution Needed
- Link to account layout docs
- OR sample account data with annotations

## Priority: HIGH (blocking task completion)
```

### 8.3 Integration Checkpoint Protocol

**When**: End of each phase (Fridays)
**Who**: Integration Team + Orchestration Lead
**Duration**: 4-6 hours

**Agenda**:
1. **Pull all completed tasks** from phase branch
2. **Run integration tests** (cross-crate compatibility)
3. **Resolve conflicts** (if any interface mismatches)
4. **Deploy to staging** server
5. **Run end-to-end tests** (full pipeline)
6. **Performance validation** (measure latency)
7. **Go/No-Go decision** (proceed to next phase?)

**Output**:
- Integration test report
- Performance benchmark results
- List of issues found (if any)
- Go/No-Go decision with rationale

### 8.4 PR Review Protocol

**When**: Task team creates PR to phase branch
**Who**: Orchestration team (Integration Engineer + Quality Overseer)

**Review Checklist**:
- [ ] CI passing (all checks green)
- [ ] Coverage >85%
- [ ] No clippy warnings
- [ ] No security vulnerabilities
- [ ] Documentation complete
- [ ] Success checklist signed off

**Approval**:
- 2 approvals required (Integration Engineer + Quality Overseer)
- If approved: Merge to phase branch
- If changes requested: Add comments, assign back to team

**SLA**: 24 hours from PR creation to review

---

## Summary: What You Need to Do Next

### Immediate Actions (Today):

1. **Create task packages** (use templates above):
   - [ ] Create `task-assignments/TASK_1.1_DexProtocol_Trait/`
   - [ ] Create `task-assignments/TASK_1.2_Raydium_Implementation/`
   - [ ] ... (repeat for all 14 tasks)

2. **Set up orchestration infrastructure**:
   - [ ] Create `orchestration/dashboard/` with HTML dashboard
   - [ ] Create `orchestration/integration/` with phase plans
   - [ ] Set up Slack channels (#orchestration, #task-1.1, #task-1.2, etc.)

3. **Create GitHub Actions workflows**:
   - [ ] Copy workflow templates to `.github/workflows/`
   - [ ] Set up secrets (STAGING_SSH_KEY, etc.)
   - [ ] Test one workflow (e.g., task-1.1-trait.yml)

4. **Assign orchestration team**:
   - [ ] Designate Orchestration Lead
   - [ ] Assign Integration Engineers
   - [ ] Assign DevOps Engineer
   - [ ] Assign Quality Overseer

5. **Launch Week 1**:
   - [ ] Assign Task 1.1 team (start immediately)
   - [ ] Notify all other teams (start Wednesday)
   - [ ] Set up daily standups

### This Week:

- [ ] Complete Task 1.1 (DexProtocol trait) by Wednesday
- [ ] Kick off Tasks 1.2, 1.3, 1.4, 2.1 on Wednesday
- [ ] Monitor progress daily
- [ ] Resolve blockers within 4-hour SLA

### Next Friday:

- [ ] Integration Checkpoint 1 (Phase 1 complete)
- [ ] Deploy to staging
- [ ] Go/No-Go decision for Phase 2

---

**Document Status**: Complete ✅
**Next Steps**: Create individual task packages (use templates above)
**Questions**: Contact @orchestration-lead
