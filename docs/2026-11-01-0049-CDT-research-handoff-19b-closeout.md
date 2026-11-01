# 2026-11-01 00:49 CDT — Handoff 19b all eligible pair checks closeout

Created: 2026-11-01 00:49 CDT

Base: `09001d5f`; branch `feat/pool-map-sample-20261031`. New run: `sample-20261101-19b`. Both older samples are preserved. The saved 19a pool list was reused; there were no discovery or per-token lookups.

## Plan and collection

The offline plan selected **91 pairs, 239 pools and 55 mints**, without a pair cap. It includes all 89 pairs reaching R4 and all 13 controls (91 distinct pairs in the union). The conservative bound was 13 RPC calls, allowing one config account per pool; the tool refuses plans over 15 before constructing a live source. The normal public price plan was 24 requests.

Actual collection: **251.88096865499392 seconds**, **24 public price requests**, **11 Tatum calls**, **zero Geyser bytes**, no partial sources and no limits reached. One mint batch was read first; three price rounds used three pool batches each, and one configuration batch completed the total of 11 calls. The three rounds began at UTC epochs 1793511772, 1793511892, 1793512012 (120 seconds apart). Collection was not repeated. A 460-second guard and hard call counters enforce the eight-minute, 40-public-request and 15-RPC limits.

## Complete checks and funnel

**NotChecked = 0.** Every eligible pair and every control has requested mint state. All 13 controls pass R1. Requested-but-unreadable mint state would remain `mint_unreadable`; none occurred. The 21 R4 failures are actual mint restrictions, not unread coverage. R1–R3 still describe the saved sample rather than a complete market census.

| Requirement | Entering | Rejected | NotChecked | Leaving / review |
| --- | ---: | ---: | ---: | ---: |
| R1 | 796 | 594 | 0 | 202 |
| R2 | 202 | 111 | 0 | 91 |
| R3 | 91 | 2 | 0 | 89 |
| R4 | 89 | 21 | 0 | 68 |
| R5 | 68 | 49 | 0 | 19 |

First-failure reasons: {'fewer_than_two_pools': 594, 'liquidity_unknown': 10, 'liquidity_below_floor': 101, 'onchain_fee_not_below_gap': 49, 'freeze_authority': 5, 'token2022_restricted_extension': 16, 'NewPool': 2}.

The result has eight candidates and six comparison pairs, plus 13 controls. List ordering and seeded comparison selection are unchanged. The within-bin field is a nullable label only; it does not participate in R5 or list selection. A bin step is Not measured when neither compared pool has one.

## Non-control gaps above combined fees

| Fee kind | Observed non-control pairs above fee | Passing R1–R4 | Within one bin | Outside one bin | Unknown bin |
| --- | ---: | ---: | ---: | ---: | ---: |
| Measured | 0 | 0 | 0 | 0 | 0 |
| MinimumFeeOnly | 6 | 5 | 5 | 1 | 0 |

**Zero non-control pairs pass with Measured fees.** Six observed non-control gaps exceed combined minimum base fees, but BP/USDC fails R4 due to freeze authority and is excluded from both lists. The five eligible minimum-fee pairs are PENGU/USDC, HYPE/USDC, USDT/FO, WBTC/USDC and JUP/MET. Four are within one bin; **one is outside one bin: JUP/MET**, in Comparison, with 78.17 bps gap, 60 bps combined minimum fee and 50 bps bin. That is still a MinimumFeeOnly clue, with variable fees and execution unmodeled. Unknown fee comparisons are not counted as passes.

## Candidates: every pair

