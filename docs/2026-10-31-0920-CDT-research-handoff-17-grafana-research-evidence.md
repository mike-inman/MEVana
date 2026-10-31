# 2026-10-31 09:20 CDT — Handoff 17: Grafana research evidence

## Purpose

Grafana is now MEVana's standard visualization tool (decided 2026-10-31). This slice puts the four validated research experiments into Postgres and shows them in one provisioned Grafana dashboard. It also creates the place where the price-gap survey (handoff 18) will send its results.

The main risk is the same as in handoffs 15 and 16: presenting weak evidence as stronger than it is. Grafana does not enforce evidence rules by itself, so this slice builds them into the schema, the SQL views and the panels.

## Scope

**In scope:**
- a small, separate, local Docker stack for research;
- a `research` Postgres schema;
- a .NET export tool and a load script;
- one Grafana dashboard.

**Out of scope:**
- MEVana-UI changes, including the Research page, which stays the official record of finished experiments;
- changes to the Research API, seed files, seed pins or `MEVana.Research.Core`;
- changes to `infrastructure/docker-compose.yml`, the existing dashboards or their data sources;
- Prometheus, Rust changes, provider calls and captures;
- deployment or public exposure.

Do not read, print, source or copy the repo-root `.env`.

## Why a separate stack

The main `infrastructure/docker-compose.yml` starts far more than research needs, and Grafana there depends on Prometheus. Its Grafana mounts also begin with `./infrastructure/...` although the file is already in `infrastructure/`, so they only resolve correctly from the repo root. Leave that file alone. Report the path issue in the closeout.

## Execution model

Work in a new worktree at `/Users/michaelinman/Repos/MEVana-worktrees/research-grafana-20261031`, on branch `feat/research-grafana-20261031`, created from local `main` at `2e670407` or later. That commit is ahead of `origin/main`, so compare against this base commit, not `origin/main`. Use OpenSpec with a new `research-grafana` capability.

## Stack

Add `infrastructure/docker-compose.research.yml` with project name `mevana-research`, containing only:

| Service | Image | Host bind | Notes |
| --- | --- | --- | --- |
| Postgres | `postgres:15` | `127.0.0.1:5434` | Own named volume, database `research` |
| Grafana | `grafana/grafana:10.4.5` | `127.0.0.1:3010` | Own named volume |

- Both binds must be loopback. Add `research_postgres = 5434` and `research_grafana = 3010` to `config/ports.toml`. Confirm both ports are free, and stop if they are not.
- Port 3000 stays free for the MEVana-UI dev server. Port 3001 is reserved for `auth_gateway`.
- Secrets live in `infrastructure/research.env`, which `*.env` already ignores. A script creates that file with generated random values if it does not exist, sets its mode to 600, and never prints values. Commit `infrastructure/research.env.example` with placeholders. Never overwrite an existing file.
- Grafana uses anonymous Viewer access, which is acceptable only because the bind is loopback. State this in the OpenSpec spec, along with the rule that exposing it needs a later handoff with real authentication. Disable sign-up.
- Add `scripts/research/grafana-up.sh` and `scripts/research/grafana-down.sh`. Down must stop containers without removing volumes.

## Data model

Add `infrastructure/research/schema.sql`, which must be safe to re-run. All tables are in the `research` schema:

| Table | Grain | Key contents |
| --- | --- | --- |
| `experiments` | One per experiment | Result id, dataset, evidence kind, status, window, versions, quote mint, quote symbol, quote decimals, scope, `request_sha256`, seed load time |
| `policy_results` | One per experiment and policy | Evaluated, detected, selected, missed, estimated, simulated and realized values, unevaluable and evaluable counts |
| `policy_rejections` | One per experiment, policy and reason | Reason, count, `is_unevaluable` |
| `shape_results` | One per experiment, shape and policy | Same counts and profits, for v2 only |
| `shape_rejections` | One per experiment, shape, policy and reason | Reason, count, `is_unevaluable` |
| `pool_direction_probes` | One per pool, direction and outcome | Pool, input token, output token, attempts, outcome, count, for v2 only |

Rules:
- Store atoms as `bigint`.
- Unmeasured values stay SQL `NULL`. Use no `COALESCE` to 0 anywhere in the schema, views or panels.
- Every profit row must carry the quote symbol and decimals, directly or through its view.
- Views convert atoms to `numeric` with exact division by `10 ^ decimals`. Never use `float`.
- `is_unevaluable` is true only for `state_gap`, `insufficient_state` and `tick_array_absent`.
- Add a read-only role, `research_reader`, with `SELECT` on the `research` schema only, including future tables. Its password comes from `research.env`. Grafana connects only as this role.

## Loading data

Add `dotnet/src/MEVana.Research.SqlExport`, a console tool with no new NuGet dependency. It must load through `ResearchArtifactStore.Load`, so that all validation and seed pins apply. It writes one SQL file that loads everything in a single transaction: delete all rows in those tables, then insert. If the store refuses the seed, the tool exits non-zero and writes no file.

