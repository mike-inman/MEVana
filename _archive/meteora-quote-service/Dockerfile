# Build stage
FROM rust:1.89-slim as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy the meteora-quote-service crate
COPY crates/meteora-quote-service ./meteora-quote-service

# Build the meteora-quote-service
RUN cd meteora-quote-service && cargo build --release

# Runtime stage
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/meteora-quote-service/target/release/meteora-quote-service /app/meteora-quote-service

ENV RUST_LOG=info

CMD ["./meteora-quote-service"]
