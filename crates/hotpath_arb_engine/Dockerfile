# Multi-stage build for hotpath_arb_engine

FROM rust:1.81-bookworm AS builder

# System deps for building rdkafka, tonic/protobuf, etc.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev librdkafka-dev \
    protobuf-compiler cmake zlib1g-dev ca-certificates curl git && \
    update-ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache deps first
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Install nightly toolchain for edition2024 dependencies and build target crate
RUN rustup toolchain install nightly && rustup default nightly
RUN cargo +nightly build --release -p hotpath_arb_engine

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates librdkafka1 curl && \
    update-ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/hotpath_arb_engine /usr/local/bin/hotpath_arb_engine

# Default ports (internal). We avoid host mapping to reduce conflicts.
EXPOSE 8088

ENV RUST_LOG=info \
    METRICS_PORT=8088

ENTRYPOINT ["/usr/local/bin/hotpath_arb_engine"]
