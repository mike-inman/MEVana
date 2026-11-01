# 2026-11-01 00:40 CDT — Handoff 19b: Check all eligible pairs

## Purpose

Handoff 19a fixed coverage, fees, on-chain prices and the dashboard. Its tests, replay and scope checks all pass. But its "zero candidates" result is not a market finding, because most eligible pairs were never checked. This small fix checks them, on the same branch, `feat/pool-map-sample-20261031`, on top of `09001d5f`.

Handoff 19's scope limits still apply. This is a sample, not a monitoring system.

## What Claude found

| Finding | Evidence | Why it matters |
| --- | --- | --- |
| Most eligible pairs were never checked | `collect19a.py` keeps 20 pairs, controls first. 66 of the 73 non-control pairs that reached R4 were marked `mint_unreadable` or `shortlist_not_checked` | Only 7 non-control pairs could become candidates |
| The 20-pair cap was not required | Only 4 of the 10 allowed RPC calls were used, and each call reads 100 accounts | The sample could have checked far more |
| Unchecked pairs count as rejected | The R4 funnel shows 73 rejected, including 67 that were never read | "Rejected" and "not checked" must be separate |
| A control was also skipped | USDC/USDT is `shortlist_not_checked` | Controls must all be read |
| All three `MinimumFeeOnly` gaps are smaller than one Meteora bin | BONK/SOL 85.65 bps with a 100 bps bin, BONK/USDC 59.26 with 80, JUP/SOL 62.12 with 80 | A gap within one bin may not be tradable, so these need a visible label |

## Hard limits

| Limit | Value |
| --- | --- |
| Total collection time | Under 8 minutes |
| Public API requests | At most 40, within published free limits |
| Solana RPC reads | At most 15 `getMultipleAccounts` calls through Tatum |
| Geyser bytes | 0 |

Use a new run id, `sample-20261101-19b`. Do not change or overwrite the two earlier samples. Do not repeat the sample unless it fails, and report why if so.

## Required changes

1. **Reuse the pool map:** start from the pool list saved in `sample-20261101-19a`. Do not repeat the per-token lookups. Public requests are for reported price samples only.
2. **Check every eligible pair:** remove the 20-pair cap. Every pair that passes R1 to R3, and every control, goes through R4 and R5. Keep up to 3 pools per pair, chosen by the existing rule. Read mints in one batch, then 3 price rounds 2 minutes apart.
3. **Enforce the limit before reading:** compute the accounts and RPC calls needed first. If the plan exceeds 15 calls, stop and report it before any live read. Do not trim pairs silently.
4. **Separate outcomes:** `NotChecked` is a separate funnel outcome, never a rejection. A mint that was requested but could not be read stays `mint_unreadable`. After this fix, `NotChecked` should be 0, and the closeout must explain any other value.
5. **Bin label:** add `gap_within_bin_step`, true when the on-chain gap is at or below the largest bin step of the two pools compared.
   - This is a label, not a rejection. It does not change R5, list rules or the candidate rule.
   - Report counts with and without labeled gaps.
   - Show the label in the price panels for candidates, comparison and controls.
6. **Thresholds:** keep the $20,000 floor, the 7-day rule, the fee comparison and all list rules unchanged.

## Tests

- No pair is dropped by a shortlist cap, using a mocked set larger than 20 pairs.
- `NotChecked` is separate from every rejection in the funnel.
- The request plan stops before any read when it exceeds the RPC limit.
- `gap_within_bin_step` at its exact boundary, unknown when the bin step is unknown, and unchanged R5 results.
- Offline replay of the new run gives identical bytes.

Keep all existing tests and dashboard checks.

## Validation

Run everything in handoff 19's validation section again, and render the dashboard in a browser at 1500 pixels.

## Done means

- All 13 controls and all pairs reaching R4 have been checked.
- The closeout lists every candidate and comparison pair with symbol, full mint, venues, reported and on-chain gaps, combined fee, fee kind, bin step, `gap_within_bin_step`, and shallowest and deepest depth.
- The closeout states plainly how many non-control pairs have an on-chain gap above their combined fee, split by `Measured` and `MinimumFeeOnly`, and how many of those are not within one bin. If an answer is zero, say so.
- The work is committed and pushed to `feat/pool-map-sample-20261031`, with a clean worktree. The stack is stopped with volumes kept, and no process remains.
- Nothing is merged. Claude repeats the checks and merges the branch.

## Prompt for Codex

```text
Implement /Users/michaelinman/Repos/MEVana/docs/2026-11-01-0040-CDT-research-handoff-19b-check-all-shortlisted-pairs.md
in /Users/michaelinman/Repos/MEVana-worktrees/pool-map-sample-20261031 on branch
feat/pool-map-sample-20261031, on top of 09001d5f. Handoff 19's scope limits still apply. This is one
short sample with run id sample-20261101-19b: no Geyser streams, under 8 minutes of collection, at most
40 public API requests and 15 Tatum getMultipleAccounts calls. Reuse the saved 19a pool list. Do not
run long jobs or leave any process running. Do not overwrite earlier samples. Never source or print
.env, and add no wallet, signing or submission path. Do not tune thresholds after seeing results.
Commit and push the branch, but do not merge. Leave the stack stopped with volumes kept. Report
validation output, requests used, NotChecked count, funnel counts, how many non-control pairs have an
on-chain gap above their combined fee by fee kind, how many of those are not within one bin, and the
final commit hash.
```
