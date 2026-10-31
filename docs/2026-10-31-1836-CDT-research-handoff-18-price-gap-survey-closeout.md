# 2026-10-31 18:36 CDT — Handoff 18 price-gap survey closeout

Created: 2026-10-31 18:36 CDT

## Scope and collection

The worktree `price-gap-survey-20261031` was created from local `main` `9124ecd4`, which contained merged handoff 17. The survey used the 40-account allowlist across 13 token pairs and Orca Whirlpool, Raydium CLMM and Meteora DLMM. Published layouts, real mainnet account fixtures, exact owners, mints, decimals and fee configuration were verified before streaming. The subscription contained only those pool accounts at Confirmed commitment. No transaction, tick-array, bin-array, wallet, signing or submission path was added.

The operator reported 964 MB of a 1.0 GB Sodae allowance remaining before the pilot. The pilot projected 37,825,877 payload bytes for 185 minutes from its 1,022,321 bytes in five minutes. Every run had a 100,000,000-byte hard limit. Actual total was **30,832,248 payload bytes** (24,278 updates), under both the reported provider allowance and the 1.5 GB handoff limit. No run reached its byte cap; all stopped at their time limit with zero gaps and zero stalls. Pre- and post-run checks found no leftover survey process or provider socket. Runs finished on 2026-10-31, before the 2026-11-04 deadline. The local research stack is left stopped with volumes kept.

| Run | Local start, CDT | Minutes | Payload bytes | Orca updates | Raydium updates | Meteora updates |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Pilot `pilot-20261031-1146` | 11:46 | 5 | 1,022,321 | 397 | 376 | 15 |
| Main `main-20261031-1152` | 11:52 | 30 | 5,879,615 | 2,770 | 1,648 | 518 |
| Main `main-20261031-1223` | 12:23 | 30 | 4,980,279 | 1,813 | 1,747 | 300 |
| Main `main-20261031-1253` | 12:53 | 30 | 5,390,690 | 1,981 | 1,884 | 321 |
| Main `main-20261031-1324` | 13:24 | 30 | 4,027,747 | 1,536 | 1,401 | 207 |
| Main `main-20261031-1355` | 13:54 | 30 | 5,340,244 | 1,636 | 2,028 | 317 |
| Main `main-20261031-1800` | 18:00 | 30 | 4,191,352 | 1,705 | 1,305 | 373 |
| **Total** | Morning, afternoon, evening | **185** | **30,832,248** | **11,838** | **10,389** | **2,051** |

Raw recordings, manifests, corrected analyses, gate output and the rendered dashboard screenshot are under `~/MEVanaData/captures/handoff18-20261031/`, outside Git. The screenshot is `dashboard-final.png` (SHA-256 `5eaf4c10ac0b965b55f976063328b335ab6df9ab883c1b5a2da61779510096dc`). The first analysis exposed reverse mint ordering in some Meteora pools. The initial derived files were preserved; the final files named `analysis-normalized.json` use consistent pair direction. No raw capture was changed or repeated. Two invocations of the corrected analyzer produced byte-identical output for each run.

## Predeclared hypotheses

The analysis counted 61,499 pool-pair observations. It excluded 39,846 (64.79%) as older than 25 slots, leaving 21,653 nonstale observations; 14,691 of those had both fees known. It found two 5 bps net-gap events, both SOL/USDT Orca/Raydium in the 12:23 run, each visible for one slot, with peaks of 6.099 and 6.396 bps. It found **zero** 10, 25 or 50 bps net-gap events.

| Id | Result | Limit |
| --- | --- | --- |
| H1, cross venue more frequent than within Orca | At 10 bps, 0 events in 11,466 known-fee cross-venue observations and 0 in 3,225 same-venue observations. No direction established. | Same-venue coverage is only the second Orca SOL/USDC pool; the groups are unbalanced. |
| H2, less popular tokens have larger gaps | Core had 13,733 known-fee observations and two 5 bps events; Established had 917 and Volatile 41, with no 5 bps events. All groups had zero 10 bps events. Largest measured gross gaps were 9.396, 71.602 and 88.798 bps for Core, Established and Volatile respectively; largest measured net gaps were 6.396, 2.190 and -23.421 bps. The net-event hypothesis is not established. | Gross maxima do not substitute for net events. Sparse updates, stale exclusions and unmodeled variable fees limit the other groups. |
| H3, gaps widen during volatility | At 10 bps, zero events in 14,405 observations below a 10 bps rolling SOL move and 265 observations in the 10–50 bps bucket. The above-50 bps bucket had zero observations; 21 observations lacked a bucket. No comparison supporting the hypothesis is possible. | No high-volatility sample and no 10 bps event. |
| H4, gaps follow sharp pool moves | At 10 bps, zero events in 41 observations within 10 slots after a sharp move and zero in 14,650 baseline observations. No difference established. | The after-move exposure is small; this is a pool-price move proxy, not a transaction stream. |

