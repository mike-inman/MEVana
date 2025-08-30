# syntax=docker/dockerfile:1

FROM rust:1.83 as builder
RUN apt-get update && apt-get install -y cmake build-essential pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/arb-test-mode /usr/local/bin/arb-test-mode
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/arb-test-mode"]
