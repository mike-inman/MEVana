# Low-Latency Arbitrage Strategy (Solana / MEVana)

## Goals & KPIs
- Decision latency: <= 10 ms p50 (Geyser receive → bundle submitted), p99 <= 25 ms
- Profit target: >= $1/min average net PnL (market-dependent)
- Safety: no silent failures; hard net-PnL gating; kill-switch

## Architecture (Hot vs Cold)
- Hot path (latency-critical): Geyser → Normalizer/Extractor → Arb Engine → Jito Bundles
  - In-proc bus or binary Kafka topic; no JSON on hot path
- Cold path (UI/observability): same normalized stream → JSON → Kafka → ui-gateway → ui-web
- Backfills enrich Redis (pool snapshots, mint metadata) used by extractor caches

## Hot Path Design (<= 10 ms)
- Co-locate Normalizer + Arb Engine (same process or shared memory)
- Binary v2 message (prost); JSON only at UI
- Zero-alloc, buffer reuse; avoid base58 strings on hot path
- Local in-memory caches for:
  - Mint decimals → UI amounts
  - Pool snapshots (tick, sqrt_price_x64, fee_bps)
  - Read-through from Redis on miss; async TTL refresh
- Logging: off in hot path; counters only
- Runtime: CPU pinning, tuned Tokio workers, LTO thin + opt-level=3

## Kafka (Low-Latency Mode)
- Producer: linger.ms=0, acks=1, compression=lz4 (or none), idempotence=false, message.timeout.ms=100
- Consumer: fetch.min.bytes=1, fetch.wait.max.ms=1–2, cooperative-sticky assignment
- Same AZ/VLAN; few partitions to reduce cross-broker hops
- Option: bypass Kafka on hot path (in-proc), keep Kafka mirrors for observability

## Arb Engine
- Consume binary v2 with UI amounts & decimal-aware price already computed
- Pre-warmed pool state for slippage checks; small constrained pathfinder
- Decision rules:
  - Net PnL > 0 after DEX fees + Jito tip + compute
  - Slippage/liquidity constraints respected; size capped by depth

## Flash Loans & Packaging
- Same-tx borrow/route/repay via CPI (Solend/Marginfi/Port)
- Single bundle per opportunity; simulate and fail fast on constraints

## Jito Integration (Tip Strategy)
- Private connection, bundle builder + simulator
- Tip sizing model based on expected profit, acceptance loss-rate vs tip, and competition
- Canary mode: tiny notional tests to calibrate curves

## Colocation Plan
- NY data center near a performant validator and Jito relay
- Bare metal, tuned kernel/NIC; standby node for failover

## Observability & SLOs
- Histograms: geyser→extractor, extractor→arb, arb decision time (p50/p90/p99)
- Counters: decode hits, extraction success, DLQ, attempts vs accepted vs filled
- PnL dashboards: gross/net/tips/fees by venue/time; kill-switch on SLO breaches

## Risk & Profitability Controls
- Hard floor: min expected net PnL after tips/fees
- Max tip % of expected profit; adaptive scaler on acceptance feedback
- Venue depth/impact sizing; cooldowns after rejections

## Testing & Rollout
- Devnet/Testnet: strategy + timing validation (no Jito)
- Mainnet canary: tiny notional, measure acceptance/latency/PnL
- CI: extractor fixtures/golden outputs, latency microbenchmarks
- Load test: replay captured raw streams at 1–5× to validate p99

## Data Enrichment (Backfills)
- graphql-backfill: seed Redis with Whirlpool pools and mint metadata
- dex-backfill: maintain snapshots via accounts stream (pools/tick arrays/positions)
- Extractor uses local caches with Redis read-through for accurate UI amounts/prices

## Work Plan & Branches
- Done:
  - feature/ui-v2-toggle-pool: UI v1/v2 toggle, Pool panel scaffold
  - feature/universal-geyser-stream: universal stream (PROGRAM_IDS) + Docker
- Next:
  - feature/normalizer-extract-service: run `tap_orca_extract` producing `*.parsed.v2` with FAST_MODE
  - feature/multi-venue-expansion: add Raydium/Meteora PROGRAM_IDS, verify decoders/extractors
  - feature/depth-slippage-model: maintain pool depth from TickArrays, expose size→price
  - feature/arb-test-mode: dry-run net-PnL and “would trade” logging
  - feature/jito-bundle-scaffold: bundle builder, tip model, canary mode
  - feature/metrics-latency-slos: histograms/dashboards/kill-switch

## Binary v2 Schema (Sketch)
- slot, ts_unix, sig, program_id, pool_id
- token_in_mint, token_out_mint
- amount_in_raw (u128), amount_out_raw (u128), price (f64)
- tick (opt), sqrt_price_x64 (opt), fee_bps
- signers[], success

## Services to add
- normalizer-extract: consumes raw geyser topic; produces `*.parsed.v2` with decimals, optional pool hints
- arb-test-mode: consumes `*.parsed.v2`, computes net-PnL, logs “would trade”

## Execution Budget
- Target p50: 1–5 ms, p99 <= 25 ms
- Weekly review: PnL vs latency trade-offs; adjust venues and tip model