## Fixed decision gate

The gate used the unchanged thresholds: at least three runs with nonstale observations, at least six 10 bps net events per hour, median visible duration at least two slots, both fees known and stale exclusions below 20%. It enumerated all 40 allowlist token-pair and venue segments, including never-observed segments. **None advances**: 13 fail and 27 have too little data. Every eligible segment has zero 10 bps events; the two 5 bps events also lasted only one slot. “Runs” below counts runs with at least one nonstale observation. N/M means no stale percentage could be measured.

| Pair | Venues | Runs | Gate | Stale exclusions % | Both fees known |
| --- | --- | ---: | --- | ---: | --- |
| BONK/SOL | Meteora/Orca | 2 | too_little_data | 66.7 | no |
| BONK/SOL | Meteora/Raydium | 2 | too_little_data | 86.7 | no |
| BONK/SOL | Orca/Raydium | 2 | too_little_data | 76.9 | yes |
| BONK/USDC | Meteora/Orca | 2 | too_little_data | 87.0 | no |
| BONK/USDC | Meteora/Raydium | 4 | fail | 83.7 | no |
| BONK/USDC | Orca/Raydium | 2 | too_little_data | 77.0 | yes |
| JTO/SOL | Meteora/Orca | 1 | too_little_data | 98.5 | no |
| JTO/SOL | Meteora/Raydium | 0 | too_little_data | 100.0 | no |
| JTO/SOL | Orca/Raydium | 1 | too_little_data | 90.0 | yes |
| JTO/USDC | Meteora/Orca | 0 | too_little_data | N/M | no |
| JTO/USDC | Meteora/Raydium | 0 | too_little_data | N/M | no |
| JTO/USDC | Orca/Raydium | 0 | too_little_data | N/M | no |
| JUP/SOL | Meteora/Orca | 5 | fail | 79.9 | no |
| JUP/SOL | Meteora/Raydium | 4 | fail | 91.3 | no |
| JUP/SOL | Orca/Raydium | 7 | fail | 59.1 | yes |
| JUP/USDC | Meteora/Orca | 7 | fail | 85.2 | no |
| JUP/USDC | Meteora/Raydium | 1 | too_little_data | 99.4 | no |
| JUP/USDC | Orca/Raydium | 2 | too_little_data | 94.1 | yes |
| PYTH/SOL | Meteora/Orca | 1 | too_little_data | 96.8 | no |
| PYTH/SOL | Meteora/Raydium | 0 | too_little_data | 100.0 | no |
| PYTH/SOL | Orca/Raydium | 7 | fail | 71.4 | yes |
| PYTH/USDC | Meteora/Orca | 0 | too_little_data | N/M | no |
| PYTH/USDC | Meteora/Raydium | 0 | too_little_data | N/M | no |
| PYTH/USDC | Orca/Raydium | 0 | too_little_data | N/M | no |
| SOL/USDC | Meteora/Orca | 7 | fail | 74.4 | no |
| SOL/USDC | Meteora/Raydium | 7 | fail | 71.8 | no |
| SOL/USDC | Orca/Orca | 7 | fail | 60.6 | yes |
| SOL/USDC | Orca/Raydium | 7 | fail | 50.3 | yes |
| SOL/USDT | Meteora/Orca | 0 | too_little_data | N/M | no |
| SOL/USDT | Meteora/Raydium | 0 | too_little_data | N/M | no |
| SOL/USDT | Orca/Raydium | 7 | fail | 54.0 | yes |
| USDC/USDT | Meteora/Orca | 1 | too_little_data | 98.5 | no |
| USDC/USDT | Meteora/Raydium | 1 | too_little_data | 97.8 | no |
| USDC/USDT | Orca/Raydium | 3 | fail | 91.4 | yes |
| WIF/SOL | Meteora/Orca | 3 | fail | 93.6 | no |
| WIF/SOL | Meteora/Raydium | 0 | too_little_data | 100.0 | no |
| WIF/SOL | Orca/Raydium | 0 | too_little_data | 100.0 | no |
| WIF/USDC | Meteora/Orca | 0 | too_little_data | N/M | no |
| WIF/USDC | Meteora/Raydium | 0 | too_little_data | N/M | no |
| WIF/USDC | Orca/Raydium | 0 | too_little_data | N/M | no |

## Research store and dashboard

The repeatable transactional loader produced 7 `gap_runs`, 280 `gap_pools`, 2,640 `gap_minute_stats`, 2 `gap_events`, 40 `gap_gate_results` and 66 `gap_observation_buckets` rows. Reloading the same seven analyses and gate left all counts unchanged. Every new row is labeled `Indicative`. The provisioned dashboard was rendered in headless Chrome at 1800 × 3500, and every SQL frame returned without error. Tables remain scrollable for all returned rows; measured zeros display as 0, while missing values display as Not measured. No green or red judgment color is used.

