# Multi-stage build for MEVana multi-endpoint deployment
FROM rust:1.89-slim-bookworm AS builder

# Install system dependencies including complete build toolchain
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    protobuf-compiler \
    ca-certificates \
    perl \
    perl-base \
    make \
    gcc \
    g++ \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build the applications in release mode - core production services
RUN cargo build --release \
    --bin hotpath_arb_engine

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

# Set permissions
RUN chown mevana:mevana /usr/local/bin/hotpath_arb_engine

# Switch to app user
USER mevana

# Default command (can be overridden)
CMD ["hotpath_arb_engine"]