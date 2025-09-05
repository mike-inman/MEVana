# Multi-stage Dockerfile for executor-service
# Build stage
FROM rust:1.79-slim as builder
WORKDIR /app
# Install build deps
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates clang && rm -rf /var/lib/apt/lists/*

# Create empty dummy project for caching dependencies
RUN USER=root cargo new --bin placeholder
WORKDIR /app/placeholder
COPY Cargo.toml ../

WORKDIR /app
# Copy full workspace
COPY . .
# Build release binary for account-registry-validate (example) and full workspace
RUN cargo build --release -p executor-service

# Runtime stage
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /srv
COPY --from=builder /app/target/release/account-registry-validate /usr/local/bin/account-registry-validate
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/account-registry-validate"]
# Default args (show help)
CMD ["--help"]
