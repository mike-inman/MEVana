# 2026-10-31 09:35 CDT — Handoff 18: Price-gap survey

## Purpose

Stage 0 has tested only Orca Whirlpool pools on SOL, USDC and USDT, for about eight minutes. That found almost nothing, and it says little about the rest of the market.

Adding exact quoting and full replay for every exchange is expensive. So this slice first runs a cheap, read-only survey of pool prices to answer one question: **where do price gaps appear often enough, large enough and long enough to justify exact modeling?**

Its result chooses what to model next. It does not show that any trade is profitable.

**Prerequisite:** handoff 17 is merged, so the `research` schema and Grafana stack exist.

## Sequencing decision

Earlier planning put a longer full capture before this survey. This handoff reverses that, for two reasons:
- A long capture with tick arrays is costly, and should target the markets this survey identifies.
- The Sodae trial is 5 GB over 7 days from 2026-10-29, so it ends around **2026-11-05**. About 305 MB was recorded as used through handoff 12. Live runs should be finished before then. Check the provider's remaining allowance rather than relying on these figures.

## Questions

Hypotheses are written down before seeing the data:

| Id | Hypothesis | How it is measured |
| --- | --- | --- |
| H1 | Gaps between exchanges are more frequent than gaps within Orca | Compare cross-venue with same-venue pool pairs on the same token pair |
| H2 | Less popular tokens have larger gaps than SOL/USDC/USDT | Compare by token group |
| H3 | Gaps widen during volatile periods | Compare by rolling 60-second SOL price move, using buckets fixed below |
| H4 | Gaps appear after sharp moves in one pool | Compare in the 10 slots after a move of at least 10 bps in one pool between updates, as a stand-in for large swaps |

Newly created pools are out of scope. They need broader streams and carry extra scam risk.

## Scope

**In scope:**
- a new crate, `crates/gap-survey`, with live recording and offline analysis;
- pool-state decoding for Raydium CLMM and Meteora DLMM, and reuse of the existing Whirlpool decoder;
- a reviewed pool allowlist;
- bounded live runs under the budget below;
- new `research` tables and one Grafana dashboard.

**Out of scope:**
- exact swap quotes, tick arrays or bin arrays, and any simulated profit;
- transaction streams;
- Raydium AMM v4 and other constant-product pools, which need vault balances and are a later slice;
- wallets, signing, submission, trading, Jito and flash loans;
- changes to `hotpath_arb_engine`, `executor-service`, `shared-core`, `whirlpool-core`, `geyser-capture` behavior, the Research API, or MEVana-UI.

The program ids already in `shared_core::programs` may be read. Do not add a wallet, signing or submission path.

