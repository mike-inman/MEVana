# 2026-10-30 11:39 CDT — Research handoff 12 closeout: slot-correct replay and one bounded snapshot capture

## Outcome

Handoff 12 produced three new, capture-backed `Simulated` v1 experiment results after correcting slot semantics and adding one opt-in tick-array snapshot. All three validate under the **unchanged** C# request/result contracts. On this short capture, pair A had two positive **estimated** opportunities. Their later-slot **simulated** totals were negative. This does not establish fill, inclusion, realized profit or a durable strategy edge. Trading stayed disabled. Merging this branch to `main` is the operator's decision.

The OpenSpec change is `capture-snapshot-slot-semantics`. The slot-correct replay implementation and its offline gate were committed at `9d0028c9` before any provider use. The snapshot capture/replay implementation was committed at `65723ead` before the one live session. Rust validation used rustup Cargo `1.92.0-nightly`; no Homebrew Rust result is used. The handoff 11 fixture files and results were left unchanged.

## Part A — offline slot correction on handoff 09

The [dated Part A gate report](2026-10-30-1112-CDT-research-handoff-12-part-a-offline-gate.md) records the pre-network tests and trial runs. The replay now builds a connection-0 graph from each slot's parent and first Processed, Confirmed, Finalized and Dead event time/sequence. Confirmed is the default: it applies buffered writes in slot/write-version/capture-sequence order only when that slot first reaches Confirmed, evaluates in confirmation order, and counts L in confirmed boundaries. Processed remains an explicit sensitivity mode. A parent before the first evaluated slot is outside the replay window, so it is not an internal data-loss gap.

The old capture's eight recorded discontinuities classify as **seven skipped leader slots, one abandoned fork and zero true gaps**. Skips do not stale state. Confirmed replay dropped writes in slots 463132684 (1), 463132685 (1) and 463132686 (6), plus 463132687 (4) and unconfirmed tail slots 463134134 (1) and 463134135 (4). No dropped write entered confirmed state. True gaps would stale pools until a later pool write or a covering post-gap snapshot; neither old nor new session had a true gap, so that recovery rule did not affect these outcomes. The old capture's 2,647 confirmed boundaries produced no positive estimates under any policy.

| Pair | Handoff 11 state gap | Corrected state gap | Handoff 11 insufficient | Corrected insufficient | Handoff 11 evaluable | Corrected evaluable |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A SOL/USDC | 26.75% | 0% | 5.20% | 5.74% | 68.05% | 94.26% |
| B SOL/USDT | 56.16% | 0% | 29.26% | 72.41% | 14.58% | 27.59% |
| C USDC/USDT | 58.01% | 0% | 29.73% | 40.16% | 12.25% | 59.84% |

The increased insufficient-state share for B and C is the honest reclassification of candidates previously hidden by false gaps. A processed-mode sensitivity on the old capture evaluated 2,651 boundaries and 79,530 A candidates per policy; it detected/selected none and had 4,560 insufficient-state and 74,970 below-threshold candidates, with zero state gaps. Two identical old-capture default runs were byte-identical. Commit-pinned final runs are under `~/MEVanaData/replay/handoff12-partA-final1/`, `handoff12-partA-final2/` and `handoff12-partA-processed-final/`.

## Part B — one snapshot-backed capture

Exactly one live session ran: `capture-1793377454811581000` under `~/MEVanaData/captures/handoff12-20261030/`, for 480.8 seconds including finalization. It used the same 18-pool allowlist, Sodae TLS Geyser stream at Processed commitment, and the fixed HTTPS Tatum RPC endpoint only for the opt-in snapshot. The plaintext Sodae RPC/Geyser configuration was not used. Pre- and post-run audits found no leftover capture process or capture-owned provider socket; the metrics listener also closed. No other live capture or RPC preflight was run in this slice.

| Capture measure | Observed | Limit/context |
| --- | ---: | --- |
| Finalized files | 4 | No `.part` files; inspector passed |
| Sodae Geyser payload bytes | 175,053,705 | About 130 MB had been used before this run; roughly 305 MB total payload remains well below the 5 GB / 7-day trial budget. Transport overhead is additional. |
| Gaps / stalls | 0 / 0 | No reconnect snapshot was needed |
| Snapshot count | 1 | ≤5 |
| Tatum RPC calls | 5 | ≤6 per snapshot |
| Tatum response bytes | 2,101,543 | ≤25,000,000 per session |
| Snapshot pool accounts | 18 present | All allowlisted pools |
| Snapshot tick-array accounts | 188 present, 46 proven absent | 13 requested offsets (−6 through +6 array spans) per pool, 234 keys total |

