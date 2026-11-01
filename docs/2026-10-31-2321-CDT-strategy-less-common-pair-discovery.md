# 2026-10-31 23:21 CDT — Strategy: discovering less common pairs

## Purpose

Find token pairs outside the mainstream that are deep enough to trade but receive less bot attention, using measured clues instead of a hand-picked list. This is a read-only research plan. Trading stays disabled. It adds no wallet, signing or submission path.

## Starting evidence

- Handoff 18 (Codex closeout, not yet verified by Claude): 0 of 40 segments passed the gate, with no 10 bps net events. Popular pools look efficient.
- A scratch rerun without the 25-slot stale cutoff found one extra 5 bps event and no 10 bps events.
- Meteora variable fees were not modeled, so 47% of observations have no net figure. Meteora gross gaps reached 98 bps.
- Constant-product pools, order books and newly created pools have not been measured.

## Target profile

The target is the middle of the market: enough depth to pay for a trade, at least two places to trade, and little repeat bot activity.

Use **depth**, not total pool value. Concentrated-liquidity pools can hold large totals with little usable liquidity near the current price. Depth here means the dollars needed to move the price by 1%.

For two similar pools, a rough estimate is:

```text
profit ≈ depth × gap² / 400
```

Depth is in dollars and gap is the after-fee gap in percent. This is before tips and network fees, and it is only a sizing guide.

| Depth at 1% | 0.1% gap | 0.3% gap | 1.0% gap |
| ---: | ---: | ---: | ---: |
| $10,000 | $0.25 | $2.25 | $25 |
| $50,000 | $1.25 | $11 | $125 |
| $250,000 | $6.25 | $56 | $625 |

Profit grows with the square of the gap, so a thinner pool can still be worthwhile if its gaps are larger. The provisional floor is **$20,000 of depth at 1%**. Stage 3 replaces it with a value based on observed tips and winning trade sizes.

## Stages

### Stage 0: verify handoff 18

Verify and merge the survey before building on it. Its 13 pairs become the mainstream control group.

### Stage 1: map all pools (no Geyser quota)

Collect a snapshot of Solana pools from public pool data feeds, then confirm a sample directly on chain. Record exchange, tokens, fee, liquidity, volume and pool age. Include Orca, Raydium, Meteora and PumpSwap, plus Phoenix order-book markets if their data is available. Repeat daily so that the list stays current.

### Stage 2: filter and score

Hard filters:
- token available in at least two pools;
- depth at or above the provisional floor;
- combined fee lower than the gaps that pool actually shows;
- no permanent delegate, active transfer hook, transfer fee or freeze authority risk;
- both pools old enough to have a price history, except for a separate new-pool list.

Score the remaining pairs on:
- **turnover** (volume divided by depth), to find deep pools that are not heavily traded;
- **price disagreement** between pools;
- **volatility**;
- **liquidity imbalance** between the deeper pool and the lagging pool;
- **fee level**.

Output three lists: Mainstream control, Candidates and Watch. Weights are provisional until stage 3 tests them.

### Stage 3: census of clues (capped provider use)

Sample transactions for mainstream, candidate and random comparison pools. For each winning arbitrage, record the route, tokens, trade size, profit, tip, bot address and pool state before it. Also record the seven clues from the discussion of 2026-10-31: large swaps, sharp moves, new pools, liquidity changes, fee jumps, transaction surges and peg breaks.

Compare those moments with similar moments when no arbitrage happened. Without that comparison, we would only be describing winners.

Before this stage, add Meteora fee modeling. Cap the first run at 300 MB, and stop if its projected use exceeds the cap. Sodae's remaining allowance and its trial end, around 2026-11-05, must be checked first.

### Stage 4: look for patterns, then test them

Questions to answer:
- Do the same routes win on different days?
- How many different bots win each route?
- What do tips take from revenue?
- Which clues appear before arbitrage more often than at comparison moments?
- Which trade sizes actually occur on routes that win?

Testing many pairs guarantees that some will look good by chance. So patterns are found on one set of days, the rules are then frozen, and they are tested on later days. A pattern that fails the later test is dropped, and thresholds do not change afterward.

### Stage 5: exact modeling

Only routes that pass stage 4 get exact quote math and replay. Results stay labeled by evidence kind, and nothing here counts as realized profit.

## Outputs

Results go to the `research` Postgres schema and Grafana, following the existing evidence rules. Pair rankings use no judgment wording and never combine quote assets.

## Stop conditions

Stop and report if:
- filters leave too few candidates to compare;
- winning routes are held by the same few bots and tips take nearly all revenue;
- no clue beats the comparison moments;
- a decoder cannot be verified;
- projected provider usage exceeds the cap.

A clear negative result is a valid outcome.

## Order of work

Decision, 2026-10-31 23:32 CDT: the user wants short samples and no long jobs. Each stage uses a small, capped sample, and stage 1 is one pass rather than a daily job.

1. Verify and merge handoff 18 (done, merge `c6aa0175`).
2. Handoff 19: stages 1 and 2, as one short sample using no Geyser quota.
3. Handoff 20: can the Meteora gaps be reached? Handoff 21: a small stage 3 sample.
4. Handoff 22: stage 4, with stage 5 only if a route passes.

## Lead from handoff 18 verification

A scratch rerun counted Meteora at its base fee only, which is a best case because variable fees can only add cost. That gave 30 events of 10 bps or more, including JUP/SOL Orca/Meteora gaps of about 50 bps visible for 45 to 50 slots. These are not confirmed opportunities. DLMM prices move in bin steps, so the active-bin price can differ from the tradable price, and variable fees are not included. Handoff 20 must check both before this lead counts as evidence.

Update, 2026-11-01 11:31 CST: handoff 19b weakened this lead further. Only 15 of 96 Meteora pools changed price across a four-minute sample, and the gaps above minimum fees were frozen on their Meteora side. Handoff 20 now tests only whether those gaps can be reached. The stage 3 census moves to handoff 21, and stage 4 to handoff 22.