## Execution model

Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/price-gap-survey-20261031`, on branch `feat/price-gap-survey-20261031`, created from local `main` after handoff 17 is merged. Use OpenSpec with a new `price-gap-survey` capability. Develop offline first.

## Pool allowlist

Commit `crates/gap-survey/allowlist.json`, with at most **40 pools**:

| Group | Pairs |
| --- | --- |
| Core | SOL/USDC, SOL/USDT, USDC/USDT |
| Established | JUP, JTO and PYTH, each against SOL or USDC |
| Volatile | BONK and WIF, each against SOL or USDC |

- Include Orca Whirlpool, Raydium CLMM and Meteora DLMM pools. For each exchange and pair, take the pools with the most liquidity, up to 3.
- Pool discovery is a one-time step using each exchange's published pool list. Check their current documentation for the endpoint. Store only public addresses, with venue, tokens, decimals and fee tier. Record the sources and retrieval time.
- Verify each pool's owner program and token mints from a one-time account read. Do not trust list data alone.
- Tokens must be real mints that you verify, not symbols matched by name.

## Live budget

These limits are mandatory and must be enforced in code, not only by procedure:

- Use existing provider profiles and credential loading from `geyser-capture`. Never source or print `.env`.
- Stream **account updates for allowlisted pool accounts only**, at Confirmed commitment. No transactions, no tick arrays, no bin arrays.
- The tool requires a `--max-bytes` value and stops cleanly when payload bytes reach it.
- Durations are positive and bounded at 3,600 seconds per run, with no reconnect beyond the `geyser-capture` backoff limits.
- **Pilot:** one 300-second run. Measure bytes per minute, gaps and update rates per venue. Stop and report if a venue has no updates, or if the projected total exceeds 1.5 GB.
- **Main runs:** up to 6 runs of up to 30 minutes, spread across at least 3 different times of day, with a combined limit of 1.5 GB. Fewer runs are acceptable if the budget requires it. Record any shortfall.
- Before and after each run, confirm no capture process or provider socket remains. No stream may sit idle. All live runs must finish by 2026-11-04.

Raw output goes under `~/MEVanaData/captures/handoff18-20261031/`, never in the repo.

## Prices and gaps

Decode each update into a mid price, using integer or exact decimal math and token decimals:

| Venue | Price source | Fee source |
| --- | --- | --- |
| Orca Whirlpool | `sqrt_price`, via `whirlpool_core::decode` | `fee_rate` in the pool |
| Raydium CLMM | `sqrt_price_x64` in pool state | Pool's `AmmConfig` trade fee, read once |
| Meteora DLMM | Active bin id and bin step | Base fee from pool parameters. A variable fee, if not computed, is recorded as not modeled. |

Derive layouts from each program's published IDL or source, and pin the version in the crate README. **Every decoder needs a fixture from a real mainnet account, with its address recorded**, and a test that its price agrees with a Whirlpool on the same pair within 1%. If a layout cannot be verified, stop and report it. Do not guess offsets.

Offline analysis must be deterministic: identical input gives byte-identical output. Replay the updates and hold the latest price per pool, with its slot. For each slot where a price changes, and for each pool pair on the same token pair, compute:

- **`gross_gap_bps`:** the absolute price difference over the mid of the two prices;
- **`net_gap_bps`:** gross gap minus both pool fees. If either fee is unknown, this is NULL, never the gross value and never 0;
- **Staleness:** the slot age of the older price. Exclude pairs where the older price is more than 25 slots old from gap statistics, and count them separately. An idle pool can otherwise look like a gap.

A gap event starts when `net_gap_bps` reaches a threshold and ends when it falls below it. Use thresholds of 5, 10, 25 and 50 bps. Record the peak and the duration in slots. The project target is 5–25 bps.

**Volatility buckets (H3):** absolute rolling 60-second move in SOL/USDC, using the Orca pool with the most liquidity: below 10 bps, 10–50 bps, above 50 bps.

## Honesty rules

- This is **indicative price data**. A gap shows a difference in mid prices, not a fillable trade. It ignores depth, slippage, network fees, tips, inclusion, competition and latency.
- Evidence label is `Indicative` on every row and panel. It must never be presented as `Simulated`, and it must not enter the experiment result contracts.
- No profit, dollar or return figure appears anywhere.
- Laptop capture times and receive order must not be read as trading latency.
- "Duration in slots" counts how long the gap stayed visible in pool state. It is not a time window anyone could have traded in.

## Data and dashboard

Add these tables to the `research` schema, all carrying an `evidence_kind` column set to `Indicative`, plus run id:

| Table | Grain |
| --- | --- |
| `gap_runs` | One per run: provider, window, bytes, update counts, gaps and stalls |
| `gap_pools` | One per pool: venue, tokens, group, fee and fee source |
| `gap_minute_stats` | One per run, minute, token pair and pool pair: observation count, median and maximum net and gross gaps, excluded stale count |
| `gap_events` | One per event: pool pair, threshold, start slot, duration, peak, volatility bucket, whether it followed a sharp move |

Load through a script like `load-seed.sh`, in one transaction, and safely re-runnable.

Add a provisioned Grafana dashboard titled **MEVana Price-Gap Survey**, with UI edits disabled. Its header must state: "Indicative mid-price gaps only. Not executable and not profit." Panels:

1. Run health: windows, bytes, gaps, update rate per venue.
2. Gap events per hour by token group and threshold.
3. Cross-venue compared with same-venue event rates (H1).
4. Net gap heatmap by token pair and minute.
5. Event duration distribution by threshold.
6. Event rates by volatility bucket (H3), and after sharp moves compared with the baseline (H4).
7. Top pool pairs by event rate at the 10 bps threshold. Ordering by event rate is allowed here, because it is a unitless measure and not profit.

`noValue` is "Not measured", and NULL must never become 0. Use no green or red judgment coloring.

## Decision gate

These criteria are fixed now, before data exists. A segment is a token pair and venue combination.

A segment **advances** to exact modeling only if all of these hold across at least 3 different runs:
1. at least 6 gap events per hour at the 10 bps net threshold;
2. a median event duration of at least 2 slots;
3. both pool fees known, with stale exclusions under 20% of observations.

Report segments that pass, segments that fail, and segments with too little data. Say so plainly if none pass. Do not change thresholds after seeing the results. Any change needs a new dated decision, with the reason and the original result kept.

## Tests

- Decoder fixtures and cross-venue agreement, as described above.
- Gap math, with hand-computed values, including NULL for unknown fees.
- Staleness exclusion, so that a pool with an old price creates no gap.
- Event start and end at exact thresholds, and events that are open when a run ends.
- Volatility buckets at their boundaries.
- Budget enforcement: `--max-bytes` and duration limits stop a mocked stream.
- Deterministic analysis, with two runs giving identical bytes.
- Dashboard JSON check, as in handoff 17, including the required header and evidence label.
- A source check that the crate has no signing, wallet or submission dependency.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy -p gap-survey --all-targets -- -D warnings
cargo test -p gap-survey
cargo test -p geyser-capture -p whirlpool-core
openspec validate --all --strict
./scripts/ci/repo_hygiene.sh
git diff --check <base-commit>...HEAD
```

