# Streaming and parsing Whirlpool accounts using gRPC

This project leverages Solana's gRPC streaming capabilities to efficiently monitor Whirlpool account data. By utilizing the IDL for parsing, it enables real-time analysis and insights into Orca Whirlpool activities on the Solana blockchain.

```
$ cargo run -- --endpoint <endpoint> --x-token <token>
```

![screenshot](assets/usage-screenshot.png?raw=true "Screenshot")

## Docs
Shyft Website: [https://shyft.to/#solana-grpc-streaming-service]  
Shyft gRPC Docs: [https://docs.shyft.to/solana-fast-grpc/grpc-docs]



X_TOKEN=efff2d23-97ae-4c5f-8b8b-9a6265001189


cd crates/dex-backfill
ENDPOINT="https://grpc.ny.shyft.to" \
X_TOKEN="efff2d23-97ae-4c5f-8b8b-9a6265001189" \
REDIS_URL="redis://127.0.0.1:6379/0" \
SHYFT_NETWORK="mainnet-beta" \
cargo run -p dex-backfill --bin dex-backfill -- \
  --include-tickarrays \
  --include-positions \
  --from-slot 361450650
  
# Optional: --from-slot 361000000 or --owner-only
