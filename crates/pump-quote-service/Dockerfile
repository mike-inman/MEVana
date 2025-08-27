# Build stage
FROM rust:1.89-slim as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy workspace dependencies first
COPY crates/shared-core ../shared-core
COPY crates/pumpfun ../pumpfun

# Copy the pump-quote-service crate
COPY crates/pump-quote-service ./pump-quote-service

# Build the pump-quote-service
RUN cd pump-quote-service && cargo build --release

# Runtime stage
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/pump-quote-service/target/release/pump-quote-service /app/pump-quote-service

ENV RUST_LOG=info

CMD ["./pump-quote-service"]
