# MEVana Roadmap (compressed to ~24h)

## Now → 2h: Orca v2 enrichment
- Local caches (mint decimals, pool snapshots) with Redis read-through
- Emit amount_in_ui, amount_out_ui, price_ui, pool_id, tick_index, sqrt_price_x64
- Latency timing (warn if >25ms)

## 2h → 3h: Fast mode (low-latency)
- FAST_MODE=1 toggles Kafka producer/consumer low-latency flags
- Disable hot-path logging; counters only

## 3h → 5h: Pool endpoint & UI
- ui-gateway GET /pool/:id → Redis `dex:pool:{id}` (+mint meta)
- UI `PoolPanel` loads selected pool from `SwapsPanel`

## 5h → 7h: Backfills warmup
- Run graphql-backfill (seed pools+mints)
- Optional dex-backfill (accounts) to keep snapshots current

## 7h → 9h: Arb test mode
- Arb stub consumes v2, logs “would trade” with size/net PnL (no Jito)
- Guard: skip non-positive net after fees/tips

## 9h → 12h: Jito scaffold (canary)
- Bundle builder stub + tip-policy interface; canary mode logs only

## 12h → 18h: End-to-end test
- Universal stream (Orca, Raydium, Meteora) + extractor (fast) + UI + arb-test
- Record latency histograms + sample “would trade” set

## Next (post-orca parity)
- Raydium/Meteora profiles, comparator diffs in UI
- Binary v2 hot stream (prost) + metrics dashboards + CI/CD pipelines
