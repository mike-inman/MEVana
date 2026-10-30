# 2026-10-30 01:04 CDT — Handoff 11: Deterministic replay and the first Simulated experiment result

## Purpose

This slice turns handoff 09's raw captures and handoff 10's exact quoter into the **first honest, reproducible experiment result** from real market data. It does four things:

1. replays captured Whirlpool state deterministically in event time;
2. generates 2-leg and 3-leg route candidates over the standard-fee allowlisted pools;
3. evaluates three selection policies under one explicit cost model;
4. emits one `Simulated` `ExperimentResult` that the existing C# validators accept.

This is the first point at which MEVana can say anything evidence-based about the research question. Even so, the result is a **ten-minute, dev/test-feed simulation**. It is not evidence of profitability. A zero-opportunity or negative result is a valid, honest outcome.

It does **not** trade, sign, stream, train a model, or change any live service.

## Execution model

Implement this directly in this repository with an interactive coding-assistant session (Codex or Claude Code), following this repo's OpenSpec workflow.

- Do **not** use the AgentEngineering factory or controller process.
- Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/replay-20261030`, on branch `feat/deterministic-replay-20261030`, created from `origin/main` (`c7f1d66e` or later).
- Merging to `main` is the operator's decision.
- **Toolchain:** use rustup's `~/.cargo/bin/cargo`, and confirm that `cargo -V` shows `1.92.0-nightly`. Homebrew's `/usr/local/bin/cargo` (1.98.1) ignores `rust-toolchain.toml`, and results from it are invalid. See `CLAUDE.md`.

## Entry state (verified 2026-10-30)

- **Handoff 10 is merged.** `whirlpool_core::exact` provides `quote_exact_in` and `quote_exact_out`, built on Apache `orca_whirlpools_core` and `orca_whirlpools_macros`, both pinned to `=1.0.4`.
  - It reproduced 1,657 of 1,660 evaluable captured swaps exactly.
  - It refuses the 5 adaptive-fee pools with `AdaptiveFeeUnsupported`.
  - It decodes fixed and dynamic tick arrays.
  - It does **not** expose the post-swap sqrt price, tick or liquidity.

  The closeout is `docs/2026-10-29-2354-CDT-research-handoff-10-closeout.md`.
- **Captures** are in `~/MEVanaData/captures/handoff09-20261029/`.
  - The **allowlist session** (about 570 s, 4 files, pools plus fixed and dynamic tick arrays plus transactions, Processed commitment, 8 recorded slot discontinuities) is the replay dataset.
  - The discovery session has no tick arrays and is **not** a replay input.
  - Frame format: `crates/geyser-capture/README.md`.
- **Standard-fee universe: 13 pools.** These are the 18 allowlisted pools minus the 5 adaptive-fee pools listed in handoff 10.
  - SOL/USDC: 6 pools, including the full-range pool `BSddxwYW…` with tick spacing 32896.
  - SOL/USDT: 5 pools.
  - USDC/USDT: 2 pools.
- **Tick-array coverage is incomplete.** A stream sends only accounts that change, so a tick array that was not written during the run was never captured. Candidates whose swap path needs an unseen tick array are rejected as `insufficient_state`, never assumed empty. Handoff 10 classified golden cases the same way.
- **Contracts** live in `dotnet/src/MEVana.Research.Core/`, with specs under `openspec/specs/research-*`.
  - `ExperimentResult`: evidence `Synthetic`, `Simulated` or `Live`.
  - A `Completed` result requires **exactly one** `FixedUniverse`, `SimpleRules` and `ModelRanking` policy entry. Each entry carries evaluated, detected, selected and rejected-by-reason counts, plus estimated, simulated and realized net profits as signed `long` quote atoms. Realized profit must be null unless the evidence is `Live`.
  - Each result cites the SHA-256 of a canonical `ExperimentRequest`, plus a catalog `DatasetMetadata` entry (Id, MetadataVersion, Provenance, coverage UTC). The window must lie inside the dataset's coverage.
- **Do not use `arb-engine/bin/arb_replay`.**
  - It is a derived-`EdgeDelta` replayer, and it has the finding F8 problems (assessment `docs/2026-10-06-2328-CT-MEVANA-Azure-AI-Rearchitecture-Assessment.md`): a 1,024-slot signal queue that loses signals, wall-clock staleness, and no seed control.
  - `arb-engine` also does not link because of the existing `rdkafka-sys` OpenSSL error.
  - `EdgeDelta` cannot carry tick arrays, so it cannot feed exact quoting.

  The crate inventory (`docs/2026-10-28-0832-…`, section 6, item 4) recommends a **new replay core over raw captures**. That supersedes handoff 09's note about converting captures to `EdgeDelta`.
- **Known baseline bugs** in the legacy `edge_integration` cycle search, which the new baseline must not repeat:
  - direction swapped twice (C5);
  - token alignment unchecked;
  - fee units mixed (C2): Whirlpool `fee_rate` is in units of 1e-6, not basis points;
  - zero-size Whirlpool edges (C4).

## Design requirements

Record all of these in the OpenSpec design, with their concrete parameter values.

1. **New crate `crates/research-replay`.**
   - Add it to the workspace `members` but **not** to `default-members`.
   - It depends on `whirlpool-core`, on `geyser-capture`'s library (for frame reading), and on crates already locked.
   - It must not depend on `arb-engine`, `edge-assembler`, `executor-service`, `hotpath_arb_engine` or Kafka.
2. **Event-time clock and ordering.**
   - Order updates by slot, then `write_version`, then capture sequence.
   - Apply all account writes for a slot, then evaluate at the slot boundary.
   - No wall-clock reads anywhere in replay logic.
   - The replay window is defined by the first and last captured slot and their receive times, recorded in UTC.
3. **State.** Keep per-pool state (the latest pool account) and a per-pool map of tick arrays (latest observed).
   - Mark a pool **stale** across any recorded gap or slot discontinuity on its connection, until the pool is written again.
   - Candidates touching stale pools are rejected as `state_gap`.
4. **Routes.**
   - **2-leg:** same pair, buy on pool X and sell on pool Y, for every ordered pair of distinct standard-fee pools with the same mint pair.
   - **3-leg:** SOL→USDC→USDT→SOL and its reverse, over every combination of one pool per pair.
   - **Token alignment** is checked at every hop: one leg's output mint must equal the next leg's input mint.
   - **Direction** comes from mint order (`a_to_b` when the input is mint A).
   - The **quote asset is USDC**, and all profits are in USDC atoms.
5. **Sizing.** Use a fixed, declared trade-size ladder in USDC atoms, for example 10, 100 and 1,000 USDC equivalent. Convert the start size for SOL-start triangles with the exact quoter at the evaluation slot. Every size is quoted exactly and leg by leg; there is no `f64` anywhere in money or price math.
6. **Cost model.** Version it (`costModelVersion`) and declare every parameter:
   - base fee of 5,000 lamports per signature;
   - a declared priority fee (micro-lamports × compute-unit estimate per route shape);
   - a declared tip in lamports.

   Convert lamports to USDC atoms through the exact quoter on a declared SOL/USDC pool at the evaluation slot. Defaults must be conservative, meaning nonzero priority fee and tip. Net profit = output − input − costs, and it may be negative.
7. **Estimated vs simulated profit.** These must not collapse into the same number.
   - **Estimated** profit is quoted against state at detection slot S.
   - **Simulated** profit re-quotes the same route and size against state at slot S + L, where L is a declared latency in slots (default 1).
   - Report missing state at S + L as `insufficient_state`, not as a loss or a win.
   - Selected trades do **not** modify replayed state. Document this limitation: we cannot know whether we would have won the fill (F8).
8. **Policies.** Every policy sees the same candidate set and the same cost model.
   - **FixedUniverse:** a fixed route set and a single fixed size. Select every candidate with estimated net > 0. No ranking.
   - **SimpleRules:** a declared minimum estimated net profit, the best size from the ladder, and at most one selection per pool per slot.
   - **ModelRanking: heuristic `ranking-v0`.** No learned model exists.
     - Rank the slot's candidates by estimated net profit per unit of input. Then take the top K (declared) under a pool-conflict constraint, so no pool is used twice in a slot.
     - Label it `ranking-v0-heuristic` in the strategy version.
     - The closeout must say plainly that **no ML model was involved** and that this entry exists to complete the contract shape and serve as the first ranking baseline.
9. **Accounting.** For each policy, report:
   - evaluated, detected (estimated net > 0) and selected counts;
   - rejected counts by unique reason code: `insufficient_state`, `state_gap`, `adaptive_fee_unsupported`, `below_threshold`, `pool_conflict`, `not_top_k`, plus any others declared;
   - missed = null.

   Total rejected + selected ≤ evaluated, as the contract requires. Every candidate falls into exactly one bucket per policy.
10. **Determinism.** The same captures, configuration and seed produce **byte-identical** outputs. If nothing is random, document that the seed is unused but still recorded.
    - Inputs are hashed: capture file SHA-256s, read from the manifests.
    - Every output carries a manifest with input hashes, config, crate version, git commit and toolchain.
11. **Outputs**, under `~/MEVanaData/replay/<run-id>/`. None is committed except the small test artifacts described in step 6.
    - A per-candidate JSONL (lossless; no bounded queues that can drop records).
    - A summary.
    - The canonical `ExperimentRequest` and `ExperimentResult` JSON.
    - A run manifest.

## Work

1. **OpenSpec.**
   - Create the change `add-deterministic-replay` (capability `research-replay`): proposal, design (with every parameter above), tasks and spec delta.
   - Also update the `research-dataset-catalog` spec delta for a capture-backed dataset entry, if needed.
   - Run `openspec validate --all --strict`.
2. **The replay core and the route, cost and policy modules.**
   - Implement them in `crates/research-replay`, with unit tests that need no network:
     - ordering and slot-boundary evaluation;
     - gap and staleness marking;
     - route generation and token alignment;
     - direction;
     - fee units (one test proving `fee_rate` 400 means 0.04%);
     - cost conversion;
     - estimated vs simulated separation;
     - each policy's selection and rejection bucketing;
     - accounting invariants.
   - Use a small synthetic in-memory state for the unit tests, and label it synthetic.
3. **Determinism test.** Run the full replay twice on the real allowlist session, and require identical output hashes. Run it a third time with a different worker or thread count, if parallelism exists, and require identical hashes again.
4. **Request and result through C#.** Add the minimum needed, test-only where possible:
   - a `DatasetMetadata` entry for the capture-backed dataset;
     - Id, for example `orca-wp-allowlist-20261029`;
     - a metadata version;
     - a provenance string without `/`, `\` or `://`;
     - coverage from the manifests;
     - suitability: `Simulated` research.
   - a .NET test, or a small console tool under `dotnet/` if the test approach is impractical, that:
     - canonicalizes the `ExperimentRequest` and produces its SHA-256;
     - validates the replay-produced `ExperimentResult` JSON with the existing syntax and semantic validators against that catalog entry.

   Do not change existing contract behavior. All 194 existing tests must still pass. If the contract cannot express the result honestly, **stop and report**. Do not bend the result to fit.
