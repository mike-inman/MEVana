## SHYFT Validation Overview

Date: 2025-08-19

### Scope
- **Goal**: Capture live Solana DEX swap transactions (Orca/Raydium), fetch authoritative Shyft API views, and publish full, prettified JSON to Kafka for visual verification and future comparison.
- **Live-only**: No mocks, only real data.

### Service
- **Service**: `shyft-validator`
- **Responsibilities**:
  - Consume normalized Orca/Raydium swaps from Kafka
  - Apply sliding window delay to accommodate Shyft API readiness
  - Call Shyft endpoints and publish results to Kafka
  - Provide health server and structured logs

### Environment
- `SHYFT_API_KEY` (required)
- `SHYFT_NETWORK` (default: `mainnet-beta`)
- `KAFKA_BROKER` (default: `localhost:9092` or `kafka:9092` in Docker)
- `HEALTH_PORT` (default: `8086`)
- `BIND_ADDRESS` (default: `0.0.0.0`)

### Kafka Topics
- **Input**
  - `orca-swaps.parsed.v2`: Normalized Orca Whirlpool swaps

- **Outputs** (new)
  - `shyft-full-transactions`: Full Shyft "parsed" endpoint JSON
  - `shyft-full-transactions-raw`: Full Shyft "raw" endpoint JSON
  - `shyft-rpc-gettransaction`: Full JSON-RPC response from `getTransaction`
  - `shyft-swaps-parsed`: Enriched join of our parsed swap + selected fields from Shyft

### Shyft Endpoints Used
- Parsed (REST): `GET /sol/v1/transaction/parsed` with header `x-api-key`
  - Swagger: `https://api.shyft.to/sol/api/explore/#/`
- Raw (REST): `GET /sol/v1/transaction/raw` with header `x-api-key`
  - Swagger: `https://api.shyft.to/sol/api/explore/#/`
- RPC (JSON-RPC): `getTransaction` via `POST https://rpc.shyft.to?api_key=...`
  - Docs: `https://docs.shyft.to/solana/rpc-calls/http/gettransaction`

### Sliding Window Readiness
- Starts at 2s initial delay (configurable), auto-adjusts based on success/failure counts
- Adapts to Geyser > Shyft readiness gap to reduce empty/missing responses

### How to Consume
- Single message (from host):
  - `docker exec kafka kafka-console-consumer --bootstrap-server localhost:9092 --topic shyft-full-transactions --from-beginning --max-messages 1`
  - `docker exec kafka kafka-console-consumer --bootstrap-server localhost:9092 --topic shyft-full-transactions-raw --from-beginning --max-messages 1`
  - `docker exec kafka kafka-console-consumer --bootstrap-server localhost:9092 --topic shyft-rpc-gettransaction --from-beginning --max-messages 1`

### Notes for AI Agents
- Input signature is taken from live swap messages (primary key)
- Data flow is idempotent per signature; publishing uses signature as Kafka key
- Outputs are verbose JSON meant for visual diffs and future automatic validators
- No hardcoded networks; all config via env

### Future Work
- Add price/fee extraction and parity checks vs. internal normalizer
- Extend pool/token discovery and balances to Kafka (prefixed topics)
- Metrics and dashboards (Prometheus/Grafana) alignment


