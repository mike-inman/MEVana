Self‑Hosted Deployment Strategy & Workflows (Production Server)

Last updated: 2025-09-16

Overview
- Goal: Fast, predictable, one‑click server builds and deploys. Build images on the server (Linux) to avoid cross‑arch issues; keep compose + env in a canonical path.
- Where: Self‑hosted GitHub Actions runner on the server executes workflows you trigger from GitHub.
- Canonical paths:
  - Project root: `/opt/MEVana`
  - Compose files: `/opt/MEVana/infrastructure/*.yml`
  - Env files (hotpath): `/opt/MEVana/.env` and `/opt/MEVana/infrastructure/.env`
  - Persisted image tag: `/opt/MEVana/.mevana_tag`
  - Docker network: `mevana_net`

Secrets & Env
- Provide endpoint + token via GitHub Actions secrets (repo → Settings → Secrets and variables → Actions):
  - Accepted: `HOTPATH_ENDPOINT` or `ENDPOINT`
  - Accepted: `HOTPATH_X_TOKEN` or `X_TOKEN`
- Workflows write both `/opt/MEVana/.env` and `/opt/MEVana/infrastructure/.env`:
  - `ENDPOINT=...`
  - `X_TOKEN=...`

Services & Ports
- hotpath‑arb‑engine (metrics): 8088 (Prometheus exporter `/metrics`)
- Prometheus UI: 9090 (`/-/healthy`)
- Grafana UI: 3000 (`/api/health`)
- Kafka UI: 8080 (metrics at `/actuator/prometheus`)

Prometheus Scraping
- File: `infrastructure/prometheus.core.yml`
- Hotpath: `hotpath-arb-engine:8088` (path `/metrics`)
- Kafka UI: `kafka-ui:8080` (path `/actuator/prometheus`)

Grafana Dashboards
- Canonical: `infrastructure/grafana/dashboards/` (provisioned into container)
- Hygiene blocks any `monitoring/grafana/dashboards/**` or top‑level `dashboards/` files.

Workflows (GitHub → Actions)
- Build and Deploy (Self‑Hosted)
  - Always builds these images locally on the server: hotpath, normalizer, discovery, executor
  - Rsyncs `infrastructure/` → `/opt/MEVana/infrastructure`
  - Writes env files and persists tag to `/opt/MEVana/.mevana_tag`
  - Runs compose from `/opt/MEVana`

- Restart (Self‑Hosted)
  - Runs compose from `/opt/MEVana`
  - Re‑uses `/opt/MEVana/.mevana_tag` (avoids `:dev` pulls)
  - Input: `services` (e.g., `prometheus`, `hotpath`, etc.)

- Diagnostics (Self‑Hosted)
  - Runs compose ps and tails logs for selected services
  - HTTP checks (Grafana/Prometheus/Hotpath)
  - Prints `/opt/MEVana/.env`, `/opt/MEVana/infrastructure/.env` (redacted) and the hotpath container env (redacted)
  - Prints `/opt/MEVana/.mevana_tag`

- Cargo Check (Self‑Hosted)
  - Installs build prerequisites and runs `cargo +nightly check --workspace` on the server
  - Use to surface compile errors fast before building images

Server Expectations
- Docker + docker compose installed
- Runner service installed at `/opt/actions-runner` and online (GitHub → Settings → Actions → Runners)
- Docker network `mevana_net` exists (workflow creates if missing)

Fast Path (MVP)
1) Run “Cargo Check (Self‑Hosted)” until green.
2) Run “Build and Deploy (Self‑Hosted)” (no inputs).
3) If a service restarts, run “Diagnostics (Self‑Hosted)” and inspect:
   - Env values present
   - container logs (auth/connectivity)
   - Prometheus targets show Up

Guardrails for Contributors & AI Assistants
- Must follow file placement rules (hygiene enforced):
  - Dashboards only in `infrastructure/grafana/dashboards/`
  - Do not create new top‑level directories without approval
  - No archives/OS files in repo
- Use workflows; do not hand‑edit server state except for verified hotfixes.
- For deviations (“color outside the lines”):
  - Open an Issue or PR comment summarizing the reason, scope, and rollback plan
  - Wait for explicit approval before structural changes

Required Reading for New Helpers
- `AGENTS.md` — repo rules and placement
- `docs/ai/ONBOARDING_AI.md` — 2‑minute orientation
- `docs/operations/SELF_HOSTED_DEPLOYMENT_GUIDE.md` (this)
- `docs/DOCUMENTATION_INDEX.md` — critical runbooks and quick links
- `docs/development/RUST_STYLE.md` — Rust code conventions

Pre‑flight Checklist (local or server)
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `bash scripts/ci/repo_hygiene.sh`

Common Issues & Fixes
- Exec format error (Restarting 255): Build images locally on the server via the self‑hosted deploy; avoid host‑built binaries.
- 404 from Prometheus endpoint link: Link points to the internal Docker address; use Prometheus query/targets instead.
- Kafka UI metrics down: Ensure scrape path is `/actuator/prometheus`; restart Prometheus.
- Env not picked up: Re‑run “Build and Deploy (Self‑Hosted)” so env files are rewritten to `/opt/MEVana/.env` and `/opt/MEVana/infrastructure/.env`.