5. **The first real run.** Replay the allowlist session with the declared defaults. Then run one sensitivity run with L = 2, plus one cost sensitivity run.
   - **Report for each run:** candidates by route shape, rejection reasons, detected and selected counts, and estimated vs simulated net per policy.
   - **Report the leading `insufficient_state` share honestly.**
   - **Determinism:** report the hashes.
   - If the result shows no detected opportunities, that is a valid result. Report it as such.
6. **Committed artifacts.** Commit, all small:
   - the canonical request and result JSON from the default run;
   - their validation test;
   - a truncated sample of candidate rows (at most 64 KB), labeled as simulated output from public data.

   No capture data is committed.
7. **Validate** with the pinned toolchain:
   - `cargo fmt -p research-replay -- --check`;
   - `cargo clippy --locked -p research-replay --all-targets -- -D warnings`;
   - `cargo test --locked -p research-replay -p whirlpool-core -p geyser-capture`;
   - `dotnet test dotnet/MEVana.slnx --verbosity quiet`: the 194 existing tests plus the new ones;
   - `openspec validate --all --strict`;
   - `./scripts/ci/repo_hygiene.sh`;
   - `git diff --check`;
   - a non-printing secret scan.

   Then archive the OpenSpec change.
8. **Closeout.** Write `docs/2026-10-XX-HHMM-CDT-research-handoff-11-closeout.md`. It must record:
   - every parameter;
   - the results of all three runs, with the policy comparison table;
   - the `insufficient_state` and `state_gap` shares;
   - the determinism hashes;
   - the contract validation;
   - limitations: 10-minute dev/test sample, no fill competition, heuristic ranker, adaptive pools excluded, Processed commitment, incomplete tick-array coverage;
   - recommended next steps, for example a tick-array RPC snapshot at capture start, longer captures on a server host, and the first learned ranker.

   Commit in logical units, push the branch, and finish clean and synchronized.

