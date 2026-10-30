# 2026-10-29 23:23 CDT — Handoff 10: Exact Whirlpool quoting with golden tests

## Purpose

Replay results are meaningful only if every simulated trade is priced exactly as the on-chain program would price it. No crate in this repo does that today. `whirlpool-core`'s `math.rs`, `quoter.rs` and `tick_window.rs` use approximate `f64` math, with broken Q64 scaling and no tick crossing (see `docs/2026-10-28-0832-CDT-rust-crates-inventory-and-capability-map.md`).

This slice adds **exact integer Whirlpool swap quoting** for standard-fee pools. It proves the quoting against real on-chain swaps already recorded in handoff 09's captures: given the pool state before a transaction, the quote must reproduce that transaction's actual vault balance changes exactly.

It does **not** replay, rank, detect opportunities or trade, and it opens **no market-data streams**.

## Execution model

Implement this directly in this repository with an interactive coding-assistant session (Codex or Claude Code), following this repo's OpenSpec workflow as handoffs 07–09 did.

- Do **not** use the AgentEngineering factory or controller process: no factory jobs, assignment profiles, permitted-write lists or Developer/Reviewer agent runs.
- Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/exact-quoting-20261030`, on branch `feat/exact-whirlpool-quoting-20261030`, created from `origin/main`.
- Merging to `main` is the operator's decision.

## Entry state (verified 2026-10-29)

- Handoff 09 is merged. `main` is at `53779e25` or later. Its final closeout is `docs/2026-10-29-2232-CDT-research-handoff-09-final-closeout.md`.
- **Captures** are in `~/MEVanaData/captures/handoff09-20261029/`: 6 finalized `.cap` files, 126 MB, from Sodae at Processed commitment. The format is documented in `crates/geyser-capture/README.md`.
  - Files start with the 8-byte magic `MEVCAP01`.
  - Each frame is a `u32` length, then `kind u8` (0 = update, 1 = gap record), then `connection u8`, `sequence u64` and `receive_ns u64`, then the raw `SubscribeUpdate` protobuf.
- **The allowlist** is `crates/geyser-capture/allowlist.json`: 18 SOL/USDC, SOL/USDT and USDC/USDT pools.
- **Adaptive-fee pools.** An offline decode by the operator found that **5 of the 18 pools use adaptive fees**, meaning their fee-tier index differs from their tick spacing:
  - `5hCDEXRC…`, `Esvfxt3j…`, `4HppGTwe…` and `9VuCJupd…` (SOL/USDC);
  - `GDrzVCH6…` (SOL/USDT).

  The other 13 pools use standard fees. One of them, `BSddxwYW…`, has tick spacing 32896, which marks a full-range "splash" pool.
- **Feasibility check** (operator, offline):
  - all 23,450 captured account updates carry `txn_signature` and `write_version`;
  - 4,941 of 4,945 captured transactions carry pre/post token balances;
  - 1,779 pool post-states were written by a captured transaction that touched exactly one allowlisted pool, and that pool is a standard-fee pool. These are the candidate golden cases.
- **Account layouts.** Layouts and discriminators for Whirlpool, fixed TickArray, DynamicTickArray (148 + 112·n bytes, pool field at offset 12), Position and Oracle are in `docs/2026-10-29-2148-CDT-geyser-account-layouts.md`. `whirlpool-core` decodes fixed TickArrays correctly since handoff 09. It does **not** decode dynamic tick arrays yet.
- **Current users of `whirlpool-core`:**
  - `edge-assembler` uses `quoter::ClmmQuoter` and `tick_window::TickWindow`;
  - `arb-engine` uses `pda`;
  - `executor-service` and `geyser-capture` use `decode`.

  None of these may change behavior in this slice.
- `cargo build --workspace` is still blocked by the existing `rdkafka-sys` OpenSSL link error in `dex-registry` and `arb-engine`. This is a separate follow-up and must not be worked around here.

## License decision (operator, 2026-10-29) — read before writing any code

Orca relicensed `orca_whirlpools_core` and `orca_whirlpools_macros` on **2025-02-27**, from Apache 2.0 to the non-commercial **Orca License**. Under that license, any use that could benefit "bodies engaged in cryptocurrency trading" requires Orca's written consent, and merely reading the code counts as "Use".

The operator chose the following basis:

- **Use `orca_whirlpools_core` version 1.0.4 (published 2025-02-21, Apache-2.0) and `orca_whirlpools_macros` version 1.0.4 (Apache-2.0).** Both are the last Apache releases.
- **Pin both exactly** in `crates/whirlpool-core/Cargo.toml`:
  - `orca_whirlpools_core = { version = "=1.0.4", … }`
  - `orca_whirlpools_macros = "=1.0.4"`, declared as a direct dependency so that it pins the transitive resolution.

  **This is a known trap.** Core 1.0.4 declares macros `"1.0.4"`, which Cargo treats as `^1.0.4`. Without the explicit pin it resolves to **macros 1.0.5, which is Orca-licensed**.
- **Add a test** (in `whirlpool-core`, or a script run by the validation step) that parses `Cargo.lock` and fails if any `orca_*` package has a version other than `1.0.4`.
- **Features.** 1.0.4's features are `default = ["floats"]`, `floats` (which pulls in `libm`) and `wasm`. Swap quoting is always compiled in. Use `default-features = false`, so that `floats` and `wasm` stay off and no float helpers are reachable.
- **Do not read, open, download, copy or paraphrase any Orca source, documentation code or crate version published on or after 2025-02-27.** That includes the GitHub `orca-so/whirlpools` repository at any recent commit, `orca_whirlpools_core` 1.0.5 and later, and 2.x.
  - The 1.0.4 crate source (Apache) and this repo's own documents, IDL and captured bytes are allowed.
  - If something seems to require newer Orca material, **stop and report**.
- **Attribution.** Add a short `crates/whirlpool-core/THIRD_PARTY.md` recording the dependency, its exact version, its Apache-2.0 license, the pin rationale and this decision.
- **Adaptive-fee pools are out of scope.** Quoting an adaptive-fee pool must return a typed error, such as `AdaptiveFeeUnsupported`, and never a silently wrong quote. Revisit adaptive fees in a later slice: either with Orca's consent, or with a clean-room implementation by someone who has not read the post-2025-02-27 source.

## Work

1. **OpenSpec.**
   - Create the change `add-exact-whirlpool-quoting` (capability `whirlpool-quoting`): proposal, design, tasks and a spec delta covering steps 2–6 and the license guardrails above.
   - Run `openspec validate --all --strict`.
2. **Dependencies.**
   - Add the two exact pins.
   - Confirm that the `Cargo.lock` diff adds only `orca_whirlpools_core 1.0.4`, `orca_whirlpools_macros 1.0.4` and any of their Apache/MIT dependencies not already locked. Report every new lockfile entry with its license.
   - No version of an already-locked crate may change.
3. **`whirlpool-core`: new `exact` module.** Add the module and leave `math.rs`, `quoter.rs` and `tick_window.rs` untouched. Note in the crate README that they are legacy approximations used by `edge-assembler` and must not be used for research results.
   - **Dynamic tick arrays.** Decode DynamicTickArray accounts, using the layout in the 2148 layouts document. Verify the result against real captured dynamic-array bytes, and reject any input whose length does not fit 148 + 112·n.
     - Add a real dynamic-array fixture (a few KB, compressed) and a test.
     - Decode both fixed and dynamic arrays into the same tick-array facade.
   - **Adapters.** Map `WhirlpoolMinimal` plus the decoded tick arrays to the 1.0.4 facades. Use integer types only, with **no `f64` anywhere in the quote path**.
   - **Tick-array sequence.** Derive the tick-array sequence for a swap direction from the pool's current tick and tick spacing, using the existing `pda` helpers. Missing arrays are a typed error, not a zero-liquidity assumption.
   - **Public API.** `quote_exact_in` and `quote_exact_out` return the input amount, output amount and fee, plus the next sqrt price, tick and liquidity if 1.0.4 exposes them.
   - **Adaptive-fee pools.** Detect them (fee-tier index at pool bytes 43..45 ≠ tick spacing) and refuse them with a typed error.
   - **Unit tests:** adapters, both tick-array kinds, refusal of adaptive pools, missing-array errors, and both directions.
4. **Golden-case extraction (offline).** Add a binary `swap_cases` to `geyser-capture`. It reads capture files only.
   - **A candidate is** a pool account update whose `txn_signature` matches a captured, **successful** transaction that touched exactly one allowlisted, standard-fee pool.
   - **Pre-state** is the same pool's immediately preceding observed update, ordered by slot and then `write_version`. If continuity cannot be shown, classify the case `missing_pre_state` and do not use it.
   - **Tick arrays.** Use, for each array the swap may traverse, the latest captured state at or before the pre-state. Classify the case `insufficient_tick_arrays` if any are missing.
   - **Observed amounts** come from the pool's two vault accounts (pool bytes 133..165 and 213..245), using pre/post token balances.
     - Map account indexes through the transaction's static keys **and** its loaded lookup-table addresses.
     - The direction is whichever vault increased.
   - Write the cases to `~/MEVanaData/golden/whirlpool-20261029/` in a documented, versioned format. Never commit them in bulk.
5. **Golden evaluation.**
   - **Quote each case both ways:**
     - exact-in, from the observed input amount;
     - exact-out, from the observed output amount.
   - **A case passes** only if one mode reproduces **both** observed vault amounts exactly. Where 1.0.4 exposes post-swap state, also compare the next sqrt price, tick and liquidity with the captured post-state.
   - **Classify every other case.** The categories are:
     - multiple swaps through the same pool in one transaction;
     - missing intermediate write;
     - insufficient tick arrays;
     - non-swap state change (for example a liquidity change);
     - unexplained.
   - **Do not tune math to fit data.** If unexplained mismatches exceed 1% of the evaluable cases, stop and report them with examples (signatures and slots).
   - **Report:** cases total, evaluable, exact matches, each mismatch class, per-pool coverage, tick-crossing coverage and both directions.
6. **Committed golden subset.**
   - Commit at most **256 KB compressed** of real cases under `crates/whirlpool-core/tests/golden/`. Cover at least 20 cases spanning both directions, at least 5 standard-fee pools across different tick spacings, and **at least 5 cases that cross a tick**.
   - Add a `whirlpool-core` test that requires an exact match on all of them.
   - Label the cases as real public on-chain data used for quote-correctness tests, not as evidence of opportunities.
7. **If usable cases are scarce** (fewer than 200 evaluable cases, or fewer than 5 tick-crossing cases): **stop and report** with a recommendation, for example an RPC tick-array snapshot at capture start. Do **not** open a new stream in this slice.
8. **Validate:**
   - `cargo fmt -p whirlpool-core -p geyser-capture -- --check`;
   - `cargo clippy -p whirlpool-core -p geyser-capture --all-targets -- -D warnings`;
   - `cargo test -p whirlpool-core -p geyser-capture`;
   - `cargo build -p edge-assembler -p arb-engine -p executor-service`, or, where the existing OpenSSL link error blocks a target, `cargo check` on it, to prove that the dependents still compile;
   - the `orca_*` lockfile license test;
   - `./scripts/ci/repo_hygiene.sh`;
   - `dotnet test dotnet/MEVana.slnx --verbosity quiet` (unchanged at 194);
   - `openspec validate --all --strict`;
   - `git diff --check`;
   - a non-printing secret scan.

   Archive the OpenSpec change after validation passes.
9. **Closeout.** Write `docs/2026-10-XX-HHMM-CDT-research-handoff-10-closeout.md`. It must record:
   - the license pins and the lockfile additions with their licenses;
   - the dynamic-array decoding verification;
   - the golden results and their classification;
   - the committed subset;
   - what remains for adaptive-fee pools;
   - open risks.

   Commit in logical units, push the branch, and finish clean and synchronized.

## Constraints

- **No network beyond downloading the two pinned crates and their dependencies from crates.io.** No Geyser, RPC or provider calls. Develop entirely against existing captures.
- **License guardrails as above**, and no Orca material dated on or after 2025-02-27.
- **Integer-only quote path**; no `f64`.
- **Rust changes are limited to:**
  - `whirlpool-core`: the new `exact` module, dynamic decoding, the dependency pins, tests, README and `THIRD_PARTY.md`;
  - `geyser-capture`: the `swap_cases` binary and its tests;
  - the resulting `Cargo.lock` additions.
- **Not changed:** `hotpath_arb_engine`, `executor-service`, `edge-assembler`, `arb-engine` and `shared-core`; the legacy `math`, `quoter` and `tick_window` modules; .NET, UI, Docker, Kafka and PostgreSQL.
- Do not commit capture data or golden cases beyond the bounded fixtures and subset.
- Do not modify, print or commit credentials or `.env`. This slice needs no credentials.
- Keep trading disabled. Date every document, and make sure no work is lost.

## Exit criteria

Either the slice stopped at step 5 or 7 with a precise report, **or** all of the following are true:

- the exact quote module exists with both versions pinned to 1.0.4, and the lockfile test passes;
- dynamic tick arrays decode and are verified against real bytes;
- the full golden corpus has been evaluated and classified, with unexplained mismatches at or below 1%;
- the committed subset passes exactly;
- adaptive-fee pools are refused explicitly;
- dependents still compile;
- the OpenSpec change is archived and the branch is pushed, clean and synchronized.

Exact matches show that quote math reproduces past on-chain swaps. They do not show that any opportunity is profitable.

## Later slices

- **Handoff 11, replay correctness:** convert captures into replay input, fix the `arb_replay` problems in finding F8, and write the first `Simulated` `ExperimentResult` from a real capture using this exact quoter.
- **Adaptive-fee quoting:** capture Oracle accounts for adaptive pools (a short, bounded run), then quote them with Orca's written consent or a clean-room implementation.
- **Workspace build remedy:** the `rdkafka-sys` OpenSSL link error.
- **Handoffs 12–13:** the research API and the UI.
