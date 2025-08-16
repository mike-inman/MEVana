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

## Immediate Next Items (in progress)
- Add and run `normalizer-extract` (bin `tap_orca_extract`) as a service:
  - IN_TOPIC: raw geyser topic from `dex-stream-universal`
  - OUT_TOPIC: `*.parsed.v2` normalized swaps (JSON initially)
  - FAST_MODE=1, counters-only logging, latency histograms
- Add and run `price-aggregator` as a service:
  - IN_TOPIC: `*.parsed.v2` normalized swaps
  - OUT_TOPIC: `prices.latest` (rolling price per pair/venue)
- Expand `PROGRAM_IDS` to include Raydium and Meteora; verify decoders/extractors
- Depth/slippage model:
  - Maintain per-pool depth snapshots from TickArrays/Positions → Redis `dex:pool_depth:{pool}`
  - Provide size→expected price function for sizing and slippage checks
- Arb test mode service:
  - Consume `*.parsed.v2`, compute net-PnL (DEX fees + tip=0 canary), log "would trade", skip non-positive
- Observability & safety:
  - Add histograms geyser→extractor and extractor→arb-test; counters for decodes/errors
  - Circuit breaker / kill-switch for arb engine
- Binary hot path: move from JSON to prost-encoded v2 structs end-to-end

## Completed Today
- `dex-stream-universal`: FAST_MODE enabled; no Kafka key on hot path; Dockerized and running on `mevana-network` against internal Kafka
- `graphql-backfill`: Dockerized service added (one-shot)
- `dex-backfill`: Dockerized, continuous updates; IDLs mounted; running
- `normalizer-extract` (bin `tap_orca_extract`): Dockerized and running; Redis read-through caches; producing `orca-swaps.parsed.v2`
- `price-aggregator`: Dockerized and running; consuming parsed swaps and emitting `prices.latest`
- `depth-builder` first cut: computes p0_ui and heuristic size→price quotes; writes to `dex:pool_depth:{pool}`
- `ui-gateway` depth API: `GET /depth/:pool` returns `dex:pool_depth:{pool}` snapshot for UI/arb-test
- `arb-test-mode`: Dockerized and running; consuming `orca-swaps.parsed.v2`, computing net-PnL, logging "WOULD TRADE" opportunities