| Panel | Rendered evidence |
| --- | --- |
| Header | The required Indicative mid-price limitation and visible-slot, timestamp and variable-fee caveats. |
| Run health | 21 rows: each of seven windows by three venues, with bytes, zero gaps/stalls and update rates. |
| Events per hour by group and threshold | 64 rows: two Core events at 5 bps in one run; measured zeros at 10, 25 and 50 bps. |
| Cross venue versus same venue (H1) | 10 measured pair/venue rows; zero 10 bps event rates. |
| Net gap heatmap | 553 token-pair/minute rows, with a real time column and blue-shaded measured net values. Unknown-fee rows are absent, not shown as zero. |
| Duration distribution | 16 threshold/duration bins; two events in the 5 bps, one-slot bin, measured zeros in the others. |
| Volatility and post-move rates (H3/H4) | Five measured or explicitly unknown cohorts; zero 10 bps event rates. |
| Top pool pairs | 11 known-fee pool pairs ordered by their measured 10 bps event rate; all rates zero. |

## What this evidence cannot show

A mid-price difference is not an executable quote. This survey omits depth, slippage, network fees, tips, inclusion, competition and execution latency. Laptop receipt order and provider timestamps were not used as trading latency. Visible duration in slots is not a tradable window. Meteora variable fees were not modeled, so net gaps involving that unknown fee remain NULL. Sparse updates left 64.79% of pair observations stale, and JTO/USDC, PYTH/USDC and WIF/USDC had no paired observations. The one-day, three-time-of-day sample does not establish behavior on other days or during the absent above-50 bps volatility bucket. Newly created pools, constant-product pools, tick/bin arrays and transaction effects were outside this survey.

## Determinism and validation

The corrected analysis SHA-256 values (each matched a second independent invocation) are:

| Run | SHA-256 of `analysis-normalized.json` |
| --- | --- |
| `main-20261031-1152` | `7d8174c4f3771756e35bac91222ff306f40d8decced83382a3208a77a3f2e4cc` |
| `main-20261031-1223` | `68ef08597e0e508e664aa21b5cc9266fcad8aeaec9c964d6b6409f01c3e05f16` |
| `main-20261031-1253` | `9dc839710f5073938f5a22ac5099d067a31e202a713230e6ab434e1fbe770cb0` |
| `main-20261031-1324` | `c75ba3691b05bb7ac76252f38b61fb1eab15decc36776a265dd2823503d93e2d` |
| `main-20261031-1355` | `3784445460f2d93a5c380cce96f65518ae79330f09126c05489d1a236d378a6f` |
| `main-20261031-1800` | `2863a83b8c26161ea54829c94859433a517b992733b802fe71a18fa31305d2d9` |
| `pilot-20261031-1146` | `1c7868182e895cc71447670f994fece099eb5f1f21cee8e64f6bdfe1b4966e50` |

The final 40-segment gate file SHA-256 is `10529bbeb0e550ccc32e0c403594f5f344451093be770027bf26918fc176ed3a`.

Final validation used rustup Cargo and rustc 1.92.0-nightly. The full command results and scope checks follow after the final rerun.

| Check | Result |
| --- | --- |
| `~/.cargo/bin/cargo -V`; pinned `rustc -V` | Both `1.92.0-nightly`. |
| `cargo fmt --all -- --check` | Exit 1 from formatting drift in six protected files already present on local `main`; none was edited in this branch. `cargo fmt -p gap-survey -- --check` passed. |
| `cargo clippy -p gap-survey --all-targets --offline -- -D warnings` | Passed. |
| `cargo test -p gap-survey --offline` | 13 passed, 0 failed. |
| `cargo test -p geyser-capture -p whirlpool-core --offline` | 26 passed, 0 failed across test targets. |
| `openspec validate --all --strict` | 13 passed, 0 failed. |
| `./scripts/ci/repo_hygiene.sh` | Passed; existing documentation timestamp and archived-shell placement warnings remain. |
| Dashboard JSON check, Grafana datasource queries and rendered view | Passed; seven query frames returned 21, 64, 10, 553, 16, 5 and 11 rows respectively, and all eight panels rendered. |
| Transactional load repeatability | Passed; second load preserved counts of 7/280/2640/2/40/66. |
| Nonprinting credential scan | 36 changed text/manifests/log files scanned, 0 credential-pattern hits; 0 `.env` or raw capture files in Git. |
| Protected scope and stream cleanup | 0 protected crate/contract files changed; 0 survey process or provider socket left. |

The repository-wide format failure is a pre-existing main-branch condition outside this handoff's permitted file scope. The new crate is formatted. No gate threshold or baseline was adjusted to obtain the decision above.

The `price-gap-survey` OpenSpec delta was synced into `openspec/specs/price-gap-survey/spec.md` and archived at `openspec/changes/archive/2026-10-31-price-gap-survey/`. Strict validation after archiving passed with 13 of 13 items.
