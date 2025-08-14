# =====================
# Stage 1: Builder
# =====================
FROM rust:1.82 as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    cmake pkg-config libssl-dev libsasl2-dev zlib1g-dev git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo manifests and dependency crates
COPY Cargo.toml Cargo.lock ./
COPY parsers ./parsers
COPY src ./src
COPY create-topics.sh /app/create-topics.sh

# Pre-fetch and build in release mode
RUN cargo fetch
RUN cargo build --release

# =====================
# Stage 2: Runtime
# =====================
# FROM debian:bullseye-slim
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /app/target/release/orca /app/orca

# Add Prometheus metrics endpoint port
EXPOSE 9090

# Add Kafka topic auto-create script (optional)
COPY --from=builder /app/create-topics.sh /app/create-topics.sh
RUN chmod +x /app/create-topics.sh

# Healthcheck
HEALTHCHECK --interval=30s --timeout=5s \
  CMD pgrep orca || exit 1

# Set environment variables
ENV RUST_LOG=info

# Entrypoint
CMD ["./orca"]
