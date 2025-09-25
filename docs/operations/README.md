Last updated: 2025-02-14

# Operations Docs Guide

When you pick up this project, read the files below in order:

1. **MEVANA_UI_HANDOFF.md** – current state of the UI/executor wallet integration and next steps.
2. **MEVANA_UI_PLAN.md** – detailed roadmap for aligning the UI, executor admin API, and testing.
3. **ADMIN_CONTROL_PANEL.md** – Grafana/Clerk control panel configuration (same wallet endpoints).

## Now (2025-09-25T03:18+00:00)

- `./scripts/lifecycle/start_system.sh` now auto-loads `.env` + `config/environments/development.env`, runs the idempotent bootstraps (`scripts/deployment/bootstrap_database.sh`, `bootstrap_kafka.sh`), and starts every service—wallet-vault, edge-assembler, arb-engine, executor, hotpath—in paper mode.
- All env samples live under `config/environments/*.env.sample`; `config/environments/development.env` points `WALLET_KEYPAIR_PATH` and `JITO_AUTH_KEYPAIR_PATH` at the repo `infrastructure/keys/...` for local runs.
- Wallet UI is wired to Clerk-issued JWTs. `.env.local` already defines `EXECUTOR_ADMIN_URL=http://127.0.0.1:9898` and `CLERK_EXECUTOR_JWT_TEMPLATE=executor-admin`. Use `pnpm dev` then `pnpm smoke:wallet` (sets `MEVANA_UI_BASE_URL` if needed) to verify rotate/deactivate/transfer against the live executor.
- Before any deploy, run `scripts/deployment/bootstrap_database.sh` and `bootstrap_kafka.sh` (safe to re-run). `release-services.sh` and `start_system.sh` invoke them automatically.
- Staging env doesn’t exist yet—next step is wiring these helpers into the self-hosted runner workflow so `pnpm smoke:wallet` gates prod pushes.
