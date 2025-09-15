# syntax=docker/dockerfile:1

FROM rust:1.83 as builder
RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
# Install nightly to support dependencies requiring edition2024 and build only this crate
RUN rustup toolchain install nightly && rustup default nightly
RUN cargo +nightly build --release -p discovery-service

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates librdkafka1 curl && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/discovery-service /usr/local/bin/discovery-service
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/discovery-service"]
