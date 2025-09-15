# syntax=docker/dockerfile:1

FROM rust:1.83 as builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev cmake protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN rustup toolchain install nightly && rustup default nightly
RUN cargo +nightly build --release -p arb-engine --bin arb-engine

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates librdkafka1 curl && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/arb-engine /usr/local/bin/arb-engine
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/arb-engine"]

