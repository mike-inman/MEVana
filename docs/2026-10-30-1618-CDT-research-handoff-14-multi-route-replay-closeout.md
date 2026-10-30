# 2026-10-30 16:18 CDT — Handoff 14 multi-route replay closeout

Created: 2026-10-30 16:18 CDT.

## Scope and answer

This was an offline replay of the handoff 12 format-v2 capture, using the R2 USDC-quoted universe: 120 triangles and 32 two-leg routes across 13 standard-fee pools. The capture supplied 2,235 Confirmed boundaries, no true gaps, and a roughly eight-minute dev/test Processed stream recorded on a laptop. Its capture and the handoff 12 baseline were read-only. No provider call, new capture, credential, `.env`, trade, or network access was used after the operator-authorized `git fetch origin`.

**Triangles added no detected or selected opportunity in this capture.** For each default policy, 187,236 of 268,200 triangle candidates (69.81%) reached `below_threshold` after a successful quote; 80,964 (30.19%) lacked sufficient executable state or had a proven absent tick array under FixedUniverse. The two detected opportunities were two-leg routes. Their selected later-slot simulated net was negative. This answers only for the observed, evaluable candidates. The missing triangle coverage is material, so the run cannot rule out opportunities in the uncaptured state.

All money figures below are integer USDC atoms. Estimated profit is from slot S and simulated profit is from S+L; neither is realized profit. The 600,000-CU triangle cost is an explicit assumption (200,000 CU per leg), with 400,000 CU for two legs. Both use the same base fee, priority fee, tip, and SOL-to-USDC reference conversion through `83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d`. ModelRanking is the `ranking-v0-heuristic` policy; **no ML or learned model was involved**.

## Gates and provenance

The ordered route fixture came from the .NET v2 enumerator and matches the Rust enumerator byte for byte: 120 triangles plus 32 two-leg routes. The final binary was built from `856e7930` with rustup Cargo `1.92.0-nightly`. All three real-data regression gates passed before these results were interpreted:

1. V1 `default` rerun: all eight request/result/candidates/summary files were byte-identical to `handoff12-snapshot-default1`; its manifest differed only in `gitCommit`.
2. All 67,050 SOL/USDC two-leg candidate rows in v2 had the same per-size `SizeQuote` values as the corresponding v1 pair A rows at the same boundary and ordered pools.
3. Two `v2-default` runs produced byte-identical request, result, candidates, summary, and manifest files.

The committed v2 request and result fixtures are byte-identical to the final default run. The request SHA-256 is the result's `requestSha256`. The unchanged v1 syntax and golden tests pass, and the v2 result validates through the existing v2 request/catalog overload.

| Default artifact | SHA-256, identical in both runs |
| --- | --- |
| `request.json` | `63788e9af74985533851fbeb6f2d24481d3f6c87e3acadcecb7387ef2b713953` |
| `result.json` | `a4223ade793105e3c060a6b1d711d02993f94161312e8ef44491f5d94bf4dd09` |
| `candidates.jsonl` | `5cca6ed9d8dd485ff723d4b7aed36ca45bb7a1171b7c321a3367c1363be89e26` |
| `summary.json` | `8edff656b82130ee7006d90687b3a303a85052b0fd20f52482b30b3e32c5f599` |
| `run-manifest.json` | `6380c1f7384cb6924a65f7699d3b2be87541a82ec3d5282bc5bfca54d8ce0b02` |

Run directories: `~/MEVanaData/replay/handoff14-final-v1-default` and `handoff14-final-v2-{default1,default2,latency2,cost,processed}`. The default manifest records the format-v2 input hashes, engine/config, 400,000/600,000 CU assumptions, Confirmed mode, L=1, and the reference pool. Candidate files remain outside Git.

## Default policy comparison

Rejections are shown as `below_threshold / insufficient_state / tick_array_absent`; no other rejection reason occurred in this run. Each policy evaluated 339,720 candidates, comprising 268,200 triangles and 71,520 two-leg routes. Shape buckets reconcile exactly to policy totals.

