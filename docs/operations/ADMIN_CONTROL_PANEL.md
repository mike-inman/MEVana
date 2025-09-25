Last updated: 2025-09-24

# Admin Control Panel Integration

## Quickstart for the next helper

- Make sure the production `.env` includes `WALLET_LABELS` (e.g. `/opt/MEVana/keys/id.json=prod,~/.config/solana/dev-id.json=dev`) so the new per-wallet Prometheus gauges publish human-readable labels.
- Grafana needs the `volkovlabs-button-panel` plugin and a Clerk server key in the panel’s secure field; use the two-step flow in this doc to mint a JWT and invoke the executor admin endpoints.
- Wallet balance metrics now expose `mevana_wallet_balance_sol{wallet="<label>"}` for every configured wallet. Verify the active wallet shows a non-zero value and the backups are pinned to `0`.
- The executor admin API exposes `/admin/wallet/status`, `/admin/wallet/transfer`, `/admin/wallet/rotate`, `/admin/wallet/deactivate`, and `/admin/wallet/airdrop`; confirm Grafana buttons hit these endpoints with Clerk tokens and review the audit log (`$AUDIT_LOG_PATH`).

This guide describes how to hook Grafana controls into the MEVana admin API using Clerk-issued JWTs, and documents the wallet management endpoints exposed by the executor service.

## Authentication flow

- The executor now accepts either the legacy `ADMIN_API_TOKEN` + optional subject allowlist or a Clerk-issued JWT (`Authorization: Bearer …`).
- Clerk JWT validation is configured through the following environment variables:
  - `CLERK_JWKS_URL` (required to enable Clerk)
  - `CLERK_ISSUER`
  - `CLERK_ALLOWED_AUDIENCES` (comma separated)
  - `CLERK_ALLOWED_ROLES` (comma separated, compared case-insensitively)
  - `CLERK_AUTHORIZED_PARTY` (optional `azp` check)
  - `CLERK_JWKS_CACHE_SECS` (defaults to 300)
- UI services mint short-lived tokens via `CLERK_EXECUTOR_JWT_TEMPLATE` (defaults to `executor-admin`); ensure this template is created in Clerk with the roles listed above.
- Tokens must carry one of the allowed roles and an optional audience. Subjects from Clerk tokens are also checked against `ADMIN_ALLOWED_SUBJECTS` when configured.
- Static tokens remain supported for break-glass scenarios; the server still enforces the dual-auth requirement when both `ADMIN_API_TOKEN` and `ADMIN_ALLOWED_SUBJECTS` are set.

## Wallet metrics with labels

- The executor now emits `mevana_wallet_balance_sol{wallet="<label>"}` for each wallet path defined in `WALLET_KEYPAIR_PATH` / `WALLET_BACKUP_PATHS`.
- Set `WALLET_LABELS` to map keypair paths to friendly labels (format: `/path/to/id.json=prod,~/.config/solana/dev.json=dev`).
- When the active wallet changes (failover or manual switch), the active label reports the balance, and all other configured wallets are held at `0` so Grafana panels stay deterministic.

## Grafana setup quick start

1. Install a button-capable plugin on the Grafana host, e.g. `volkovlabs-button-panel`:
   ```bash
   export GF_INSTALL_PLUGINS="volkovlabs-button-panel"
   docker compose restart grafana
   ```
   Enable unsigned plugins if required: `GF_PLUGINS_ALLOW_LOADING_UNSIGNED_PLUGINS=volkovlabs-button-panel`.
2. Store the Clerk server API key (or a pre-minted service token) in the panel's secure field. Configure the button to:
   - Perform a pre-call to Clerk's `jwt_template` endpoint to mint a short-lived token (≤60 s expiry).
   - Pass the returned JWT in the `Authorization` header to the MEVana admin endpoint.
3. Rate limits are enforced per remote IP (defaults: registry refresh 30/min, wallet actions 30/min, token metadata 120/min). Grafana should retry sparingly and surface rate-limit messages to the operator.

## Admin API endpoints

All responses are JSON. Every request must include either a valid Clerk JWT or whatever legacy headers you still rely on (`x-admin-token` / `x-client-subject`).

### `GET /admin/wallet/status`
Returns the active wallet, balance snapshot, token balances, PnL counters, and wallet health metadata.
```bash
curl -s \
  -H "Authorization: Bearer $JWT" \
  https://<host>:9898/admin/wallet/status | jq
```

### `POST /admin/wallet/transfer`
Transfers SOL from the active wallet to a destination.
```bash
curl -s -X POST \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"destination":"<SOL-address>","amount_sol":0.5,"skip_preflight":false}' \
  https://<host>:9898/admin/wallet/transfer
```
- The service rejects zero/negative amounts and amounts that exhaust the fee buffer.
- The response includes the signature, source/destination, lamports, and recent blockhash.

### `POST /admin/wallet/airdrop`
Requests SOL airdrops for dev/test wallets.
```bash
curl -s -X POST \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"cluster":"devnet","amount_sol":1.0,"wallet":"<optional-alt-wallet>"}' \
  https://<host>:9898/admin/wallet/airdrop
```
- `amount_sol` defaults to `1.0` and is capped at `MAX_AIRDROP_SOL` (5 SOL).
- Supported clusters: `devnet`, `testnet`, `localnet`. Provide `rpc_url` to override the default endpoints. Mainnet requests are rejected.

### Legacy endpoints
Existing admin routes (`/admin/registry/refresh`, `/admin/trading/status|enable|disable`) continue to operate, now sharing the Clerk-aware guard. Pairing Clerk with the legacy token is optional; either mechanism can be used to bootstrap access.

## Grafana panel wiring tips

- Use the plugin's secure config store for the Clerk server key. Never bake JWTs straight into dashboards.
- Chain two requests in the panel: first to Clerk for the JWT, second to the MEVana endpoint with `Authorization: Bearer <token>`.
- Surface response payloads or errors in the panel body so operators can see transfer signatures and status codes without leaving Grafana.
- Record successful actions in your runbook (the executor already emits audit logs tagged with the requestor's subject and auth mode).

## Operational reminders

- Clerk JWKS are cached in-process (default 5 minutes). Adjust `CLERK_JWKS_CACHE_SECS` if you rotate keys more aggressively.
- Static tokens and subject allowlists still work; keep `ADMIN_API_TOKEN` handy for emergency access or for non-interactive automation.
- Audit log entries now capture whether the request authenticated via Clerk or static token, plus any signer subject.
- Wallet transfers and airdrops trigger an immediate balance refresh; expect up to 30 seconds for Prometheus gauges to reflect the new value.
- `POST /admin/wallet/rotate` – manually rotate to the next wallet (or a specified address) for operational failover.
- `POST /admin/wallet/deactivate` – mark a wallet unhealthy and trigger failover if it is the active signer.
