Last updated: 2025-09-15

Great problem to be working on. You’re already on the right stack for sub-10ms decisioning: Geyser → Kafka → Redis + in-memory Rust services, and Jito for atomic execution. The main work now is deciding exactly what to ingest, how to normalize it, and how to turn it into a fast “edge graph” you can search and turn into bundles.

Below is an overview, the exact data you need, how to compare it, how to organize it, and a step-by-step plan to get to first profitable execution and then to scale.

High-level architecture (latency-oriented)
- Ingest: One service reads Geyser account updates; per-protocol parsers decode pool/orderbook state.
- Normalize: Convert protocol-native state into a unified “edge” model (A→B swap function with capacity, fees, and latency class).
- Hot state: Keep the minimal quoteable state in RAM (and optionally Redis for cross-process sharing). Kafka is for fan-out/replay; Redis is for hot reads if you don’t keep everything in-process.
- Opportunity engine: Builds an asset graph per slot (or rolling) and searches for profitable cycles; sizes the trade.
- Executor: Builds 1-transaction atomic routes (flash-loan + swaps + repay), simulates, then submits Jito bundles with a tip.
- Observability: Export Prometheus metrics, Grafana dashboards, and record snapshots for backtesting.

What exact information you need (by category)

1) Tokens and global
- Token mint pubkeys, decimals, token program, known canonical ATAs for your signer(s).
- A curated token allowlist to avoid honeypots/scams; include stablecoins, majors, and vetted long-tail if desired.
- Recent blockhash, current slot, leader schedule/next leader region.
- Priority-fee market conditions: expected compute unit price, typical tips for inclusion in your region.

2) AMM CPMM (e.g., Raydium v2 CPMM, Orca v1, SPL-Token-Swap style)
For each pool:
- Token A/B mint addresses and decimals.
- Vault token account balances (true on-chain reserves).
- Trade fee (liquidity provider fee), admin/protocol fee, fee denominator.
- Amp factor (if stableswap-type) or indicator of invariant type.
- Pool authority PDA and all accounts needed for the swap instruction template.
- Any special constraints (e.g., Raydium open-time, paused flag).
Derived per update:
- Instantaneous marginal price A→B and B→A.
- Edge function f(dx): output given input, including LP fee, protocol fee.
- Capacity and slippage curve for sizing.

3) CLMM/Concentrated Liquidity (e.g., Orca Whirlpools, Raydium CLMM, Crema/others)
For each pool:
- Current sqrt_price_x64, tick_current_index.
- Current active liquidity, fee rate, protocol fee.
- Tick arrays: initialized ticks with net liquidity and fee growth outside; enough arrays around current tick to cover a bounded trade size.
- Tick spacing, token decimals, mints.
- Program accounts needed for swaps (pool state, tick arrays, vaults).
Derived:
- Piecewise swap function by walking ticks (simulate crossing to compute output for dx).
- Marginal price and depth around current tick.
- Expected compute units and account metas for swap.

4) DLMM (e.g., Meteora DLMM)
- Bin list, current active bin, liquidity per bin, fee structure.
- Program-specific params for routing swaps across bins.
Derived:
- Discrete bin-walking quoter function similar to CLMM.

5) CLOBs (e.g., Phoenix, OpenBook/Serum)
For each market:
- L2 order book: price levels and sizes on both sides, plus maker/taker fee schedule.
- Market base/quote mints and lot sizes, tick size, min order size.
- Event queue or sequence/last update to ensure you’re reading a coherent snapshot.
Derived:
- Sweepable depth curve to compute cost/outcome for aggressive marketable orders.
- Edge function that integrates taker fee and partial fills across levels.

6) Flash-loan venues (e.g., Solend, Mango v4, Marginfi, Jet)
For each borrowed token:
- Available liquidity, flash-loan fee, oracle requirement (if any), required accounts for instruction.
- Instruction template to borrow and repay in the same tx.
Derived:
- Max borrow size per token and net fee impact.

7) Execution and network
- Prebuilt instruction templates for each DEX route with static accounts where possible; parameterized by amounts.
- Compute unit budgeting per route and an estimate of compute-unit price needed under load.
- Jito block engine endpoint(s), region, and target tip levels.
- Your signer(s) and pre-created ATAs for all tokens you may temporarily hold.
- Known account locks for each instruction to avoid conflicts where possible.

What to compare (core arbitrage math)
- Cross-venue two-leg arbitrage A↔B:
  - Compare A→B edge on venue 1 with B→A edge on venue 2. Compute net final A after two swaps for a tentative size dx.
  - Subtract total costs: DEX LP/protocol fees, flash-loan fee, Jito tip, priority fee (CU price × CU consumed), any rent/ATA creation (avoid at runtime).
