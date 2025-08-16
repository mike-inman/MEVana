# syntax=docker/dockerfile:1

FROM rust:1.83 as builder
WORKDIR /app
# Copy minimal manifests to leverage dependency caching
COPY Cargo.toml ./
COPY crates/normalizer-service/Cargo.toml crates/normalizer-service/Cargo.toml
COPY crates/normalizer-runtime/Cargo.toml crates/normalizer-runtime/Cargo.toml
COPY crates/normalizer-codegen/Cargo.toml crates/normalizer-codegen/Cargo.toml
RUN mkdir -p crates/normalizer-service/src crates/normalizer-runtime/src crates/normalizer-codegen/src
# Fetch deps
RUN cargo fetch
# Now copy sources for just the needed crates and inputs
COPY crates/normalizer-service/ crates/normalizer-service/
COPY crates/normalizer-runtime/ crates/normalizer-runtime/
COPY crates/normalizer-codegen/ crates/normalizer-codegen/
COPY idls/ idls/
COPY profiles/ profiles/
RUN cargo build --release -p normalizer-service --bin tap_orca_extract

FROM debian:bookworm-slim as runtime
RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/tap_orca_extract /usr/local/bin/normalizer-extract
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/normalizer-extract"]


