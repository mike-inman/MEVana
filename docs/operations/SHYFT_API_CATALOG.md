Last updated: 2025-09-15

## Shyft API Catalog (AI‑friendly)

Date: 2025-08-19

### Swagger Reference
- Primary Swagger: `https://api.shyft.to/sol/api/explore/#/`

### Authentication
- REST (Swagger endpoints): send `x-api-key: <SHYFT_API_KEY>` header
- RPC (JSON-RPC): append `?api_key=<SHYFT_API_KEY>` to the RPC base URL (`https://rpc.shyft.to`)

### Network
- Default: `mainnet-beta` (set via `SHYFT_NETWORK`)

### Transaction APIs (REST)
- `GET /sol/v1/transaction/parsed` — Complete parsed transaction (human-friendly)
- `GET /sol/v1/transaction/raw` — Raw transaction and meta (includes logs, inner instructions)
- `GET /sol/v1/transaction/history` — History for a wallet
- `GET /sol/v1/transaction/status` — Status for a signature

### DeFi / Pools (REST)
- `GET /sol/v1/pool/get_pool` — Get Pool by Address
- `GET /sol/v1/pool/get_pools_by_liquidity_pair` — Pools by token pair
- `GET /sol/v1/pool/get_all_pools_for_token` — All pools for a given token address
- `GET /sol/v1/pool/get_liquidity_details_of_pool` — Liquidity/tick/fee details

### Tokens / Wallets (REST)
- `GET /sol/v1/token/get_balance` — Token balance for owner + mint
- `GET /sol/v1/wallet/balance` — Wallet SOL balance (depending on catalog)
- `GET /sol/v1/wallet/tokens` — All token balances (if available in current catalog)

### RPC (JSON‑RPC)
- Endpoint: `POST https://rpc.shyft.to?api_key=<KEY>`
- `getTransaction` — Full JSON transaction (configurable encoding, version)
  - Parameters: `signature`, config `{ commitment, maxSupportedTransactionVersion, encoding }`
  - Docs: `https://docs.shyft.to/solana/rpc-calls/http/gettransaction`

### Kafka Topics Used in This Project
- Inputs:
  - `orca-swaps.parsed.v2`
- Outputs:
  - `shyft-full-transactions` — Full Shyft parsed JSON (REST)
  - `shyft-full-transactions-raw` — Full Shyft raw JSON (REST)
  - `shyft-rpc-gettransaction` — Full JSON-RPC getTransaction response (RPC)
  - `shyft-swaps-parsed` — Enriched join (our parsing + selected Shyft fields)

### Notes for Agents
- Always prefer `x-api-key` header for REST and `?api_key=` for RPC
- Use signature as Kafka key to keep publishing idempotent
- For newest signatures from Geyser, use sliding delay to avoid 404/empty responses


