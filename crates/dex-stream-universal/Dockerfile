# syntax=docker/dockerfile:1

FROM rust:1.79 as builder
WORKDIR /app
# Build only this crate (small context)
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/dex-stream-universal /usr/local/bin/dex-stream-universal
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/dex-stream-universal"]