- Triangular (A→B→C→A) within or across venues:
  - Product of edge rates > 1 after fees? Then solve for optimal size respecting capacity on each edge.
- CLOB↔AMM crossovers:
  - If AMM marginal price < CLOB best bid (or > best ask), compute optimal size limited by AMM slippage curve and CLOB depth net of taker fee.
- Sizing:
  - CPMM: closed-form or 1D search maximizing profit π(dx) = out(dx) − dx − fees − tip − priority.
  - CLMM/DLMM: simulate stepping across ticks/bins until marginal profit hits zero.
  - CLOB: walk book levels; combine with AMM curve on each side.
- Risk/latency:
  - Include probability of state change before inclusion; bid tip accordingly. Prefer private bundles to reduce exposure.

How to organize the data

Kafka topics (examples)
- geyser.accounts.raw: All updates from your validator(s).
- parsed.raydium.cpmm.state, parsed.orca.whirlpool.state, parsed.phoenix.l2, parsed.meteora.dlmm: Per-protocol decoded updates.
- quotes.edge.delta: Normalized edge updates A→B with small structs your searcher consumes.
- execution.results, execution.errors: For feedback loop and statistics.

Redis/in-memory keys
- pool:state:{protocol}:{pool_pubkey}: Minimal state needed to quote.
- edge:{A_mint}:{B_mint}:{venue_id}: Latest marginal price, capacity, fee, quoter_id, updated_slot.
- flashloan:{venue}:{mint}: max_borrow, fee_bps.
- network:recent_blockhash, network:slot, jito:tip_estimate.

In-memory graph (Rust)
- HashMap<EdgeId, Edge> where Edge holds:
  - token_in, token_out, venue_id, quoter_fn_ptr, last_updated_slot, fee model, capacity, latency class.
- For ultra-low latency, avoid Redis on the hot path; keep the entire active edge graph in one process with lock-free or RwLock-free data structures (e.g., crossbeam epoch or sharded locks). Redis is for cross-process sharing and quick warmup.

Per-slot snapshotting
- Apply Geyser updates with (slot, write_version). Publish a “graph snapshot ready” event at a slot boundary or on micro-batches (e.g., every 1–2 ms) to bound staleness.
- For Phoenix, ensure you process event queues/orderbook updates in order for consistency.

Step-by-step plan to get it working

Phase 1 — Foundation
1) Choose venues and build decoders
- Start with: Orca Whirlpools (CLMM), Raydium CPMM, Phoenix (CLOB), Meteora DLMM. Add others later.
- Implement zero-copy decoders in Rust for each program; unit test against known accounts.
- Maintain a manifest of pools/markets: pubkeys, mints, decimals, tick spacing, fee tiers, and the static accounts needed in swap instructions.

2) Normalize to edges
- Define a common Edge struct and a Quoter trait:
  - quote_out(dx) -> out, fees, capacity_used
  - max_size() -> capacity
  - accounts() -> pre-baked metas needed for swapping
- Implement quoters:
  - CPMM: closed-form formula, instant.
  - CLMM: tick-walk; keep adjacent tick arrays cached in RAM.
  - DLMM: bin-walk.
  - CLOB: walk L2 levels.

3) Build per-slot coherent state
- Geyser → parser workers → protocol-specific state stores.
- Publish normalized edge updates to the searcher only when all components needed for quoting that edge are present and from the same or later slot.
- Keep only “quote-critical” fields in the hot path; everything else goes to Postgres for history.

Phase 2 — First arbitrage
4) Start with simple two-leg cross-venue same-pair
- For each token pair with at least two edges in opposite directions:
  - Compute instantaneous rate discrepancy; if above threshold, run sizing:
    - CPMM↔CPMM: use closed-form/1D search.
    - CPMM↔CLOB: integrate AMM curve vs CLOB depth.
- Include fees:
  - LP fees, protocol fees, flash-loan fee (if borrowing), estimated priority fee, conservative Jito tip.

5) Build transaction templates
- Pre-generate instruction metas for:
  - Flash-loan begin/end for Solend/Mango/… (ensure repay calculation exact with decimals).
  - Swap instructions for each protocol; avoid dynamic account discovery in the hot path.
- Pre-create ATAs for your signer for all potential intermediate tokens you’ll ever hold.
- Pre-set a compute budget instruction (set CU limit, set CU price).

6) Fast simulate and submit
- Use Jito Block Engine simulateBundle or standard simulateTransaction on a low-latency RPC colocated in the same region; target sub-3ms RTT.
- If simulation OK and EV > min_threshold after tip/fees, submit a 1-tx bundle with an appropriate tip. Maintain multiple tip levels if contention is high.

