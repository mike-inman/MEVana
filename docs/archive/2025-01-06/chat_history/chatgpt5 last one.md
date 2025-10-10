Last updated: 2025-09-15


Short version
- You’re close. With what you have plus the “whirlpool-core” pieces, you’re 3 components away from first real fills: an exact quoter, a simple searcher, and an executor that submits Jito bundles.
- Sub-10 ms “signal → bundle submitted” inside your box is achievable if you keep the hot path in one process and prebuild everything.
- Landed-trade odds at the start are modest (5–15% of simulated-positive opportunities). With tuning (tips, CU price, retries) you can reach 30–50% landed on clean two-leg arbs.
- Trades per minute depend on market regime and coverage. Expect:
  - Quiet: <1/min
  - Normal: 1–5/min with 6–10 venues
  - Spiky/memecoin hours: 10–40/min if you include long tail and CLOB crossovers
These are “landed” ranges; you’ll simulate/submit more than you land.

How far you are (from your current state)
- Decoding pools and tick arrays: basically done.
- Next 3 deliverables to reach first fills:
  1) Exact CLMM quoter
     - Integer Q64.64 math; walk multiple ticks/boundaries; no floats.
     - Unit-test against Orca SDK outputs.
  2) Minimal searcher
     - Build two-leg candidates (same pair across two venues).
     - Use quote_out on both legs, include LP+protocol fees, flash-loan fee, tip, CU price.
     - 1D size search (binary/Newton) on dx to maximize EV.
  3) Executor + Jito
     - Prebaked swap instruction metas per venue; flash-loan begin/repay.
     - ComputeBudget instruction (CU limit + price).
     - Simulate transaction; submit Jito bundle with adaptive tip; retry if stale.

If you focus, a first profitable live fill is 3–7 focused days away; 1–2 weeks is more realistic including retries, tip tuning, and observability.

Can you actually hit sub-10 ms?
- Yes, for “decision → bundle submit,” if you do:
  - Keep the entire edge graph in one process, in memory, no Redis in the hot path.
  - Candidate gen + sizing: ≤2 ms for up to a few hundred candidates per micro-batch.
  - Tx build: precomputed metas; only set amounts + blockhash (≤1 ms).
  - Simulation: colocate with a low-latency RPC or Jito simulateBundle API (2–3 ms). If you skip simulation you can be <5 ms, but risk rises.
  - Submit to Jito Block Engine in your region: 1–2 ms.
- Total attainable: 6–10 ms median internal, assuming careful engineering (pinned threads, preallocation, no logging/allocation on hot path).

What limits how many trades you land
- Competition: Other searchers will see the same spreads. Landing = latency + tip + clean instructions (no account locks).
- Tips and CU price: Under-tipping means getting cut. Over-tipping erodes EV. You’ll need an adaptive tip schedule (percentiles by time-of-day and leader).
- State churn: Price can change before your bundle lands. Your sim-positive rate might be 3–5× your landed rate early on.
- Compute: CLMM routes that cross many ticks cost more CU; keep CU within safe headroom to avoid throttling.
- Venue coverage: More edges → more opportunities, but only if you keep quoters hot and prune intelligently.

Ballpark landed trades per minute (after you scale)
- Two-leg, same-pair, across a handful of venues (Orca/Raydium CLMM/CPMM + 1 CLOB):
  - Quiet hours: 0–1/min
  - Normal: 1–5/min
  - Spiky: 5–15/min
- If you add long-tail tokens, DLMMs, and CLOB/AMM crossovers and accept smaller EVs:
  - Quiet: ~1/min
  - Normal: 3–10/min
  - Spiky: 10–40/min
Above assumes you’re comfortable firing multiple bundles per minute and paying variable tips. Your submission rate will be higher; landed rate improves as you tune.

What drives your landed odds from 5–15% → 30–50%
- Tip strategy: Track inclusion vs. tip percentile by leader. Keep 2–3 tip levels; re-bundle quickly at +Δtip if sim still positive.
- Freshness gating: Trade only if both legs updated within last X ms and Δslot ≤ 1. Cuts sims that are likely to flip.
- Pre-created ATAs for all intermediates; avoid rent/creation on hot path.
- Instruction conflicts: Prefer venues/pools with low lock contention at that moment; have alternate path candidates ready.
- Private bundles, not public mempool.
- Reduce legs: Inventory tiny balances of common tokens to skip flash-loan legs for some paths, shaving CU/time.

Minimal milestone plan from here
- Day 1–2: Exact CLMM quoter (multi-tick) + unit tests; expose quote_out(dx) + size_to_next hints.
- Day 2–3: CPMM module (reserves decode + quoter). Now you can do simple AMM↔AMM arbs.
- Day 3–4: Edge assembler emits EdgeDeltas; searcher runs two-leg detector with sizing and EV including tip/CU.
- Day 4–5: Executor: build tx, simulate, Jito bundle submit. Precompute metas; flash-loan path wired.
- Day 5–7: Tip tuner, retries, observability (EV before/after tip, landed rate, latency histograms). First small live fills.

Practical knobs that matter for sub-10 ms
- Co-location: Same region as your Geyser source and your Jito engine.
- CPU: Pin the search thread, disable turbo scaling, huge pages, jemalloc tuned for low latency.
- Data: No Strings or heap work in hot structs; use fixed-size pubkeys and integers.
- Batching: Ingest deltas continuously, but detect/signal in micro-batches (e.g., every 1–2 ms) to avoid thrash.

Reality check
- A 10 ms engine is necessary but not sufficient. Landing arbs on Solana is a tip/latency/coverage game. Expect to iterate on tips and candidate pruning to get a steady landed flow.
- You’ll likely see your first landed trades with modest EV and higher tips. As you add venues and polish sizing, EV improves and tips fall as a % of PnL.

When you’re back, we can:
- Drop in the exact multi-tick CLMM quoter.
- Add a CPMM module (Raydium/SPL Token Swap) so you can do your first AMM↔AMM arbs.
- Define EdgeDelta and the searcher’s candidate loop, plus a simple EV-maximizing size solver.