The pool query used Confirmed/base64 and each array batch used `minContextSlot` at least the pool query's context. The client used a 10-second timeout, at most two retries, no retry on 401/403, rustls, a fixed HTTPS host and redirects disabled. The key was read only through the existing environment-before-approved-root-`.env` rule, sent as `x-api-key` only to Tatum, and absent from the committed artifacts. Kind-2 JSON frames record pool/array key, context, presence, and for present accounts Whirlpool owner, lamports, raw-data base64 and SHA-256; stats record calls/bytes. File manifests use formatVersion 2 and count those frames. `capture_inspect` verified hashes, lengths, sequence, account identities and counts. Old format-v1 captures still inspect and replay.

Snapshot state is applied at its captured receive event with its confirmed context slot C. A stream write for the same account at slot ≤ C is ignored; a later write applies. A proven-absent tick-array PDA is classified `tick_array_absent` only when the exact quoter asks for it; an unseen array remains `insufficient_state`. No array was assumed empty. The new capture had no true gap, so the post-gap snapshot recovery rule was not exercised by real data; synthetic tests cover the rule's state handling.

## New replay policy results

The two default runs `handoff12-snapshot-default1` and `handoff12-snapshot-default2` are byte-identical. Both use the same fixed handoff 11 ladder of 10/100/1,000 quote units, 5,000-lamport base fee, 1,000 micro-lamports per CU × 400,000 CU, 10,000-lamport tip, L=1 confirmed slot, SimpleRules minimum 1,000 quote atoms, ranking top K=1 and unused seed 0. Exact integer Whirlpool quotes and quote-asset cost conversion remain unchanged. A uses the SOL/USDC `83v8…` reference; B and C use SOL/USDT `Fwew…`. Conversion is a price input only; a reference pool can independently be a normal route leg in its own pair's evaluated universe. A/B/C evaluate 30/20/2 ordered same-pair two-leg routes. Triangles and five adaptive-fee pools remain excluded.

The new result window is `2026-10-30T16:24:16.0436210Z` to `2026-10-30T16:32:14.5720580Z`, inside the catalog's capture coverage. There were 2,235 confirmed evaluation boundaries, zero true gaps and 13 unconfirmed tail writes dropped (11 in slot 463353643 and two in 463353644). Every policy's rejected counts plus selected count equal evaluated count. All profits below are in that pair's quote atoms (USDC for A; USDT for B/C). `missed` and realized profit are null.

### A — SOL/USDC

| Policy | Evaluated | Detected | Selected | Below threshold | Insufficient | Estimated net | Simulated net |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FixedUniverse | 67,050 | 2 | 2 | 66,868 | 180 | +1,582 | −5,191 |
| SimpleRules | 67,050 | 2 | 1 | 66,869 | 180 | +1,117 | −2,269 |
| ModelRanking (`ranking-v0-heuristic`) | 67,050 | 2 | 1 | 66,869 | 180 | +1,117 | −2,269 |

Insufficient state is **0.27%**. There is no state-gap or proven-absent rejection. The two positive estimates are tiny; later-slot requotes turned selected totals negative. These are simulated route outcomes, not fills.

### B — SOL/USDT

| Policy | Evaluated | Detected | Selected | Below threshold | Insufficient | Proven absent | Estimated / simulated net |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FixedUniverse | 44,700 | 0 | 0 | 20,061 | 17,952 | 6,687 | 0 / 0 |
| SimpleRules | 44,700 | 0 | 0 | 20,061 | 22,410 | 2,229 | 0 / 0 |
| ModelRanking | 44,700 | 0 | 0 | 20,061 | 22,410 | 2,229 | 0 / 0 |

The **evaluable share is 44.88%**, still thin. FixedUniverse has 40.16% insufficient and 14.96% proven-absent rejections; the ladder policies have 50.13% insufficient and 4.99% proven-absent, because candidate reason bucketing considers all quoted sizes. A proven-absent array means that requested path was not executable at the snapshot context. It is not an assumed zero-liquidity fill. No state gaps occurred.

### C — USDC/USDT

| Policy | Evaluated | Detected | Selected | Below threshold | Insufficient | Estimated / simulated net |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| FixedUniverse | 4,470 | 0 | 0 | 4,458 | 12 | 0 / 0 |
| SimpleRules | 4,470 | 0 | 0 | 4,458 | 12 | 0 / 0 |
| ModelRanking | 4,470 | 0 | 0 | 4,458 | 12 | 0 / 0 |

