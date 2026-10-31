# 2026-10-31 10:05 CDT — Handoff 17a research dashboard fixes closeout

Created: 2026-10-31 10:05 CDT

The Grafana display was revised on `feat/research-grafana-20261031` after browser review of Handoff 17. The coverage reduction and time-based heatmap were removed. The dashboard defaults to the 152-route universe; coverage has twelve separate labeled bars and a twelve-row count table, with two-decimal shares. Exact `numeric` profit views use `trim_scale`, so profit strings have no padded fractional zeroes. The read-only role, schema data, seed pins and contract behavior remain unchanged.

## Queried values, unchanged from Handoff 17

| Scope | Quote | FixedUniverse evaluable / evaluated | Share |
| --- | --- | ---: | ---: |
| A SOL/USDC | USDC | 66,870 / 67,050 | 99.73% |
| B SOL/USDT | USDT | 20,061 / 44,700 | 44.88% |
| C USDC/USDT | USDT | 4,458 / 4,470 | 99.73% |
| Whole 152-route universe | USDC | 258,564 / 339,720 | 76.11% |
| Triangle shape within the universe | USDC | 187,236 / 268,200 | 69.81% |

The universe FixedUniverse and SimpleRules simulated profits are exactly `-0.005191 USDC` and `-0.002269 USDC`; a measured triangle zero is `0 USDC`. Missed and realized remain SQL `NULL`. These results are Simulated from about eight minutes of capture, with missing-state limits. ModelRanking is ranking-v0 heuristic, not ML.

## Panel data for the default universe and v1 experiment A

These descriptions are based on the provisioned dashboard JSON and anonymous Grafana data-source API responses. The browser access host was unavailable, so visual layout at 1500 pixels could not be independently inspected here. The API returned the row counts below; Claude's browser check remains necessary.

| Panel | 152-route universe selected | A SOL/USDC selected |
| --- | --- | --- |
| 1. Evidence limits | Static Simulated, eight-minute, no-realized-results notice. | Same notice. |
| 2. Experiments | Four rows: A, B, C and universe, with evidence kind, status, readable route shapes, quote symbol and UTC window. | Same four-row catalog. |
| 3. Coverage bars | Twelve labeled scope/policy bars, three per experiment. Shares: A 99.73%, B 44.88%, C 99.73%, universe 76.11%. | Same twelve global bars; no combined score. |
| 4. Coverage counts | Twelve rows with both evaluable and evaluated counts, thousands separators and two-decimal shares. | Same twelve global rows. |
| 5. Where candidates went | Three policy stacks with fixed neutral colors. FixedUniverse includes 258,562 below-threshold, 54,408 insufficient-state and 26,748 tick-array-absent candidates. | Three stacks; FixedUniverse has 66,868 below-threshold and 180 insufficient-state candidates. |
| 6. Policy comparison | Three policy rows, with detected/selected, exact USDC profit, and missed/realized shown as Not measured. FixedUniverse simulated `-0.005191 USDC`; SimpleRules `-0.002269 USDC`. | Three policy rows with the same quoted profit values for A; missed/realized Not measured. |
| 7. Shape coverage | Six rows: triangle and two-leg per policy. FixedUniverse triangle share 69.81%, two-leg share 99.73%. | One `Not measured for this experiment` row. |
| 8. Diagnostic probe | Time-free table of 26 pool directions with attempts, the three missing-state reason columns and raw probe input atoms. | One `Not measured for this experiment` row. |

The panel SQL has no fake timestamp. The three rejection reasons use distinct neutral colors. Table width budgets are at most 1,400 pixels, with the 12-row coverage table given 16 Grafana grid units of height. These configuration checks do not replace visual confirmation.

## Validation and operational state

- `dotnet build MEVana.slnx -warnaserror`: passed, zero warnings/errors.
- `dotnet test MEVana.slnx`: 303 passed, zero failed/skipped.
- `openspec validate --all --strict`: 13 passed before archiving the amendment and 12 passed after archive; zero failed. The `research-grafana` spec was synced and the OpenSpec change archived.
- `./scripts/ci/repo_hygiene.sh`: passed, with existing soft warnings for historical docs and scripts. Its exception now covers the dedicated research dashboard directory.
- `docker compose -p mevana-research -f infrastructure/docker-compose.research.yml config --quiet`: passed with the research environment loaded for interpolation.
- `python3 scripts/research/check-dashboard.py`: passed, including default, per-row coverage, share precision, color, no-time and fallback guards.
- `python3 scripts/research/validate-live.py`: passed from both checkouts. The anonymous Grafana API returned row counts 4/12/12/3/3/6/26 for the universe and 4/12/12/3/3/1/1 for v1 across panels 2–8. All six table digests remained identical after another load.
- Grafana's container environment has the reader and admin variable names and no `POSTGRES_PASSWORD` variable. No value was printed.
- A second detached checkout without `research.env` generated new credentials and loaded against the existing volumes. The original Grafana admin password still authenticated; the second checkout's newly generated admin password did not. The second checkout and its mode-600 credential file were retained at `/Users/michaelinman/Repos/MEVana-worktrees/research-grafana-second-20261031` so no local password is lost. The intended worktree's reader password was restored by reloading from it.
- The stack was stopped after validation. Ports 5434 and 3010 are clear; both named volumes remain. No repo-root `.env` was read, no provider call was made, and no volume was deleted. The protected Rust, API, Core, seed and main compose paths remain unchanged.

No deployment or merge was performed.