| Shape | Policy | Evaluated | Detected | Selected | Rejections (below / insufficient / absent) | Estimated net | Simulated net |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Triangle | FixedUniverse | 268,200 | 0 | 0 | 187,236 / 54,216 / 26,748 | 0 | 0 |
| Triangle | SimpleRules | 268,200 | 0 | 0 | 187,236 / 58,674 / 22,290 | 0 | 0 |
| Triangle | ModelRanking | 268,200 | 0 | 0 | 187,236 / 58,674 / 22,290 | 0 | 0 |
| Two leg | FixedUniverse | 71,520 | 2 | 2 | 71,326 / 192 / 0 | 1,582 | -5,191 |
| Two leg | SimpleRules | 71,520 | 2 | 1 | 71,327 / 192 / 0 | 1,117 | -2,269 |
| Two leg | ModelRanking | 71,520 | 2 | 1 | 71,327 / 192 / 0 | 1,117 | -2,269 |

The fixed policy selected both detected two-leg opportunities. SimpleRules and heuristic ModelRanking each selected one. These observed selections provide no evidence that ranking beat the simple baseline; their estimated and simulated totals coincide here.

## Sensitivities

Each cell is `evaluated / detected / selected / estimated net / simulated net`. Triangle results were zero for detection, selection, and both profit measures in every sensitivity. The `v2-cost` profile raises the tip to 100,000 lamports; `v2-latency2` uses L=2; `v2-processed` applies Processed state.

| Profile | Shape | FixedUniverse | SimpleRules | ModelRanking |
| --- | --- | --- | --- | --- |
| L=2 | Triangle | 268,200 / 0 / 0 / 0 / 0 | 268,200 / 0 / 0 / 0 / 0 | 268,200 / 0 / 0 / 0 / 0 |
| L=2 | Two leg | 71,520 / 2 / 2 / 1,582 / -5,191 | 71,520 / 2 / 1 / 1,117 / -2,269 | 71,520 / 2 / 1 / 1,117 / -2,269 |
| Higher tip | Triangle | 268,200 / 0 / 0 / 0 / 0 | 268,200 / 0 / 0 / 0 / 0 | 268,200 / 0 / 0 / 0 / 0 |
| Higher tip | Two leg | 71,520 / 0 / 0 / 0 / 0 | 71,520 / 0 / 0 / 0 / 0 | 71,520 / 0 / 0 / 0 / 0 |
| Processed | Triangle | 268,440 / 0 / 0 / 0 / 0 | 268,440 / 0 / 0 / 0 / 0 | 268,440 / 0 / 0 / 0 / 0 |
| Processed | Two leg | 71,584 / 2 / 2 / 1,582 / -5,191 | 71,584 / 2 / 1 / 1,117 / -2,269 | 71,584 / 2 / 1 / 1,117 / -2,269 |

L=2 had the same policy and shape totals as default. Higher tip removed the two detections. Processed mode yielded 2,237 boundaries and two more initially insufficient-state boundaries; it did not change the selected opportunities or profit. In Processed mode, triangle `insufficient_state` rejections increase by 240 per policy; two-leg `insufficient_state` increases by 64 per policy. There were no true gaps in any run.

## Pool and direction coverage

The diagnostic probe independently quoted each pool in both directions at raw input amounts 10,000,000, 100,000,000, and 1,000,000,000 atoms on every Confirmed boundary. Thus every row has 6,705 attempts. These raw input atoms are **diagnostic probes**, not necessarily the amount that a later route leg receives after the previous leg. Route outcomes still use exact chained amounts and report the first failing leg. Pool prefixes below are unique within the 13-pool request; full IDs are in `summary.json` and the request.

