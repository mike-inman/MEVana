# 2026-10-30 08:28 CDT — Research handoff 11 closeout: deterministic offline replay

**Status:** Three capture-backed, same-pair v1 `Simulated` results completed and validated. The replay found **zero positive estimated opportunities** in this 10-minute dev/test capture. This is a local simulation result, not evidence that any policy is profitable or unprofitable in production. Trading remained disabled. The branch is for operator review; merging to `main` is the operator's decision.

## Scope and method

The source was the handoff 09 allowlist session `capture-1793330490539629000` under `~/MEVanaData/captures/handoff09-20261029/`: four capture files, 18 allowlisted pools, Processed commitment, dev/test tier, slots 463131459–463134134, 2,652 observed evaluation boundaries and eight recorded slot discontinuities. The result window is `2026-10-30T03:21:31.3406970Z` to `2026-10-30T03:31:00.3739430Z`. Capture file hashes were verified against their manifests before replay. All quotes use `whirlpool_core::exact`, with integer-only amount and cost math. Writes are ordered by slot, write version and capture sequence; all writes at a slot are applied before evaluation. No network, provider, RPC, Geyser, wallet or execution path was used.

Revision 2's supplied-array edge bound is in separate commit `557e42b5` (`fix: bound exact Whirlpool quotes to supplied tick arrays`). Its 10-second watchdog test covers both directions. Exhaustion reports `MissingTickArray`, which replay classifies as `insufficient_state`. The previously slow SOL/USDC case at slot 463131460, first leg pool `FpCM…`, 1,000 USDC units completed in less than the probe's 1 ms resolution; its second leg on `83v8…` returned `insufficient_state` in the same resolution. The stalled SOL/USDT case at slot 463131598 likewise completed or rejected immediately. No remeasured quote exceeded about 1 second.

The three v1 requests each name exactly their evaluated standard-fee pools: A `[SOL, USDC]`, six pools and 30 ordered two-leg routes; B `[SOL, USDT]`, five pools and 20 routes; C `[USDC, USDT]`, two pools and two routes. A quotes profit in USDC atoms; B and C in USDT atoms. Each route starts with the quote asset, buys base on one pool and sells base on a distinct pool. Token continuity is checked at each hop and direction follows each pool's mint order. Triangles await a versioned multi-pair v2 contract. Five adaptive-fee pools are outside this universe.

## Declared parameters

| Parameter | Default | Pair A sensitivity |
| --- | --- | --- |
| Quote-atom size ladder | 10,000,000; 100,000,000; 1,000,000,000 (10, 100, 1,000 six-decimal quote units) | Same |
| FixedUniverse size | 10,000,000 quote atoms | Same |
| Base fee | One signature, 5,000 lamports | Same |
| Priority fee | 1,000 micro-lamports/CU × 400,000 CU = 400 lamports | Same |
| Tip | 10,000 lamports | 100,000 lamports in cost run |
| Total SOL cost before conversion | 15,400 lamports | 105,400 lamports in cost run |
| Detection-to-simulation latency | L = 1 captured slot | L = 2 in latency run |
| SimpleRules threshold | At least 1,000 positive quote atoms; maximum one selected route per pool per slot | Same |
| ModelRanking | `ranking-v0-heuristic`, estimated net/input integer ratio, top K = 1, pool conflict constraint | Same |
| Seed/workers | Seed 0 recorded but unused; single-threaded deterministic replay | Same |

The cost model quotes the SOL-denominated cost into each pair's quote asset **at the evaluation slot** through an exact-in reference-pool quote. A uses SOL/USDC pool `83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d`; B and C use SOL/USDT pool `FwewVm8u6tFPGewAyHmWAqad9hmF7mvqxK4mJ7iNqqGC`. The conversion quote is a **price input only**: it is never traded and creates no route candidate. Both reference pools also occur as ordinary route legs in their own pair's evaluated universe, because those universes include all six A and all five B standard-fee pools. C's SOL/USDT reference is external to its two-pool `poolIds`. Each request's `poolIds` still contains exactly the pools evaluated as route legs; cost-model references are recorded separately in the manifest and `costModelVersion`.

Estimated net profit uses state at detection slot S. A selected candidate would be requoted at S+L for simulated net profit; missing later state would be marked `insufficient_state` or `state_gap`, never a win or loss. Selected candidates never alter replayed state. `missed` and realized profit are null. Every route at every observed boundary receives one selected or rejected bucket per policy. No randomness or parallelism is used, so no worker-count run applies.

## Default policy comparisons

All profits below are in each pair's quote atoms. Each policy saw the same candidate routes and cost model. The policies have identical counts here because no candidate had positive estimated net profit at any eligible size.

### A — SOL/USDC

| Policy | Evaluated | Detected | Selected | Below threshold | Insufficient state | State gap | Estimated net | Simulated net |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FixedUniverse | 79,560 | 0 | 0 | 54,138 | 4,141 | 21,281 | 0 | 0 |
| SimpleRules | 79,560 | 0 | 0 | 54,138 | 4,141 | 21,281 | 0 | 0 |
| ModelRanking | 79,560 | 0 | 0 | 54,138 | 4,141 | 21,281 | 0 | 0 |

Insufficient-state share: **5.20%**; state-gap share: **26.75%**. The remaining 68.05% were evaluable but below threshold. There were 30 route shapes × 2,652 boundaries.

### B — SOL/USDT

| Policy | Evaluated | Detected | Selected | Below threshold | Insufficient state | State gap | Estimated net | Simulated net |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FixedUniverse | 53,040 | 0 | 0 | 7,734 | 15,517 | 29,789 | 0 | 0 |
| SimpleRules | 53,040 | 0 | 0 | 7,734 | 15,517 | 29,789 | 0 | 0 |
| ModelRanking | 53,040 | 0 | 0 | 7,734 | 15,517 | 29,789 | 0 | 0 |

