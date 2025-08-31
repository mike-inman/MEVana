# Build stage
FROM rust:1.88-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/app

# Copy the entire workspace from the root
COPY . .

# Set DATABASE_URL for sqlx macros (offline mode)
ENV DATABASE_URL=postgresql://user:pass@localhost/db

# Build the binary from the dex-stream-universal crate
RUN cd crates/dex-stream-universal && cargo build --release --bin dex-stream-universal

# Production stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false app

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /usr/src/app/crates/dex-stream-universal/target/release/dex-stream-universal /app/

# Create necessary directories
RUN mkdir -p /app/config && chown -R app:app /app

# Switch to non-root user
USER app

# Expose health check port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://:8080/health || exit 1

# Default command
CMD ["./dex-stream-universal"]




