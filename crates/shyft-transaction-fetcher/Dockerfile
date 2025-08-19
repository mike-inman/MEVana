# Multi-stage build for shyft-transaction-fetcher
FROM docker.io/library/rust:1.83 as builder

# Install build dependencies
RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the shyft-transaction-fetcher binary
RUN cargo build --release -p shyft-transaction-fetcher

# Runtime stage
FROM docker.io/library/debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates curl && update-ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/shyft-transaction-fetcher /usr/local/bin/shyft-transaction-fetcher

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/shyft-transaction-fetcher"]

# Default command (can be overridden)
CMD ["--help"]
