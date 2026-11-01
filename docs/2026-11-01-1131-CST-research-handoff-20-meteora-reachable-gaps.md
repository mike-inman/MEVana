# 2026-11-01 11:31 CST — Handoff 20: Can the Meteora gaps be reached?

## Purpose

Handoff 19 is merged (`b71f5f2d`). Every eligible pair was checked, and the only non-control gaps above fees use Meteora's minimum fee. Those gaps look stale. This handoff asks one question with one short sample: **can a trade actually reach those prices at a cost below the gap?**

If no pair can, the Meteora lead closes, and that is a valid result. No new pair search belongs in this handoff.

Work in `/Users/michaelinman/Repos/MEVana-worktrees/pool-map-sample-20261031` on a new branch, `feat/meteora-reachable-gaps-20261101`, created from `main`. Keep that worktree's `infrastructure/research.env` as it is.

## What Claude found

| Finding | Evidence | Why it matters |
| --- | --- | --- |
| Most Meteora prices did not move | Only 15 of 96 Meteora pools changed price across the three rounds, four minutes apart | The price field can sit still while the market moves |
| Gaps were frozen | PENGU/USDC read 78.93 bps in all three rounds and JUP/MET read 78.17 in all three. HYPE/USDC, BP/USDC and Bonk/USDC were also unchanged | A live 78 bps gap over fees would normally be traded away |
| The Meteora pools were often idle | Field read by Claude at offset 56 of `LbPair`, believed to be the last update time: PENGU pools 3.4 and 5.4 hours, Bonk/USDC 1.4 hours. This offset is not yet verified | The active bin may be empty or one-sided, which makes its price nominal |
| Variable fees are not zero | JUP/MET pools show a nonzero volatility accumulator in the same unverified reading | The 60 bps "minimum" may be well below the real cost, and JUP/MET clears it by only 18 bps |
| Depth is missing where it matters | All Meteora-only pairs show depth as Not measured. No candidate or comparison pair exceeds $7,869 where measured | Even a real gap may pay very little |
| R5 reads only the largest gap | `fixes.py` builds `passes` but never uses it. A smaller gap between cheaper pools could be rejected wrongly. Claude's recount found no such case in 19b | Correct rule, wrong code path |
| Passing pairs show `First failure: Not measured` | Rendered at 1500 pixels | Nothing failed, so "Not measured" is untrue |
| Table panels show about five rows | Candidates has 8 rows and Controls has 13, and the rest need scrolling inside the panel | Rows can be missed |

Nine Raydium pools have no fee because they use dynamic fees. That label is correct and is not a defect.

## Hard limits

| Limit | Value |
| --- | --- |
| Total collection time | Under 5 minutes |
| Public API requests | 0 |
| Solana RPC reads | At most 15 `getMultipleAccounts` calls through Tatum |
| Geyser bytes | 0 |

Use run id `sample-20261101-20`. Do not change or overwrite earlier samples. Compute the account and call plan first, and stop before any live read if it exceeds the limit. Do not repeat the sample unless it fails, and report why if so.

## Pairs in scope

Take the pool pairs from `sample-20261101-19b` whose on-chain gap was above their combined fee in any round, using minimum fees. That is PENGU/USDC, HYPE/USDC, USDT/FO, WBTC/USDC and JUP/MET, plus the Bonk/SOL, Bonk/USDC, USDC/JUP and JUP/SOL controls.

- Exclude BP/USDC, which failed R4.
- Read addresses from the saved bundle. Do not type them by hand.

## Required changes

1. **Meteora decoding:** add a read-only decoder for DLMM `LbPair` volatility and fee fields and for `BinArray` reserves.
   - Verify every offset against Meteora's published IDL, and record the IDL source and version in the closeout.
   - Add real-account fixtures. Add no signing, transaction building or submission code.
   - Confirm or correct Claude's offset 56 reading.
