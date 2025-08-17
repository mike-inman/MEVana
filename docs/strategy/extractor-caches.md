# Extractor Caches (Orca v2)

## Keys & Shapes
- Pools: `dex:pool:{POOL}`
  - mint_a, mint_b, decimals_a, decimals_b, fee_bps, sqrt_price_x64, tick, updated_at, pair[, pair_symbol], updated_slot
- Mints: `dex:mint:{MINT}`
  - decimals[, symbol][, name][, updated_at]

## Behavior
- Local LRU caches for mint meta and pool snapshots
- Read-through from Redis on miss; async TTL refresh (stale-ok)
- Extractor computes:
  - amount_in_ui, amount_out_ui based on decimals
  - price_ui = amount_out_ui / amount_in_ui
- Emits pool fields if present (tick, sqrt_price_x64)

## Failure Modes
- If Redis unavailable or fields missing: fall back to raw u128s; still emit v2 with raw fields
- Log once per interval on cache miss storm; do not block hot path

## Tunables
- LRU capacity per cache (default 1024)
- TTLs (e.g., 60s for pool snapshots, 1h for mint meta)
