# Re-Architecture 5 – Modular Stack with External Strategy Integration
Created: 2025-10-02

## Context
Incorporates lessons from `Solana-Arbitrage-Bot1`, `solana-defi`, and the curated material in `solana-awesome` while preserving the safety-first, service-oriented architecture defined in Re-Architecture 4.

## Updated System Overview
```
┌──────────────┐   ┌────────────────┐   ┌────────────────────┐   ┌────────────────────┐
│ Data Feeds   │──▶│ Ingestion Hub  │──▶│ Normalized Event Bus│──▶│ Strategy Runtime   │
└──────────────┘   └────────────────┘   └────────────────────┘   └───────┬────────────┘
                                                                       Signals │
                                                                               ▼
                                                      ┌────────────────────┐  Advisories ┌───────────────┐
                                                      │ Risk & Policy Gate │◀────────────│ AI Orchestrator│
                                                      └──────────┬─────────┘             └───────────────┘
                                                             Orders │
                                                                     ▼
                                        ┌──────────────────┐   ┌──────────────────┐
                                        │ Execution Router │──▶│ Venue Adapters   │
                                        └────────┬─────────┘   └──────┬──────────┘
                                                 │ Fills            │
                                                 ▼                  ▼
                                   ┌────────────────────────┐ ┌─────────────────────┐
                                   │ Portfolio Authority    │ │ Event & Metrics Bus │
                                   └──────────┬─────────────┘ └──────────┬─────────┘
                                              │                           │
                                              ▼                           ▼
                  ┌──────────────────────────┐   ┌────────────────────────┐   ┌────────────────────────┐
                  │ Community Strategy Sandbox│   │ Feature Store (Live)   │   │ Ops Control Plane      │
                  └──────────────────────────┘   └────────────────────────┘   └────────────────────────┘
```

### New Elements
- **Community Strategy Sandbox** – isolated runtime that can execute third-party strategy plugins (e.g., triangle or multi-DEX paths from Solana-Arbitrage-Bot1) behind strict capital and latency limits. Approved signals are revalidated by the main Strategy Runtime before entering risk checks.
- **Knowledge Hub** – the control plane serves curated references (lifted from `solana-awesome`) alongside runbooks so operators have protocol and API context without leaving the console.
- **Shyft/Helius Adapter Pack** – ingestion hub ships optional connectors inspired by `solana-defi` examples, reimplemented with typed Rust clients and signed request support.

## Extended Interface Contracts
| Layer / Module | Trait / Schema | Notes |
|----------------|----------------|-------|
| Venue Integration | `trait DexIntegration` (adopted) | `async fn find_opportunities(state) -> Vec<DexOpportunity>`; bridges strategy plugins with venue metadata. |
| Strategy Runtime | `trait StrategyPlugin` | Adds `fn categories()` to tag plugins (two-hop, triangle, sniping, etc.) for operator toggles. |
| Community Sandbox | `trait SandboxHarness` | `fn validate(plugin_manifest)`; `async fn execute(inputs) -> Vec<SandboxSignal>`; results converted into `TradeSignal` after sanitization. |
| Risk & Policy | `trait RiskAdvisor` | Gains `fn source()` field to record whether a signal originated internally or from the sandbox/community feed. |

## Workspace Adjustments
```
crates/
├── data-ingest/
│   └── connectors/{geyser,rpc,shyft,helius}.rs
├── strategy/
│   ├── core/
│   ├── plugins/{arb_two_hop,arb_triangle,arb_multi_dex,sniping,copy_trade}/
│   └── sandbox/
│       ├── manifest.rs
│       ├── sandbox_host.rs
│       └── wasm_runtime.rs (optional)
├── community/
│   ├── loader/            # Parses third-party manifests, enforces policy
│   └── vetting-tools/     # Static analyzers, lint checks for external code
└── ops/
    └── knowledge-hub/     # Serves curated resource cards in control plane
```

## Integration Flow for External Strategies
1. **Import & Vetting** – operator uploads or references a third-party manifest (e.g., from Solana-Arbitrage-Bot1). Vetting tools check licensing, dependencies, and banned API usage.
2. **Sandbox Execution** – the community sandbox runs the plugin in a constrained VM (optional WASM) with simulated capital. Generated signals are labeled `source=community`.
3. **Strategy Runtime Validation** – signals pass through deterministic checks (profitability bounds, available liquidity checks, Jestr event correlation) before entering the main bus.
4. **Risk Gate** – denies any signal exceeding sandbox quotas or missing provenance metadata.
5. **Portfolio Enforcement** – sandbox-origin orders can only use segregated “experimental” wallet tiers managed by the Portfolio Authority.

## Operational Considerations
- **Telemetry Tags** – metrics include `signal_source` (core vs community), `strategy_category`, and `dex_integration` to track third-party performance.
- **Kill Switches** – ops console exposes per-plugin toggles and global sandbox disable; defaults to “off” in production unless explicitly whitelisted.
- **Documentation Surfacing** – knowledge hub cross-links each active venue adapter to the relevant entries in `solana-awesome` so operators can review protocol quirks quickly.

## Next Steps
- Implement `DexIntegration` and `StrategyPlugin` traits across existing internal strategies.
- Build the sandbox harness with initial support for Rust CDylib plugins; add unit tests comparing community outputs against baseline safety thresholds.
- Populate the knowledge hub with curated links (RPC tuning, DEX runbooks, MEV execution guides) and link it to control-plane help panels.

## Attribution
Authored by: Codex (GPT-5)

## Reusable Artifacts Harvested
- **Strategy & DEX Traits**: adopt the method signatures from `Solana-Arbitrage-Bot1/solana-mev/strategies/strategy.rs` and `.../dex/dex_integration.rs` so third-party crates can plug into our runtime with minimal shims. Document these in `crates/strategy/core` and `crates/execution/router` as the canonical interfaces.
- **Venue Parsers**: use the Orca pool parsing flow (`solana-mev/dex/orca.rs`) as a template when we implement our Rust adapters—decode program accounts via bincode, derive associated token accounts, and enrich with decimal metadata. Capture these steps in a shared `venue-parser` module.
- **Strategy Patterns**: seed built-in plugin configs (two-hop, triangle, multi-DEX) with the thresholds and sequencing outlined in `Solana-Arbitrage-Bot1/README.md` (0.5% min profit, 1% slippage cap, DEX precedence).
- **External API References**: translate the GraphQL/WebSocket usage from `solana-defi/Bots/Typescript` into typed integration tests for the Shyft adapter, ensuring our connectors honor required headers, session hashes, and rate limits.

## Implementation Checklist Addendum
1. Publish the standardized `StrategyPlugin` and `DexIntegration` traits in `crates/strategy/core` and `crates/execution/router`.
2. Build `venue-parser` helpers for Orca, Raydium, and Meteora based on the reference implementations; include unit tests that deserialize recorded account snapshots.
3. Author default configuration bundles (`configs/default/triangle.toml`, etc.) populated with the documented profit/slippage heuristics.
4. Create Shyft/Jupiter adapter smoke tests that replay the GraphQL queries from `solana-defi`, validating our ingestion stack before enabling community strategies.

Completed: 2025-10-02 – Codex (GPT-5)
