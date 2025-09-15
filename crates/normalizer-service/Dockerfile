# syntax=docker/dockerfile:1
# NO HARDCODED NETWORK ADDRESSES - All configurable via environment variables
# Date: 2025-08-18

FROM rust:1.83 as builder
RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Build from workspace root with .dockerignore trimming the context
COPY . .
# Install nightly to support dependencies requiring edition2024
RUN rustup toolchain install nightly && rustup default nightly
RUN cargo +nightly build --release -p normalizer-service --bin tap_orca_extract
RUN cargo +nightly build --release -p normalizer-service --bin tap_orca_debug

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates librdkafka1 curl && update-ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/tap_orca_extract /usr/local/bin/normalizer-service
COPY --from=builder /app/target/release/tap_orca_debug /usr/local/bin/tap_orca_debug

# Set default environment variables
ENV RUST_LOG=info
ENV HEALTH_PORT=8085
ENV BIND_ADDRESS=0.0.0.0
ENV KAFKA_BROKER=kafka:9092
ENV IN_TOPIC=dex-transactions
ENV OUT_TOPIC=orca-swaps.parsed.v2
ENV GROUP_ID=tap-orca-extract
ENV REDIS_URL=redis://redis:6379/0
ENV MINT_DECIMALS_TTL_MS=60000
ENV POOL_SNAPSHOT_TTL_MS=60000
ENV FAST_MODE=1

# Expose health check port
EXPOSE 8085

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://:8085/health || exit 1

ENTRYPOINT ["/usr/local/bin/normalizer-service"]