Insufficient-state share: **29.25%**; state-gap share: **56.16%**. Only 14.58% were evaluable and below threshold. There were 20 route shapes × 2,652 boundaries.

### C — USDC/USDT

| Policy | Evaluated | Detected | Selected | Below threshold | Insufficient state | State gap | Estimated net | Simulated net |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FixedUniverse | 5,304 | 0 | 0 | 650 | 1,577 | 3,077 | 0 | 0 |
| SimpleRules | 5,304 | 0 | 0 | 650 | 1,577 | 3,077 | 0 | 0 |
| ModelRanking | 5,304 | 0 | 0 | 650 | 1,577 | 3,077 | 0 | 0 |

Insufficient-state share: **29.73%**; state-gap share: **58.01%**. Only 12.25% were evaluable and below threshold. There were two route shapes × 2,652 boundaries.

All rejected counts sum to evaluated counts; no `pool_conflict`, `not_top_k`, `adaptive_fee_unsupported` or `invalid_quote` bucket occurred in these runs. The `ranking-v0-heuristic` entry is a labeled baseline to satisfy the v1 result shape. **No ML model was involved.** With no selections, simulated net is zero because there were no later-slot route simulations, not because estimated and simulated values were collapsed.

## Pair A sensitivities

| Run | L | Tip | Evaluated per policy | Detected / selected per policy | Rejections per policy (below / insufficient / gap) | Estimated / simulated net per policy |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Default | 1 | 10,000 lamports | 79,560 | 0 / 0 | 54,138 / 4,141 / 21,281 | 0 / 0 |
| Latency | 2 | 10,000 lamports | 79,560 | 0 / 0 | 54,138 / 4,141 / 21,281 | 0 / 0 |
| Cost | 1 | 100,000 lamports | 79,560 | 0 / 0 | 54,138 / 4,141 / 21,281 | 0 / 0 |

Each row applies to FixedUniverse, SimpleRules and ModelRanking. Since no route was selected, changing L cannot affect simulated totals in this sample. The higher tip did not change the rejection buckets or detection count. Sensitivities were not run for B or C, per Revision 1.

## Reproducibility and contract validation

The two full default runs, `handoff11-default-run2` and `handoff11-default-run3`, were byte-identical for every output file. The temporary `handoff11-default-run1` stopped before result emission because of a UTC formatting error and is **not** a completed artifact. That formatter was corrected in `88798c77`. The complete runs record Git commit `88798c77`, crate version `0.1.0`, pinned Rust toolchain `1.92.0-nightly`, seed 0 and full config. Their SHA-256 hashes are:

| Output | SHA-256 |
| --- | --- |
| A request | `e86bef8d5065c208f300e0f5427c7cd5f82bc83571246f0db26eb688fc359403` |
| A result | `563c1a8c66e9f20ef0c67cc7bea465e7a3ad736cd3c6fffda0a640be6045c21e` |
| B request | `90e3ca5feb6add08e4be3fb797562a6da82efe2aa0afd313c915f97e5a27ce77` |
| B result | `d1f766a368b519565dd42c679ef0333a30ec267bcc6e3e1d014f068ee4c18231` |
| C request | `82712d0a783de283c44f3957cb2004c416f46eff94448330c7a27eb07c9d13b6` |
| C result | `da116ffb4895738c519bf967091ed4960f9a01e031ed683e857c1835b386875f` |
| Full candidate JSONL | `ca2858659c51e4fdad5461b99ed6b043f6784efc5a198bdec2c00684937cc28a` |
| Summary | `3a76594fa6afd9d07a040a8a66dc743bdb63a8d01aa5de120673e5a7630b93b9` |
| Run manifest | `50f34ae2946254a6c22cc628871d999b6f7ccc3d918c7a1f9d0c5e3ab196afd1` |

The four input capture hashes are in the run manifest. The six small committed request/result fixtures are byte-identical to the default outputs. The C# validation theory is enabled for A, B and C and checks canonical request bytes, request syntax/semantics, result syntax/semantics and canonical result bytes against the existing unchanged v1 validators and a capture-backed test catalog entry. `dotnet test dotnet/MEVana.slnx --verbosity quiet` passed **197/197** tests: the 194 prior tests and three new fixture cases.

Handoff step 7 also passed on rustup Cargo `1.92.0-nightly`: `cargo fmt -p research-replay -- --check`; offline locked `cargo clippy -p research-replay --all-targets -- -D warnings`; offline locked `cargo test -p research-replay -p whirlpool-core -p geyser-capture`; `openspec validate --all --strict` (8/8); `./scripts/ci/repo_hygiene.sh`; `git diff --check`; and a non-printing credential-pattern scan over changed implementation and fixtures. Repo hygiene emits existing timestamp and script-placement warnings but exits successfully.

## Interpretation and next steps

This is a short, Processed, dev/test capture with incomplete tick-array coverage and gaps. B and C have particularly little evaluable state. Missing state was rejected, never assumed empty; adaptive-fee pools were excluded. No selected trades means the replay does not measure fill competition, transaction inclusion, execution slippage, or realized profit. It also cannot distinguish whether a strategy would have won a fill (F8). The equal policy results here give no evidence that ranking beats the two baselines.

For a more informative next experiment, capture an initial tick-array state snapshot (requiring a separately authorized RPC/provider plan), gather a longer capture on a server host, reconcile Processed with stronger commitments, then repeat the same deterministic analysis. A learned ranker belongs to a later slice. Triangles need the v2 request contract first. No provider access was used for this handoff.
