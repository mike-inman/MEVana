# 2026-10-31 09:48 CDT — Handoff 17a: Research dashboard fixes

## Purpose

Handoff 17 is not ready to merge. Its data is correct: the values, null handling, read-only role, loopback binds, seed pins and tests all checked out. But the dashboard does not display that data properly in a browser. API checks passed because each panel returned rows, which does not show that a panel renders correctly.

This follow-up fixes the display on the same branch, `feat/research-grafana-20261031`, on top of `343c05ab`. Everything in handoff 17's scope limits still applies.

## What Claude found in the browser

| Panel | Problem | Why it matters |
| --- | --- | --- |
| Coverage bar gauge | Shows one unlabeled "76.1" instead of 12 rows | Panel 3 does not reduce per row, so it hides SOL/USDT at 44.88% and looks like one overall score, against the no-aggregation rule |
| Missing state by pool | Heatmap is empty on every experiment, or says "no heatmap fields found" | The query fakes a time axis with `seed_loaded_at`, which falls outside the dashboard's time range |
| Where candidates went | Every reason is the same blue; the axis labels overlap | The reasons cannot be told apart |
| Dashboard default | Opens on experiment A, so the shape and probe panels are empty | The richest experiment is hidden on arrival |
| All profit values | Show as `0.001582000000000000 USDC` | Hard to read |
| Share values | Show `99.7`, `69.8` | The agreed precision is two decimals, such as 99.73% and 69.81% |
| Counts | Show `339720` | Hard to read at this size |
| Coverage table | Shows 6 of 12 rows, with cut-off experiment names | Half the evidence needs scrolling |
| Experiments table | Route shapes show raw JSON, such as `["two_leg"]` | Hard to read |

## Required changes

1. **Coverage:** show all 12 experiment and policy rows at once, each labeled, with the exact share to two decimals. A horizontal bar chart or a per-row bar gauge is fine. Use the readable scope label, such as `SOL/USDT — FixedUniverse`, not only the long id. No single combined number may appear.
2. **Missing state by pool:** replace the heatmap with a time-free panel, such as a horizontal stacked bar or a table with single-hue cell shading. Show each pool and direction with `insufficient_state`, `tick_array_absent` and `state_gap` counts, and keep the diagnostic probe label. This replaces the handoff 17 heatmap requirement. The query must not use `seed_loaded_at` as a time column.
3. **Where candidates went:** give each reason its own fixed color, using neutral hues with no green or red. Use readable axis spacing.
4. **Default:** the `experiment` variable opens on `replay-v2-463353643-usdc-confirmed-l1-tip10000`. When a v1 experiment is selected, the shape and probe panels must say "Not measured for this experiment" or similar, rather than a raw Grafana message.
5. **Formatting:**
   - Use `trim_scale()` in the views so profits show their true decimals: `-0.005191 USDC`, and 0 as `0 USDC`.
   - Show shares to two decimals, and counts with thousands separators.
   - Show route shapes as readable text.
   - Keep exact `numeric` values. Do not round profit.
6. **Layout:** the coverage table shows all 12 rows without scrolling, with full experiment labels. On a 1500-pixel-wide viewport, no panel may cut off its text.
7. **Secrets:** the Grafana container currently receives all of `research.env`, including the Postgres superuser password. Pass it only `RESEARCH_READER_PASSWORD` and `GF_SECURITY_ADMIN_PASSWORD`.
8. **Hygiene script:** change the `repo_hygiene.sh` exception from the one named file to the directory `./infrastructure/research/grafana/dashboards/*`, so that handoff 18's dashboard does not need another edit.
9. **Second checkout:** after merge, `main` gets a new `research.env` while the volumes keep the old stack. Confirm that `grafana-up.sh` and `load-seed.sh` still work from a checkout with no `research.env` against the existing volumes. Fix anything that breaks, and state in the runbook that the Grafana admin password does not change this way. Do not delete volumes to pass this check.

## Tests

Extend `check-dashboard.py` so these can't return unnoticed:
- bar gauge and stat panels must show every row (`reduceOptions.values: true`), or not be used for multi-row data;
- no panel may alias `seed_loaded_at` as `time`;
- the rejection panel must set a fixed color per reason;
- the variable default must be the universe experiment;
- share fields must have 2 decimals.

Keep all existing checks. Extend `validate-live.py` to confirm `-0.005191`, `-0.002269` and 0 come back in trimmed form, and that the Grafana container's environment does not contain `POSTGRES_PASSWORD`, checking variable names only and never printing values.

Then render the dashboard yourself in a browser or with Grafana's panel view if you can, for both the universe experiment and a v1 experiment, and describe what each panel shows in the closeout. If you cannot, say so.

## Validation

Run everything in handoff 17's validation section again, along with `check-dashboard.py` and `validate-live.py`. All coverage and profit values must still match handoff 17.

## Done means

- Every problem in the table above is fixed.
- 303 or more tests pass, and no existing test is weakened.
- The handoff 17 closeout and runbook are updated.
- The work is committed and pushed to `feat/research-grafana-20261031`, with a clean worktree and the stack stopped with volumes kept.
- Nothing is merged. Claude repeats the browser check and fast-forwards `main`.

## Prompt for Codex

```text
Implement docs/2026-10-31-0948-CDT-research-handoff-17a-dashboard-fixes.md in
/Users/michaelinman/Repos/MEVana-worktrees/research-grafana-20261031 on branch
feat/research-grafana-20261031. Handoff 17's scope limits still apply. Do not delete Docker volumes,
read the repo-root .env, or print any secret. All coverage and profit values must still match
handoff 17; if one differs, stop and report it. Commit and push the branch, but do not merge. Leave
the stack stopped with volumes kept. Report validation output, what each panel shows for the universe
and a v1 experiment, and the final commit hash.
```
