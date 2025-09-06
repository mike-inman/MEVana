# syntax=docker/dockerfile:1
# NO HARDCODED NETWORK ADDRESSES - All configurable via environment variables
# Date: 2025-08-18

FROM rust:1.83 as builder
RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Build from workspace root with .dockerignore trimming the context
COPY . .
RUN cargo build --release -p shyft-pool-depth

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y ca-certificates curl && update-ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/shyft-pool-depth /usr/local/bin/shyft-pool-depth

# Set default environment variables
ENV RUST_LOG=info
ENV HEALTH_PORT=8087
ENV BIND_ADDRESS=0.0.0.0
ENV SHYFT_NETWORK=mainnet-beta
ENV REDIS_URL=redis://redis:6379/0
ENV UPDATE_INTERVAL=30
ENV MAX_POOLS=100
ENV DRY_RUN=false

# Expose health check port
EXPOSE 8087

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://:8087/health || exit 1

ENTRYPOINT ["/usr/local/bin/shyft-pool-depth"]
