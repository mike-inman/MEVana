# syntax=docker/dockerfile:1

FROM rust:1.83 as builder
RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# Build from workspace root with .dockerignore trimming the context
COPY . .
RUN cargo build --release -p normalizer-service --bin tap_orca_extract --bin depth_builder

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/tap_orca_extract /usr/local/bin/normalizer-extract
COPY --from=builder /app/target/release/depth_builder /usr/local/bin/depth-builder
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/normalizer-extract"]


