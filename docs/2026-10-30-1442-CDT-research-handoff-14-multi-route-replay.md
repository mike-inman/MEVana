# 2026-10-30 14:42 CDT — Handoff 14: Triangle and multi-pair replay

## Purpose

Handoff 13 can describe a SOL–USDC–USDT route universe, but nothing evaluates one yet. This slice replays that universe offline, against the existing handoff 12 snapshot capture, and produces one `Simulated` result linked to a v2 request.

The question for this slice is narrow: **on this capture, do triangles add anything over two-leg routes, under the same cost model and policies?**

This slice is:
- offline replay plus .NET contract work and OpenSpec;
- **no network, no provider calls, no new capture, no credentials, no `.env`**;
- no trading of any kind.

A result with no opportunities is a valid outcome. It only counts as evidence for the routes that were actually evaluable, so coverage must be reported beside every result.

## Execution model

Implement this directly in this repository with an interactive coding-assistant session, following the OpenSpec workflow.
- Do **not** use the AgentEngineering factory.
- Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/multi-route-replay-20261030`, on branch `feat/multi-route-replay-20261030`.
- Branch from `origin/main`, and first confirm that it contains merge commit `af10ea3d`. If it does not, stop and report: the operator has not pushed `main` yet.
- Merging is the operator's decision.
- Use rustup Cargo (`~/.cargo/bin/cargo`, `1.92.0-nightly`). Results from the Homebrew compiler are invalid.

## Entry state (verified 2026-10-30)

| Area | Location | State |
| --- | --- | --- |
| Capture | `~/MEVanaData/captures/handoff12-20261030/capture-1793377454811581000` | Format v2, 480.8 s, zero true gaps, 2,235 confirmed boundaries. Read-only for this slice. |
| v1 reference run | `~/MEVanaData/replay/handoff12-snapshot-default1` | Read-only regression baseline |
| Route shape | `crates/research-replay/src/lib.rs` | `Route` has `first`, `second`, `base`, `quote`. `quote_route` is hard-coded to two legs. |
| Policies | `crates/research-replay/src/policy.rs` | `decide` tie-breaks and detects pool conflicts on `first` and `second` only. |
| Engine | `crates/research-replay/src/engine.rs` | Per-pair loop. `CandidateRow`, `route_cache` and the later-slot requote all assume two pools. Cost uses `compute_units_two_leg`. |
| Output | `crates/research-replay/src/main.rs` | Writes v1 requests and results, with `request_canonicalization_version: 1` hard-coded at line 203. |
| v2 request | `dotnet/src/MEVana.Research.Core/ExperimentRequestV2*.cs` | Contract, canonicalizer and route enumerator are done. |
| Result contract | `ExperimentResultSyntaxValidator.cs`, `ExperimentResultCanonicalizer.cs` | Both accept version 1 only, so a version-2 result cannot be canonicalized yet. The 3-argument v2 `Validate` overload already handles linkage and catalog checks. |

## Design

### 1. Experiment scope

Run one experiment on the R2 scope, against dataset `orca-wp-snapshot-20261030`:

| Field | Value |
| --- | --- |
| `quoteTokenId` | USDC |
| `tokenIds` | SOL, USDC, USDT |
| `poolIds` | The 13 standard-fee pools |
| `routeShapes` | `two_leg`, `triangle` |
| `priceInputPoolIds` | `83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d` (SOL/USDC reference) |
| Trade bounds | 10,000,000 to 1,000,000,000 quote atoms |
| Strategy | `ranking-v0-heuristic` |
| Window | The boundary window already computed by `main.rs`, in the same 7-digit UTC format |

Expected routes: **152** (120 triangles, 30 SOL/USDC two-leg, 2 USDC/USDT two-leg). All profit is in USDC atoms.

SOL/USDT two-leg routes are not in this universe, because that pair does not contain the quote token. USDC/USDT two-leg routes here run USDC → USDT → USDC, which is the reverse of v1 pair C's quote direction, so they are not comparable to pair C.

### 2. Multi-leg routes

Add a route type holding an ordered pool list and token path. A route is quoted exact-in along the path: each leg's output is the next leg's input. If a leg fails, the route takes that failure, and the first failing leg in route order is the one reported.

The Rust route list must match `ExperimentRequestV2RouteEnumerator` exactly: same shape definitions and same order (shape, then pool sequence, then token path, all ordinal).

**Cross-language parity gate.** Commit one fixture holding the full ordered R2 route list. A .NET test asserts that the C# enumerator produces it, and a Rust test asserts that the Rust enumerator produces it. Rust must not generate this fixture.

### 3. Cost model

Add `compute_units_triangle: 600_000`, keeping `compute_units_two_leg: 400_000`. That is 200,000 CU per leg, and it is a labeled assumption rather than a measurement. Everything else in the cost model is unchanged: base fee, priority fee, tip and conversion through the reference pool.

Cost differs by shape, so the cost cache must be keyed by shape. Use a new `cost_model_version` such as `cost-v2-usdc-83v8iPyZihDE-10000`, keep it within the 64-character identifier limit, and record both CU values in the run manifest.

### 4. Policies

Apply the existing policies to the whole 152-route universe at each boundary, with the same thresholds, ladder, `ranking_top_k` of 1 and seed.
- `pool_conflict` covers every pool on a route.
- Tie-breaks compare the ordered pool list. For two-leg routes this must give the same order as today.
- Do not add a new policy, ranker or reason code unless a test shows one is required.

### 5. v2 result path

Keep `ExperimentResult` unchanged. Add a v2 syntax check and canonicalizer:
- **Syntax:** reuse the v1 rules through the normalized-copy approach already used by the v2 `Validate` overload, and require `requestCanonicalizationVersion == 2`.
- **Canonicalizer:** add `ExperimentResultV2Canonicalizer`. The output has the same fields and order as v1, with `requestCanonicalizationVersion: 2`. The top-level `canonicalizationVersion` stays 1, because the result format itself has not changed.
- **Authorized v1 edit:** extracting the private JSON-writing body of `ExperimentResultCanonicalizer` into an internal shared helper is allowed. Its behavior and bytes must not change, and the existing golden tests must pass unedited. This is the only permitted edit to an existing v1 file.
- `ExperimentResultSyntaxValidator` stays version-1-only. The amended handoff 13 rejection stays in force: a version-2 result against a v1 request returns `unsupported_canonicalization_version`.

The Rust request and result JSON must match the C# canonical bytes exactly.

### 6. CLI and output

- Add profiles `v2-default`, `v2-latency2`, `v2-cost` and `v2-processed`, with the same L, tip and commitment values as the existing profiles.
- v2 profiles must refuse a capture that is not format version 2.
- Write `request.json`, `result.json`, `candidates.jsonl`, `summary.json` and `run-manifest.json`.
- Use a separate v2 candidate row with `shape`, `pools` and `tokens`. Do not change the v1 row.
- Use engine version `research-replay-v2-0_1_0`. The v1 literal `research-replay-0_2_0` stays unchanged.
- Refuse to overwrite an existing output directory, as today.

Expect about 339,720 candidates per policy (152 × 2,235), so check available disk before running. Output goes under `~/MEVanaData/replay/`, not into the repo.

### 7. Coverage reporting

`ExperimentResult` holds one total per policy, so put the breakdown in `summary.json`:
- for each policy and shape: evaluated, detected, selected, rejections by reason, estimated net and simulated net;
- for each pool and swap direction: quote attempts and outcomes by reason. Failures on later legs that first-failure attribution hides must be counted here by evaluating each leg's availability separately, without changing route outcomes.

Per-shape numbers must sum to the policy totals, and the replay must fail if they don't.

Do not predict coverage from the handoff 12 pair B figure of 44.88%. That figure covers routes using two SOL/USDT pools, while a triangle uses one, and handoff 12's candidates show the shortfall is uneven across pools:

| SOL/USDT pool | Evaluable share of two-leg routes through it |
| --- | ---: |
| `BmXVhzBH…` | 0.0% |
| `AhhoxZDm…` | 37.4% |
| `B6LL9aCW…` | 62.3% |
| `FwewVm8u…` | 62.3% |
| `HcoJqG32…` | 62.3% |

Measure triangle coverage directly and explain it by pool.

## Tests

**Rust**
- Multi-leg quoting: a synthetic triangle chains amounts, and a failure on each leg position is reported correctly.
- Route parity against the committed fixture; 120 + 32 = 152.
- Policy conflicts across shapes and on a route's third pool; deterministic tie-breaks.
- Shape-specific cost, and a cache that never crosses shapes.
- Accounting invariants, including the per-shape sums.

**.NET (new files only, apart from the authorized helper extraction)**
- v2 result syntax and canonical bytes, including rejection of version 1 by the v2 path.
- The committed v2 request canonicalizes to identical bytes and its SHA-256 equals the result's `requestSha256`.
- The committed v2 result validates through the 3-argument v2 overload against `CaptureReplayCatalog.CreateSnapshot()`.
- The enumerator returns 152 routes for the committed request.
- All existing tests pass unedited.

## Regression gates

Run these on real data before reporting any result:

1. **v1 is unchanged.** Rerun profile `default` on the snapshot capture. `a/b/c-request.json`, `a/b/c-result.json`, `candidates.jsonl` and `summary.json` must be byte-identical to `handoff12-snapshot-default1`. `run-manifest.json` may differ only in its git commit.
2. **Two-leg quotes are unchanged.** In the `v2-default` run, each of the 30 SOL/USDC two-leg routes must have the same per-size `SizeQuote` values as the same route at the same boundary in the v1 pair A candidates. Policy totals will differ, because conflicts and top-K now span the whole universe. That is expected, and only the quotes are compared.
3. **Determinism.** Two `v2-default` runs must produce byte-identical request, result, candidates, summary and manifest output.

If a gate fails, stop and report it. Do not adjust a baseline.

## Runs

Run `v2-default` twice, then `v2-latency2`, `v2-cost` and `v2-processed` once each. Commit the `v2-default` request and result as `Fixtures/` files. Do not commit candidate files.

## Constraints

- **No v1 behavior change**, in Rust or .NET. Existing fixture files stay unedited.
- Do not change `whirlpool-core`, `geyser-capture`, `hotpath_arb_engine`, `executor-service` or `shared-core`.
- Do not write to the capture directory or to existing replay directories.
- Do not add adaptive-fee pools. They remain excluded.
- Date every document. Commit and push all work; lose nothing.

## Validation

- `cargo fmt --all --check`, using rustup Cargo;
- `cargo clippy -p research-replay --all-targets --offline --locked -- -D warnings`;
- `cargo test -p research-replay -p geyser-capture -p whirlpool-core --offline --locked`;
- `dotnet build` and `dotnet test dotnet/MEVana.slnx`;
- `openspec validate --all --strict`;
- `./scripts/ci/repo_hygiene.sh`;
- `git diff --check`;
- a non-printing credential scan over the diff.

## Closeout

Write a dated closeout doc covering:
- the regression gate results and determinism hashes;
- policy tables for `v2-default`, split by shape;
- sensitivity tables for the three other profiles;
- the per-pool, per-direction coverage table;
- a direct answer to this slice's question, stating what share of triangles was evaluable and how far the answer can be trusted;
- limitations: about eight minutes of dev/test Processed data on a laptop, the labeled 600,000 CU assumption, no fill or inclusion modeling, a heuristic ranker, and no learned model;
- confirmation that v1 is unchanged.

Archive the OpenSpec change. Push the branch clean and synchronized.

## Exit criteria

The slice either stopped with a precise report, **or** all of the following are true:
- the route parity fixture passes in both languages;
- all three regression gates pass;
- the committed v2 request and result validate under the .NET contracts;
- coverage is reported per shape and per pool;
- everything above is documented, archived and pushed.

If a requirement here cannot be met without changing v1 behavior, or a gate fails, stop and report instead.

## Later slices

- Handoff 15: read-only Research API, serving v1 and v2.
- Handoff 16: MEVana-UI research page.
- Better SOL/USDT tick-array coverage, if this slice's per-pool table shows it would change the answer.
- A longer, stronger-commitment capture on a server host.