| Symbol | Full mint pair | Compared / sampled venues | Reported gap bps | On-chain gap bps | Combined fee bps | Fee kind | Bin bps | Within bin | Shallowest USD estimate (venue) | Deepest USD estimate (venue) |
| --- | --- | --- | ---: | ---: | ---: | --- | ---: | --- | --- | --- |
| PENGU / USDC | 2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | meteora / meteora | 1716.94 | 78.93 | 45.00 | MinimumFeeOnly | 125.00 | Yes | Not measured (Not measured) | Not measured (Not measured) |
| RAY / USDT | 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R|Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB | raydium | 24.42 | Not measured | Not measured | Not measured | Not measured | Not measured | 123.25 (raydium) | 123.25 (raydium) |
| USDT / FO | Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB|JDzPbXboQYWVmdxXS3LbvjM52RtsV1QaSv2AzoCiai2o | meteora / meteora | 36.67 | 48.25 | 21.00 | MinimumFeeOnly | 100.00 | Yes | Not measured (Not measured) | Not measured (Not measured) |
| USDC / 2Z | EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v|J6pQQ3FAcJQeWPPGppWRb4nM8jU3wLyYbRrLh7feMfvd | raydium / raydium | 11.82 | 6.89 | Not measured | Not measured | Not measured | Not measured | 329.52 (raydium) | 1576.82 (raydium) |
| SPX / SOL | J3NKxxXZcnNiMjKw9hYb2K4LUxgwB6t1FtPtQVsv3KFr|So11111111111111111111111111111111111111112 | orca / raydium | 37.73 | Not measured | Not measured | Not measured | Not measured | Not measured | 5660.90 (orca) | 5660.90 (orca) |
| STONK / USDC | 6GmAFSYs4gk3FDao5FzzySQpPZaWsa4rUJHacpMpUNgx|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | raydium / meteora | 15.71 | 8.90 | Not measured | Not measured | 8.00 | No | 3836.28 (raydium) | 7868.59 (orca) |
| HYPE / USDC | 98sMhvDwXj1RQi5c5Mndm3vPe9cBqPrbLaufMXFNMh5g|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | meteora / meteora | 40.24 | 39.04 | 20.00 | MinimumFeeOnly | 100.00 | Yes | Not measured (Not measured) | Not measured (Not measured) |
| USDC / wXMR | EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v|WXMRyRZhsa19ety5erZhHg4N3xj3EVN92u94422teJp | meteora | 74.12 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured (Not measured) | Not measured (Not measured) |
## Comparison: every pair

| Symbol | Full mint pair | Compared / sampled venues | Reported gap bps | On-chain gap bps | Combined fee bps | Fee kind | Bin bps | Within bin | Shallowest USD estimate (venue) | Deepest USD estimate (venue) |
| --- | --- | --- | ---: | ---: | ---: | --- | ---: | --- | --- | --- |
| WBTC / USDC | 3NZ9JMVBmGAqocybic2c7LQCJScmgsAZ6vQqTDzcqmJh|EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | meteora / orca | 9.06 | 21.26 | 15.00 | MinimumFeeOnly | 50.00 | Yes | 1454.60 (orca) | 1454.60 (orca) |
| NIO / SOL | 7SL2VnfYsChvog5oQY9Nmtqu1wceaMiQzWoqoLQpump|So11111111111111111111111111111111111111112 | meteora / pumpswap | 42.61 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured (Not measured) | Not measured (Not measured) |
| HOOKED / SOL | C1mBfBoDkwWfd6uTFZp62ARHLjeVp3bDpCDMfMZtPngE|So11111111111111111111111111111111111111112 | meteora | 243.62 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured (Not measured) | Not measured (Not measured) |
| 2Z / SOL | J6pQQ3FAcJQeWPPGppWRb4nM8jU3wLyYbRrLh7feMfvd|So11111111111111111111111111111111111111112 | orca / raydium | 15.02 | 2.96 | Not measured | Not measured | Not measured | Not measured | 628.51 (orca) | 778.79 (raydium) |
| JUP / MET | JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN|METvsvVRapdj9cFLzq4Tr43xK4tAjQfwX76z3n6mWQL | meteora / meteora | 37.92 | 78.17 | 60.00 | MinimumFeeOnly | 50.00 | No | Not measured (Not measured) | Not measured (Not measured) |
| SOL / wXMR | So11111111111111111111111111111111111111112|WXMRyRZhsa19ety5erZhHg4N3xj3EVN92u94422teJp | meteora / raydium | 13.26 | 4.58 | Not measured | Not measured | 8.00 | Yes | 1234.00 (raydium) | 1234.00 (raydium) |

## Controls

