## shyft-validator

Date: 2025-08-19

### Summary
`shyft-validator` watches normalized Orca/Raydium swaps, calls Shyft APIs (parsed, raw, and RPC getTransaction), and publishes complete JSON to Kafka for visual inspection and downstream validation.

### Inputs
- Kafka topic: `orca-swaps.parsed.v2`

### Outputs
- `shyft-full-transactions` — Full Shyft parsed JSON
- `shyft-full-transactions-raw` — Full Shyft raw JSON
- `shyft-rpc-gettransaction` — Full Shyft JSON-RPC getTransaction response
- `shyft-swaps-parsed` — Our parsed swap + selected Shyft fields

### Configuration
- `SHYFT_API_KEY` — required
- `SHYFT_NETWORK` — default `mainnet-beta`
- `KAFKA_BROKER` — `kafka:9092` in Docker, or `localhost:9092` locally
- `HEALTH_PORT` — default `8086`
- `BIND_ADDRESS` — default `0.0.0.0`

### Health
- `GET /health` on `${BIND_ADDRESS}:${HEALTH_PORT}`

### Notes
- Sliding window delay auto-tunes to avoid requesting too soon after Geyser emission
- Signature is used as Kafka key for idempotent publishing


