# Design — 2026-10-30 CDT

## Context

See `proposal.md`. Handoff 09 provides four finalized raw capture files; handoff 10 provides standard-fee exact quotes. Revision 1 of handoff 11 defines three separately scoped v1 requests. The existing request/result validators remain unchanged.

## Goals / Non-Goals

**Goals:** deterministic event-time reconstruction, exact integer economics, three honest same-pair `Simulated` results, bounded reproducible artifacts.

**Non-Goals:** triangles, adaptive fees, provider access, transaction construction, execution, learned ranking, live or paper trading. No UI, Docker, Kafka or PostgreSQL changes.

## Decisions

1. **Crate boundary.** Add `crates/research-replay` to workspace `members`, not `default-members`. It reads frames through `geyser-capture::format`, uses `whirlpool_core::exact` for every quote, and uses only locked dependencies. It has no dependency on arb-engine, edge-assembler, executor-service, hotpath or Kafka. This avoids the existing derived-edge replay's queue and staleness problems.
2. **Event time.** Verify each manifest SHA-256 and identity, then sort account writes by `(slot, write_version, capture_sequence)`. Apply all writes for a slot before evaluating that boundary. The clock is captured slots only; no wall-clock reads occur in replay logic. The result window uses the first and last captured slots and their first/last receive UTC timestamps. Processed slot notifications supply otherwise unchanged boundaries.
3. **State and gaps.** Keep latest pool state and a keyed latest-tick-array map for each pool. An account gap or slot discontinuity marks prior pool state stale; only a later pool write clears staleness. A stale route or reference quote is `state_gap`. An unseen pool or required tick array is `insufficient_state`, never an empty array. Bad quote identities are `invalid_quote`.
4. **Routes.** The six standard-fee SOL/USDC pools yield 30 ordered distinct-pool routes; five SOL/USDT yield 20; two USDC/USDT yield 2. For each route the first hop takes the quote token to the base token, and the second returns base to quote. Each hop verifies token continuity and derives `a_to_b` from the pool's mint order. A, B and C quote in USDC, USDT and USDT atoms respectively. Each canonical v1 request lists exactly its evaluated pair's pool IDs; triangles are deferred to v2.
5. **Sizing.** The declared quote-atom ladder is `[10_000_000, 100_000_000, 1_000_000_000]`, corresponding to 10, 100 and 1,000 six-decimal quote units. FixedUniverse uses only the first size. Every other size is requoted exactly hop by hop. No floating-point money or price math is used; checked signed `i64` totals hold net profit.
6. **Cost model.** `cost-v1` has one 5,000-lamport signature fee, 1,000 micro-lamports per CU × 400,000 CU (400 lamports, rounded up), and a 10,000-lamport tip: 15,400 lamports total. The cost sensitivity uses a 100,000-lamport tip, totaling 105,400 lamports. Each cost is converted at the evaluation slot with exact-in SOL quoting through a declared standard-fee reference pool: A uses `83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d`; B and C use `FwewVm8u6tFPGewAyHmWAqad9hmF7mvqxK4mJ7iNqqGC`. The cost quote is a price input only and never creates a candidate or trade. A's reference pool is also one of its six route pools; excluding it would violate Revision 1's six-pool scope. The full reference key and all parameters are in the run manifest; the pair, reference prefix and tip are in `costModelVersion`.
7. **Two profit channels.** Estimated net uses state at detection slot S. Selected routes are requoted against state at S+L, default L=1 slot, with pair A sensitivity L=2. Missing future state appears as `simulationStatus: insufficient_state` or `state_gap`; it is neither a zero-valued win nor a loss. The result's simulated total sums only available later quotes and the summary counts unavailable simulations separately. Selections never mutate state; fill competition is unknowable from these data.
8. **Policies.** All policies receive the same routes and cost. FixedUniverse selects all positive fixed-size estimates, without ranking. SimpleRules chooses the size with maximum estimated net, requires at least 1,000 quote atoms, sorts by estimated net and takes at most one candidate per pool per slot. ModelRanking is the labeled `ranking-v0-heuristic` baseline, chooses the best size, sorts by integer estimated net/input ratio using cross products, takes top K=1 with the same pool-conflict rule. No ML model is involved.
9. **Accounting.** Each route at each boundary is one candidate per policy. Every candidate has one bucket: selected or rejected with `insufficient_state`, `state_gap`, `adaptive_fee_unsupported`, `below_threshold`, `pool_conflict`, `not_top_k` or `invalid_quote`. `detected` counts positive estimates under that policy's sizing rule. `missed` and realized profit are null. Selected candidates whose later quote is unavailable remain selected, with a separately counted simulation status. Rejected plus selected equals evaluated.
10. **Determinism.** No random operation exists. Seed 0 is recorded but unused. Stable ordered maps, stable tie-breaking and a single-threaded loop yield byte-identical output for fixed input/config/commit/toolchain. Input hashes are read from and verified against manifests. Each run manifest records hashes, config, crate version, Git commit, pinned toolchain and output hashes.
11. **Outputs.** A new directory under `~/MEVanaData/replay/<run-id>/` contains lossless per-candidate JSONL, summary JSON, canonical v1 request/result JSON for the selected pairs and a run manifest. Three default results and pair A's L=2/cost sensitivities are produced. Only the three small default request/result pairs and a ≤64 KB sample of public simulated candidate rows are committed.

## Risks / Trade-offs

- Incomplete tick-array coverage → classify as insufficient state and report its share.
- Processed dev/test feed and eight slot discontinuities → mark stale state and avoid causal/live claims.
- Reference pool is a captured price input; for A it also belongs to the required route universe → keep the cost quote distinct from route candidates and disclose the overlap.
- Later-slot requote cannot establish fill order or success → call results Simulated only.
- Exact quoter may reject valid routes needing unseen arrays → do not synthesize state.

## Migration Plan

There is no service migration. The new crate is opt-in. Full capture and replay output stay local. Merging this branch is an operator decision.
