# 2026-10-31 09:39 CDT — Handoff 17 Grafana research evidence closeout

Created the isolated `mevana-research` Postgres/Grafana stack, pinned-seed SQL export, rerunnable schema and loader, read-only Grafana data source, evidence dashboard, validation scripts and [operator runbook](operations/2026-10-31-0936-CDT-research-grafana-runbook.md). The official record of finished experiments remains the MEVana-UI Research page. No Research API, Core, seed, Rust, MEVana-UI or main compose file changed. No provider call or capture occurred.

Created: 2026-10-31 09:39 CDT

## Values queried from the local database

| Scope | Quote | FixedUniverse evaluable / evaluated | Share |
| --- | --- | ---: | ---: |
| A SOL/USDC | USDC | 66,870 / 67,050 | 99.73% |
| B SOL/USDT | USDT | 20,061 / 44,700 | 44.88% |
| C USDC/USDT | USDT | 4,458 / 4,470 | 99.73% |
| Whole 152-route universe | USDC | 258,564 / 339,720 | 76.11% |
| Triangle shape within the universe | USDC | 187,236 / 268,200 | 69.81% |

The universe's FixedUniverse simulated profit is `-0.005191 USDC`; SimpleRules is `-0.002269 USDC`. Postgres displays additional trailing zeroes in its exact `numeric` representation. Both values match the handoff numerically. Missed and realized values remain SQL `NULL`. The two coverage percentages for the universe and triangle have distinct scopes and must not be combined.

The source is Simulated evidence from about eight minutes of laptop capture. No fills, inclusion, slippage or realized profit were modeled. Missing tick arrays limit evaluability, particularly for SOL/USDT. ModelRanking is ranking-v0 heuristic, with no learned model.

## Validation

- `dotnet build MEVana.slnx -warnaserror`: passed, zero warnings and errors.
- `dotnet test MEVana.slnx`: 303 passed, zero failed or skipped. Three new export tests cover the real seed, SQL literal escaping and tamper refusal.
- Tampered-seed CLI check: nonzero exit and no output file.
- `openspec validate --all --strict`: 12 passed, zero failed.
- `./scripts/ci/repo_hygiene.sh`: passed; historical docs and scripts produce existing soft warnings. The check now allows and parses the single dedicated research dashboard file.
- `docker compose -p mevana-research -f infrastructure/docker-compose.research.yml config --quiet`: passed.
- `python3 scripts/research/check-dashboard.py`: passed (eight panels, scope and null guard).
- `python3 scripts/research/validate-live.py`: passed. Grafana anonymous Viewer API loaded the provisioned dashboard, and all seven SQL-backed panels returned rows: 4, 12, 12, 3, 3, 6 and 27. The reader role could SELECT current and future research tables, and could not insert, update, delete, create or read a separate public user table.
- Two loads produced identical rows in all six tables: experiments 4, policy results 12, policy rejections 30, shape results 6, shape rejections 15 and pool direction probes 52.
- Postgres bound only to `127.0.0.1:5434`; Grafana bound only to `127.0.0.1:3010`. The ignored credential file has mode 600 and was neither printed nor committed. The repository-root `.env` was not read.

Docker's default address pools were exhausted, so this stack declares the unused `10.252.61.0/24` bridge subnet. The main `infrastructure/docker-compose.yml` Grafana mounts start with `./infrastructure/...` even though that file already lives in `infrastructure/`; those mounts resolve correctly only when run from the repository root. That file was left untouched.

The stack is stopped, ports 5434 and 3010 are clear, and both named volumes remain. No deployment or merge was performed.
