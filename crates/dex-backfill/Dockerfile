# syntax=docker/dockerfile:1

FROM rust:1.83 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/pump_fun_grpc_txn_stream_n_parse /usr/local/bin/dex-backfill
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/dex-backfill"]