2. **Sample:** take 2 rounds, 2 minutes apart. Each round reads each pool and the bin arrays around its active bin, in batched calls with slots stored. Take enough arrays to reach the nearest liquidity on both sides, up to the RPC limit. If liquidity is not found, record `Not measured`.
3. **Other side of each pair:** for Orca pools, use `whirlpool_core::exact` when the needed tick arrays were read. Otherwise label the result `Indicative`, and never count it as `Reachable`.
4. **Reachable test:** for each pool pair, in both directions, and at sizes of $100 and $1,000, compute:
   - the average buy price and average sell price after walking real bins;
   - the fee, including the variable fee from current on-chain state, labeled `Modeled`;
   - the modeled output minus input in dollars, before transaction, priority and tip costs.
5. **Result per pair:** one of `Reachable`, `NotReachable`, `Indicative` or `Not measured`.
   - `Reachable` means modeled output exceeds input at a size, in the same round, with slots no more than 25 apart.
   - Fixed before results: do not change the sizes or this rule after seeing data.
   - Also report the empty bins between the active bin and the nearest liquidity on each side.
6. **Staleness fields:** add `price_changed_across_rounds` and verified `last_update_age_seconds` for each Meteora pool. These are labels and do not change list rules.
7. **R5 fix:** a pair passes when any compared pool pair passes, using the `passes` list. Rerun the saved 19b data offline and report any change. Keep 19b's stored result unchanged.
8. **Dashboard:** add a panel for this result, with one row per pair, size and direction.
   - Show the true empty state when no failure exists, rather than `Not measured`.
   - Make the existing tables tall enough to show all their rows at 1500 pixels.
   - Keep all evidence rules from handoffs 17 and 17a.

## Tests

- Fixture decoding, including offset checks against real accounts.
- Bin walking: an active bin with one-sided liquidity, empty bins, and a size that crosses several bins.
- Variable fee math against values worked by hand from the IDL formula, with a real example if one is available.
- Reachable outcomes at their exact boundary, and `Indicative` never counting as `Reachable`.
- R5 passing through a smaller gap when the largest gap has a higher fee.
- The plan stops before any read when it exceeds the RPC limit.
- Offline replay of the new run gives identical bytes.

Keep all existing tests and dashboard checks.

## Validation

Use rustup's cargo. Run `cargo test -p gap-survey`, plus fmt and clippy for that package, every `scripts/research/pool_map/test_*.py`, both dashboard checks, `openspec validate --all --strict`, `./scripts/ci/repo_hygiene.sh` and `git diff --check`. Run a credential scan. Render the dashboard in a browser at 1500 pixels, and describe what each new panel shows.

Do not change `hotpath_arb_engine`, `executor-service` or `shared-core`.

## Done means

- The closeout lists every in-scope pool pair with its result, by size and direction, with full pool addresses and modeled dollars.
- The closeout states plainly how many non-control pairs are `Reachable`. If the answer is zero, say so.
- The closeout says whether a repeated gap came from empty bins, variable fees, or something else, and marks anything that cannot be told apart as unknown.
- The work is committed and pushed, with a clean worktree. The stack is stopped with volumes kept, and no process remains.
- Nothing is merged. Claude repeats the checks and merges the branch.

## Prompt for Codex

```text
Implement /Users/michaelinman/Repos/MEVana/docs/2026-11-01-1131-CST-research-handoff-20-meteora-reachable-gaps.md
in /Users/michaelinman/Repos/MEVana-worktrees/pool-map-sample-20261031 on a new branch,
feat/meteora-reachable-gaps-20261101, created from main. This is one short sample with run id
sample-20261101-20: no Geyser streams, no public API requests, under 5 minutes of collection and at
most 15 Tatum getMultipleAccounts calls. Verify every Meteora offset against the published IDL. Do not
run long jobs or leave any process running. Do not overwrite earlier samples. Never source or print
.env, and add no wallet, signing, transaction building or submission path. Do not change the sizes or
the Reachable rule after seeing results. Commit and push the branch, but do not merge. Leave the stack
stopped with volumes kept. Report validation output, RPC calls used, the Reachable result per pair by
size and direction, how many non-control pairs are Reachable, what explains the repeated gaps, and the
final commit hash.
```
