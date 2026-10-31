# 2026-10-31 09:36 CDT — Local research Grafana runbook

Created: 2026-10-31 09:36 CDT
Last updated: 2026-10-31 10:05 CDT

This local stack displays the four pinned Stage 0 experiments. MEVana-UI remains the official record of finished experiments. Grafana is for local evidence inspection and future research survey tables. The stack does not contact Solana providers or execute trades.

## Start and load

From this repository worktree:

```bash
./scripts/research/grafana-up.sh
./scripts/research/load-seed.sh
```

The up script requires free host ports 5434 and 3010. It generates `infrastructure/research.env` once, sets mode 600, and never overwrites it. Keep that ignored file and its values private. The stack has its own Postgres and Grafana named volumes. Docker bridge address-pool exhaustion on this host required the dedicated `10.252.61.0/24` network; check for a collision before changing that allocation elsewhere.

Grafana receives only `RESEARCH_READER_PASSWORD` and `GF_SECURITY_ADMIN_PASSWORD`; it never receives the Postgres superuser password. Direct Compose commands must pass `--env-file infrastructure/research.env` so the selective Grafana environment resolves. Keep the credential file in each checkout. A second checkout without that file can run the up script to generate new values, then run the loader against the retained volumes. The loader updates the reader role password to the new checkout's value. **The Grafana admin password remains the one initialized in the original Grafana volume**; generating a new file does not rotate it. Run up and load from the same checkout, and rerun the loader when returning to another checkout. Do not remove volumes to reconcile credentials.

Open `http://127.0.0.1:3010/d/mevana-research-evidence` from this machine. Anonymous Viewer access is allowed only because the host bind is loopback. Do not publish or forward the port; real authentication requires a later handoff. The data source uses `research_reader`, which can select research tables and views but cannot write or read other user schemas.

The loader validates the committed seed through `ResearchArtifactStore.Load`, including its pins, before touching the database. It then reapplies the schema and replaces all six research tables in one transaction. Repeating the load leaves identical rows and keeps the original seed load timestamp. A failed validation produces no SQL output or database change.

## Check and stop

```bash
python3 scripts/research/check-dashboard.py
docker compose --env-file infrastructure/research.env -p mevana-research -f infrastructure/docker-compose.research.yml config --quiet
python3 scripts/research/validate-live.py  # while the stack is up
./scripts/research/grafana-down.sh
```

The down script stops and removes only this stack's containers and network. It does not remove volumes or the ignored credential file. Do not use `down --volumes` for routine shutdown.

The dashboard labels Simulated evidence and the roughly eight-minute capture. `76.11%` is whole-universe FixedUniverse coverage; `69.81%` is triangle-only coverage. Missing-state diagnostics are raw probe amounts, not chained routes. Missed and realized results remain unmeasured, and profits from different quote assets are not combined.

The dashboard defaults to the 152-route universe. Coverage presents twelve labeled experiment/policy bars and a twelve-row count table. The selected-experiment panels show separate rejection reasons, policy results, shape coverage and pool-direction probes. Selecting a v1 experiment yields an explicit “Not measured for this experiment” row in the shape and probe tables. Exact profits are displayed without padded decimals, with their quote symbols. Counts have thousands separators and shares have two decimals.