Add `scripts/research/load-seed.sh`, which runs the export and applies schema and data through `psql` inside the Postgres container. Running it twice must give identical rows.

SQL values must be correctly escaped. Use `NULL`, never `0` or an empty string, for missing values.

## Dashboard

Add data source provisioning under `infrastructure/research/grafana/`, using `research_reader` with the password from the environment. Add one file-provisioned dashboard titled **MEVana Research Evidence**, with UI edits disabled. Use the same Grafana schema version as the curated dashboards.

**Header:** a text panel stating "Simulated research only. About eight minutes of capture. No realized results." The evidence kind must also come from the data in the first table.

**Panels:**

1. **Experiments:** table with evidence kind, status, scope, quote symbol and window.
2. **Coverage:** bar gauge per experiment and policy, showing evaluable share, with a table beside it giving both counts.
3. **Where candidates went:** stacked bars of rejection reasons per experiment and policy, in counts. Each experiment is separate.
4. **Policy comparison:** table per experiment and policy with detected, selected, estimated, simulated, missed and realized. Show the quote symbol on every profit value.
5. **Shape coverage:** triangle and two-leg for the universe experiment, labeled with their scope.
6. **Missing state by pool:** heatmap of `pool_direction_probes`, under the label "Diagnostic probe: raw input amounts, not chained route amounts".

**Panel rules:**
- A dashboard variable selects one experiment for panels 3–6. Do not offer an "All" option on any panel showing profit.
- Never sum, average or rank across experiments, because they use different quote assets and overlap.
- Set `noValue` to "Not measured", and do not use null-as-zero transforms or `connectNulls`.
- Use no green or red profit thresholds, and no judgment wording such as profitable, winner, best or recommended.

## Tests

- **Export:** with the real seed, the SQL has 4 experiments and 12 policy rows. Nulls stay `NULL`, values are escaped, and a tampered seed copy produces no file and a non-zero exit.
- **Database, using the local stack:** query these and compare. If a value differs, stop and report it. Do not change the expected value.

  | Experiment | Quote | Evaluable / evaluated | Share |
  | --- | --- | ---: | ---: |
  | A SOL/USDC | USDC | 66,870 / 67,050 | 99.73% |
  | B SOL/USDT | USDT | 20,061 / 44,700 | 44.88% |
  | C USDC/USDT | USDT | 4,458 / 4,470 | 99.73% |
  | 152-route universe | USDC | 258,564 / 339,720 | 76.11% |
  | Triangle shape | USDC | 187,236 / 268,200 | 69.81% |

  Also confirm universe FixedUniverse simulated profit is `-0.005191`, SimpleRules is `-0.002269`, and missed and realized values are `NULL`.
- **Access:** `research_reader` can select and cannot insert, update, delete or read outside `research`.
- **Grafana:** query each panel's SQL through Grafana's data source API, as an anonymous viewer, and confirm that the provisioned dashboard loads and every panel returns data.
- **Dashboard check:** a repo test reads the dashboard JSON and fails on `COALESCE`, `connectNulls`, null-as-zero, an "All" option on profit panels, or forbidden wording.
- **Idempotence:** two loads give identical row counts and values.

## Validation

```bash
cd dotnet && dotnet build MEVana.slnx -warnaserror && dotnet test MEVana.slnx
openspec validate --all --strict
./scripts/ci/repo_hygiene.sh
git diff --check <base-commit>...HEAD
docker compose -p mevana-research -f infrastructure/docker-compose.research.yml config --quiet
```

Also confirm that:
- `crates/`, `MEVana.Research.Core`, `MEVana.Research.Api`, `dotnet/data/` and `infrastructure/docker-compose.yml` are unchanged;
- both ports bind only to `127.0.0.1`;
- no `research.env` or secret is committed, and the credential scan passes.

## Done means

- `scripts/research/grafana-up.sh` and `load-seed.sh` bring up the dashboard at `http://127.0.0.1:3010` from a clean start.
- Every value above matches, with all panel rules met.
- A runbook is in `docs/operations/`, and a dated closeout is in `docs/`.
- The work is committed and pushed to `feat/research-grafana-20261031`, with a clean worktree.
- Leave the stack stopped, with its volumes kept. Nothing is merged. Claude verifies the branch, checks the dashboard in a browser, and fast-forwards `main`.

## Prompt for Codex

```text
Implement /Users/michaelinman/Repos/MEVana/docs/2026-10-31-0920-CDT-research-handoff-17-grafana-research-evidence.md
in a new worktree at /Users/michaelinman/Repos/MEVana-worktrees/research-grafana-20261031 on branch
feat/research-grafana-20261031, created from local main. Use OpenSpec. Do not read or print the
repo-root .env, change infrastructure/docker-compose.yml, change MEVana-UI, or make provider calls.
If a queried value differs from the handoff, stop and report it. Commit and push the branch, but do
not merge. Leave the stack stopped with volumes kept. Report validation output, the queried values,
and the final commit hash.
```