| Pool prefix | Direction | Success | Insufficient state | Tick array absent |
| --- | --- | ---: | ---: | ---: |
| `21gTfxAn` | USDC→SOL | 4,458 | 2,247 | 0 |
| `21gTfxAn` | SOL→USDC | 6,687 | 18 | 0 |
| `4fuUiYxT` | USDC→USDT | 6,702 | 3 | 0 |
| `4fuUiYxT` | USDT→USDC | 6,702 | 3 | 0 |
| `7qbRF6Ys` | USDC→SOL | 6,687 | 18 | 0 |
| `7qbRF6Ys` | SOL→USDC | 6,687 | 18 | 0 |
| `83v8iPyZ` | USDC→SOL | 6,687 | 18 | 0 |
| `83v8iPyZ` | SOL→USDC | 6,687 | 18 | 0 |
| `AhhoxZDm` | USDT→SOL | 6,687 | 18 | 0 |
| `AhhoxZDm` | SOL→USDT | 2,229 | 18 | 4,458 |
| `B6LL9aCW` | USDT→SOL | 4,458 | 2,247 | 0 |
| `B6LL9aCW` | SOL→USDT | 6,687 | 18 | 0 |
| `BSddxwYW` | USDC→SOL | 6,687 | 18 | 0 |
| `BSddxwYW` | SOL→USDC | 6,687 | 18 | 0 |
| `BmXVhzBH` | USDT→SOL | 0 | 6,705 | 0 |
| `BmXVhzBH` | SOL→USDT | 512 | 6,193 | 0 |
| `Czfq3xZZ` | USDC→SOL | 6,705 | 0 | 0 |
| `Czfq3xZZ` | SOL→USDC | 6,705 | 0 | 0 |
| `ELqXddRq` | USDC→USDT | 4,458 | 18 | 2,229 |
| `ELqXddRq` | USDT→USDC | 4,458 | 18 | 2,229 |
| `FpCMFDFG` | USDC→SOL | 6,696 | 9 | 0 |
| `FpCMFDFG` | SOL→USDC | 6,696 | 9 | 0 |
| `FwewVm8u` | USDT→SOL | 6,687 | 18 | 0 |
| `FwewVm8u` | SOL→USDT | 6,687 | 18 | 0 |
| `HcoJqG32` | USDT→SOL | 2,229 | 4,476 | 0 |
| `HcoJqG32` | SOL→USDT | 4,458 | 2,247 | 0 |

The largest SOL/USDT limitations are `BmXVhzBH`, with no USDT→SOL diagnostic success, and `HcoJqG32`, with only one-third USDT→SOL success. `AhhoxZDm` has proven absent arrays in two-thirds of SOL→USDT probes; `B6LL9aCW` has one-third insufficient USDT→SOL probes. `ELqXddRq` has absent arrays in one-third of probes in either USDC/USDT direction. These differences explain why the triangle evaluable share cannot be inferred from the old pair B two-pool figure. The 69.81% route-level figure comes from actual chained triangle outcomes, not the independent probes.

## Validation and limitations

- `~/.cargo/bin/cargo -V`: `cargo 1.92.0-nightly (24bb93c38 2025-09-10)`.
- `cargo fmt -p research-replay --check`: passed. `cargo fmt --all --check`: **failed on pre-existing formatting in six untouched files** in api-service, executor-service, hotpath, kafka_db_writer, monitoring-service, and shared-core. Those files are byte-identical to `origin/main`; this slice did not reformat out-of-scope code.
- `cargo clippy -p research-replay --all-targets --offline --locked -- -D warnings`: passed.
- `cargo test -p research-replay -p geyser-capture -p whirlpool-core --offline --locked`: passed, including parity, multi-leg failure, accountancy, capture refusal, output refusal, exact quotes and golden swaps.
- `dotnet build dotnet/MEVana.slnx --no-restore`: passed, zero warnings/errors. `dotnet test dotnet/MEVana.slnx --no-build --no-restore`: 217 passed, none failed/skipped.
- `openspec validate --all --strict`: passed. `./scripts/ci/repo_hygiene.sh`: passed with existing documentation timestamp and script-location warnings. `git diff --check` and `git diff origin/main --check`: passed. A non-printing credential scan over the branch diff found no potential credential value.

V1 behavior is unchanged: its Rust artifacts passed the byte-level regression gate and the existing .NET golden tests passed unedited. The only existing v1 .NET file edit extracts its JSON writer into an internal shared helper without changing the validation or bytes. No existing v1 fixture changed.

The evidence is limited to roughly eight minutes of dev/test Processed data on a laptop, with Confirmed replay as the default; missing tick-array state leaves 30.19% of triangle candidates unevaluable. The 600,000-CU triangle cost is assumed rather than measured. The simulation has no fill, inclusion, execution, or adverse selection model, and no realized profit. ModelRanking is a labeled heuristic, not ML. A longer capture with stronger commitment and better tick-array coverage is needed before drawing a broader market conclusion.

The OpenSpec change `multi-route-replay` is archived with its specs synced. This branch remains separate from `main`; merging is the operator's decision.
