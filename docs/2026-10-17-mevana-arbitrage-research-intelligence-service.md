# MEVana Arbitrage Research & Intelligence Service
**Status:** Research / architecture proposal (not an implementation authorization)  
**Date:** 2026-10-17  
**Project:** MEVana  
**Purpose:** Consolidated research from the 2026-10-17 conversation, including restored original sections 1–4 and subsequent sections 5–15.

> **Recovery note:** The initial answer's sections 1–4 remained available in the conversation context when this document was written. Their substance is reproduced and expanded here. The latter sections 5–15 were consolidated and edited for technical consistency. All performance, profitability, and probability numbers in examples are hypothetical and are **not** measured MEVana results.

## Executive summary

Build an **independent research and opportunity-discovery service** that learns **where, why, and when** profitable arbitrage may emerge, without interfering with the execution-critical trading loop. It consumes normalized market data, identifies candidate routes and structural market relationships, tests them historically and in shadow mode, ranks watchlists, and publishes short-lived hints to the core engine. The trading engine remains solely responsible for live validation, risk checks, transaction construction, submission and execution.

Three research capabilities:
1. **Discovery:** find pools, routes and market relationships.
2. **Prediction:** forecast conditions under which dislocations emerge.
3. **Optimization:** allocate finite feed, CPU, memory and execution-attention budgets toward high-value opportunities.

Use a **two-speed research design**: long-horizon experimentation (hours to weeks) plus short-horizon intelligence (slots to seconds). Neither should block the real-time path.

---

## 1. High-level architecture (restored)

