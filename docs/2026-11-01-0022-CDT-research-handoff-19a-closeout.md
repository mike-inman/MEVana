# 2026-11-01 00:22 CDT — Handoff 19a pool map fixes closeout

Created: 2026-11-01 00:22 CDT

Branch: feat/pool-map-sample-20261031, based on eb9f62d3. The first sample is preserved byte-for-byte. The new run is `sample-20261101-19a`.

## Collection and recovery

Collection used **148 public requests**, **4 Tatum getMultipleAccounts calls**, **415.0486810207367 seconds**, and **zero Geyser bytes**. No limits were reached. Tokens skipped: **0**. Coverage performed 134 token lookups, including every control token. 50 responses returned at least 30 pools and are conservatively flagged as possibly capped; source completeness is unknown. No endpoint limit was bypassed. Both public sources worked; partial-source count is 0.

All three price rounds were saved 120 seconds apart, with source context slots retained. Raydium configuration was read once and carried into the verified decoder. Initial offline post-processing failed because the adapter treated a Raydium AmmConfig as a pool account. The adapter was repaired and the saved bytes replayed; no price round or sample was repeated. A real-account regression test now covers this case. Two stable control-token lookups were completed in the same bounded run after the price rounds when coverage auditing found them missing. Both count in the 148-request total and 415.05-second window. The collector now includes stable control tokens from the outset. The failed-processing manifest remains saved externally alongside the recovered manifest.

Raw responses, derived bundle, result, replay, slot-bearing decoded rounds and screenshot are under `~/MEVanaData/pool-map/sample-20261101-19a/`. Credentials were loaded only by the existing named-value Tatum loader.

## Coverage and requirements

There are 2142 distinct pool-map entries: fluxbeam: 2; memefun: 2; meteora: 824; meteora-damm-v2: 1; meteora-dbc: 1; meteoradbc: 41; orca: 353; pancakeswap: 46; pump-fun: 12; pumpfun: 296; pumpswap: 136; raydium: 427; stonkfun: 1. Orca, Raydium CLMM, Meteora DLMM and PumpSwap are covered. 350 entries are BondingCurve and three are unverified venue type; both groups are excluded from R1/R2 and gaps but retained in the map and NewPool list. Meteora DBC classification follows its official program description: https://github.com/MeteoraAg/dynamic-bonding-curve .

**All 13 controls pass R1. No control fails R1.** R1 is still sampled coverage, not an assertion of complete market counts.

| Requirement | Entering | Rejected | Leaving / review |
| --- | ---: | ---: | ---: |
| R1 | 796 | 594 | 202 |
| R2 | 202 | 111 | 91 |
| R3 | 91 | 2 | 89 |
| R4 | 89 | 73 | 16 |
| R5 | 16 | 12 | 4 |

First-failure reasons: {'fewer_than_two_pools': 594, 'liquidity_unknown': 10, 'liquidity_below_floor': 101, 'mint_unreadable': 61, 'onchain_fee_not_below_gap': 12, 'shortlist_not_checked': 6, 'NewPool': 2, 'freeze_authority': 4, 'token2022_restricted_extension': 2}. The shortlist is limited to 20 pairs and up to three pools each. Unread or unshortlisted mint state fails R4 explicitly. The USDC/USDT control was discovered by the final stable-token coverage audit and lacks a checked shortlist mint; it fails R4, rather than assuming safety.

Candidate list: **empty**. Comparison list: **empty**. **Zero candidates have an on-chain gap above combined fees: zero Measured and zero MinimumFeeOnly.** Three controls exceed combined minimum base fees (BONK/SOL, BONK/USDC, JUP/SOL), all MinimumFeeOnly; variable Meteora fees remain unmodeled. One control has R5 Not measured. None of these is an executable-trade claim. The liquidity floor, seven-day age rule, median liquidity and turnover ordering rules are unchanged.

## Controls and candidates: reported versus on-chain

The on-chain gap and fee use the same two pools; prices must be read within 25 slots. All supported pool comparisons in each round retain the account context slot. Other venues remain Not measured. A minimum base fee pass is explicitly distinct from a measured fee pass.

