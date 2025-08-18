# syntax=docker/dockerfile:1
# NO HARDCODED NETWORK ADDRESSES - All configurable via environment variables
# Date: 2025-08-18

FROM rust:1.83 as builder
RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Build from workspace root with .dockerignore trimming the context
COPY . .
RUN cargo build --release -p shyft-validator

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y ca-certificates curl && update-ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/shyft-validator /usr/local/bin/shyft-validator

# Set default environment variables
ENV RUST_LOG=info
ENV HEALTH_PORT=8086
ENV BIND_ADDRESS=0.0.0.0
ENV SHYFT_NETWORK=mainnet-beta

# Expose health check port
EXPOSE 8086

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://:8086/health || exit 1

ENTRYPOINT ["/usr/local/bin/shyft-validator"]
