Last updated: 2025-09-15

## MEVana – Status Update (2025-08-20 UTC)

### Summary
- Live-only pipeline extended with a new token registry service and an enriched raw price feed joined to registry data. All networking settings are externalized via env. No mocks, no fallbacks.

### What’s New
- token-registry-service (new crate)
  - Purpose: Authoritative, live-backed token metadata by mint (decimals, symbol, stability tag).
  - Live sources: Solana RPC getAccountInfo (base64) for SPL Mint decimals; known stable mints (USDC/USDT).
  - Cache: Redis read-through with TTL; keys `TOKEN_REGISTRY:{mint}`.
  - API: `GET /v1/token/:mint` → `{ mint, symbol?, decimals?, is_stable, ... }` and `/healthz`.
  - Config: `RPC_URL`, `BIND_ADDRESS`, `REDIS_URL`, `CACHE_TTL_SECS`.
  - Verified: Health OK; USDC returns decimals=6 and is_stable=true.

- shyft-raw-price-feed (enhanced)
  - Source: `shyft-full-transactions-raw` topic (Shyft transaction/raw) → publishes to `prices.latest.raw`.
  - New: Registry HTTP join (optional) via `TOKEN_REGISTRY_URL`; in-process LRU cache to avoid repeated lookups.
  - Output fields now include:
    - `label`, `pair` (symbol-based), `base_symbol`, `quote_symbol`.
    - `base_decimals?`, `quote_decimals?`, `base_is_stable?`, `quote_is_stable?`.
    - `price_usdc`/`price_usdt` when quoted in USDC/USDT.
  - Fallbacks: If registry is unavailable, falls back to Shyft-provided symbols, then local mint→symbol mapping.
  - Config: `KAFKA_BROKER`, `INPUT_TOPIC`, `OUTPUT_TOPIC`, `GROUP_ID`, `TOKEN_REGISTRY_URL`.

### Live-Data Guarantees
- Price feed uses only live Shyft raw transaction data; registry uses live RPC for decimals. No mock data or offline files.

### Ops Notes
- Run registry locally:
```bash
RPC_URL='https://rpc.shyft.to?api_key=YOUR_KEY' \
BIND_ADDRESS='127.0.0.1:8084' \
REDIS_URL='redis://127.0.0.1:6379/0' \
CACHE_TTL_SECS='600' \
cargo run -p token-registry-service
```
- Run price feed locally:
```bash
KAFKA_BROKER=localhost:29092 \
INPUT_TOPIC=shyft-full-transactions-raw \
OUTPUT_TOPIC=prices.latest.raw \
GROUP_ID=shyft-raw-price-feed \
TOKEN_REGISTRY_URL=http://127.0.0.1:8084 \
cargo run -p shyft-raw-price-feed
```
- Verify output:
```bash
docker exec kafka kafka-console-consumer \
  --bootstrap-server localhost:9092 \
  --topic prices.latest.raw \
  --max-messages 5
```

### Direction (Near-Term)
1) Extend registry with optional pre-warm from observed mints; keep live confirmation path. 2) Add Prometheus metrics (cache hits/misses, lookup latency). 3) Optional compacted Kafka topic `token-registry` for downstream consumers. 4) Docker compose entries with correct network naming and health endpoints.

### Compliance
- Networking from env; no hardcoded addresses. Health endpoints present. Build and run outside Docker supported. Live data only.


