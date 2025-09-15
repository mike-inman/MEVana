Last updated: 2025-09-15

## Active components (2025-08-27)

- dex-stream-universal (8084): primary Geyser stream (Jupiter included via PROGRAM_IDS when needed)
- whirlpool-tx-stream (8091): TX-only stream for Whirlpool using slim Dockerfile; produces to `dex-transactions`
- jupiter-parser-service: running in pass-through mode (no IDL-based decode yet); emits combined/swap/limit topics
- shyft-pool-depth: seeds Redis with pool summaries (used later for quotes)
- Postgres/Kafka/Redis/Prometheus/Grafana: infrastructure baseline

## Near-term essentials for arbitrage signals

- Enable Jupiter decoding (use curated IDLs) and emit enriched fields (signature, accounts, ix data)
- Implement whirlpool-state-streamer (pool + adjacent tick arrays) → Redis
- Finish whirlpool-quote-surface (consume Redis, compute quotes via `whirlpool-core`, publish `quotes.whirlpool.v1`)
- Cross-check sampler vs Shyft (<0.05% mismatch) for Jupiter + Whirlpool
- Minimal arbitrage detector: compare cross-DEX quotes (Jupiter route vs Whirlpool quote); emit signal topic

## Notes on deprecation/clutter

- Keep legacy stream_and_parse_whirlpool_account_updates for reference; new path replaces it with state streamer + `whirlpool-core`
- Keep IDL registry synced but prefer curated IDLs for stability

## Current Kafka topics of interest

- dex-transactions: raw TXs (filtered per service)
- jupiter.combined / jupiter.swap / jupiter.limit: parser outputs (pass-through pending decode)
- quotes.whirlpool.v1: planned output from quote surface


