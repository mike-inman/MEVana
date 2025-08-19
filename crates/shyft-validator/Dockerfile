# Multi-stage build for enhanced shyft-validator
FROM docker.io/library/rust:1.83 as builder

# Install build dependencies
RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the enhanced shyft-validator binary
RUN cargo build --release -p shyft-validator

# Runtime stage
FROM docker.io/library/debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates curl && update-ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/shyft-validator /usr/local/bin/shyft-validator

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/shyft-validator"]

# Default command (can be overridden)
CMD ["--help"]
