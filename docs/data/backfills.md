# Backfills (GraphQL & Accounts)
Last updated: 2025-09-15

## GraphQL Backfill
- Seeds Redis pool snapshots and mint metadata using Shyft GraphQL + REST
- Keys written:
  - `dex:pool:{POOL}`: mint_a/b, decimals_a/b, fee_bps, sqrt_price_x64, tick, updated_at, pair[, pair_symbol]
  - `dex:pair:latest_pool:{mintA/mintB}` → {POOL}
  - `dex:pair_symbol:latest_pool:{SYM_A/SYM_B}` → {POOL} (if both symbols known)
  - `dex:mint:{MINT}`: decimals[, symbol][, name][, updated_at]

## Accounts Backfill (dex-backfill)
- Streams Whirlpool accounts via Geyser and updates `dex:pool:*` (optional tick arrays/positions to streams)
- Recommended to keep snapshots current over time; extractor caches read-through Redis

## Schedule
- Run GraphQL backfill once at startup and once daily
- Run accounts backfill as a service (optional in staging)
