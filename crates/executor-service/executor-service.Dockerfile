# Multi-stage Dockerfile for executor-service

# Builder stage with nightly toolchain (needed for edition2024 deps)
FROM rust:1.83-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates clang cmake build-essential protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

# Copy full workspace
COPY . .

# Install nightly and build package
RUN rustup toolchain install nightly && rustup default nightly
RUN cargo +nightly build --release -p executor-service

# Runtime stage
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* && update-ca-certificates
WORKDIR /srv
COPY --from=builder /app/target/release/executor-service /usr/local/bin/executor-service
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/executor-service"]
