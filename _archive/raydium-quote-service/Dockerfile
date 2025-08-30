# Build stage
FROM rust:1.89-slim as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy the raydium-quote-service crate
COPY crates/raydium-quote-service ./raydium-quote-service

# Build the raydium-quote-service
RUN cd raydium-quote-service && cargo build --release

# Runtime stage
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/raydium-quote-service/target/release/raydium-quote-service /app/raydium-quote-service

ENV RUST_LOG=info

CMD ["./raydium-quote-service"]
