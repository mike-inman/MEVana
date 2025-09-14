# Multi-stage build for MEVana multi-endpoint deployment
FROM rust:1.75-slim-bullseye AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    protobuf-compiler \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build the applications in release mode
RUN cargo build --release \
    --bin hotpath_arb_engine \
    --bin dex-stream-integrated

# Runtime stage - minimal image
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN useradd -r -s /bin/false mevana

# Copy binaries from builder stage
COPY --from=builder /app/target/release/hotpath_arb_engine /usr/local/bin/
COPY --from=builder /app/target/release/dex-stream-integrated /usr/local/bin/

# Set permissions
RUN chown mevana:mevana /usr/local/bin/hotpath_arb_engine /usr/local/bin/dex-stream-integrated

# Switch to app user
USER mevana

# Default command (can be overridden)
CMD ["hotpath_arb_engine"]