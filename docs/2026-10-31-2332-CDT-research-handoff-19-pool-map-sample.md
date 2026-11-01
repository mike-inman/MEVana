# 2026-10-31 23:32 CDT — Handoff 19: Pool map sample

## Purpose

Handoff 18 found no 10 bps net events in 13 mainstream-leaning pairs. This slice takes a short sample of the wider Solana pool market and shows which less common pairs pass a sequence of basic requirements. It implements stages 1 and 2 of `docs/2026-10-31-2321-CDT-strategy-less-common-pair-discovery.md`.

The user does not want long jobs. **This is a sample, not a monitoring system.** Total collection time must stay under 20 minutes.

## Scope

**In scope:**
- a small read-only tool that collects public pool data;
- bounded on-chain checks of the mints and pools on the shortlist;
- the requirement sequence below, with a reason recorded for every rejected pair;
- a load into the `research` schema and one Grafana dashboard.

**Out of scope:**
- Geyser streams of any kind, including Sodae and Tatum streams;
- transaction scraping, the census of winning trades, and Meteora variable fee modeling, which belong to handoff 20;
- daemons, schedulers, retries that run unattended, or repeat runs;
- paid API plans, new accounts, or scraping of web pages;
- MEVana-UI changes, and any wallet, signing or submission path.

Do not change `hotpath_arb_engine`, `executor-service` or `shared-core`. Do not change handoff 18's results, thresholds or seed data.