The evaluable share is **99.73%**; 0.27% were insufficient before snapshot state was available. No state-gap or proven-absent rejection occurred.

### Pair A sensitivities

| Run | L / mode | Tip | FixedUniverse detected / selected; estimated / simulated | SimpleRules and ModelRanking detected / selected; estimated / simulated |
| --- | --- | ---: | --- | --- |
| Default | 1 confirmed | 10,000 lamports | 2 / 2; +1,582 / −5,191 | 2 / 1; +1,117 / −2,269 |
| Latency | 2 confirmed | 10,000 lamports | 2 / 2; +1,582 / −5,191 | 2 / 1; +1,117 / −2,269 |
| Cost | 1 confirmed | 100,000 lamports | 0 / 0; 0 / 0 | 0 / 0; 0 / 0 |
| Processed | 1 processed | 10,000 lamports | 2 / 2; +1,582 / −5,191 | 2 / 1; +1,117 / −2,269 |

The L=2 requotes matched L=1 in this sample, consistent with unchanged relevant state across those adjacent confirmation boundaries. The higher tip removed both positive estimates. Processed mode evaluated 2,237 boundaries and 67,110 A candidates per policy, with 240 insufficient-state rejections; confirmed default evaluated 2,235 and 67,050, with 180 insufficient. No sensitivity run was requested for B or C.

## Determinism and contract checks

The default runs used engine `research-replay-0_2_0`, crate `0.2.0`, Git commit `65723ead` and rustup `1.92.0-nightly`. SHA-256 hashes below matched in both runs, including every byte of the candidate JSONL and run manifest:

| Output | SHA-256 |
| --- | --- |
| A request | `474dec245a21450366f9339b20d58ffc3962c04f5c99e049e7d33bd2c9a8c373` |
| A result | `00d36a8a44df3ed53b1c475b045fc0cf13470e888cefa472fd73265581191a5f` |
| B request | `7c20450f5289fa9c1f19804f49e161b4297a900f8ac3ba47b1c5f129ce91117d` |
| B result | `bd1bfa458a60934e8b5aa4fdf0eed5ddc89c414ef4d1f176177128582cb0fd47` |
| C request | `868c9996c6d491d53c0ddc5adb957b19c796ded330df23d1d000723b977ac414` |
| C result | `b211afe876d4769d15b242d6a6ef52e96b86129ea2a6512c2cc3c5a925c07d85` |
| Candidate JSONL | `c8912162ce60849bd68d48c983d3afd18ce0e04d0e93c72ea51521a851e8a7d2` |
| Summary | `769da20d3af2de28c728dd8f49c14eff6b6ba82f15587b207a4920d11685ad33` |
| Run manifest | `562152e7ae395099368b214f244b478606df8815e82d7d5ff593a9572d546e41` |

The six committed new request/result JSON files are byte-identical to the first default run. New C# tests canonicalize each request, compare its hash through the unchanged validators, validate all three results against the capture-backed catalog entry, and compare canonical result bytes. The `tick_array_absent` free reason-code check also passed. The full .NET suite passed **201/201**, including the 194 original tests, three handoff 11 results, the new reason-code test and three handoff 12 results.

## Validation and limits

The pinned validation list passed: Cargo format check; offline locked Clippy `-D warnings`; offline locked tests for `geyser-capture`, `research-replay` and `whirlpool-core`; the .NET suite; strict OpenSpec validation; repository hygiene (existing timestamp/script-placement warnings); `git diff --check`; and non-printing credential scans of the diff and the new capture's manifests/log. The exact quoter and all forbidden crates/services were unchanged. Only the authorized Sodae stream and bounded Tatum calls used network access; neither remained running.

This is a **dev/test Processed feed**, about eight minutes on a laptop, not a production or long-horizon sample. Confirmed replay follows captured status order but the underlying stream has Processed account data. The replay cannot model fill competition, transaction inclusion, slippage or realized P&L. The two A estimates are too small and too few to support ranking claims; both simulated outcomes were negative. B remains materially limited by unseen and absent arrays even with a 13-offset snapshot. `ModelRanking` is the labeled `ranking-v0-heuristic`; **no ML model was involved**. A longer, stronger-commitment capture on a server host and a more complete state acquisition plan are the next research steps. Triangles still require a v2 request contract; a learned ranker belongs to a later slice.