| Control | First failure / review | Reported gap bps | On-chain gap bps | Combined fee bps | Fee kind | Bin step bps |
| --- | --- | ---: | ---: | ---: | --- | ---: |
| $WIF / SOL | Not measured | 21.49 | Not measured | Not measured | Not measured | Not measured |
| $WIF / USDC | R2 | Not measured | Not measured | Not measured | Not measured | Not measured |
| Bonk / SOL | Minimum-fee review | 149.89 | 85.65 | 30.00 | MinimumFeeOnly | 100.00 |
| Bonk / USDC | Minimum-fee review | 67.93 | 59.26 | 50.00 | MinimumFeeOnly | 80.00 |
| JUP / SOL | Minimum-fee review | 47.68 | 62.12 | 30.00 | MinimumFeeOnly | 80.00 |
| PYTH / SOL | R5 | 20.13 | 16.64 | 30.00 | Measured | Not measured |
| SOL / JTO | R5 | 12.70 | 42.63 | 130.00 | Measured | Not measured |
| USDC / JTO | R5 | 71.55 | 47.43 | 130.00 | Measured | Not measured |
| USDC / JUP | R5 | 334.76 | 49.30 | 110.00 | MinimumFeeOnly | 200.00 |
| USDC / PYTH | R2 | Not measured | Not measured | Not measured | Not measured | Not measured |
| USDC / SOL | R5 | 34.02 | 0.27 | 8.00 | Measured | Not measured |
| USDC / USDT | R4 | Not measured | Not measured | Not measured | Not measured | Not measured |
| USDT / SOL | R5 | 33.08 | 1.51 | 3.00 | Measured | Not measured |

## Depth, storage and dashboard

Active-liquidity depth estimates are stored per pool, with deepest and shallowest estimates and venues retained for each pair. USD conversion is Reported; the active liquidity is OnChain. These estimates ignore tick crossings. Exact numeric values remain stored as Decimal strings; display rounds bps/turnover/imbalance to two decimals and USD to comma-separated whole amounts.

Loading twice produced identical sorted row content (SHA-256 `1a950393f809970692f41aa23b18f7255f14860011bfc7a056fa382636be07e8`). Latest-run filtering preserves the first sample in the database. Result and raw-response replay are byte-identical using the same serializer. Result SHA-256: `81afb479ea9885d7299f2636b6106d2fb08f75a30ec2c99d9c77d4e28770c1af`.

Rendered at **1500 pixels** in headless Chrome, with full mints confined to their own column. Tables scroll to all rows. Unknowns display Not measured; the measured candidate count is 0.

| Panel | Rendered content |
| --- | --- |
| Header | Required sample limitation, fixed list rule and evidence caveats. |
| Run health | One compact row with readable UTC start/end, 148 API calls, four RPC calls, zero skipped tokens, 50 cap warnings, zero candidates and no partial sources. |
| Funnel | Plain rejection-reason rows for all five requirements; no raw JSON cells. |
| Candidates: prices/fees | No rows; no candidate measured or minimum-fee pass. |
| Candidates: depth/clues | No rows; estimates remain Not measured. |
| Candidates: mint checks | No rows. |
| Controls: prices/fees | All 13 controls, separate reported/on-chain bps, combined fee, fee kind, bin step and review reason. |
| Controls: depth/clues | All 13 controls, two-decimal turnover/movement/imbalance and labeled deepest/shallowest estimates with venues. |
| Controls: mint checks | All 13 controls with plain freeze and safety-check fields; exemptions remain disclosed in stored details. |
| NewPool | 505 recent entries with symbols, venue type, readable creation time, Reported liquidity and identifiers. Bonding curves remain visible here. |

## Limitations and validation

This is one short sample. Reported prices can lag; active-bin prices are quantized and do not establish a fillable DLMM quote. Variable Meteora fees are not modeled. Capped token lists, unverified venues and the bounded shortlist leave coverage incomplete. On-chain comparison uses only the sampled accounts; unsupported venues stay unknown. No threshold was changed after seeing results.

Validation: rustup Cargo 1.92.0-nightly; offline build, package fmt and clippy passed; 13 Python test groups passed; gap-survey has 13 unit tests plus the new real-account adapter integration test, all passed. Both dashboard checks, SQL query checks, repeat-load content check, strict OpenSpec validation, repo hygiene, diff check and nonprinting credential scan are run at completion. The stack is stopped with volumes kept; no collector, helper or review browser remains running.

Completion evidence: the aggregate SHA-256 of every file under `sample-20261031-19` matched the pre-work fingerprint. The nonprinting scan checked 21 changed files/manifests and found zero credential patterns; no environment or raw-response file is committed. Protected crates and contracts have zero changes. Strict OpenSpec validation after archiving passed 14/14. The change is archived at `openspec/changes/archive/2026-11-01-pool-map-fixes/`. The first run remains in the store and filesystem.