## Execution model

Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/pool-map-sample-20261031`, on branch `feat/pool-map-sample-20261031`, created from local `main` at `c6aa0175` or later. Local `main` is ahead of `origin/main`, so compare against this base commit. Use OpenSpec with a new `pool-map-sample` capability. Use rustup's cargo.

## Hard limits

| Limit | Value |
| --- | --- |
| Total collection time | Under 20 minutes |
| Public API requests | At most 300, within each provider's published free limit |
| Solana RPC reads | At most 20 `getMultipleAccounts` calls through Tatum |
| Geyser bytes | 0 |

The tool must enforce these limits and stop with a clear message when one is reached. On HTTP 429 or a 5xx response, wait as the provider says, retry at most twice, then record the source as partial. Do not work around rate limits.

## Collection

Confirm each endpoint and its limits in the provider's current documentation first. If a source is unavailable or its terms do not allow this use, skip it and report that. Stop if no public source works.

1. Use GeckoTerminal and DexScreener public endpoints, with no key. Collect top Solana pools by exchange and recently created pools.
2. Cover at least Orca, Raydium, Meteora and PumpSwap where the source supports them. Report any venue that could not be sampled.
3. For each pool, keep address, exchange, pool type if reported, both mints, reported price, reported liquidity, 24-hour volume, fee if reported, creation time, source and collection time.
4. Take **3 price samples 5 minutes apart** for the pools on the shortlist, and store each one.

Preserve raw responses under `~/MEVanaData/pool-map/<run-id>/`, outside Git. Reported values are third-party data, so label them `Reported`. Values read from chain are labeled `OnChain`.

## Requirement sequence

Apply these in order. A pair stops at the first requirement it fails, and the first failed requirement is stored.

| Step | Requirement | Measure | Unknown value |
| --- | --- | --- | --- |
| R1 | At least 2 pools for the pair | Pool count in the sample | Fail |
| R2 | Enough liquidity in at least 2 pools | Reported liquidity at or above $20,000 | Fail, reason `liquidity_unknown` |
| R3 | Pools old enough | At least 7 days old, otherwise move to a separate `NewPool` list | `age_unknown` |
| R4 | Token is safe to hold and transfer | On-chain mint read, below | Fail |
| R5 | Fees below observed gaps | Largest sampled price gap exceeds the two pools' combined fee | `Not measured`, kept for review |

Group token pairs regardless of mint order, and treat wrapped SOL as SOL. The $20,000 value is provisional and is reported liquidity, not depth. Do not tune it after seeing results.

For R4, read each token's mint account. Reject a token when any of these is present or cannot be read:
- freeze authority still set;
- Token-2022 transfer fee, transfer hook, permanent delegate, non-transferable or default-frozen state.

Stablecoins and other tokens on a short, named exemption list keep their freeze authority flag visible but are not rejected for it. Every entry needs a stated reason.

## Clues for each surviving pair

Record these as separate columns. Do not combine them into one score.

- **Turnover:** 24-hour volume divided by reported liquidity.
- **Price disagreement:** largest gap between its pools across the 3 samples.
- **Sample movement:** largest price change across the 3 samples.
- **Liquidity imbalance:** deepest pool's liquidity divided by the next pool's.
- **Combined fee:** lowest two-pool fee, or `Not measured`.
- **Near-price depth:** for Orca and Raydium CLMM only, estimate the dollars needed to move price 1% from the on-chain active `liquidity`. This ignores tick crossings and must be labeled an estimate. Other venues are `Not measured`.

Limit near-price depth reads to the shortlist, using the RPC limit above.

## Output lists

- **Control:** the 13 handoff 18 pairs, shown even when they fail a step.
- **Candidates:** up to 30 passing pairs that are not controls. Order them for review by lowest turnover among pairs at or above the median liquidity, and show the rule in the panel.
- **Comparison:** up to 10 passing pairs chosen by a fixed seed derived from the run id, so that handoff 20 has a non-selected group.
- **NewPool:** recently created pools listed separately. They are not candidates yet.

## Storage

Add a `pair-discovery` crate or extend an existing tool, whichever keeps the code smaller. Reuse the existing credential loader, which reads named values only. Never source or print `.env`.

Add idempotent tables to `infrastructure/research/schema.sql`, with `run_id` and `evidence_kind` on every row, following the existing pattern:
- `pool_map_runs`: time, request counts, partial sources, limits reached;
- `pool_map_pools`: one row per pool;
- `pair_requirements`: one row per pair with the first failed step and reason;
- `pair_clues`: one row per surviving pair with its list.

Unknown values stay `NULL`. Loading twice must give identical rows. Keep money values exact and never use `float`.

## Dashboard

Add **MEVana Pair Discovery** to `infrastructure/research/grafana/dashboards/`.

Header: "One short sample. Reported data and indicative prices. Not executable and not profit."

Panels:
1. Run health: sources, requests, partial sources and limits.
2. Requirement funnel: pairs entering and leaving each step, with rejection reasons in fixed neutral colors.
3. Candidates table, with every clue column.
4. Control table, using the same columns.
5. NewPool table.

Follow the evidence rules from handoffs 17 and 17a: `NULL` shows "Not measured", a real zero shows 0, no `COALESCE`, no green or red, and no judgment wording such as profitable, winner, best or recommended. Extend the dashboard check to cover this file. Render it in a browser and describe what each panel shows.

## Tests

- Pair grouping across reversed mint order and wrapped SOL.
- Each requirement step at its exact boundary, and first-failure storage.
- Unknown values never pass silently and never become 0.
- Mint parsing with fixtures for a plain SPL token, freeze authority, and each rejected Token-2022 extension.
- Near-price depth with a hand-computed value.
- Limits: request, RPC, time and 429 handling with a mocked source.
- Comparison selection is deterministic.
- Offline rerun from saved raw responses gives identical bytes.
- A source check that the code has no Geyser, signing, wallet or submission path.

## Validation

```bash
cargo fmt -p <new-crate> -- --check
cargo clippy -p <new-crate> --all-targets -- -D warnings
cargo test -p <new-crate>
openspec validate --all --strict
./scripts/ci/repo_hygiene.sh
git diff --check <base-commit>...HEAD
python3 scripts/research/check-dashboard.py
python3 scripts/research/check-gap-dashboard.py
```

Also confirm that protected crates and handoff 18 data are unchanged, that no secret or raw response is committed, that the credential scan passes, and that no process is left running.

## Done means

- One live sample finished within every hard limit. Do not repeat it unless the first failed, and report why if so.
- A dated closeout in `docs/` reports requests used, venues covered, funnel counts, the candidate, control and new-pool lists, and what the sample cannot show.
- The work is committed and pushed to the feature branch, with a clean worktree. The research stack is stopped with volumes kept.
- Nothing is merged. Claude verifies the branch and merges it.

## What this sample cannot show

One short sample shows that a pair is worth a closer look. It cannot show that arbitrage exists, that a gap can be executed, or that a pattern lasts. Reported prices can be delayed, so sampled price gaps are clues only.

## Prompt for Codex

```text
Implement /Users/michaelinman/Repos/MEVana/docs/2026-10-31-2332-CDT-research-handoff-19-pool-map-sample.md
in a new worktree at /Users/michaelinman/Repos/MEVana-worktrees/pool-map-sample-20261031 on branch
feat/pool-map-sample-20261031, created from local main at c6aa0175 or later. Use OpenSpec. Build and
test offline first. This is one short sample: no Geyser streams, under 20 minutes of collection, at
most 300 public API requests and 20 Tatum getMultipleAccounts calls. Do not run long jobs or leave
any process running. Stop and report if a limit would be exceeded or no public source works. Never
source or print .env, and add no wallet, signing or submission path. Do not tune the provisional
thresholds after seeing results. Commit and push the branch, but do not merge. Leave the stack stopped
with volumes kept. Report validation output, requests used, venues covered, funnel counts, the
candidate list, and the final commit hash.
```
