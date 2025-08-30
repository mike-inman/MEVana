# Jupiter Parser Service Dockerfile
FROM rust:1.83 as builder

RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy entire workspace for dependencies
COPY . .

# Build Jupiter parser service
RUN cargo build --release -p jupiter-parser-service

FROM debian:bookworm-slim as runtime

RUN apt-get update && apt-get install -y ca-certificates curl && update-ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/jupiter-parser-service /usr/local/bin/jupiter-parser-service

# Set default environment variables
ENV RUST_LOG=info
ENV SERVICE_PORT=8086
ENV BIND_ADDRESS=0.0.0.0
ENV KAFKA_BROKERS=kafka:9092
ENV KAFKA_TOPIC=dex-transactions
ENV REDIS_URL=redis://redis:6379

# Expose service port
EXPOSE 8086

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://localhost:8086/health || exit 1

ENTRYPOINT ["/usr/local/bin/jupiter-parser-service"]
