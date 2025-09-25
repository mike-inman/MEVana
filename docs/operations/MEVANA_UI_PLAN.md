Last updated: 2025-02-14

# MEVana UI – Wallet Integration Plan

## Context & Goals
- **UI stack**: Next.js 15+, React 19, Clerk auth, Tailwind 4, TanStack Query. Lives in a separate repo (`MEVana-UI`).
- **Existing wallet UI**: `src/app/wallets/page.tsx` already handles listing wallets, rotating, backup, testing airdrops, mock transfers, and balance checks.
- **Proxy layer**: Next.js API routes (`src/app/api/wallet`, `api/backup`, `api/balance`, `api/airdrop`, etc.) forward calls to a presumed “wallet-vault” service; they add Clerk auth or fall back to a dev user.
- **New backend capability**: MEVana executor exposes `/admin/wallet/status`, `/admin/wallet/transfer`, `/admin/wallet/airdrop` with Clerk JWT support and the labelled Prometheus gauge.
- **Objective**: reuse the existing wallet UI, point it at the executor, remove mock/synthetic data, and enable real-time updates + automated integration tests.

## Backend alignment (executor vs. UI expectations)
- **Status data**: Executor’s `/admin/wallet/status` returns one active wallet and label map; UI expects multiple tiers, stats, backup info. We need to supply/augment data so the UI isn’t limited to a single wallet.
  - Option A: extend executor admin API to return full wallet inventory (indexed by tier / metadata) and cached balances; Prometheus labelled gauge already supports per-wallet numbers.
  - Option B: stand up the `wallet-vault` service (per existing design) and have executor provide status + control actions while vault tracks metadata. UI proxies to both.*
- **Control actions**: executor now supports status, transfer, airdrop, rotation (via wallet service functions). UI exposes buttons for create, rotate, deactivate, backup—map these to executor endpoints or add matching routes to the admin API.
- **Balance / airdrop**: UI currently calls `/api/balance` (Solana RPC) and `/api/airdrop` directly. We should route these through executor for consistency, respecting test/dev/local flags.

## Real-time visibility
- **Prometheus metric** `mevana_wallet_balance_sol{wallet="label"}` now publishes current balance. Grafana should render this with minimal lag.
- **UI auto-refresh**: wallet page already hits `/api/balance` per wallet; rewire that to executor so every button click reflects instantly.
- **Optional SSE / websockets**: docs (“Execution Monitor Pack”) outline SSE endpoints for opportunities/bundles/fills/wallet; we can reuse that approach for wallet updates if we want push-based refresh.

## Security & auth
- **Clerk integration**: Next.js API routes use `auth()` to require a user; fallback `dev-user-123` should be removed in production and replaced with service account tokens.
- **Executor auth**: supports static admin token + Clerk JWT. We’ll generate short-lived tokens in the UI server-side (e.g., via a service user) to call `/admin/wallet/*` safely.
- **No synthetic data**: disable mock servers and placeholder values once integration is complete. Add a “demo mode” flag if we still want sample dashboards.

## Integration testing
- **Goal**: run UI + executor end-to-end without typing real credentials.
  - Introduce a **test harness** (e.g., Playwright or Vitest + MSW) that spins up the executor admin API (or a local mock) and runs scripted flows (status → transfer → airdrop) with known seeds.
  - Provide **fake service-user tokens** via env vars so tests don’t hit Clerk or require interactive login.
  - Ensure env `NEXT_PUBLIC_SHOW_SYNTHETIC_DATA=false` by default so tests aren’t polluted by mock server responses.

## Action items
1. **Documented configuration** (this file + README update)
 - Env vars for UI: Clerk keys, executor host, admin token/jwt, feature flags to disable mock data.
    - `EXECUTOR_ADMIN_URL` (e.g. `http://149.28.56.125:9898`)
    - `EXECUTOR_ADMIN_TOKEN` (matches executor's `ADMIN_API_TOKEN`)
    - `NEXT_PUBLIC_WALLET_VAULT_URL` can point to executor if the dedicated vault service is not yet deployed.
   - How to run UI alongside executor in dev/prod.
2. **Update Next API routes**
   - `/api/wallet`, `/api/balance`, `/api/airdrop` → call executor admin endpoints.
   - Add `/api/wallet/transfer`, `/api/wallet/rotate`, etc., or reuse existing ones to forward to executor.
3. **Enhance executor admin API**
   - Return full wallet inventory (and backups) so UI’s analytics don’t break.
   - Add rotation/deactivation endpoints if not already exposed.
   - Provide an SSE or polling endpoint for near-real-time updates.
4. **UI adjustments**
   - Replace stubbed transfer call with real integration.
   - Surface real Prometheus values (balance gauge) in wallet dashboard.
   - Remove or clearly label any synthetic/mock data (create toggles or separate routes).
5. **Testing plan**
   - Add automated integration tests hitting executor (scripted sequences using Clerk service tokens).
   - Document how to run tests locally (no manual login) and in CI.
6. **Observability**
   - Expand Grafana dashboards to include requests coming from the UI (button events) + wallet metrics.
   - Add logs/audit UI to show who triggered which wallet action.

## Next steps (immediate)
- Wire `/api/wallet*` routes to executor → reuse existing UI flows.
- Implement Clerk service-user JWT fetcher so both Grafana and UI share the same auth path.
- Remove mock data from the wallet page (or guard with a clear “demo mode” flag).
- Build Playwright/Vitest script that runs a full wallet transfer/airdrop round-trip against a dev instance.
