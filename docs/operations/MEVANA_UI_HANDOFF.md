Last updated: 2025-02-14

# MEVana UI & Executor Wallet Handoff Checklist

Read this first. The items below describe exactly where the UI ↔ executor integration stands and where to continue.

## What’s live now
- `docs/operations/MEVANA_UI_PLAN.md` – master plan for UI integration (env vars, next steps, testing goals). Updated 2025‑02‑14.
- UI (`MEVana-UI` repo) routes wired to the executor admin API:
  - `GET /api/wallet/status` → `/admin/wallet/status`
  - `POST /api/wallet/transfer` → `/admin/wallet/transfer`
  - `POST /api/airdrop` → `/admin/wallet/airdrop`
  - `POST /api/balance` → executor status (no more direct RPC calls)
- Wallet page now reflects real data via the above routes. Transfer & airdrop buttons call the executor; balance auto-refreshes after each action.
- Grafana doc (`ADMIN_CONTROL_PANEL.md`) already describes the executor endpoints and metrics; no synthetic data is shown once env flags are set.

## Environment vars required
Set in the UI environment (e.g., `.env.local`, deployment secrets):
```
NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=...
CLERK_SECRET_KEY=...
EXECUTOR_ADMIN_URL=http://149.28.56.125:9898
EXECUTOR_ADMIN_TOKEN=ops-control-token
NEXT_PUBLIC_ENABLE_SYNTHETIC_DATA=false
```
(Also preserve any existing `NEXT_PUBLIC_WALLET_VAULT_URL` if you’re still hitting the older mock service.)

## Outstanding work
1. Extend executor admin API (or implement wallet-vault) so `/admin/wallet/status` returns full inventory and tier metadata—not just the active wallet.
2. Implement rotate/deactivate/create/backups endpoints (UI buttons currently show 501 “not available yet”).
3. Replace the static admin token with a Clerk service-user JWT flow so both Grafana and the UI share the same short-lived auth.
4. Remove or clearly gate mock servers (check `mock-mev-server.js`, notes under `docs/notes_*`). Default should be real data only.
5. Add integration tests (Playwright/Vitest) that run UI ↔ executor end-to-end without manual login (use mock Clerk tokens/service user ID).
6. Audit the theme/layout structure so future design tweaks propagate globally—current pages mix layout wrappers and Tailwind utility classes.
7. Fix the legacy ESLint errors (`npm run lint` fails due to `any` types, unused imports). Most warnings predate today’s work.

## Where to look in the UI repo
- `src/app/wallets/page.tsx` – main wallet dashboard; add real rotate/deactivate logic here.
- `src/services/wallet-vault.ts` & `src/hooks/useWalletVault.ts` – central API and React Query hook.
- `src/app/api/*` – Next.js API routes bridging to executor (see `executorAdmin.ts`).
- `docs/Execution Monitor Pack` – earlier design notes for SSE/LT endpoints if pushing events from executor.

## Production verification
- Executor metrics: `curl http://127.0.0.1:9904/metrics | grep mevana_wallet_balance_sol` → expect labelled series (`wallet="prod"`).
- Admin API: `curl -H 'Authorization: Bearer <token>' http://149.28.56.125:9898/admin/wallet/status`.
- UI smoke tests: `curl http://localhost:3000/api/wallet/status`, `.../wallet/transfer` (with JSON body).

## Caution flags
- Any UI panel still showing synthetic/mock data must be disabled or clearly labeled. `NEXT_PUBLIC_ENABLE_SYNTHETIC_DATA=false` by default.
- Clerk fallback “dev user” exists for local development; remove or guard it in production.
- Transfer and airdrop success rely on executor’s admin token – confirm the config before running in prod.
- Theme consistency: adjustments belong in `src/app/providers.tsx` and `src/styles/globals.css`; avoid per-page overrides.

## Handoff summary
1. Review `MEVANA_UI_PLAN.md` for the roadmap and env requirements.
2. Use this handoff doc as the jump-off point for the next helper.
3. Continue with backend enhancements (full wallet data, rotation, backups) and Clerk JWT flow, then tackle the testing + theme cleanup.