\`\`\`text
 Solana Yellowstone/SHYFT | DEX/CEX | bridge | oracle | history
                          |
              normalization / replay log
                          |
                       Kafka
                          |
       +------------------+------------------+
       |                                     |
       v                                     v
 Independent Arbitrage Research          Core Trading
 - graph universe                        - hot market state
 - structural signals                    - exact route quote
 - statistics / prediction               - feasibility/risk
 - simulation / validation               - transaction build
 - watchlist scoring                     - submission/inclusion
       |                                     |
       +---- WATCH / TRIGGER hypotheses ---->|
       ^                                     |
       +--------- execution feedback --------+
\`\`\`

Research is **advisory**: never authorizes a trade and must not form a synchronous dependency of the core execution path. Its failure cannot stop normal execution. It can run on a separate server and use heavier algorithms and longer-lived storage.

Distinct outputs:
- **WATCH:** a pool/route/relationship deserving monitoring (minutes to days, revocable).
- **TRIGGER:** a newly observed condition that may justify immediate reprioritization (slots or seconds, strict TTL).
- **RESEARCH_RESULT:** historical validation / experiment results, not a live trading instruction.

## 2. Arbitrage family taxonomy (restored)

| Family | Mechanism | Initial priority | Key caveat |
|---|---|---|---|
| Cross-DEX same-asset | Buy on one pool, sell on another | Very high | Depth and fees |
| Triangular cycle | A → B → C → A | Very high | Price impact across hops |
| Multi-hop cycle | Four to six or more hops | Very high | Account, compute, size constraints |
| CLMM/DLMM | Different liquidity at ticks/bins | Very high | Correct tick/bin and dynamic-fee modeling |
| Predictive flow | Trade flow likely to create dislocation | High | Beware stale feeds/data leakage |
| CEX–DEX | Centralized orderbook vs Solana pool | High | Custody and non-atomic legs |
| Liquid staking | LST market price vs redeemable SOL value | High | Unstaking delays/risks |
| Stablecoin | Temporary stable asset deviations | High | Depeg/issuer risks |
| Cross-chain | Same asset across chains | Medium | Prefunded inventories, rebalancing |
| Funding/spot-perp | Funding and basis capture | Medium | Hedge and liquidation risks |
| Liquidation-driven | Market dislocations around liquidations | Medium | Venue-specific mechanics |
| New pool / migration | Launch-time mispricing | Medium | Adverse selection, toxic tokens |

**Initial research focus:** cross-DEX cycles, 3–6-hop cycles, CLMM/DLMM cliffs, and price/flow lead-lag. Later extend to inventory-backed cross-chain and funding strategies.

## 3. Mathematical toolbox (restored)

### 3A. Weighted directed multigraph and negative cycles

- **Nodes:** token mints (plus chain, venue and custody domain where appropriate).
- **Edges:** executable swaps, including distinct parallel pool edges for the same token pair.
- **Edge metadata:** protocol, pool, direction, rate by size, account set, liquidity, fee, slot, staleness and risk.

For a *small fixed trade size*, let \`r_ij\` be the effective output/input rate of edge \`i→j\` (after LP fees, before any costs not incorporated into the rate). Define:

\`\`\`math
w_{ij} = -\ln(r_{ij})
\`\`\`

A cycle with \`Σ w_e < 0\` has \`Π r_e > 1\`, a **theoretical** gain at the rates assumed. That is **not** proof of executable arbitrage because every edge is size-dependent and costs, freshness, state consistency, and feasibility matter.

Algorithms:
- Strongly connected components to prune regions without cycles.
- Bellman–Ford for negative-cycle detection in a fixed-weight approximation.
- Johnson-style elementary cycle enumeration, bounded by hop count and budget.
- K-shortest/alternative paths for targeted route expansion.
- Incremental dependency indexes: recompute only cycles touched by updated pools.
- Search with profitability upper bounds, deduplication and token/pool allowlists.

Avoid exhaustive enumeration of all simple cycles at production scale.

### 3B. Exact quote composition and optimal sizing

For a constant-product pool with input reserve \`X\`, output reserve \`Y\`, input \`x\` and fractional LP fee \`f\`:

\`\`\`math
y(x) = \frac{Y(1-f)x}{X+(1-f)x}
\`\`\`

For a cyclic route \`C\`, compose protocol-specific quote functions:

\`\`\`math
F_C(x)=f_n(f_{n-1}(\dots f_1(x)))
\`\`\`

Then optimize total net return:

\`\`\`math
\Pi(x)=F_C(x)-x-C_{\mathrm{execution}}(x)-C_{\mathrm{risk}}(x)
\`\`\`

\`\`\`math
x^*=\arg\max_{0\le x\le x_{\max}}\Pi(x)
\`\`\`

Use numerical bounded search/Brent where suitable. For CLMM/DLMM, use exact tick/bin state and dynamic fees; optimization may be piecewise/non-smooth and requires interval or breakpoint-aware search. **Final trade quotes require protocol-correct integer arithmetic, slot-consistent state and live simulation.** Risk costs must be defined in the same input-token units, without double counting.

### 3C. Statistical spread, z-score and mean reversion

For markets A and B pricing comparable exposures:

\`\`\`math
S_t=\ln(P_{A,t})-\ln(P_{B,t}),\quad Z_t=(S_t-\mu_t)/\sigma_t
\`\`\`

Measure deviation frequency, reversion half-life, fill probability, fee-adjusted edge and observed opportunity duration. For LSTs or otherwise nonidentical exposures, adjust for the economically justified basis before interpreting deviations. Z-scores **screen** but do not establish actionable profit; regime changes and stale quotes create false signals.

### 3D. Predictive order flow

A simple flow feature:

\`\`\`math
OFI_t=(V_{\mathrm{buy}}-V_{\mathrm{sell}})/(V_{\mathrm{buy}}+V_{\mathrm{sell}})
\`\`\`

Model:

\`\`\`math
P(\text{net-profitable opportunity within }\Delta t\mid X_t)
\`\`\`

Inputs: relative swap size, reserve/tick/bin changes, liquidity concentration, directional flow, stale quote time, slot activity, external reference return and volatility. Baseline logistic regression and gradient-boosted trees before expensive sequence/deep models. Validate time alignment and avoid lookahead.

## 4. Dynamic liquidity graph (restored)

A static routing graph discovers relationships; **a time-varying, stateful multigraph** ranks them as market state changes.

Per directed edge maintain:
- token mints, pool ID and protocol version;
- estimated exchange rates and size-specific quote curve;
- liquidity/depth and tick/bin availability;
- LP fee and volatility-dependent fees;
- latest observed slot, stream timestamp and confidence in freshness;
- compute estimate, writable account dependencies, account count;
- historical opportunity rate and empirical execution success.

**Incremental reaction to a pool update:**
1. Decode and validate changed state.
2. Find graph edges backed by that pool.
3. Look up routes affected by those edges.
4. Re-estimate cheap profitability bounds and liquidity capacity.
5. Run exact pricing only for survivors.
6. Promote WATCH/TRIGGER hypotheses with TTL and state-slot references.

A dormant route can remain on a low-priority watchlist when prior evidence says a particular flow or liquidity change frequently activates it.

---

## 5. Structural / advanced strategies

### 5.1 Liquidity imbalance forecasting

Predict which pool combinations will diverge under comparable trade flow. In constant-product AMMs, an *approximate marginal spot-price movement* for input \`x\` is:

\`\`\`math
I(x)=1-\left(\frac{X}{X+(1-f)x}\right)^2
\`\`\`

This is **not** the trader's average slippage. Model direction, actual reserves and swap fees; use protocol-specific models in production. Compare impact across pools and flag high-sensitivity combinations.

### 5.2 CLMM tick / DLMM bin liquidity cliffs

Observe proximity to tick/bin boundaries, liquidity immediately beyond the boundary, depth asymmetry and dynamic fees. A rough screening feature:

\`\`\`math
B=L_{\mathrm{current-range}}/(L_{\mathrm{next-range}}+\epsilon)
\`\`\`

High \`B\` may indicate a depth cliff for a relevant trade direction, **not** an independently profitable trade. Add distance-to-boundary and forecast incoming flow.

### 5.3 Lead-lag between venues

\`\`\`math
\rho_{AB}(\tau)=\operatorname{Corr}(r_A(t),r_B(t+\tau))
\`\`\`

A positive useful lag may suggest A leads B, but must survive clock/feed synchronization, out-of-sample testing, transaction costs, and data-latency corrections. Use known observation timestamps and state slots.

### 5.4 Large-swap impact prediction

Research whether swaps of size \`x/depth\` on a particular pool predict temporary mispricing against competitors. Features: relative trade size, direction, liquidity elsewhere, volatility, correction history and competing solver activity. Report measured conditional probability, expected net payoff, and lifetime with confidence intervals. Do not treat hypothetical percentages as actual results.

### 5.5 Correlated assets / redemption values

For SOL and LST:

\`\`\`math
D=P_{\mathrm{LST}}-R_tP_{\mathrm{SOL}}
\`\`\`

\`R_t\` is current claimable SOL per staking token, adjusted for protocol rules. Include conversion/redemption availability, unstaking wait, liquidity, basis, oracle and peg risk. Generally non-atomic.

### 5.6 Cross-chain prefunded inventory arbitrage

Prefer holding inventory on both chains and trading independently when eligible. Bridging is a **subsequent rebalancing activity**, not a guaranteed atomic settlement mechanism:

\`\`\`math
\Pi_{\mathrm{cycle}}=\Pi_{\mathrm{trade}}-C_{\mathrm{rebalance}}-C_{\mathrm{inventory-risk}}
\`\`\`

Model uncertain inclusion, two-leg failure, chain finality, custody, gas, bridge fees and inventory cost. Inventory limits and circuit breakers are mandatory.

### 5.7 CLOB–AMM

Compare **executable depth-weighted** orderbook prices against the complete AMM quote curve; simple midprice comparisons are insufficient. Account for queue position, adverse selection, partial fills, maker/taker fees and non-atomic venue combinations.

### 5.8 Additional candidates

Include spot-perp basis/funding capture, stablecoin temporary dislocations, lending/liquidation effects, launch/migration routes, and cross-venue correlation anomalies. Each needs a strategy-specific risk and replay model.

---

## 6. Research experimentation lifecycle

\`\`\`text
Candidate discovery
  -> historical point-in-time replay
  -> shadow/paper execution with realistic latency
  -> evidence-based watchlist promotion
  -> live execution feedback
  -> continuous recalibration, demotion or retirement
\`\`\`

Persist *negative* evidence as well as wins. Backtests must only use data available at decision time; preserve ingestion timestamp separately from on-chain event/slot timestamp to quantify information delay. Use chronological train/validation/test splits and market-regime holdouts.

## 7. Opportunity scores

Separate:
- **Research confidence:** evidence that a relationship tends to produce executable net opportunity.
- **Execution priority:** the current marginal benefit of monitoring/quoting it.

Expected value for opportunity \`i\` in a common unit:

\`\`\`math
EV_i=p_iG_i-(1-p_i)L_i-C_i
\`\`\`

\`G_i\`: success payoff before shared costs; \`L_i\`: cost of unsuccessful outcome including failed-leg exposure; \`C_i\`: costs not already reflected in either payoff. Prevent double counting and calibrate \`p_i\`.

Risk-adjusted utility:

\`\`\`math
U_i=EV_i-\lambda R_i-\mu B_i
\`\`\`

\`R_i\`: relevant risk estimate; \`B_i\`: monitoring/compute/data budget burden. Use uncertainty bands, capital cost, expected lifetime, exposure, sample size and protocol risk as additional filters. Production priorities should be derived from measured values, not an arbitrary single percentage.

## 8. Research-to-trading event contract

WATCH and TRIGGER should be versioned, idempotent, include **source slot**, as-of time, TTL, feature/model version, strategy ID, pool+mint IDs, route edges, expected size region, evidence/calibration fields and monitoring priority.

Illustrative JSON:

\`\`\`json
{
  "schema_version": 1,
  "hypothesis_id": "hyp-10481",
  "kind": "WATCH",
  "strategy": "CYCLIC_ARBITRAGE",
  "state": "ACTIVE",
  "route": {
    "tokens": ["USDC", "SOL", "TOKEN_X", "USDC"],
    "pool_ids": ["pool_a", "pool_b", "pool_c"]
  },
  "trigger": {"type": "PRICE_DIVERGENCE", "minimum_edge_bps": 35},
  "research": {
    "sample_count": 420,
    "confidence_estimate": 0.82,
    "historical_candidates_per_day": 7.4
  },
  "observed_slot": 0,
  "suggested_input_usdc": 2500,
  "priority": 90,
  "expires_at": "2026-10-17T12:00:00Z"
}
\`\`\`

**Illustration only:** addresses, slots and statistics are placeholders. Distinguish confidence estimates from calibrated executable-success probability.

The core verifies current state, protocol correctness, exact trade size, slippage, token restrictions, compute and account constraints, risk, projected profit, fees, and transaction inclusion. Research never directly submits a trade.

## 9. Data architecture and persistence

Proposed architecture aligned with existing MEVana directions (verify actual current repo before implementation):

- Yellowstone/SHYFT account/transaction/slot feeds, plus independent CEX/oracle/bridge feeds where needed.
- Shared normalization, deduplication and pool decoding.
- Kafka topics for normalized updates, watchlists, triggers and trade outcomes.
- Parquet on object storage for immutable replay datasets.
- PostgreSQL for hypotheses, experiments, model versions and outcomes.
- Redis for hot candidate ranks / ephemeral caches.
- Python (NumPy, SciPy, Polars, scikit-learn, LightGBM) for offline research; Rust for latency-relevant graph operations if benchmarks justify it.
- Prometheus/Grafana for system and economic telemetry.

Historical events should record: pool states, reserve/liquidity/tick/bin changes, reference prices, observable swaps, candidates *not* taken, simulated quotes, observed slot, local receive timestamp, fees, modeled inclusion time, realized fills and failures. Historical reconstructions must respect the **information available then**.

Avoid adding a graph database at MVP scale unless benchmarks establish a need.

## 10. Monitoring budget optimization

Not every pool and route deserves equal scrutiny.

\`\`\`math
\max\sum_i v_i x_i,\quad \text{subject to }\sum_i c_i x_i\le B,\quad x_i\in\{0,1\}
\`\`\`

This knapsack approximation ranks value \`v_i\` against monitoring cost \`c_i\`. Extend it to account for shared data subscriptions and route overlap (joint costs), and allocate three tiers:
1. Broad low-cost discovery universe.
2. Medium-frequency route watchlist.
3. High-frequency, high-priority exact quote set.

Complex routes must pass preliminary Solana transaction size, account, compute-budget and atomicity feasibility checks. Current limits should be verified against applicable network/runtime configuration.

## 11. Exploration vs exploitation

Use techniques such as Thompson Sampling or UCB for **research attention allocation**, exploring uncertain route families while exploiting established performers. Do not use bandits to relax exposure, signing or execution authorization gates. Budget experimentation explicitly and account for change in market regime.

## 12. Incremental implementation plan

**Phase 1 – Universe builder:** supported token mint registry, protocol/pool discovery, directed multigraph, bounded cycles, edge freshness, cheap viability filters. Output: ranked route universe.

**Phase 2 – Historical opportunity analyzer:** event-time datasets, point-in-time replay, quote composition, cost/failure simulation, outcome statistics. Output: defensible profitable-route evidence.

**Phase 3 – Predictive research:** depth-cliff detectors, OFI and lag features, calibrated logistic/boosted models, baseline comparisons. Output: forward-looking watchlist ranking.

**Phase 4 – Trading integration:** WATCH/TRIGGER Kafka contracts, TTL/revocation, core-side independent validation, shadow A/B experiments, execution feedback. Output: measurable improvement vs existing scanning.

**Phase 5 – Expanded strategies:** cross-chain prefunded inventory, CEX–DEX, funding/basis, exploration policy and resource optimization. Output: extensible multi-strategy platform.

Do **not** rewrite core trading as the first step. First inspect repository reality and identify reusable graph, parser, historical, metrics and event contracts.

## 13. Evaluation metrics / experiment design

Measure:
- Candidate and executable-net-profitable opportunities discovered per day.
- Estimated *and realized* net profit per day, after all attributable costs.
- Shadow/counterfactual opportunity capture and live capture rate.
- Precision/recall on opportunities labeled with realistic executable criteria.
- Median and p95 lifetime; route-staleness and feed latency.
- Quote-to-execution error and failed inclusion rate.
- CPU, memory, RPC/subscription cost and **profit per monitored pool**.
- Missed opportunities and false-positive route promotions.
- Probability calibration error and performance across market regimes.

Run paired/time-aligned **A/B or shadow** comparisons of current core prioritization and research-guided watchlists; hold comparable market windows, feed access and compute constraints constant. No meaningful lift means the complexity has not justified itself.

## 14. Proposed service boundary: MAIS

**MEVana Arbitrage Intelligence Service (MAIS)**, independently deployable and restartable, with read-only market consumption and advisory event publication.

\`\`\`text
mevana-research/
  ingestion/
  market_universe/
  protocol_models/
  graph/
  strategies/
    cyclic/
    liquidity_cliffs/
    lead_lag/
    statistical/
    cross_chain/
  simulation/
  scoring/
  watchlists/
  feedback/
  storage/
  api/
  tests/
\`\`\`

This is a **suggested** folder structure, not an assertion about existing code. Deploy where spare capacity and dependable data access exist; keep production execution on infrastructure justified by measured latency and reliability (e.g., Vultr vs Hetzner benchmarking already being discussed). Avoid duplicative costly feed subscriptions when a shared normalized feed is reliable and accessible.

## 15. Concrete next step and acceptance criteria

Commission a **repo-grounded discovery/design slice**, without changing trading behavior:

1. Inventory the existing protocol decoders, routers, price/math libraries, live feed topology and Kafka/Redis/PostgreSQL usage.
2. Identify reusable graph and replay machinery; measure missing history.
3. Specify versioned WATCH/TRIGGER and feedback schemas plus idempotency, TTL and source slot behavior.
4. Implement a tiny proof of concept on two or three real supported DEX protocols, with point-in-time replay and at least one exact cyclic quote.
5. Compare its measured false-positive rate and costs with the current route-scanning baseline.
6. Document findings, tests, deployment isolation and go/no-go criteria.

**Acceptance:** no production execution changes, no signing keys in research, deterministic historical fixtures, reproducible simulation results, measurable performance/cost budget, and explicit handling of missed/out-of-order/stale updates.

## Additional research and safety notes

- Atomic Solana route execution is constrained by transaction/account/compute limits; cross-chain/CEX trades are not atomic.
- Latency, failed transactions, dynamic pool fees, priority fees, rent/account preparation, and inventory rebalance can erase paper spreads.
- Avoid tokens with unknown or dangerous transfer, freeze, token extension, tax, or redemption behavior unless explicitly supported and risk-approved.
- Model uncertainty and opportunity **decay**; classify a WATCH differently from an executable current quote.
- Any provider, protocol API and implementation-specific constraint should be reverified against authoritative documentation during the implementation slice.

## Relevant primary documentation to verify during implementation

- Solana transaction and compute documentation: https://solana.com/docs/core/transactions and https://solana.com/docs/core/fees
- Yellowstone gRPC: https://github.com/rpcpool/yellowstone-grpc
- SHYFT: https://docs.shyft.to/
- Orca developer docs: https://dev.orca.so/
- Raydium docs: https://docs.raydium.io/
- Meteora docs: https://docs.meteora.ag/
- Wormhole docs: https://wormhole.com/docs/
- Circle CCTP docs: https://developers.circle.com/cctp

These URLs are starting points, not endorsements of any exact provider plan, feature or API revision.