| Symbol | First failure / review | Reported bps | On-chain bps | Combined fee bps | Fee kind | Bin bps | Within bin |
| --- | --- | ---: | ---: | ---: | --- | ---: | --- |
| $WIF / SOL | Not measured | 26.85 | Not measured | Not measured | Not measured | Not measured | Not measured |
| $WIF / USDC | R2 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| Bonk / SOL | Minimum-fee review | 146.65 | 76.30 | 30.00 | MinimumFeeOnly | 100.00 | Yes |
| Bonk / USDC | Minimum-fee review | 102.04 | 61.76 | 50.00 | MinimumFeeOnly | 80.00 | Yes |
| JUP / SOL | Minimum-fee review | 32.83 | 34.90 | 20.00 | MinimumFeeOnly | 80.00 | Yes |
| PYTH / SOL | R5 | 8.59 | 5.55 | 30.00 | Measured | Not measured | Not measured |
| SOL / JTO | R5 | 15.24 | 44.35 | 130.00 | Measured | Not measured | Not measured |
| USDC / JTO | R5 | 87.52 | 47.43 | 130.00 | Measured | Not measured | Not measured |
| USDC / JUP | Minimum-fee review | 330.93 | 43.07 | 20.00 | MinimumFeeOnly | 50.00 | Yes |
| USDC / PYTH | R2 | Not measured | Not measured | Not measured | Not measured | Not measured | Not measured |
| USDC / SOL | R5 | 31.96 | 0.12 | 8.00 | Measured | Not measured | Not measured |
| USDC / USDT | R5 | 1.12 | 0.03 | 2.00 | Measured | Not measured | Not measured |
| USDT / SOL | R5 | 43.14 | 0.66 | 3.00 | Measured | Not measured | Not measured |

## Replay, storage and dashboard

Raw seed, account bytes, reported sample responses and slot-bearing decoded rounds stay outside Git under `~/MEVanaData/pool-map/sample-20261101-19b/`. Replay reconstructs the result from those saved inputs using the same serializer; it matches collection bytes exactly. SHA-256: `8544a297a2191e1fc2a36e0766865e7827b6b4319a5b36e8a751b5b73e83a4e9`.

The dashboard was rendered at 1500 pixels. Run health shows 24 API calls, 11 RPC calls, eight candidates and NotChecked 0. Funnel rows show NotChecked separately from rejection. Three panels each for Candidates (eight rows), Comparison (six rows) and Controls (13 rows) show prices/fee/bin labels, depth/clues, and plain mint checks. Full mints have a dedicated column. The NewPool panel retains the saved-map recent pools. Unknowns stay Not measured and real zeros stay 0. No green/red judgment color or raw JSON cells were introduced. The screenshot is `dashboard-final.png` outside Git.

## Limits and validation

This short sample cannot establish an executable gap, persistent behavior or actual variable fees. Reported liquidity, volume, pool age and USD conversions come from the reused map and retain its coverage and cap limitations; only reported price samples and bounded on-chain reads were refreshed. Depth is an active-liquidity estimate that ignores tick crossings. Unsupported venues and unknown fees remain unmeasured. Price comparisons require account slots no more than 25 apart. No threshold, list rule or stale cutoff was tuned after seeing the result.

Offline validation: rustup Cargo 1.92.0-nightly; helper build, package fmt, clippy and all 14 Rust tests passed. All 17 Python test groups passed, including a mocked map larger than 20 pairs, preflight refusal before live reads, NotChecked versus mint_unreadable, exact bin boundary, unknown bin and unchanged R5 results. Both dashboard checks, strict OpenSpec validation, repo hygiene and diff check run again at completion. Credentials remain private and no raw response is committed.

Repeat-load sorted row content SHA-256: `3a8a586c74a7dd9e9a98e145075531250c541e8bbec2bca3d4ac49d0893e791f`. Every round requested all 239 pools and decoded the 204 pools supported by the existing decoder; other venues remain Not measured. Supported-account slot ranges were 463978392–463978400, 463978946–463978950 and 463979509–463979514. The entire round spans are below the 25-slot cutoff.

Completion checks: aggregate file hashes for both earlier sample directories matched their pre-work fingerprints. The nonprinting scan checked 16 changed files/manifests with zero credential-pattern hits, no environment/raw-response paths and no protected crate/contract changes. OpenSpec is archived at `openspec/changes/archive/2026-11-01-check-all-pool-pairs/`; strict validation after archive passed 14/14. The research stack is stopped, both research volumes remain, and no collector, helper or review browser remains. Nothing was merged.
