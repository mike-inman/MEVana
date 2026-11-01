# 2026-10-31 23:58 CDT — Handoff 19a: Pool map fixes

## Purpose

Handoff 19 is not ready to merge. Its limits, mint checks, depth formula and offline replay all checked out. But its funnel and candidate list do not yet answer the question, for the reasons below. This follow-up fixes them on the same branch, `feat/pool-map-sample-20261031`, on top of `eb9f62d3`, with one more short sample.

Handoff 19's scope limits still apply. This is a sample, not a monitoring system.

## What Claude found

| Finding | Evidence | Why it matters |
| --- | --- | --- |
| R1 measures our coverage, not the market | 11 of 13 controls failed "fewer than two pools". JUP/SOL failed, but one public lookup returned 16 JUP/SOL pools | 301 of 354 rejections are unreliable, so pairs may have been dropped wrongly |
| Reported gaps are noisy | Reported SOL/USDC gap was 31.7 bps. Handoff 18's largest on-chain Core gross gap was 9.396 bps | Most candidate gaps (9 to 47 bps) are within control noise |
| Fees are nearly all unknown | 19 of 522 pools have a fee, all Orca. No candidate has a measured fee comparison | R5 cannot run on any candidate |
| Depth shows only the shallowest pool, unlabeled | `near_price_depth_usd_estimate` is the minimum across eligible pools | Readers cannot see which pool limits a trade |
| Bonding-curve venues count as pools | `pump-fun`, `pumpfun` and `raydium-launchlab` appear in the pool map | They are not comparable to AMM pools for R1 and R2 |
| Dashboard is hard to read | Raw mints, 70-digit decimals, raw JSON, epoch times, truncated cells | Results cannot be reviewed |

## Hard limits

| Limit | Value |
| --- | --- |
| Total collection time | Under 10 minutes |
| Public API requests | At most 220, within published free limits |
| Solana RPC reads | At most 10 `getMultipleAccounts` calls through Tatum |
| Geyser bytes | 0 |

Use a new run id. Do not change or overwrite `sample-20261031-19`. Do not repeat the sample unless it fails, and report why if so.

## Required changes

1. **Coverage:** after the broad listing, call DexScreener's per-token pools endpoint for every non-stable token that appears in a pool at or above the $20,000 reported floor, plus all control tokens.
   - Process tokens by highest reported liquidity first, and record any tokens skipped because of the request limit.
   - The endpoint may return a capped list. Record when a token returns the cap, because its pool count may be incomplete.
   - Report how many of the 13 controls pass R1. If any control still fails R1, name it and explain why before reporting candidates.
2. **Venue types:** mark bonding-curve venues as `BondingCurve`, and exclude them from R1 and R2 and from the gap comparison. Keep them visible in the pool map, and keep their pairs in the NewPool list where relevant.
3. **On-chain prices:** for shortlisted Orca, Raydium CLMM and Meteora DLMM pools, read current price from the chain.
   - Take **3 rounds, 2 minutes apart**. Each round reads all accounts in the same batched call where possible, and stores the slot.
   - Reuse `gap_survey::decode::decode_pool` through a small read-only helper rather than writing a second decoder.
   - Other venues are `Not measured`.
   - Store `onchain_gap_bps` separately from the existing reported gap. Compare only pools read in the same slot or within 25 slots.
   - Meteora gaps carry a `bin_step_bps` column, because bin prices move in steps.
4. **Fees:** read on-chain fees for those three venues, including Raydium `AmmConfig`.
   - Meteora uses base fee as a **minimum**, labeled `MinimumBaseFee`, because variable fees are not modeled.
   - R5 compares `onchain_gap_bps` with the combined fee of the same two pools. Report three outcomes: `Measured`, `MinimumFeeOnly` and `Not measured`.
   - A `MinimumFeeOnly` pass is not a measured pass, and stays labeled.
5. **Depth:** store the estimate per pool. Show deepest and shallowest values for each pair, with the venue of each. Keep the estimate label.
6. **Thresholds:** keep the $20,000 reported floor, the 7-day age rule and all list rules unchanged. Do not add a new noise threshold after seeing results. Show control on-chain and reported gaps beside candidates so that readers can compare them.
7. **Dashboard:**
   - show token symbols, with the full mint available in a separate column or link;
   - show bps, turnover and imbalance to 2 decimals, and dollars with thousands separators and no decimals, while keeping exact stored values;
   - show times as readable timestamps;
   - replace raw JSON with plain columns such as `Freeze flag` and `Mint check`;
   - remove empty space in run health, and render partial sources readably;
   - add reported gap, on-chain gap, fee kind and deepest and shallowest depth columns to the Candidates and Control panels;
   - ensure no cell is cut off at 1500 pixels wide, except full mints in their own column;
   - keep all evidence rules from handoffs 17 and 17a.

## Tests

- Per-token merge, including duplicate pools across sources and capped responses.
- Bonding-curve exclusion from R1, R2 and gaps.
- Same-slot gap math with reversed mint order, and exclusion of pools more than 25 slots apart.
- Fee outcomes for `Measured`, `MinimumFeeOnly` and `Not measured`.
- Per-pool depth output, with deepest and shallowest selection.
- Limits, and one test that a skipped token is recorded.
- Offline replay from raw responses produces the same values as the collection output. Handoff 19's replay matched in value but not in file bytes, so write both files with one serializer.
- Extend `check-dashboard.py` for rounding, no raw JSON cells, and symbol columns.

Keep all existing checks, and add real-account fixtures where the existing `decode_pool` fixtures do not already cover a case.

## Validation

Run everything in handoff 19's validation section again. Also run `cargo test -p gap-survey` if a helper is added there. Render the dashboard in a browser and describe what each panel shows.

## Done means

- Every finding in the table is fixed.
- The closeout reports R1 control results, requests used, tokens skipped, venues covered, funnel counts, and a table comparing reported and on-chain gaps for controls and candidates.
- The closeout states plainly how many candidates have an on-chain gap above their combined fee, and with which fee kind. If the answer is zero, say so.
- The work is committed and pushed to `feat/pool-map-sample-20261031`, with a clean worktree. The stack is stopped with volumes kept, and no process remains.
- Nothing is merged. Claude repeats the checks and merges the branch.

## Prompt for Codex

```text
Implement /Users/michaelinman/Repos/MEVana/docs/2026-10-31-2358-CDT-research-handoff-19a-pool-map-fixes.md
in /Users/michaelinman/Repos/MEVana-worktrees/pool-map-sample-20261031 on branch
feat/pool-map-sample-20261031, on top of eb9f62d3. Handoff 19's scope limits still apply. This is one
short sample with a new run id: no Geyser streams, under 10 minutes of collection, at most 220 public
API requests and 10 Tatum getMultipleAccounts calls. Do not run long jobs or leave any process running.
Do not overwrite sample-20261031-19. Never source or print .env, and add no wallet, signing or
submission path. Do not tune thresholds after seeing results. Commit and push the branch, but do not
merge. Leave the stack stopped with volumes kept. Report validation output, requests used, tokens
skipped, how many controls pass R1, funnel counts, how many candidates have an on-chain gap above
their combined fee and with which fee kind, what each dashboard panel shows, and the final commit hash.
```