## Constraints

- **No network:** no Geyser, RPC or provider calls. Develop and run entirely from existing captures. If coverage is too thin to be meaningful, report that instead of capturing more.
- **No signing,** no wallets, and no transaction construction or submission. Trading stays disabled.
- **Integer-only money and price math.** Use `whirlpool_core::exact` for all quotes. Do not read post-2025-02-27 Orca material.
- **Rust changes are limited to** the new `research-replay` crate, the workspace `members` entry and the `Cargo.lock` additions. Do not change `whirlpool-core` or `geyser-capture` except for a **minimal, documented, read-only API exposure**, such as making the frame reader public, if one is needed.
- **.NET changes** are limited to the catalog entry and validation tests or tool from step 4, and must not change existing contract behavior.
- **Not changed:** `arb-engine`, `edge-assembler`, `executor-service`, `hotpath_arb_engine` and `shared-core`; UI, Docker, Kafka and PostgreSQL.
- No credentials are needed. Do not read, print or commit `.env`.
- Date every document, and make sure no work is lost.

## Exit criteria

Either the slice stopped with a precise report (contract cannot express the result, or the data is too thin), **or** all of the following are true:

- `research-replay` exists, is specified and is tested;
- replay is deterministic, with matching hashes;
- one `Simulated` `ExperimentResult` from the real allowlist session validates under the existing C# validators;
- the sensitivity runs are reported;
- the closeout states the limitations plainly;
- the OpenSpec change is archived and the branch is pushed, clean and synchronized.

This result shows what the replay produced on ten minutes of dev/test data. It does not show that any strategy is profitable.

## Later slices

- **Capture improvements:** an RPC tick-array snapshot at capture start, Confirmed/Finalized reconciliation, and longer captures on a server host (needs an operator decision on Vultr or Hetzner and on a provider plan).
- **First learned ranker:** this is where `ModelRanking` stops being heuristic.
- **Adaptive-fee quoting:** Oracle capture, plus Orca consent or a clean-room implementation.
- **Workspace build remedy:** the `rdkafka-sys` OpenSSL link error.
- **Handoffs 12–13:** the read-only Research API serving the catalog and results, and the MEVana-UI research page.
