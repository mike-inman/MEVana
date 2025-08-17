# UI Pool Panel

## Gateway API
- GET `/pool/:id` → returns Redis hash `dex:pool:{id}` plus optional mint meta (future)

## UI Wiring
- `SwapsPanel`: on swap click, set selected `pool_id`
- `PoolPanel`: fetch `/pool/:id` and display:
  - tick, sqrt_price_x64, fee_bps, mint_a, mint_b, pair(or symbol)

## Future
- Add mint symbols and vault balances
- Link to live price chart for the pair
