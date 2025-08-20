FROM docker.io/library/rust:1.83 as builder
WORKDIR /app
COPY ../../ .
RUN cargo build --release -p shyft-raw-price-feed

FROM docker.io/library/debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/shyft-raw-price-feed /usr/local/bin/shyft-raw-price-feed
ENTRYPOINT ["/usr/local/bin/shyft-raw-price-feed"]