Phase 3 — Scale up search
7) Triangular/multi-hop cycles
- Build a directed asset graph with edges as defined.
- Use log-arbitrage detection for small-size pass (product of marginal rates > 1 + epsilon), then run exact sizing on candidates.
- Restrict graph to whitelisted tokens and venues; prune edges with too little liquidity or stale updates.

8) Sizing and optimization
- Implement fast 1D profit maximization per path:
  - CPMM: analytic or Newton/binary search with monotonic profit curve.
  - CLMM/DLMM: tick/bin stepping with early exit when marginal profit ≈ 0.
  - CLOB: stop when the next level makes profit negative.
- Always include tip and CU price in the objective; they can be a material fraction at small sizes.

9) Latency engineering
- Co-locate your boxes with a Jito block engine region and a fast RPC/Geyser source.
- Pin threads, use huge pages, disable CPU scaling, and bind NIC interrupts away from your hot threads.
- Preallocate and reuse buffers; avoid heap churn in the hot path.
- Use a single-threaded tokio current_thread executor or a custom loop for the critical searcher to avoid context switches.
- Keep cryptographic signing fast: batch sign or use an optimized Ed25519 library. Prebuild all instruction data; only fill in amounts and blockhash.

10) Robustness
- Handle write conflicts: if simulation shows account in-use or state changed, either:
  - Re-quote with latest state and rebuild, or
  - Increase tip and resubmit quickly if EV still positive.
- Keep a rolling window of last N slots to assess variance and choose tip levels.
- Implement circuit breakers on EV, failure rate, and per-venue anomalies.

Exact formulas and sizing essentials

CPMM quote (ignoring protocol fee split, include it in fee):
- Given reserves X (in), Y (out), fee_bps:
  - dx_eff = dx * (1 - fee)
  - out = (Y * dx_eff) / (X + dx_eff)
- Profit across two pools is out_2 − dx − fees − tip − priority.
- Solve for dx maximizing profit. If you don’t want calculus, a fast monotonic binary search on dx ∈ [0, capacity] works.

CLMM/DLMM
- Walk ticks/bins:
  - For each segment with liquidity L and price P, compute how much dx you can trade before crossing the next boundary; accumulate out and fees; stop when profit turns negative or you run out of liquidity.
- Cache neighboring tick arrays in memory and keep them warm.

CLOB
- Walk asks for buy or bids for sell, summing size×price and taker fee. Stop when the next level would flip profit negative when combined with the other leg.

Execution bundling
- Prefer a single transaction:
  - [ComputeBudget, FlashLoanBegin, Swap1, Swap2, …, FlashLoanRepay]
- If a venue requires multiple tx, use Jito bundles to enforce order and add sufficient tip; but multi-tx increases failure risk.
- Keep compute units within safe limits; many CLMM crossings can be heavy. If CU is too high, reduce size or choose another path.

Organizing for “any viable opportunity” without preselecting pairs
- Maintain a dynamic asset graph:
  - Nodes: whitelisted tokens.
  - Edges: every live market/pool you ingest.
- Build and update a best-edge index:
  - For each ordered pair (A, B), track the top K edges by marginal rate and capacity.
  - Only generate path candidates using these K for speed.
- Heuristics:
  - Ignore edges with capacity < X or last_update older than Y ms.
  - Ignore tokens with suspicious mint authorities or nonstandard decimals unless explicitly allowed.

Observability and control
- Metrics:
  - End-to-end latency (update→opportunity→bundle submit), EV per bundle, realized PnL, hit rate, tip percentiles, CU used.
  - Per-venue stale rate and decode failures.
- Dashboards: Top profitable edges, current spreads, and recent misses (profitable sim that failed to land).
- Postgres: store normalized snapshots for replay/backtest. Use Kafka offsets to replay the exact slot stream into the engine.

Practical tips
- Precompute and store per-venue account metas as byte arrays; avoid Vec allocations on the hot path.
- Pre-create and fund ATAs for all tokens you might temporarily hold; keep dust management off the hot path.
- Keep a conservative fallback tip schedule when contention spikes (e.g., during memecoin launches).
- Use private Jito bundles to avoid being copied/sandwiched.

Initial milestone checklist
- Decode and normalize Raydium CPMM + Orca Whirlpools + Phoenix.
- In-memory edge graph with real-time quoters.
- Two-leg arbitrage detector with sizing and EV calculation including tip/CU.
- Transaction template library; flash-loan route working on devnet/test pools; simulation green.
- Jito bundle submission end-to-end with a trivial profitable case on mainnet (small size).
- Metrics and replay harness.

Once this is solid, expand to triangular and DLMM, add more flash-loan sources, and iterate on latency and tip strategies.

If you want, describe the exact venues you plan to start with (e.g., Orca Whirlpools + Phoenix), and I can outline the concrete account fields and swap instruction metas you’ll need for those first, including a minimal Rust quoter function sketch.