Use rustup's cargo (`~/.cargo/bin/cargo`). Confirm `cargo -V` shows `1.92.0-nightly`.

Also confirm that:
- protected crates and contracts are unchanged;
- no secret, raw capture or `.env` value is committed, and the credential scan passes;
- total payload bytes are within budget, and no stream or capture process remains.

## Done means

- All live runs finished within budget, with results loaded and visible in the dashboard.
- The closeout reports each hypothesis, the gate result per segment, bytes used, data gaps and shortfalls.
- The closeout lists what the data cannot show, using the honesty rules above.
- The work is committed and pushed to `feat/price-gap-survey-20261031`, with a clean worktree.
- Leave the stack stopped, with its volumes kept. Nothing is merged. Claude verifies the branch and the results, and fast-forwards `main`.

## Later slices

Depending on the gate:
- exact quote math and replay for the segments that pass, using a longer full capture from a server host;
- Raydium AMM v4 and other constant-product pools;
- only after simulated evidence supports it, execution cost work, including Jito and flash loans.

## Prompt for Codex

```text
Implement /Users/michaelinman/Repos/MEVana/docs/2026-10-31-0935-CDT-research-handoff-18-price-gap-survey.md
in a new worktree at /Users/michaelinman/Repos/MEVana-worktrees/price-gap-survey-20261031 on branch
feat/price-gap-survey-20261031, created from local main after handoff 17 is merged. Use OpenSpec.
Build and test offline first. Live streams are approved only within the handoff's budget: pool
accounts only, one 300-second pilot, then up to 6 runs of up to 30 minutes, 1.5 GB combined, finished
by 2026-11-04. Stop and report if a decoder cannot be verified or the projected bytes exceed budget.
Never source or print .env, and add no wallet, signing or submission path. Do not change the gate
thresholds. Commit and push the branch, but do not merge. Report validation output, bytes used, each
hypothesis, the gate result per segment, and the final commit hash.
```
